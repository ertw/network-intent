# Core-router configuration parity

Generated from the sanitized backup fixtures and `examples/core-router.net`. Comparison is by UCI section meaning and option values, with firewall rule order preserved.

All backup options are accounted for. The only intended behavior change is LAN DHCP `limit 199` → `limit 100`, giving `10.9.8.100–10.9.8.199`. Two Wi-Fi credentials remain external secret bindings.

Normalization permits anonymous section identifiers, option ordering, singleton list representation, equivalent IPv4 address/netmask notation, and IPv6 compression. These are configuration comparisons, not observations of running firmware.

## network

| Section | Option | Backup | Generated | Result |
| --- | --- | --- | --- | --- |
| interface:loopback | device | lo | lo | Equivalent |
| interface:loopback | proto | static | static | Equivalent |
| interface:loopback | ipaddr | ["127.0.0.1/8"] | ["127.0.0.1/8"] | Equivalent |
| globals:globals | dhcp_default_duid | 0004ecf7fc0d101f44ab9683172b87cd03ad | 0004ecf7fc0d101f44ab9683172b87cd03ad | Equivalent |
| globals:globals | ula_prefix | fdbe:c414:cd71::/48 | fdbe:c414:cd71::/48 | Equivalent |
| globals:globals | packet_steering | 1 | 1 | Equivalent |
| device:br-lan | name | br-lan | br-lan | Equivalent |
| device:br-lan | type | bridge | bridge | Equivalent |
| device:br-lan | ports | ["lan2", "lan3"] | ["lan2", "lan3"] | Equivalent |
| interface:lan | device | br-lan | br-lan | Equivalent |
| interface:lan | proto | static | static | Equivalent |
| interface:lan | ip6assign | 60 | 60 | Equivalent |
| interface:lan | multipath | off | off | Equivalent |
| interface:lan | ipaddr | ["10.9.8.1/24"] | ["10.9.8.1/24"] | Equivalent |
| interface:wan | device | bond-wan | bond-wan | Equivalent |
| interface:wan | proto | dhcp | dhcp | Equivalent |
| interface:wan | multipath | off | off | Equivalent |
| interface:wan6 | device | bond-wan | bond-wan | Equivalent |
| interface:wan6 | proto | dhcpv6 | dhcpv6 | Equivalent |
| interface:wan6 | reqaddress | try | try | Equivalent |
| interface:wan6 | reqprefix | auto | auto | Equivalent |
| interface:wan6 | norelease | 1 | 1 | Equivalent |
| interface:wan6 | multipath | off | off | Equivalent |
| interface:modem | proto | static | static | Equivalent |
| interface:modem | device | bond-wan | bond-wan | Equivalent |
| interface:modem | ipaddr | ["192.168.100.2/24"] | ["192.168.100.2/24"] | Equivalent |
| interface:modem | multipath | off | off | Equivalent |
| interface:modem | netmask | 255.255.255.0 | 255.255.255.0 (from /24) | Equivalent: encoded in address prefix |
| device:bond-wan | type | bonding | bonding | Equivalent |
| device:bond-wan | name | bond-wan | bond-wan | Equivalent |
| device:bond-wan | mtu | 1500 | 1500 | Equivalent |
| device:bond-wan | macaddr | e8:9f:80:69:cb:51 | e8:9f:80:69:cb:51 | Equivalent |
| device:bond-wan | policy | 802.3ad | 802.3ad | Equivalent |
| device:bond-wan | xmit_hash_policy | layer3+4 | layer3+4 | Equivalent |
| device:bond-wan | ad_select | stable | stable | Equivalent |
| device:bond-wan | lacp_rate | fast | fast | Equivalent |
| device:bond-wan | min_links | 1 | 1 | Equivalent |
| device:bond-wan | monitor_interval | 100 | 100 | Equivalent |
| device:bond-wan | ports | ["lan1", "wan"] | ["lan1", "wan"] | Equivalent |

## dhcp

| Section | Option | Backup | Generated | Result |
| --- | --- | --- | --- | --- |
| dnsmasq | domainneeded | 1 | 1 | Equivalent |
| dnsmasq | boguspriv | 1 | 1 | Equivalent |
| dnsmasq | filterwin2k | 0 | 0 | Equivalent |
| dnsmasq | localise_queries | 1 | 1 | Equivalent |
| dnsmasq | rebind_protection | 1 | 1 | Equivalent |
| dnsmasq | rebind_localhost | 1 | 1 | Equivalent |
| dnsmasq | local | /lan/ | /lan/ | Equivalent |
| dnsmasq | domain | lan | lan | Equivalent |
| dnsmasq | expandhosts | 1 | 1 | Equivalent |
| dnsmasq | nonegcache | 0 | 0 | Equivalent |
| dnsmasq | cachesize | 1000 | 1000 | Equivalent |
| dnsmasq | authoritative | 1 | 1 | Equivalent |
| dnsmasq | readethers | 1 | 1 | Equivalent |
| dnsmasq | leasefile | /tmp/dhcp.leases | /tmp/dhcp.leases | Equivalent |
| dnsmasq | resolvfile | /tmp/resolv.conf.d/resolv.conf.auto | /tmp/resolv.conf.d/resolv.conf.auto | Equivalent |
| dnsmasq | nonwildcard | 1 | 1 | Equivalent |
| dnsmasq | localservice | 1 | 1 | Equivalent |
| dnsmasq | ednspacket_max | 1232 | 1232 | Equivalent |
| dnsmasq | filter_aaaa | 0 | 0 | Equivalent |
| dnsmasq | filter_a | 0 | 0 | Equivalent |
| dhcp:lan | interface | lan | lan | Equivalent |
| dhcp:lan | start | 100 | 100 | Equivalent |
| dhcp:lan | limit | 199 | 100 | Approved correction: 100 addresses |
| dhcp:lan | leasetime | 12h | 12h | Equivalent |
| dhcp:lan | dhcpv4 | server | server | Equivalent |
| dhcp:lan | dhcpv6 | server | server | Equivalent |
| dhcp:lan | ra | server | server | Equivalent |
| dhcp:lan | ra_preference | medium | medium | Equivalent |
| dhcp:lan | ra_flags | ["managed-config", "other-config"] | ["managed-config", "other-config"] | Equivalent |
| dhcp:wan | interface | wan | wan | Equivalent |
| dhcp:wan | ignore | 1 | 1 | Equivalent |
| odhcpd:odhcpd | maindhcp | 0 | 0 | Equivalent |
| odhcpd:odhcpd | leasefile | /tmp/odhcpd.leases | /tmp/odhcpd.leases | Equivalent |
| odhcpd:odhcpd | leasetrigger | /usr/sbin/odhcpd-update | /usr/sbin/odhcpd-update | Equivalent |
| odhcpd:odhcpd | loglevel | 4 | 4 | Equivalent |
| odhcpd:odhcpd | piodir | /tmp/odhcpd-piodir | /tmp/odhcpd-piodir | Equivalent |
| odhcpd:odhcpd | hostsdir | /tmp/hosts | /tmp/hosts | Equivalent |

## firewall

| Section | Option | Backup | Generated | Result |
| --- | --- | --- | --- | --- |
| defaults | syn_flood | 1 | 1 | Equivalent |
| defaults | input | REJECT | REJECT | Equivalent |
| defaults | output | ACCEPT | ACCEPT | Equivalent |
| defaults | forward | REJECT | REJECT | Equivalent |
| zone:lan | name | lan | lan | Equivalent |
| zone:lan | input | ACCEPT | ACCEPT | Equivalent |
| zone:lan | output | ACCEPT | ACCEPT | Equivalent |
| zone:lan | forward | ACCEPT | ACCEPT | Equivalent |
| zone:lan | network | ["lan"] | ["lan"] | Equivalent |
| zone:wan | name | wan | wan | Equivalent |
| zone:wan | input | REJECT | REJECT | Equivalent |
| zone:wan | output | ACCEPT | ACCEPT | Equivalent |
| zone:wan | forward | DROP | DROP | Equivalent |
| zone:wan | masq | 1 | 1 | Equivalent |
| zone:wan | mtu_fix | 1 | 1 | Equivalent |
| zone:wan | network | ["wan", "wan6", "modem"] | ["wan", "wan6", "modem"] | Equivalent |
| forwarding:lan->wan | src | lan | lan | Equivalent |
| forwarding:lan->wan | dest | wan | wan | Equivalent |
| rule:Allow-DHCP-Renew | name | Allow-DHCP-Renew | Allow-DHCP-Renew | Equivalent |
| rule:Allow-DHCP-Renew | src | wan | wan | Equivalent |
| rule:Allow-DHCP-Renew | proto | udp | udp | Equivalent |
| rule:Allow-DHCP-Renew | dest_port | 68 | 68 | Equivalent |
| rule:Allow-DHCP-Renew | target | ACCEPT | ACCEPT | Equivalent |
| rule:Allow-DHCP-Renew | family | ipv4 | ipv4 | Equivalent |
| rule:Allow-Ping | name | Allow-Ping | Allow-Ping | Equivalent |
| rule:Allow-Ping | src | wan | wan | Equivalent |
| rule:Allow-Ping | proto | icmp | icmp | Equivalent |
| rule:Allow-Ping | icmp_type | ["echo-request"] | ["echo-request"] | Equivalent |
| rule:Allow-Ping | family | ipv4 | ipv4 | Equivalent |
| rule:Allow-Ping | target | ACCEPT | ACCEPT | Equivalent |
| rule:Allow-IGMP | name | Allow-IGMP | Allow-IGMP | Equivalent |
| rule:Allow-IGMP | src | wan | wan | Equivalent |
| rule:Allow-IGMP | proto | igmp | igmp | Equivalent |
| rule:Allow-IGMP | family | ipv4 | ipv4 | Equivalent |
| rule:Allow-IGMP | target | ACCEPT | ACCEPT | Equivalent |
| rule:Allow-DHCPv6 | name | Allow-DHCPv6 | Allow-DHCPv6 | Equivalent |
| rule:Allow-DHCPv6 | src | wan | wan | Equivalent |
| rule:Allow-DHCPv6 | proto | udp | udp | Equivalent |
| rule:Allow-DHCPv6 | dest_port | 546 | 546 | Equivalent |
| rule:Allow-DHCPv6 | family | ipv6 | ipv6 | Equivalent |
| rule:Allow-DHCPv6 | target | ACCEPT | ACCEPT | Equivalent |
| rule:Allow-MLD | name | Allow-MLD | Allow-MLD | Equivalent |
| rule:Allow-MLD | src | wan | wan | Equivalent |
| rule:Allow-MLD | proto | icmp | icmp | Equivalent |
| rule:Allow-MLD | src_ip | fe80::/10 | fe80::/10 | Equivalent |
| rule:Allow-MLD | family | ipv6 | ipv6 | Equivalent |
| rule:Allow-MLD | target | ACCEPT | ACCEPT | Equivalent |
| rule:Allow-MLD | icmp_type | ["130/0", "131/0", "132/0", "143/0"] | ["130/0", "131/0", "132/0", "143/0"] | Equivalent |
| rule:Allow-ICMPv6-Input | name | Allow-ICMPv6-Input | Allow-ICMPv6-Input | Equivalent |
| rule:Allow-ICMPv6-Input | src | wan | wan | Equivalent |
| rule:Allow-ICMPv6-Input | proto | icmp | icmp | Equivalent |
| rule:Allow-ICMPv6-Input | limit | 1000/sec | 1000/sec | Equivalent |
| rule:Allow-ICMPv6-Input | family | ipv6 | ipv6 | Equivalent |
| rule:Allow-ICMPv6-Input | target | ACCEPT | ACCEPT | Equivalent |
| rule:Allow-ICMPv6-Input | icmp_type | ["echo-request", "echo-reply", "destination-unreachable", "packet-too-big", "time-exceeded", "bad-header", "unknown-header-type", "router-solicitation", "neighbour-solicitation", "router-advertisement", "neighbour-advertisement"] | ["echo-request", "echo-reply", "destination-unreachable", "packet-too-big", "time-exceeded", "bad-header", "unknown-header-type", "router-solicitation", "neighbour-solicitation", "router-advertisement", "neighbour-advertisement"] | Equivalent |
| rule:Allow-ICMPv6-Forward | name | Allow-ICMPv6-Forward | Allow-ICMPv6-Forward | Equivalent |
| rule:Allow-ICMPv6-Forward | src | wan | wan | Equivalent |
| rule:Allow-ICMPv6-Forward | dest | * | * | Equivalent |
| rule:Allow-ICMPv6-Forward | proto | icmp | icmp | Equivalent |
| rule:Allow-ICMPv6-Forward | limit | 1000/sec | 1000/sec | Equivalent |
| rule:Allow-ICMPv6-Forward | family | ipv6 | ipv6 | Equivalent |
| rule:Allow-ICMPv6-Forward | target | ACCEPT | ACCEPT | Equivalent |
| rule:Allow-ICMPv6-Forward | icmp_type | ["echo-request", "echo-reply", "destination-unreachable", "packet-too-big", "time-exceeded", "bad-header", "unknown-header-type"] | ["echo-request", "echo-reply", "destination-unreachable", "packet-too-big", "time-exceeded", "bad-header", "unknown-header-type"] | Equivalent |
| rule:Allow-IPSec-ESP | name | Allow-IPSec-ESP | Allow-IPSec-ESP | Equivalent |
| rule:Allow-IPSec-ESP | src | wan | wan | Equivalent |
| rule:Allow-IPSec-ESP | dest | lan | lan | Equivalent |
| rule:Allow-IPSec-ESP | proto | esp | esp | Equivalent |
| rule:Allow-IPSec-ESP | target | ACCEPT | ACCEPT | Equivalent |
| rule:Allow-ISAKMP | name | Allow-ISAKMP | Allow-ISAKMP | Equivalent |
| rule:Allow-ISAKMP | src | wan | wan | Equivalent |
| rule:Allow-ISAKMP | dest | lan | lan | Equivalent |
| rule:Allow-ISAKMP | dest_port | 500 | 500 | Equivalent |
| rule:Allow-ISAKMP | proto | udp | udp | Equivalent |
| rule:Allow-ISAKMP | target | ACCEPT | ACCEPT | Equivalent |

## wireless

| Section | Option | Backup | Generated | Result |
| --- | --- | --- | --- | --- |
| wifi-device:radio0 | type | mac80211 | mac80211 | Equivalent |
| wifi-device:radio0 | path | platform/soc@0/c000000.wifi | platform/soc@0/c000000.wifi | Equivalent |
| wifi-device:radio0 | band | 5g | 5g | Equivalent |
| wifi-device:radio0 | channel | 36 | 36 | Equivalent |
| wifi-device:radio0 | htmode | HE80 | HE80 | Equivalent |
| wifi-iface:default_radio0 | device | radio0 | radio0 | Equivalent |
| wifi-iface:default_radio0 | network | ["lan"] | ["lan"] | Equivalent |
| wifi-iface:default_radio0 | mode | ap | ap | Equivalent |
| wifi-iface:default_radio0 | ssid | OpenWrt | OpenWrt | Equivalent |
| wifi-iface:default_radio0 | encryption | none | none | Equivalent |
| wifi-iface:default_radio0 | disabled | 1 | 1 | Equivalent |
| wifi-device:radio1 | type | mac80211 | mac80211 | Equivalent |
| wifi-device:radio1 | path | platform/soc@0/c000000.wifi+1 | platform/soc@0/c000000.wifi+1 | Equivalent |
| wifi-device:radio1 | band | 2g | 2g | Equivalent |
| wifi-device:radio1 | channel | 6 | 6 | Equivalent |
| wifi-device:radio1 | htmode | HT20 | HT20 | Equivalent |
| wifi-device:radio1 | country | US | US | Equivalent |
| wifi-device:radio1 | cell_density | 0 | 0 | Equivalent |
| wifi-iface:default_radio1 | device | radio1 | radio1 | Equivalent |
| wifi-iface:default_radio1 | network | ["lan"] | ["lan"] | Equivalent |
| wifi-iface:default_radio1 | mode | ap | ap | Equivalent |
| wifi-iface:default_radio1 | ssid | _iot | _iot | Equivalent |
| wifi-iface:default_radio1 | encryption | psk2 | psk2 | Equivalent |
| wifi-iface:default_radio1 | key | __REDACTED_WIFI_CREDENTIAL__ | __NETC_SECRET_default_radio1_key__ | External secret binding |
| wifi-device:radio2 | type | mac80211 | mac80211 | Equivalent |
| wifi-device:radio2 | path | platform/soc@0/c000000.wifi+2 | platform/soc@0/c000000.wifi+2 | Equivalent |
| wifi-device:radio2 | band | 5g | 5g | Equivalent |
| wifi-device:radio2 | channel | 100 | 100 | Equivalent |
| wifi-device:radio2 | htmode | HE80 | HE80 | Equivalent |
| wifi-device:radio2 | cell_density | 0 | 0 | Equivalent |
| wifi-device:radio2 | country | US | US | Equivalent |
| wifi-iface:wifinet2 | device | radio2 | radio2 | Equivalent |
| wifi-iface:wifinet2 | mode | ap | ap | Equivalent |
| wifi-iface:wifinet2 | ssid | _ | _ | Equivalent |
| wifi-iface:wifinet2 | encryption | sae | sae | Equivalent |
| wifi-iface:wifinet2 | key | __REDACTED_WIFI_CREDENTIAL__ | __NETC_SECRET_wifinet2_key__ | External secret binding |
| wifi-iface:wifinet2 | ocv | 0 | 0 | Equivalent |
| wifi-iface:wifinet2 | network | ["lan"] | ["lan"] | Equivalent |
