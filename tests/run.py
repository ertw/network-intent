#!/usr/bin/env python3
"""Independent standard-library acceptance tests for the public netc CLI.

Run from any directory with ``python3 tests/run.py`` after building netc.
Tests use only public CLI behavior; no implementation modules are imported.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests" / "fixtures"
HOME = ROOT / "examples" / "home.net"
MINIMAL = (FIXTURES / "minimal.net").read_text()
DEPENDENCIES = (FIXTURES / "dependencies.net").read_text()
NETC = Path(os.environ.get("NETC", str(ROOT / "netc")))


def uci_sections(content):
    """Read generated UCI syntax for assertions about intended behavior."""
    sections = []
    current = None
    for line in content.splitlines():
        words = shlex.split(line, comments=True)
        if not words:
            continue
        if words[0] == "config":
            current = {"type": words[1], "name": words[2] if len(words) > 2 else None, "options": {}, "lists": {}}
            sections.append(current)
        elif words[0] == "option" and len(words) == 3 and current is not None:
            if words[1] in current["options"]:
                raise AssertionError(f"Duplicate generated option: {line}")
            current["options"][words[1]] = words[2]
        elif words[0] == "list" and len(words) == 3 and current is not None:
            current["lists"].setdefault(words[1], []).append(words[2])
        else:
            raise AssertionError(f"Unexpected UCI statement: {line}")
    return sections


class NetcAcceptance(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temporary = tempfile.TemporaryDirectory(prefix="netc-acceptance-")
        cls.directory = Path(cls.temporary.name)

    @classmethod
    def tearDownClass(cls):
        cls.temporary.cleanup()

    def source(self, content: str, name: str = "case.net") -> Path:
        path = self.directory / name
        path.write_text(content)
        return path

    def invoke(self, *args, timeout=30):
        result = subprocess.run(
            [str(NETC), *(str(arg) for arg in args)],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
        )
        self.assertNotIn("Exception", result.stderr, result.stderr)
        return result

    def succeed(self, *args):
        result = self.invoke(*args)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return result.stdout

    def check_json(self, source: str | Path, *, valid: bool):
        path = source if isinstance(source, Path) else self.source(source)
        result = self.invoke("check", path, "--format", "json")
        try:
            body = json.loads(result.stdout)
        except json.JSONDecodeError:
            self.fail("Expected clean JSON stdout: " + repr(result.stdout + result.stderr))
        self.assertIsInstance(body, dict)
        self.assertIn("diagnostics", body)
        self.assertIsInstance(body["diagnostics"], list)
        if valid:
            self.assertEqual(result.returncode, 0, body)
            self.assertEqual(body["diagnostics"], [], body)
        else:
            self.assertNotEqual(result.returncode, 0, body)
            self.assertTrue(body["diagnostics"], body)
            for diagnostic in body["diagnostics"]:
                self.assertRegex(diagnostic["code"], r"^[a-z][a-z0-9.-]+$")
                self.assertEqual(diagnostic["severity"], "error")
                self.assertTrue(diagnostic["title"])
                span = diagnostic["primarySpan"]
                self.assertEqual(Path(span["file"]), path)
                self.assertGreaterEqual(span["line"], 1)
                self.assertGreaterEqual(span["column"], 1)
                self.assertGreaterEqual(span["endLine"], span["line"])
                self.assertIsInstance(diagnostic["related"], list)
        return body

    def reject(self, content, code=None, marker=None, related=False):
        body = self.check_json(content, valid=False)
        diagnostics = body["diagnostics"]
        if code:
            matched = [item for item in diagnostics if item["code"] == code]
            self.assertTrue(matched, f"Expected {code}: {diagnostics}")
        else:
            matched = diagnostics
        if marker:
            line = next(i for i, text in enumerate(content.splitlines(), 1) if marker in text)
            self.assertTrue(
                any(item["primarySpan"]["line"] == line for item in matched),
                f"Expected diagnostic at line {line}: {matched}",
            )
        if related:
            self.assertTrue(any(item["related"] for item in matched), matched)
        return diagnostics

    def test_home_language_2_0(self):
        self.check_json(HOME, valid=True)

    def test_json_export_has_typed_references_and_assurance(self):
        body = self.check_json(HOME, valid=True)
        self.assertEqual(body["languageVersion"], "2.0")
        self.assertEqual(body["observed"], "unknown")
        model = body["model"]
        self.assertEqual(model["state"], "Desired")
        self.assertEqual(model["exportVersion"], "2.0")
        vlan_ids = {vlan["id"] for vlan in model["vlans"]}
        for host in model["hosts"]:
            self.assertIs(type(host["vlanRef"]), int)
            self.assertIn(host["vlanRef"], vlan_ids)
        for device in model["devices"]:
            for port in device["ports"]:
                self.assertIs(type(port["ownerRef"]), int)
                self.assertEqual(port["ownerRef"], device["id"])
                self.assertTrue(set(port["vlanRefs"]) <= vlan_ids)
        self.assertIsInstance(model["certificates"]["serviceOrder"], list)
        self.assertIsInstance(model["certificates"]["routeOrder"], list)

    def test_minimal_vertical_slice(self):
        self.check_json(MINIMAL, valid=True)
        for target in ("core", "gateway"):
            output = self.succeed("compile", FIXTURES / "minimal.net", "--target", target)
            if target == "core":
                self.assertIn("vlan 20", output)
                self.assertIn("name servers", output)
            else:
                self.assertIn("10.0.20.1", output)
                self.assertIn("10.0.20.10", output)

    def test_explicit_supported_version_required(self):
        self.reject(MINIMAL.replace("network-language 2.0\n", ""))
        self.reject(MINIMAL.replace("network-language 2.0", "network-language 99.0"))

    def test_unknown_syntax_is_rejected(self):
        self.reject(MINIMAL.replace("domain home.arpa", "mysterious unsafe-setting"))
        self.reject(MINIMAL + "unexpected trailing tokens\n")
        self.reject(MINIMAL[:-2])

    def test_vlan_boundaries(self):
        for number in (1, 4094):
            with self.subTest(valid_vlan=number):
                self.check_json(MINIMAL.replace("vlan servers 20", f"vlan servers {number}"), valid=True)
        for number in (0, 4095, 999999):
            with self.subTest(invalid_vlan=number):
                self.reject(MINIMAL.replace("vlan servers 20", f"vlan servers {number}"), "vlan.invalid-id", marker=f"vlan servers {number}")

    def test_invalid_ipv4_and_prefix(self):
        for prefix, code in (("10.0.256.0/24", "address.invalid-ipv4"), ("10.0.20.0/33", "address.invalid-prefix"), ("10.0.20/24", "address.invalid-ipv4"), ("10.0.20.1/24", "address.noncanonical-prefix")):
            with self.subTest(prefix=prefix):
                self.reject(MINIMAL.replace("10.0.20.0/24", prefix), code, marker="subnet ")

    def test_relative_address_crosses_octet(self):
        source = MINIMAL.replace("10.0.20.0/24", "10.0.20.0/23").replace("host nas +10", "host nas +300")
        self.check_json(source, valid=True)
        self.assertIn("10.0.21.44", self.succeed("docs", self.source(source)))

    def test_relative_address_uses_network_base_for_varied_prefixes(self):
        cases = (
            ("10.0.20.128/25", 10, "10.0.20.138"),
            ("10.0.0.0/16", 257, "10.0.1.1"),
            ("10.0.20.0/23", 510, "10.0.21.254"),
        )
        for prefix, offset, address in cases:
            with self.subTest(prefix=prefix, offset=offset):
                source = MINIMAL.replace("10.0.20.0/24", prefix).replace("host nas +10", f"host nas +{offset}")
                self.check_json(source, valid=True)
                self.assertIn(address, self.succeed("docs", self.source(source)))

    def test_relative_address_outside_subnet(self):
        self.reject(MINIMAL.replace("host nas +10", "host nas +256"), "address.offset-outside-prefix", marker="host nas")

    def test_reserved_network_and_broadcast_addresses(self):
        for offset, code in ((0, "address.network-address"), (255, "address.broadcast-address")):
            with self.subTest(offset=offset):
                self.reject(MINIMAL.replace("host nas +10", f"host nas +{offset}"), code, marker="host nas")

    def test_absolute_assignment_membership(self):
        self.check_json(MINIMAL.replace("host nas +10", "host nas 10.0.20.10"), valid=True)
        self.reject(MINIMAL.replace("host nas +10", "host nas 10.0.21.10"), "address.outside-prefix", marker="host nas")

    def test_static_collision_has_related_declaration(self):
        source = MINIMAL.replace("host nas +10", "host nas +10\n    host duplicate +10")
        self.reject(source, marker="host duplicate", related=True)

    def test_gateway_collision(self):
        self.reject(MINIMAL.replace("host nas +10", "host nas +1"), marker="host nas")

    def test_dhcp_ranges(self):
        for pool in ("+0 .. +50", "+10 .. +50", "+100 .. +256", "+199 .. +100", "+1 .. +50"):
            with self.subTest(pool=pool):
                self.reject(MINIMAL.replace("gateway +1", f"gateway +1\n    dhcp {pool}"))
        source = MINIMAL.replace("gateway +1", "gateway +1\n    dhcp +100 .. +199")
        self.check_json(source, valid=True)
        output = self.succeed("compile", self.source(source), "--target", "gateway")
        self.assertIn("100", output)

    def test_overlapping_prefixes_have_related_declaration(self):
        source = MINIMAL.replace("  switch core", "  vlan conflict 30 {\n    subnet 10.0.20.128/25\n    gateway +1\n  }\n  switch core")
        self.reject(source, related=True)

    def test_unknown_vlan_reference(self):
        self.reject(MINIMAL.replace("access servers", "access missing", 1), marker="access missing")

    def test_duplicate_vlan_name_or_id(self):
        for declaration in ("vlan servers 30", "vlan other 20"):
            with self.subTest(declaration=declaration):
                source = MINIMAL.replace("  switch core", f"  {declaration} {{\n    subnet 10.0.30.0/24\n  }}\n  switch core")
                self.reject(source)

    def test_duplicate_device_port_and_host_names(self):
        cases = (
            MINIMAL.replace("router gateway", "router core"),
            MINIMAL.replace("port lan2 { access servers }", "port lan2 { access servers }\n    port lan2 { access servers }"),
            MINIMAL.replace("host nas +10", "host nas +10\n    host nas +11"),
        )
        for source in cases:
            with self.subTest(source=source):
                self.reject(source)

    def test_missing_required_subnet(self):
        self.reject(MINIMAL.replace("    subnet 10.0.20.0/24\n", ""))

    def test_conflicting_port_modes(self):
        source = MINIMAL.replace("port Gi1/0/2 { access servers }", "port Gi1/0/2 {\n      access servers\n      trunk servers\n    }")
        self.reject(source)

    def test_missing_port_self_link_and_incompatible_link(self):
        for endpoint in ("gateway.missing", "core.Gi1/0/2"):
            with self.subTest(endpoint=endpoint):
                source = MINIMAL.replace("port Gi1/0/2 { access servers }", "port Gi1/0/2 {\n      access servers\n      connect " + endpoint + "\n    }")
                self.reject(source, marker="connect ")
        source = HOME.read_text().replace("trunk trusted servers iot\n      connect gateway.lan4", "access trusted\n      connect gateway.lan4")
        self.reject(source)

    def test_multiple_peers_rejected(self):
        source = MINIMAL.replace("port Gi1/0/2 { access servers }", "port Gi1/0/2 {\n      access servers\n      connect gateway.lan2\n    }\n    port Gi1/0/3 {\n      access servers\n      connect gateway.lan2\n    }")
        self.reject(source)

    def test_reciprocal_physical_links_normalize(self):
        source = MINIMAL.replace("port Gi1/0/2 { access servers }", "port Gi1/0/2 {\n      access servers\n      connect gateway.lan2\n    }")
        source = source.replace("port lan2 { access servers }", "port lan2 {\n      access servers\n      connect core.Gi1/0/2\n    }")
        self.check_json(source, valid=True)

    def test_physical_cycles_are_valid(self):
        source = """network-language 2.0
network ring {
  vlan servers 20 { subnet 10.0.20.0/24 }
  switch a {
    driver cisco-ios
    port p1 { access servers
      connect b.p1 }
    port p2 { access servers }
  }
  switch b {
    driver cisco-ios
    port p1 { access servers }
    port p2 { access servers
      connect c.p1 }
  }
  switch c {
    driver cisco-ios
    port p1 { access servers }
    port p2 { access servers
      connect a.p2 }
  }
}
"""
        self.check_json(source, valid=True)

    def test_unknown_policy_endpoint_and_service(self):
        source = HOME.read_text()
        self.reject(source.replace("iot -> gateway allow dns, ntp", "iot -> absent allow dns"), marker="iot -> absent")
        self.reject(source.replace("iot -> gateway allow dns, ntp", "iot -> gateway allow absent"), marker="iot -> gateway allow absent")

    def test_policy_service_lists_require_commas(self):
        source = HOME.read_text()
        for services in ("dns ntp", "dns,", ",dns", "dns,,ntp"):
            with self.subTest(services=services):
                self.reject(source.replace("allow dns, ntp", "allow " + services), "policy.invalid-service-list")
        self.check_json(source.replace("allow dns, ntp", "allow dns,ntp"), valid=True)

    def test_dhcp_declaration_rejects_gateway_control_denial(self):
        source = HOME.read_text()
        for policy in ("iot -> gateway deny", "iot -> gateway deny dns"):
            with self.subTest(policy=policy):
                self.reject(source.replace("iot -> gateway allow dns, ntp", policy), "policy.dhcp-control-conflict", marker=policy)
        self.check_json(source.replace("iot -> gateway allow dns, ntp", "iot -> gateway deny ntp"), valid=True)
        alias = source.replace("  policy {", "  service custom-dns { tcp 53 }\n  policy {")
        self.reject(alias.replace("iot -> gateway allow dns, ntp", "iot -> gateway deny custom-dns"), "policy.dhcp-control-conflict")

    def test_gateway_requires_a_routing_owner(self):
        self.reject(MINIMAL.replace("router gateway", "switch gateway"), "network.missing-routing-owner")

    def test_service_transport_port_bounds(self):
        for port in (0, 65536):
            with self.subTest(port=port):
                self.reject(DEPENDENCIES.replace("tcp 5432", f"tcp {port}"), marker=f"tcp {port}")

    def test_routes_and_service_dependencies(self):
        self.check_json(DEPENDENCIES, valid=True)
        output = self.succeed("compile", self.source(DEPENDENCIES), "--target", "gateway")
        self.assertIn("10.50.0.0", output)
        self.assertIn("10.0.20.2", output)

    def test_route_dependency_cycle_has_witness(self):
        source = DEPENDENCIES.replace("metric 10", "metric 10\n    depends remote")
        diagnostics = self.reject(source, "routing.dependency-cycle")
        text = json.dumps(diagnostics)
        self.assertIn("branch", text)
        self.assertIn("remote", text)

    def test_service_dependency_cycle_has_witness(self):
        source = DEPENDENCIES.replace("service database { tcp 5432 }", "service database {\n    tcp 5432\n    depends api\n  }")
        diagnostics = self.reject(source, "service.dependency-cycle")
        text = json.dumps(diagnostics)
        self.assertIn("api", text)
        self.assertIn("database", text)

    def test_unknown_dependency_and_off_link_next_hop(self):
        self.reject(DEPENDENCIES.replace("depends branch", "depends absent"))
        self.reject(DEPENDENCIES.replace("via 10.0.20.2", "via 192.0.2.2"))

    def test_capability_failure_is_explicit(self):
        result = self.invoke("compile", HOME, "--target", "gateway", "--max-tagged-vlans", "2")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("backend.capability-mismatch", result.stdout + result.stderr)
        self.assertIn("lan4", result.stdout + result.stderr)

    def test_unknown_driver_is_not_silently_accepted(self):
        source = MINIMAL.replace("driver openwrt", "driver unsupported-vendor")
        result = self.invoke("compile", self.source(source), "--target", "gateway")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("backend.", result.stdout + result.stderr)

    def test_target_specific_name_and_interface_constraints(self):
        for length, valid in ((32, True), (33, False)):
            with self.subTest(cisco_vlan_length=length):
                source = MINIMAL.replace("servers", "v" * length)
                self.check_json(source, valid=True)
                result = self.invoke("compile", self.source(source), "--target", "core")
                self.assertEqual(result.returncode == 0, valid, result.stdout + result.stderr)
                if not valid:
                    self.assertIn("backend.capability-mismatch", result.stdout + result.stderr)
        for label, valid in (("a" * 15, True), ("a" * 16, False), ("lan/2", False), (".", False), ("..", False)):
            with self.subTest(openwrt_label=label):
                source = MINIMAL.replace("port lan2", "port " + label)
                result = self.invoke("compile", self.source(source), "--target", "gateway")
                self.assertEqual(result.returncode == 0, valid, result.stdout + result.stderr)
                if not valid:
                    self.assertIn("backend.capability-mismatch", result.stdout + result.stderr)
        result = self.invoke("compile", self.source(MINIMAL.replace("Gi1/0/2", "Unknown0")), "--target", "core")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("backend.capability-mismatch", result.stdout + result.stderr)

    def test_dhcp_aggregate_capacity_is_declared_and_checked(self):
        source = MINIMAL.replace("10.0.20.0/24", "10.0.0.0/15").replace("gateway +1", "gateway +1\n    dhcp +100 .. +65634")
        self.check_json(source, valid=True)
        body = json.loads(self.succeed("compile", self.source(source), "--target", "gateway", "--format", "json"))
        dhcp = next(file["content"] for file in body["targets"][0]["files"] if file["path"] == "/etc/config/dhcp")
        dnsmasq = next(section["options"] for section in uci_sections(dhcp) if section["type"] == "dnsmasq")
        self.assertEqual(dnsmasq["dhcpleasemax"], "65535")
        too_large = source.replace("+65634", "+65635")
        self.check_json(too_large, valid=True)
        result = self.invoke("compile", self.source(too_large), "--target", "gateway")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("backend.capability-mismatch", result.stdout + result.stderr)

    def test_reviewed_backend_goldens(self):
        body = json.loads(self.succeed("compile", HOME, "--all", "--format", "json"))
        expected = {
            ("gateway", "/etc/config/network"): "gateway.network.uci",
            ("gateway", "/etc/config/dhcp"): "gateway.dhcp.uci",
            ("gateway", "/etc/config/firewall"): "gateway.firewall.uci",
            ("core", "config.ios"): "core.config.ios",
        }
        actual = {(target["target"], file["path"]): file["content"] for target in body["targets"] for file in target["files"]}
        self.assertEqual(set(actual), set(expected))
        for artifact, golden in expected.items():
            with self.subTest(artifact=artifact):
                self.assertEqual(actual[artifact].encode(), (FIXTURES / "golden" / golden).read_bytes())

    def test_generated_config_is_deterministic(self):
        for target in ("core", "gateway"):
            with self.subTest(target=target):
                first = self.succeed("compile", HOME, "--target", target)
                self.assertEqual(first, self.succeed("compile", HOME, "--target", target))
        first = self.succeed("compile", HOME, "--all")
        self.assertEqual(first, self.succeed("compile", HOME, "--all"))

    def test_ios_concrete_vlan_and_port_realization(self):
        output = self.succeed("compile", HOME, "--target", "core")
        for text in ("vlan 20", "name servers", "interface Gi1/0/2", "switchport mode access", "switchport access vlan 20", "switchport mode trunk", "10,20,30"):
            self.assertIn(text, output)

    def test_openwrt_network_dhcp_and_firewall(self):
        output = self.succeed("compile", HOME, "--target", "gateway")
        for text in ("config interface", "config bridge-vlan", "config dhcp", "config zone", "config rule", "10.0.20.1", "53", "123"):
            self.assertIn(text, output)

    def test_compile_json_provenance_and_intended_state(self):
        body = json.loads(self.succeed("compile", HOME, "--all", "--format", "json"))
        self.assertEqual(body["exportVersion"], "2.0")
        self.assertEqual({target["target"] for target in body["targets"]}, {"core", "gateway"})
        for target in body["targets"]:
            self.assertEqual(target["state"], "Intended")
            self.assertEqual(target["realization"], "conditional")
            self.assertTrue(target["assumptions"])
            self.assertTrue(target["files"])
            self.assertTrue(target["sourceMap"])
            for entry in target["sourceMap"]:
                self.assertEqual(Path(entry["source"]["file"]), HOME)
                self.assertGreaterEqual(entry["source"]["line"], 1)
                self.assertGreaterEqual(entry["source"]["column"], 1)
                self.assertTrue(entry["derivation"])
        gateway = next(target for target in body["targets"] if target["target"] == "gateway")
        self.assertEqual({file["path"] for file in gateway["files"]}, {"/etc/config/network", "/etc/config/dhcp", "/etc/config/firewall"})
        dns_entries = [entry for entry in gateway["sourceMap"] if "service dns" in entry["derivation"]]
        self.assertEqual(len(dns_entries), 2)
        dns_line = next(i for i, line in enumerate(HOME.read_text().splitlines(), 1) if "iot -> gateway allow dns" in line)
        self.assertTrue(all(entry["source"]["line"] == dns_line for entry in dns_entries))

    def test_openwrt_policy_control_plane_and_forwarding_semantics(self):
        body = json.loads(self.succeed("compile", HOME, "--target", "gateway", "--format", "json"))
        files = {file["path"]: file["content"] for file in body["targets"][0]["files"]}
        sections = uci_sections(files["/etc/config/firewall"])
        defaults = next(section["options"] for section in sections if section["type"] == "defaults")
        self.assertEqual(defaults["input"], "DROP")
        self.assertEqual(defaults["forward"], "DROP")
        rules = [section["options"] for section in sections if section["type"] == "rule"]
        dns = [rule for rule in rules if rule.get("src") == "v30" and rule.get("dest_port") == "53"]
        self.assertEqual({protocol for rule in dns for protocol in rule["proto"].split()}, {"udp", "tcp"})
        self.assertTrue(all(rule["target"] == "ACCEPT" and "dest" not in rule for rule in dns))
        self.assertTrue(any(rule.get("src") == "v30" and rule.get("dest_port") == "123" and "dest" not in rule and rule["proto"] == "udp" for rule in rules))
        for destination in ("v10", "v20"):
            self.assertTrue(any(rule.get("src") == "v30" and rule.get("dest") == destination and rule["target"] == "DROP" for rule in rules))
        self.assertTrue(any(rule.get("src") == "v10" and rule.get("dest") == "v20" and rule["target"] == "ACCEPT" for rule in rules))
        self.assertTrue(any(rule.get("src") == "v30" and rule.get("dest") == "wan" and rule["target"] == "ACCEPT" for rule in rules))
        # Intent does not declare NAT; compilation must not infer masquerading.
        self.assertFalse(any("masq" in section["options"] for section in sections))
        dhcp_sections = uci_sections(files["/etc/config/dhcp"])
        dnsmasq = next(section["options"] for section in dhcp_sections if section["type"] == "dnsmasq")
        self.assertEqual(dnsmasq["dhcpleasemax"], "241")
        for zone in ("v10", "v30"):
            controls = [rule for rule in rules if rule.get("src") == zone and "dest" not in rule and rule["target"] == "ACCEPT"]
            self.assertTrue(any(rule.get("dest_port") == "67" and rule["proto"] == "udp" for rule in controls))
            self.assertEqual({protocol for rule in controls if rule.get("dest_port") == "53" for protocol in rule["proto"].split()}, {"tcp", "udp"})
        pools = [section["options"] for section in dhcp_sections if section["type"] == "dhcp"]
        trusted = next(pool for pool in pools if pool["interface"] == "v10")
        iot = next(pool for pool in pools if pool["interface"] == "v30")
        self.assertEqual((trusted["start"], trusted["limit"]), ("100", "100"))
        self.assertEqual((iot["start"], iot["limit"]), ("100", "141"))

    def test_docs_and_graph_come_from_current_source(self):
        source = MINIMAL.replace("host nas +10", "host storage +15")
        path = self.source(source)
        docs = self.succeed("docs", path)
        for text in ("storage", "10.0.20.15", "servers", "gateway", "core"):
            self.assertIn(text, docs)
        self.assertRegex(docs.lower(), r"desired")
        graph = self.succeed("graph", HOME)
        self.assertRegex(graph, r"graph |flowchart ")
        self.assertIn("core", graph)
        self.assertIn("gateway", graph)
        self.assertEqual(graph, self.succeed("graph", HOME))

    def test_formatter_is_idempotent_and_preserves_comments(self):
        source = MINIMAL.replace("network home {", "# inventory comment\nnetwork home {").replace("host nas +10", "host nas +10 # storage comment")
        path = self.source(source)
        first = self.succeed("fmt", path)
        self.assertIn("inventory comment", first)
        self.assertIn("storage comment", first)
        self.check_json(first, valid=True)
        second = self.succeed("fmt", self.source(first, "formatted.net"))
        self.assertEqual(first, second)
        for target in ("core", "gateway"):
            before = self.succeed("compile", path, "--target", target)
            after = self.succeed("compile", self.source(first, "formatted.net"), "--target", target)
            # Config headers may deliberately preserve original provenance.
            stable = lambda value: "\n".join(line for line in value.splitlines() if not line.startswith(("#", "!")))
            self.assertEqual(stable(before), stable(after))

    def test_unterminated_and_adversarial_strings(self):
        self.reject(MINIMAL.replace("domain home.arpa", 'domain "unterminated'))
        source = MINIMAL.replace("port Gi1/0/2 { access servers }", 'port Gi1/0/2 {\n      access servers\n      description "NAS\\nshutdown"\n    }')
        result = self.invoke("compile", self.source(source), "--target", "core")
        # Rejection is acceptable; successful rendering must keep payload data.
        if result.returncode == 0:
            self.assertNotIn("\nshutdown\n", result.stdout)
            self.assertNotIn("\n shutdown\n", result.stdout)

    def test_c1_control_characters_are_rejected(self):
        for codepoint in (0x80, 0x85, 0x9F):
            with self.subTest(codepoint=codepoint):
                source = MINIMAL.replace("port Gi1/0/2 { access servers }", 'port Gi1/0/2 {\n      access servers\n      description "NAS' + chr(codepoint) + 'shutdown"\n    }')
                self.reject(source, "backend.unsafe-value")

    def test_compilation_errors_do_not_emit_partial_config(self):
        source = MINIMAL.replace("host nas +10", "host nas +256")
        result = self.invoke("compile", self.source(source), "--all")
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("config interface", result.stdout)
        self.assertNotIn("switchport mode", result.stdout)

    def test_large_invalid_input_is_bounded(self):
        source = MINIMAL.replace("host nas +10", "host nas +" + "9" * 2048)
        result = self.invoke("check", self.source(source), "--format", "json", timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("diagnostics", json.loads(result.stdout))

    def test_oversized_input_has_structured_limit_diagnostic(self):
        self.reject(" " * (1024 * 1024 + 1), "syntax.limit")

    def test_persistent_negative_fixtures(self):
        for path in sorted((FIXTURES / "errors").glob("*.net")):
            if path.name == "renderer-newline.net":
                continue  # This is a target-rendering test, not a parse requirement.
            with self.subTest(fixture=path.name):
                self.check_json(path, valid=False)


if __name__ == "__main__":
    unittest.main(verbosity=2)
