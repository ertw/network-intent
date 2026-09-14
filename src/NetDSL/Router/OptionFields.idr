module NetDSL.Router.OptionFields

import NetDSL.Common
import NetDSL.Router.Options
import NetDSL.Router.Address
import Data.List

%default total

public export
boolText : Bool -> String
boolText True = "1"
boolText False = "0"

public export
fieldsGlobalOptions : GlobalOptions -> List (String,String)
fieldsGlobalOptions o =
  maybe [] (\v => [("dhcp_default_duid",v)]) o.dhcpDefaultDuid ++
  maybe [] (\v => [("ula_prefix",showPrefix6 v)]) o.ulaPrefix ++
  maybe [] (\v => [("packet_steering",boolText v)]) o.packetSteering

public export
fieldsBondOptions : BondOptions -> List (String,String)
fieldsBondOptions o =
  maybe [] (\v => [("mtu",show v)]) o.mtu ++
  maybe [] (\v => [("macaddr",v)]) o.macAddress ++
  maybe [] (\v => [("policy",showBondPolicy v)]) o.policy ++
  maybe [] (\v => [("xmit_hash_policy",showHashPolicy v)]) o.hashPolicy ++
  maybe [] (\v => [("ad_select",showBondSelection v)]) o.selection ++
  maybe [] (\v => [("lacp_rate",showLACPRate v)]) o.lacpRate ++
  maybe [] (\v => [("min_links",show v)]) o.minLinks ++
  maybe [] (\v => [("monitor_interval",show v)]) o.monitorInterval

public export
fieldsInterfaceOptions : InterfaceOptions -> List (String,String)
fieldsInterfaceOptions o =
  maybe [] (\v => [("proto",showInterfaceProtocol v)]) o.protocol ++
  maybe [] (\v => [("ip6assign",show v)]) o.ipv6Assignment ++
  maybe [] (\v => [("multipath",boolText v)]) o.multipath ++
  maybe [] (\v => [("reqaddress",showAddressRequest v)]) o.requestAddress ++
  maybe [] (\v => [("reqprefix",showPrefixRequest v)]) o.requestPrefix ++
  maybe [] (\v => [("norelease",boolText v)]) o.noRelease

public export
fieldsDNSOptions : DNSOptions -> List (String,String)
fieldsDNSOptions o =
  maybe [] (\v => [("domainneeded",boolText v)]) o.domainneeded ++
  maybe [] (\v => [("boguspriv",boolText v)]) o.boguspriv ++
  maybe [] (\v => [("filterwin2k",boolText v)]) o.filterwin2k ++
  maybe [] (\v => [("localise_queries",boolText v)]) o.localiseQueries ++
  maybe [] (\v => [("rebind_protection",boolText v)]) o.rebindProtection ++
  maybe [] (\v => [("rebind_localhost",boolText v)]) o.rebindLocalhost ++
  maybe [] (\v => [("expandhosts",boolText v)]) o.expandhosts ++
  maybe [] (\v => [("nonegcache",boolText v)]) o.nonegcache ++
  maybe [] (\v => [("authoritative",boolText v)]) o.authoritative ++
  maybe [] (\v => [("readethers",boolText v)]) o.readethers ++
  maybe [] (\v => [("nonwildcard",boolText v)]) o.nonwildcard ++
  maybe [] (\v => [("localservice",boolText v)]) o.localservice ++
  maybe [] (\v => [("filter_aaaa",boolText v)]) o.filterAaaa ++
  maybe [] (\v => [("filter_a",boolText v)]) o.filterA ++
  maybe [] (\v => [("local",v)]) o.local ++
  maybe [] (\v => [("domain",v)]) o.domain ++
  maybe [] (\v => [("leasefile",v)]) o.leasefile ++
  maybe [] (\v => [("resolvfile",v)]) o.resolvfile ++
  maybe [] (\v => [("cachesize",show v)]) o.cacheSize ++
  maybe [] (\v => [("ednspacket_max",show v)]) o.ednsPacketMax ++
  maybe [] (\v => [("dhcpleasemax",show v)]) o.leaseMax

public export
fieldsDHCPOptions : DHCPOptions -> List (String,String)
fieldsDHCPOptions o =
  maybe [] (\v => [("ignore",boolText v)]) o.ignore ++
  maybe [] (\v => [("leasetime",v)]) o.leaseTime ++
  maybe [] (\v => [("dhcpv4",showServerMode v)]) o.ipv4 ++
  maybe [] (\v => [("dhcpv6",showServerMode v)]) o.ipv6 ++
  maybe [] (\v => [("ra",showServerMode v)]) o.ra ++
  maybe [] (\v => [("ra_preference",showRAPreference v)]) o.raPreference

public export
fieldsODHCPOptions : ODHCPOptions -> List (String,String)
fieldsODHCPOptions o =
  maybe [] (\v => [("maindhcp",boolText v)]) o.mainDhcp ++
  maybe [] (\v => [("leasefile",v)]) o.leasefile ++
  maybe [] (\v => [("leasetrigger",v)]) o.leaseTrigger ++
  maybe [] (\v => [("loglevel",show v)]) o.logLevel ++
  maybe [] (\v => [("piodir",v)]) o.pioDirectory ++
  maybe [] (\v => [("hostsdir",v)]) o.hostsDirectory

public export
fieldsFirewallDefaults : FirewallDefaults -> List (String,String)
fieldsFirewallDefaults o =
  maybe [] (\v => [("syn_flood",boolText v)]) o.synFlood ++
  maybe [] (\v => [("input",showVerdict v)]) o.input ++
  maybe [] (\v => [("output",showVerdict v)]) o.output ++
  maybe [] (\v => [("forward",showVerdict v)]) o.forward

public export
fieldsZoneOptions : ZoneOptions -> List (String,String)
fieldsZoneOptions o =
  maybe [] (\v => [("input",showVerdict v)]) o.input ++
  maybe [] (\v => [("output",showVerdict v)]) o.output ++
  maybe [] (\v => [("forward",showVerdict v)]) o.forward ++
  maybe [] (\v => [("masq",boolText v)]) o.masquerade ++
  maybe [] (\v => [("mtu_fix",boolText v)]) o.mssAdjust

public export
fieldsRuleOptions : RuleOptions -> List (String,String)
fieldsRuleOptions o =
  maybe [] (\v => [("family",showRuleFamily v)]) o.family ++
  maybe [] (\v => [("dest_port",show v)]) o.destinationPort ++
  maybe [] (\v => [("limit",v)]) o.limit ++
  maybe [] (\v => [("target",showVerdict v)]) o.action

public export
fieldsRadioOptions : RadioOptions -> List (String,String)
fieldsRadioOptions o =
  maybe [] (\v => [("type",showRadioDriver v)]) o.driver ++
  maybe [] (\v => [("path",v)]) o.path ++
  maybe [] (\v => [("band",showBand v)]) o.band ++
  maybe [] (\v => [("channel",show v)]) o.channel ++
  maybe [] (\v => [("htmode",showChannelWidth v)]) o.width ++
  maybe [] (\v => [("country",v)]) o.country ++
  maybe [] (\v => [("cell_density",show v)]) o.cellDensity

public export
fieldsWiFiOptions : WiFiOptions -> List (String,String)
fieldsWiFiOptions o =
  maybe [] (\v => [("mode",showWiFiMode v)]) o.mode ++
  maybe [] (\v => [("ssid",v)]) o.ssid ++
  maybe [] (\v => [("encryption",showSecurity v)]) o.security ++
  maybe [] (\v => [("disabled",boolText v)]) o.disabled ++
  maybe [] (\v => [("ocv",boolText v)]) o.ocv ++
  maybe [] (\v => [("wds",boolText v)]) o.wds ++
  maybe [] (\v => [("hidden",boolText v)]) o.hidden ++
  maybe [] (\v => [("bssid",v)]) o.bssid ++
  maybe [] (\v => [("macaddr",v)]) o.macAddress

