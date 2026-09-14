"""Independent UCI comparison against the sanitized router backup.

This is a test oracle, not an importer or an implementation of DSL semantics.
The backup stays unchanged, including its original oversized DHCP limit.
"""
import ipaddress
import json
from pathlib import Path
from run import uci_sections

ROOT = Path(__file__).resolve().parents[1]
BACKUP = ROOT / 'tests/fixtures/core-router/backup'


def identity(package, section):
    kind = section['type']
    if kind in ('device', 'zone', 'rule'):
        return kind + ':' + section['options']['name']
    if kind in ('interface', 'dhcp', 'wifi-device', 'wifi-iface', 'odhcpd', 'globals'):
        return kind + ':' + section['name']
    if kind == 'forwarding':
        return 'forwarding:' + section['options']['src'] + '->' + section['options']['dest']
    return kind


def normalized(package, section):
    fields = dict(section['options'])
    for key, values in section['lists'].items():
        if key in fields:
            raise AssertionError(f'Mixed scalar/list {key}')
        fields[key] = list(values)
    for key in ('ports', 'network', 'icmp_type'):
        if key in fields and isinstance(fields[key], str):
            fields[key] = [fields[key]]
    if package == 'network' and 'ipaddr' in fields:
        values = fields['ipaddr']
        if isinstance(values, str):
            values = [values]
        mask = fields.pop('netmask', None)
        fields['ipaddr'] = [str(ipaddress.ip_interface(value if '/' in value else value + '/' + (mask or '32'))) for value in values]
    if 'ula_prefix' in fields:
        fields['ula_prefix'] = str(ipaddress.ip_network(fields['ula_prefix']))
    if 'src_ip' in fields:
        fields['src_ip'] = str(ipaddress.ip_network(fields['src_ip']))
    return fields


def compare(target, backup=BACKUP, correct_dhcp=True):
    artifacts = {f['path']: f['content'] for f in target['files']}
    manifest = json.loads(artifacts['secret-bindings.json'])
    bindings = {b['section']: b for b in manifest['bindings']}
    rows = []
    for package in ('network', 'dhcp', 'firewall', 'wireless'):
        path = '/etc/config/' + package + ('.template' if package == 'wireless' else '')
        old = uci_sections((backup / (package + '.uci')).read_text())
        new = uci_sections(artifacts[path])
        raw_original = {identity(package, s): s for s in old}
        old_map = {identity(package, s): normalized(package, s) for s in old}
        new_map = {identity(package, s): normalized(package, s) for s in new}
        assert len(old_map) == len(old) and len(new_map) == len(new), 'Duplicate section identities'
        assert old_map.keys() == new_map.keys(), (package, old_map.keys() ^ new_map.keys())
        # Rule order is behaviorally significant; do not normalize it away.
        assert [identity(package,s) for s in old if s['type']=='rule'] == [identity(package,s) for s in new if s['type']=='rule']
        for section, original in old_map.items():
            generated = new_map[section]
            assert original.keys() == generated.keys(), (package, section, original.keys() ^ generated.keys())
            for key, value in original.items():
                actual = generated[key]
                status = 'Equivalent'
                if correct_dhcp and (package, section, key) == ('dhcp', 'dhcp:lan', 'limit'):
                    assert value == '199' and actual == '100'
                    status = 'Approved correction: 100 addresses'
                elif package == 'wireless' and key == 'key':
                    assert value == '__REDACTED_WIFI_CREDENTIAL__'
                    binding = bindings[section.split(':', 1)[1]]
                    assert actual == binding['placeholder']
                    assert binding['templateArtifact'] == path
                    assert binding['installationPath'] == '/etc/config/wireless'
                    assert binding['option'] == 'key' and binding['kind'] == 'wifi-credential'
                    status = 'External secret binding'
                else:
                    assert value == actual, (package, section, key, value, actual)
                rows.append((package, section, key, value, actual, status))
            # Keep an explicit row for the source netmask, even though the
            # comparison above folds it into the generated address prefix.
            raw = raw_original[section]['options']
            if package == 'network' and 'netmask' in raw:
                address = ipaddress.ip_interface(generated['ipaddr'][0])
                assert raw['netmask'] == str(address.netmask)
                rows.append((package, section, 'netmask', raw['netmask'],
                             f'{address.netmask} (from /{address.network.prefixlen})',
                             'Equivalent: encoded in address prefix'))
    return rows


def report(target):
    rows = compare(target)
    out = ['# Core-router configuration parity', '',
           'Generated from the sanitized backup fixtures and `examples/core-router.net`. '
           'Comparison is by UCI section meaning and option values, with firewall rule order preserved.', '',
           'All backup options are accounted for. The only intended behavior change is LAN DHCP '
           '`limit 199` → `limit 100`, giving `10.9.8.100–10.9.8.199`. Two Wi-Fi credentials '
           'remain external secret bindings.', '',
           'Normalization permits anonymous section identifiers, option ordering, singleton '
           'list representation, equivalent IPv4 address/netmask notation, and IPv6 compression. '
           'These are configuration comparisons, not observations of running firmware.', '']
    def cell(value):
        return (json.dumps(value, ensure_ascii=False) if isinstance(value,list) else str(value)).replace('|','&#124;').replace('`','&#96;')
    for package in ('network','dhcp','firewall','wireless'):
        out += ['## '+package, '', '| Section | Option | Backup | Generated | Result |', '| --- | --- | --- | --- | --- |']
        for p,section,key,old,new,status in rows:
            if p == package:
                out.append('| '+' | '.join(cell(v) for v in (section,key,old,new,status))+' |')
        out.append('')
    return '\n'.join(out)
