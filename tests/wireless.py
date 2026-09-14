#!/usr/bin/env python3
"""Two-device UCI parity and public-CLI wireless segment acceptance tests."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from run import NETC, ROOT, uci_sections
from router_parity import compare

SOURCE = (ROOT / 'examples/wds-network.net').read_text()
BACKUP = ROOT / 'tests/fixtures/wds-network/backup'
LINK = 'wireless-link satellite.wifinet3 -> gateway.wifinet3'


def parity_report(targets):
    lines = ['# Gateway and satellite configuration parity', '',
             'Compared against sanitized networking packages from the supplied exports. '
             'Firewall rule order is checked. System and management packages are outside compiler ownership.', '',
             'The gateway DHCP count changes from 199 to 100. All six Wi-Fi credentials are '
             'external bindings using three shared references. The satellite DNS address `10.8.8.1` '
             'and its inactive DHCP pool are preserved.', '']
    for target in targets:
        name = target['target']
        rows = compare(target, BACKUP / name, name == 'gateway')
        lines += ['## ' + name, '', f'{len(rows)} settings accounted for.', '',
                  '| Package | Section | Setting | Result |', '| --- | --- | --- | --- |']
        lines += ['| ' + ' | '.join((p, s, k, status)) + ' |' for p, s, k, _, _, status in rows]
        lines += ['']
    lines += ['Matching configuration does not establish running firmware acceptance, RF connectivity, '
              'BSSID identity, ISP service, or secret availability.', '']
    return '\n'.join(lines)


class WirelessAcceptance(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory(prefix='netc-wireless-')
        cls.path = Path(cls.tmp.name) / 'network.net'

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def invoke(self, command, source=SOURCE, *args):
        self.path.write_text(source)
        return subprocess.run([str(NETC), command, str(self.path), *args], cwd=ROOT,
                              text=True, capture_output=True, timeout=30)

    def check(self, source=SOURCE):
        result = self.invoke('check', source, '--format', 'json')
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return json.loads(result.stdout)['model']

    def reject(self, source, code):
        result = self.invoke('check', source, '--format', 'json')
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(code, [d['code'] for d in json.loads(result.stdout)['diagnostics']])

    def targets(self, source=SOURCE):
        result = self.invoke('compile', source, '--all', '--format', 'json')
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return json.loads(result.stdout)['targets']

    def test_supplied_exports_have_parity(self):
        targets = self.targets()
        self.assertEqual([t['target'] for t in targets], ['gateway', 'satellite'])
        for target in targets:
            rows = compare(target, BACKUP / target['target'], target['target'] == 'gateway')
            self.assertGreater(len(rows), 150)
            self.assertEqual(sum(r[-1] == 'External secret binding' for r in rows), 3)
            self.assertEqual(sum(r[-1].startswith('Approved correction') for r in rows),
                             int(target['target'] == 'gateway'))
        self.assertEqual(parity_report(targets), (ROOT / 'docs/wds-network-parity.md').read_text())

    def test_roles_and_typed_link_export(self):
        model = self.check()
        self.assertEqual(model['enforcerRef'], 0)
        self.assertEqual([d['routingOwner'] for d in model['devices']], [True, False])
        link = model['wirelessLinks'][0]
        self.assertEqual(link['station'], {'deviceRef': 1, 'interfaceRef': 0})
        self.assertEqual(link['accessPoint'], {'deviceRef': 0, 'interfaceRef': 1})
        satellite = model['devices'][1]['routing']
        self.assertTrue(satellite['bridges'][0]['stp'])
        lan = next(i for i in satellite['interfaces'] if i['name'] == 'lan')
        self.assertEqual(lan['gateway'], '10.9.8.1')
        self.assertEqual(lan['dnsServers'], ['10.8.8.1'])
        pool = satellite['dhcpServers'][0]['pool']
        self.assertEqual((pool['count'], pool['activeLeaseCount']), (150, 0))
        self.reject(SOURCE.replace('device satellite', 'router satellite'), 'policy.ambiguous-enforcer')
        self.check(SOURCE.replace('router gateway', 'device gateway'))

    def test_gateway_dns_and_unattached_validation(self):
        for before, after, code in [
            ('gateway 10.9.8.1', 'gateway 10.9.9.1', 'routing.gateway-unreachable'),
            ('gateway 10.9.8.1', 'gateway 10.9.8.2', 'routing.gateway-unreachable'),
            ('gateway 10.9.8.1', 'gateway 10.9.8.255', 'routing.gateway-unreachable'),
            ('dns 10.8.8.1', 'dns 10.8.8.1 10.8.8.1', 'router.duplicate-dns'),
            ('stp true', 'stp invalid', 'syntax.invalid-value'),
            ('attach none\n        protocol dhcp', 'attach none\n        protocol static\n        address 172.16.0.1/24', 'router.unattached-protocol'),
        ]:
            with self.subTest(after=after):
                self.reject(SOURCE.replace(before, after), code)
        satellite = self.targets()[1]
        network = next(f['content'] for f in satellite['files'] if f['path'] == '/etc/config/network')
        wwan = next(s for s in uci_sections(network) if s['name'] == 'wwan')
        self.assertEqual(wwan['options'], {'proto': 'dhcp'})

    def test_link_mismatch_checks(self):
        for before, after, code in [
            (LINK, 'wireless-link gateway.wifinet3 -> satellite.wifinet3', 'wireless.link-role'),
            (LINK, 'wireless-link satellite.wifinet3 -> satellite.wifinet1', 'wireless.link-device'),
            (LINK, LINK + '\n  ' + LINK, 'wireless.multiple-uplinks'),
            (LINK, 'wireless-link missing.wifinet3 -> gateway.wifinet3', 'reference.unknown-wireless-device'),
            (LINK, 'wireless-link satellite.missing -> gateway.wifinet3', 'reference.unknown-wireless-interface'),
            ('hidden true', 'hidden true\n        disabled true', 'wireless.link-disabled'),
            ('hidden true', 'hidden true\n        mac-address "02:00:00:00:00:01"', 'wireless.link-bssid'),
        ]:
            with self.subTest(after=after):
                self.reject(SOURCE.replace(before, after), code)
        for before, after, code in [
            ('wds true', 'wds false', 'wireless.link-wds'),
            ('ssid "_backhaul"', 'ssid "different"', 'wireless.link-ssid'),
            ('credential secret://home/wifi/backhaul', 'credential secret://home/wifi/other', 'wireless.link-credential'),
            ('channel 100', 'channel 104', 'wireless.link-channel'),
            ('security sae', 'security psk2', 'wireless.link-security'),
        ]:
            with self.subTest(after=after):
                self.reject(SOURCE.replace(before, after, 1), code)
        # Different PHY generations (HE80/VHT80) in the supplied link are valid.
        self.check(SOURCE.replace('hidden true', 'hidden true\n        mac-address "e8:9f:80:69:cb:54"'))

    def test_station_and_mac_constraints(self):
        for before, after, code in [
            ('mode sta', 'mode sta\n        hidden true', 'wireless.hidden-mode'),
            ('mode sta', 'mode ap', 'wireless.station-fields'),
            ('bssid "E8:9F:80:69:CB:54"', 'bssid "invalid"', 'wireless.bssid'),
            ('bssid "E8:9F:80:69:CB:54"', 'bssid "E8:9F:80:69:CB:54"\n        mac-address "invalid"', 'wireless.mac-address'),
            ('bssid "E8:9F:80:69:CB:54"\n        wds true', 'bssid "E8:9F:80:69:CB:54"\n        wds false', 'wireless.station-bridge'),
        ]:
            with self.subTest(after=after):
                self.reject(SOURCE.replace(before, after), code)

    def test_segment_address_and_active_pool_conflicts(self):
        self.reject(SOURCE.replace('address 10.9.8.2/24', 'address 10.9.8.1/24'), 'address.duplicate-segment-ip')
        self.reject(SOURCE.replace('address 10.9.8.2/24', 'address 10.9.8.101/24'), 'address.segment-dhcp-overlap')
        self.reject(SOURCE.replace('ipv4 disabled\n        ignore true', 'ipv4 server'), 'dhcp.segment-pool-overlap')
        self.check(SOURCE.replace('address 10.9.8.2/24', 'address 10.9.8.201/24'))
        # The compiler makes no shared-segment inference when the link is absent.
        self.check(SOURCE.replace(LINK, '').replace('address 10.9.8.2/24', 'address 10.9.8.101/24'))

    def test_transitive_wireless_segment_conflicts(self):
        start = SOURCE.index('  device satellite {')
        end = SOURCE.index('\n  wireless-link')
        third = SOURCE[start:end].replace('device satellite', 'device third').replace('10.9.8.2/24', '10.9.8.3/24')
        third = third.replace('ssid "_backhaul"', 'ssid "_"').replace('secret://home/wifi/backhaul', 'secret://home/wifi/main').replace('channel 100', 'channel 36')
        source = SOURCE.replace('  ' + LINK, third + '\n  ' + LINK + '\n  wireless-link third.wifinet3 -> satellite.wifinet1')
        source = source.replace('wireless-interface wifinet1 {', 'wireless-interface wifinet1 {\n        wds true')
        self.check(source)
        self.reject(source.replace('10.9.8.3/24', '10.9.8.101/24'), 'address.segment-dhcp-overlap')
        self.reject(source.replace('10.9.8.3/24', '10.9.8.1/24'), 'address.duplicate-segment-ip')

    def test_inactive_pool_does_not_require_serving_dependencies(self):
        source = SOURCE.replace('dhcp start +100 max 150', 'dhcp start +2 max 200')
        self.check(source)  # Includes local management IP, but allocates no leases.
        target = self.targets(source)[1]
        dhcp = next(f['content'] for f in target['files'] if f['path'] == '/etc/config/dhcp')
        dns = next(s for s in uci_sections(dhcp) if s['type'] == 'dnsmasq')
        self.assertNotIn('dhcpleasemax', dns['options'])
        self.reject(SOURCE.replace('ipv4 disabled\n        ignore true', 'ipv4 server\n        ignore true'), 'dhcp.ignored-server')
        self.reject(SOURCE.replace('dhcp start +100 max 100', 'dhcp start +100 max 199'), 'address.outside-prefix')

    def test_determinism_secrets_and_no_partial_compile(self):
        targets = self.targets()
        self.assertEqual(targets, self.targets())
        refs = []
        for target in targets:
            self.assertTrue(target['requiresSecretBinding'])
            files = {f['path']: f['content'] for f in target['files']}
            self.assertNotIn('/etc/config/wireless', files)
            bindings = json.loads(files['secret-bindings.json'])['bindings']
            refs += [b['reference'] for b in bindings]
        self.assertEqual(len(set(refs)), 3)
        broken = SOURCE.replace('device satellite {\n    driver openwrt', 'device satellite {\n    driver cisco-ios')
        result = self.invoke('compile', broken, '--all', '--format', 'json')
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('targets', json.loads(result.stdout))

    def test_docs_and_formatter_keep_wireless_link(self):
        graph = self.invoke('graph').stdout
        self.assertIn('r1ap0 -. WDS .-> r0ap1', graph)
        formatted = self.invoke('fmt').stdout
        self.assertIn(LINK, formatted)
        self.check(formatted)
        markdown = self.invoke('docs').stdout
        self.assertIn('Device satellite', markdown)
        self.assertIn('| sta |', markdown)


if __name__ == '__main__':
    unittest.main(verbosity=2)
