# Typed router settings

These fields occur inside `router NAME { routing { ... } }`. Boolean values use `true` or `false`. Unknown fields and duplicate scalar settings are errors. Omitted optional settings are omitted from UCI, except derived DHCP lease capacity when more than 150 leases are requested. Requirements between fields are described in the language specification.

## globals

| DSL field | Type / range | OpenWrt field |
| --- | --- | --- |
| `dhcp-default-duid` | String | `dhcp_default_duid` |
| `ula-prefix` | IPv6Prefix | `ula_prefix` |
| `packet-steering` | Bool | `packet_steering` |

## bond

| DSL field | Type / range | OpenWrt field |
| --- | --- | --- |
| `mtu` | 68–65535 | `mtu` |
| `mac-address` | String | `macaddr` |
| `policy` | `802.3ad` | `policy` |
| `hash-policy` | `layer2`, `layer2+3`, `layer3+4` | `xmit_hash_policy` |
| `selection` | `stable`, `bandwidth`, `count` | `ad_select` |
| `lacp-rate` | `fast`, `slow` | `lacp_rate` |
| `min-links` | 1–65535 | `min_links` |
| `monitor-interval` | 1–4294967295 | `monitor_interval` |

## interface

| DSL field | Type / range | OpenWrt field |
| --- | --- | --- |
| `protocol` | `static`, `dhcp`, `dhcpv6`, `none` | `proto` |
| `ipv6-assignment` | 0–128 | `ip6assign` |
| `multipath` | Bool | `multipath` |
| `request-address` | `try`, `force`, `none` | `reqaddress` |
| `request-prefix` | `auto`, `no` | `reqprefix` |
| `no-release` | Bool | `norelease` |

## dns

| DSL field | Type / range | OpenWrt field |
| --- | --- | --- |
| `domainneeded` | Bool | `domainneeded` |
| `boguspriv` | Bool | `boguspriv` |
| `filterwin2k` | Bool | `filterwin2k` |
| `localise-queries` | Bool | `localise_queries` |
| `rebind-protection` | Bool | `rebind_protection` |
| `rebind-localhost` | Bool | `rebind_localhost` |
| `expandhosts` | Bool | `expandhosts` |
| `nonegcache` | Bool | `nonegcache` |
| `authoritative` | Bool | `authoritative` |
| `readethers` | Bool | `readethers` |
| `nonwildcard` | Bool | `nonwildcard` |
| `localservice` | Bool | `localservice` |
| `filter-aaaa` | Bool | `filter_aaaa` |
| `filter-a` | Bool | `filter_a` |
| `local` | String | `local` |
| `domain` | String | `domain` |
| `leasefile` | String | `leasefile` |
| `resolvfile` | String | `resolvfile` |
| `cache-size` | 0–1000000 | `cachesize` |
| `edns-packet-max` | 512–65535 | `ednspacket_max` |
| `lease-max` | 1–65535 | `dhcpleasemax` |

## dhcp-server

| DSL field | Type / range | OpenWrt field |
| --- | --- | --- |
| `ignore` | Bool | `ignore` |
| `lease-time` | String | `leasetime` |
| `ipv4` | `server`, `disabled` | `dhcpv4` |
| `ipv6` | `server`, `disabled` | `dhcpv6` |
| `ra` | `server`, `disabled` | `ra` |
| `ra-preference` | `medium`, `high`, `low` | `ra_preference` |

## odhcp

| DSL field | Type / range | OpenWrt field |
| --- | --- | --- |
| `main-dhcp` | Bool | `maindhcp` |
| `leasefile` | String | `leasefile` |
| `lease-trigger` | String | `leasetrigger` |
| `log-level` | 0–7 | `loglevel` |
| `pio-directory` | String | `piodir` |
| `hosts-directory` | String | `hostsdir` |

## firewall

| DSL field | Type / range | OpenWrt field |
| --- | --- | --- |
| `syn-flood` | Bool | `syn_flood` |
| `input` | `ACCEPT`, `REJECT`, `DROP` | `input` |
| `output` | `ACCEPT`, `REJECT`, `DROP` | `output` |
| `forward` | `ACCEPT`, `REJECT`, `DROP` | `forward` |

## zone

| DSL field | Type / range | OpenWrt field |
| --- | --- | --- |
| `input` | `ACCEPT`, `REJECT`, `DROP` | `input` |
| `output` | `ACCEPT`, `REJECT`, `DROP` | `output` |
| `forward` | `ACCEPT`, `REJECT`, `DROP` | `forward` |
| `masquerade` | Bool | `masq` |
| `mss-adjust` | Bool | `mtu_fix` |

## firewall-rule

| DSL field | Type / range | OpenWrt field |
| --- | --- | --- |
| `family` | `ipv4`, `ipv6`, `any` | `family` |
| `destination-port` | 1–65535 | `dest_port` |
| `limit` | String | `limit` |
| `action` | `ACCEPT`, `REJECT`, `DROP` | `target` |

## radio

| DSL field | Type / range | OpenWrt field |
| --- | --- | --- |
| `driver` | `mac80211` | `type` |
| `path` | String | `path` |
| `band` | `2g`, `5g`, `6g` | `band` |
| `channel` | 1–233 | `channel` |
| `width` | `HT20`, `HT40`, `HE20`, `HE40`, `HE80`, `HE160`, `VHT20`, `VHT40`, `VHT80`, `VHT160` | `htmode` |
| `country` | String | `country` |
| `cell-density` | 0–3 | `cell_density` |

## access-point

| DSL field | Type / range | OpenWrt field |
| --- | --- | --- |
| `mode` | `ap` | `mode` |
| `ssid` | String | `ssid` |
| `security` | `none`, `psk2`, `sae` | `encryption` |
| `disabled` | Bool | `disabled` |
| `ocv` | Bool | `ocv` |
