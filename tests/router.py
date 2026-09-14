#!/usr/bin/env python3
"""Public CLI acceptance tests for typed router intent and secret templates."""
import json
from pathlib import Path
import re
import subprocess
import tempfile
import unittest
from run import NETC, ROOT, uci_sections
from router_parity import compare, report

CORE = (ROOT / 'examples/core-router.net').read_text()
GOLDEN = ROOT / 'tests/fixtures/core-router/golden'


class RouterAcceptance(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory(prefix='netc-router-')
        cls.directory = Path(cls.tmp.name)

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def invoke(self, command, source=CORE, *args):
        path = self.directory / 'router.net'
        path.write_text(source)
        return subprocess.run([str(NETC),command,str(path),*args],cwd=ROOT,text=True,capture_output=True,timeout=30)

    def check(self, source=CORE):
        result = self.invoke('check',source,'--format','json')
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        return json.loads(result.stdout)

    def reject(self, source, code=None):
        result = self.invoke('check',source,'--format','json')
        self.assertNotEqual(result.returncode,0,result.stdout+result.stderr)
        data=json.loads(result.stdout)
        if code:self.assertIn(code,[d['code'] for d in data['diagnostics']],data)
        return result

    def compile(self, source=CORE):
        result=self.invoke('compile',source,'--target','gateway','--format','json')
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        return json.loads(result.stdout)['targets'][0]

    def files(self,source=CORE):
        return {f['path']:f['content'] for f in self.compile(source)['files']}

    def test_all_backup_options_and_order_have_parity(self):
        rows=compare(self.compile())
        self.assertEqual(len(rows),188)
        self.assertEqual(sum(row[-1].startswith('Approved correction') for row in rows),1)
        self.assertEqual(sum(row[-1]=='External secret binding' for row in rows),2)

    def test_reviewed_goldens_and_parity_report(self):
        target=self.compile()
        for f in target['files']:
            name=Path(f['path']).name
            self.assertEqual(f['content'],(GOLDEN/name).read_text(),name)
        self.assertEqual(report(target),(ROOT/'docs/core-router-parity.md').read_text())

    def test_version_1_is_rejected(self):
        self.reject(CORE.replace('network-language 2.0','network-language 1.0'),'version.unsupported')

    def test_finite_and_count_pool_forms_are_identical(self):
        for pool in ('dhcp +100 .. +199','dhcp 10.9.8.100 .. 10.9.8.199'):
            self.assertEqual(self.files(),self.files(CORE.replace('dhcp start +100 max 100',pool)))
        # The shared pool parser also supports count form inside VLAN shorthand.
        home=(ROOT/'examples/home.net').read_text()
        first=self.invoke('compile',home,'--target','gateway','--format','json')
        second=self.invoke('compile',home.replace('dhcp +100 .. +199','dhcp start +100 max 100'),'--target','gateway','--format','json')
        self.assertEqual(first.returncode,0,first.stderr)
        self.assertEqual(second.returncode,0,second.stderr)
        self.assertEqual(json.loads(first.stdout)['targets'][0]['files'],json.loads(second.stdout)['targets'][0]['files'])

    def test_pool_count_boundaries_and_capacity(self):
        for count in (1,100,155):
            source=CORE.replace('max 100',f'max {count}')
            self.check(source)
            sections=uci_sections(self.files(source)['/etc/config/dhcp'])
            lan=next(s for s in sections if s['type']=='dhcp' and s['name']=='lan')
            self.assertEqual(lan['options']['limit'],str(count))
            if count>150:
                dns=next(s for s in sections if s['type']=='dnsmasq')
                self.assertEqual(dns['options']['dhcpleasemax'],str(count))
        for count in ('0','-1','156','199','4294967296','9'*30):
            with self.subTest(count=count):self.reject(CORE.replace('max 100','max '+count))
        self.reject(CORE.replace('dhcp start +100 max 100','dhcp start +1 max 100'),'address.static-dhcp-overlap')
        self.reject(CORE.replace('dhcp start +100 max 100','dhcp +199 .. +100'),'address.inverted-dhcp-range')
        self.reject(CORE.replace('cache-size 1000','cache-size 1000\n        lease-max 50'),'dhcp.lease-capacity')

    def test_pool_crosses_octet_and_respects_prefix(self):
        source=CORE.replace('10.9.8.1/24','10.9.8.1/23').replace('dhcp start +100 max 100','dhcp start +250 max 20')
        pool=self.check(source)['model']['devices'][0]['routing']['dhcpServers'][0]['pool']
        self.assertEqual(pool,{'first':'10.9.8.250','last':'10.9.9.13','count':20})

    def test_shared_bond_interfaces_and_no_vlan_invention(self):
        sections=uci_sections(self.files()['/etc/config/network'])
        interfaces={s['name']:s for s in sections if s['type']=='interface'}
        self.assertEqual({interfaces[n]['options']['device'] for n in ('wan','wan6','modem')},{'bond-wan'})
        self.assertFalse(any(s['type']=='bridge-vlan' for s in sections))
        self.assertNotIn('br-net',self.files()['/etc/config/network'])

    def test_invalid_link_references_membership_and_cycles(self):
        for before,after,code in (
            ('members lan1 wan','members lan1 unknown','reference.unknown-router-entity'),
            ('members lan1 wan','members lan1 lan1','topology.multiple-masters'),
            ('members lan2 lan3','members lan2 wan','topology.multiple-masters'),
            ('members lan2 lan3','members br-lan lan3','topology.attachment-cycle'),
            ('members lan2 lan3','members lo lan3','reference.unknown-attachment'),
            ('members lan1 wan','members lan1','topology.empty-bond'),
            ('min-links 1','min-links 3','bond.min-links'),
            ('attach br-lan','attach lan2','topology.slave-interface'),
        ):
            with self.subTest(after=after):self.reject(CORE.replace(before,after),code)
        cycle=CORE.replace('members lan2 lan3','members second lan3').replace('      bond bond-wan {','      bridge second { members br-lan }\n      bond bond-wan {')
        self.reject(cycle,'topology.attachment-cycle')

    def test_target_constraints_and_no_partial_output(self):
        cases=[CORE.replace('driver openwrt','driver cisco-ios'),
               CORE.replace('interface modem {','interface bridge_0 {').replace('interfaces wan wan6 modem','interfaces wan wan6 bridge_0'),
               CORE.replace('interface modem {','interface bad-name {').replace('interfaces wan wan6 modem','interfaces wan wan6 bad-name')]
        for source in cases:
            result=self.invoke('compile',source,'--target','gateway','--format','json')
            self.assertNotEqual(result.returncode,0,result.stdout)
            self.assertNotIn('config interface',result.stdout)
            self.assertNotIn('targets',json.loads(result.stdout))

    def test_invalid_protocol_and_address_combinations(self):
        self.reject(CORE.replace('protocol dhcp\n','protocol static\n'),'address.protocol-conflict')
        self.reject(CORE.replace('protocol static\n','protocol dhcp\n',1),'address.protocol-conflict')
        self.reject(CORE.replace('request-address try','request-address try\n        address 10.0.0.1/24'),'address.protocol-conflict')
        self.reject(CORE.replace('192.168.100.2/24','10.9.8.2/24'),'address.prefix-overlap')
        self.reject(CORE.replace('192.168.100.2/24','10.9.8.1/24'),'address.duplicate-ip')
        self.reject(CORE.replace('protocol dhcp\n','protocol dhcp\n        request-address try\n'),'router.dhcpv6-options')

    def test_ipv6_checked_addresses_and_prefixes(self):
        for address in ('::','::1','2001:db8::1','2001:0db8:0000:0000:0000:0000:0000:0001'):
            self.check(CORE.replace('address 192.168.100.2/24','address6 '+address+'/64'))
        for address in (':::','1::2::3','12345::1','gggg::1','1:2:3:4:5:6:7','1:2:3:4:5:6:7:8:9','fe80::1%lan',':1:2:3:4:5:6:7','1:2:3:4:5:6:7:'):
            with self.subTest(address=address):self.reject(CORE.replace('address 192.168.100.2/24','address6 '+address+'/64'))
        self.reject(CORE.replace('fdbe:c414:cd71::/48','fdbe:c414:cd71::1/48'),'address.noncanonical-prefix')
        self.reject(CORE.replace('fdbe:c414:cd71::/48','fdbe:c414:cd71::/129'),'address.invalid-ipv6-prefix')
        self.reject(CORE.replace('ipv6-assignment 60','ipv6-assignment 129'),'syntax.invalid-number')
        self.reject(CORE.replace('address 192.168.100.2/24','address6 2001:db8::1/129'),'address.invalid-ipv6-prefix')

    def test_firewall_rule_constraints(self):
        for before,after,code in (
            ('destination-port 68','destination-port 0','syntax.invalid-number'),
            ('destination-port 68','destination-port 65536','syntax.invalid-number'),
            ('protocols esp','protocols esp\n        destination-port 80','policy.port-protocol'),
            ('source-prefix fe80::/10','source-prefix 10.0.0.0/8','policy.family-prefix'),
            ('protocols igmp\n        family ipv4','protocols igmp\n        family ipv6','policy.family-protocol'),
            ('icmp-types echo-request\n        family ipv4','icmp-types packet-too-big\n        family ipv4','policy.icmp-family'),
            ('limit "1000/sec"','limit "invalid"','policy.rate'),
            ('protocols esp','protocols esp\n        icmp-types echo-request','policy.icmp-protocol'),
            ('protocols esp','protocols all esp','policy.protocol-required'),
            ('interfaces wan wan6 modem','interfaces wan wan6 modem lan','policy.multiple-zones'),
        ):
            with self.subTest(after=after):self.reject(CORE.replace(before,after),code)

    def test_dhcp_control_conflicts(self):
        self.reject(CORE.replace('zone lan {\n        interfaces lan\n        input ACCEPT','zone lan {\n        interfaces lan\n        input DROP'),'policy.dhcp-control-conflict')
        denial='''      firewall-rule BlockDNS {
        source lan
        protocols udp
        destination-port 53
        family ipv4
        action DROP
      }
'''
        self.reject(CORE.replace('      firewall-rule Allow-DHCP-Renew {',denial+'      firewall-rule Allow-DHCP-Renew {'),'policy.dhcp-control-conflict')

    def test_wireless_validation_and_secret_redaction(self):
        self.reject(CORE.replace('credential secret://core-router/wifi/iot',''),'secret.wifi-credential')
        for literal in ('synthetic-password-please-hide','/tmp/private','secret://../wifi','secret://wifi//iot','secret://wifi/$(id)'):
            result=self.reject(CORE.replace('secret://core-router/wifi/iot',literal),'secret.invalid-reference')
            self.assertNotIn(literal,result.stdout+result.stderr)
        for before,after,code in (
            ('radio radio1\n','radio missing\n','reference.unknown-router-entity'),
            ('security psk2','security none','secret.wifi-credential'),
            ('country "US"','country "USA"','wireless.country'),
            ('ssid "_iot"','ssid "'+'a'*33+'"','wireless.ssid'),
            ('ssid "_iot"','ssid "'+'é'*17+'"','wireless.ssid'),
            ('width HT20','width HE80','wireless.band-width'),
            ('channel 6\n','channel 36\n','wireless.band-channel'),
        ):
            with self.subTest(after=after):self.reject(CORE.replace(before,after),code)

    def test_templates_and_binding_manifest_are_consistent(self):
        target=self.compile();files={f['path']:f['content'] for f in target['files']}
        self.assertTrue(target['requiresSecretBinding'])
        self.assertEqual(target['readiness'],'requires-secret-binding')
        self.assertNotIn('/etc/config/wireless',files)
        template=files['/etc/config/wireless.template']
        manifest=json.loads(files['secret-bindings.json'])
        self.assertEqual(manifest['manifestVersion'],1)
        self.assertEqual(manifest['status'],'requires-secret-binding')
        self.assertEqual(len(manifest['bindings']),2)
        placeholders=[]
        for b in manifest['bindings']:
            self.assertEqual(template.count(b['placeholder']),1)
            self.assertTrue(b['reference'].startswith('secret://core-router/wifi/'))
            self.assertIn(b['security'],('psk2','sae'))
            placeholders.append(b['placeholder'])
        self.assertEqual(len(set(placeholders)),2)
        self.assertNotIn('secret://',template)
        self.assertNotIn('__REDACTED',json.dumps(target))
        self.assertEqual(files,self.files())
        origins=target['sourceMap']
        self.assertEqual(sum(s['generated'].startswith('secret-bindings.json#') for s in origins),2)
        self.assertTrue(any(s['generated'].startswith('/etc/config/wireless.template#') for s in origins))

    def test_secret_free_wireless_is_a_config_not_a_template(self):
        source=re.sub(r'^\s*credential .*\n','',CORE,flags=re.M).replace('security psk2','security none').replace('security sae','security none')
        target=self.compile(source);paths={f['path'] for f in target['files']}
        self.assertFalse(target['requiresSecretBinding'])
        self.assertIn('/etc/config/wireless',paths)
        self.assertNotIn('secret-bindings.json',paths)

    def test_uci_identifiers_unique_and_quoting_roundtrip(self):
        source=CORE.replace('ssid "_iot"','ssid "O\'Brien \\\"lab\\\" \\\\ edge"')
        files=self.files(source)
        for path,content in files.items():
            if path.endswith('.json'):continue
            sections=uci_sections(content)
            self.assertEqual(len({s['name'] for s in sections}),len(sections),path)
            self.assertTrue(all(re.fullmatch(r'[A-Za-z0-9_]+',s['name']) for s in sections),path)
        ap=next(s for s in uci_sections(files['/etc/config/wireless.template']) if s['name']=='default_radio1')
        self.assertEqual(ap['options']['ssid'],'O\'Brien "lab" \\ edge')
        self.reject(CORE.replace('ssid "_iot"','ssid "bad\\nline"'),'backend.unsafe-value')

    def test_docs_export_graph_and_format_include_router_entities(self):
        model=self.check()['model']['devices'][0]['routing']
        self.assertEqual(len(model['physicals']),4)
        self.assertEqual(len(model['interfaces']),5)
        self.assertEqual(len(model['rules']),9)
        self.assertEqual(len(model['radios']),3)
        self.assertEqual(len(model['accessPoints']),3)
        self.assertEqual(model['interfaces'][2]['dynamicAddressState'],'Unknown')
        self.assertEqual(model['accessPoints'][1]['credentialRef'],'secret://core-router/wifi/iot')
        docs=self.invoke('docs');graph=self.invoke('graph');fmt=self.invoke('fmt')
        for result in (docs,graph,fmt):self.assertEqual(result.returncode,0,result.stderr)
        for word in ('bond-wan','br-lan','radio1','default_radio1'):
            self.assertIn(word,docs.stdout);self.assertIn(word,graph.stdout)
        self.assertEqual(self.files(),self.files(fmt.stdout))
        self.assertEqual(fmt.stdout,self.invoke('fmt',fmt.stdout).stdout)
        self.check(fmt.stdout)

    def test_schema_catalog_contains_typed_router_contract(self):
        p=subprocess.run([str(NETC),'schema'],text=True,capture_output=True,check=True)
        schema=json.loads(p.stdout)
        self.assertEqual(schema['languageVersion'],'2.0')
        self.assertEqual(schema['routingSchema']['blocks']['interface']['fields']['protocol']['values'],['static','dhcp','dhcpv6','none'])
        self.assertEqual(schema['routingSchema']['secrets']['resolution'],'external-only')
        self.assertEqual(schema,json.loads((ROOT/'docs/schema/2.0.json').read_text()))


if __name__=='__main__':
    unittest.main(verbosity=2)
