module NetDSL.Router.OptionJSON

import NetDSL.Common
import NetDSL.Router.Options
import NetDSL.Router.Address
import Data.List

%default total

private
boolJSON : Bool -> String
boolJSON True = "true"
boolJSON False = "false"

public export
jsonGlobalOptions : GlobalOptions -> String
jsonGlobalOptions o = "{" ++ join "," (
  maybe [] (\v => ["\"dhcp-default-duid\":" ++ jsonString v]) o.dhcpDefaultDuid ++
  maybe [] (\v => ["\"ula-prefix\":" ++ jsonString (showPrefix6 v)]) o.ulaPrefix ++
  maybe [] (\v => ["\"packet-steering\":" ++ boolJSON v]) o.packetSteering) ++ "}"

public export
jsonBondOptions : BondOptions -> String
jsonBondOptions o = "{" ++ join "," (
  maybe [] (\v => ["\"mtu\":" ++ show v]) o.mtu ++
  maybe [] (\v => ["\"mac-address\":" ++ jsonString v]) o.macAddress ++
  maybe [] (\v => ["\"policy\":" ++ jsonString (showBondPolicy v)]) o.policy ++
  maybe [] (\v => ["\"hash-policy\":" ++ jsonString (showHashPolicy v)]) o.hashPolicy ++
  maybe [] (\v => ["\"selection\":" ++ jsonString (showBondSelection v)]) o.selection ++
  maybe [] (\v => ["\"lacp-rate\":" ++ jsonString (showLACPRate v)]) o.lacpRate ++
  maybe [] (\v => ["\"min-links\":" ++ show v]) o.minLinks ++
  maybe [] (\v => ["\"monitor-interval\":" ++ show v]) o.monitorInterval) ++ "}"

public export
jsonInterfaceOptions : InterfaceOptions -> String
jsonInterfaceOptions o = "{" ++ join "," (
  maybe [] (\v => ["\"protocol\":" ++ jsonString (showInterfaceProtocol v)]) o.protocol ++
  maybe [] (\v => ["\"ipv6-assignment\":" ++ show v]) o.ipv6Assignment ++
  maybe [] (\v => ["\"multipath\":" ++ boolJSON v]) o.multipath ++
  maybe [] (\v => ["\"request-address\":" ++ jsonString (showAddressRequest v)]) o.requestAddress ++
  maybe [] (\v => ["\"request-prefix\":" ++ jsonString (showPrefixRequest v)]) o.requestPrefix ++
  maybe [] (\v => ["\"no-release\":" ++ boolJSON v]) o.noRelease) ++ "}"

public export
jsonDNSOptions : DNSOptions -> String
jsonDNSOptions o = "{" ++ join "," (
  maybe [] (\v => ["\"domainneeded\":" ++ boolJSON v]) o.domainneeded ++
  maybe [] (\v => ["\"boguspriv\":" ++ boolJSON v]) o.boguspriv ++
  maybe [] (\v => ["\"filterwin2k\":" ++ boolJSON v]) o.filterwin2k ++
  maybe [] (\v => ["\"localise-queries\":" ++ boolJSON v]) o.localiseQueries ++
  maybe [] (\v => ["\"rebind-protection\":" ++ boolJSON v]) o.rebindProtection ++
  maybe [] (\v => ["\"rebind-localhost\":" ++ boolJSON v]) o.rebindLocalhost ++
  maybe [] (\v => ["\"expandhosts\":" ++ boolJSON v]) o.expandhosts ++
  maybe [] (\v => ["\"nonegcache\":" ++ boolJSON v]) o.nonegcache ++
  maybe [] (\v => ["\"authoritative\":" ++ boolJSON v]) o.authoritative ++
  maybe [] (\v => ["\"readethers\":" ++ boolJSON v]) o.readethers ++
  maybe [] (\v => ["\"nonwildcard\":" ++ boolJSON v]) o.nonwildcard ++
  maybe [] (\v => ["\"localservice\":" ++ boolJSON v]) o.localservice ++
  maybe [] (\v => ["\"filter-aaaa\":" ++ boolJSON v]) o.filterAaaa ++
  maybe [] (\v => ["\"filter-a\":" ++ boolJSON v]) o.filterA ++
  maybe [] (\v => ["\"local\":" ++ jsonString v]) o.local ++
  maybe [] (\v => ["\"domain\":" ++ jsonString v]) o.domain ++
  maybe [] (\v => ["\"leasefile\":" ++ jsonString v]) o.leasefile ++
  maybe [] (\v => ["\"resolvfile\":" ++ jsonString v]) o.resolvfile ++
  maybe [] (\v => ["\"cache-size\":" ++ show v]) o.cacheSize ++
  maybe [] (\v => ["\"edns-packet-max\":" ++ show v]) o.ednsPacketMax ++
  maybe [] (\v => ["\"lease-max\":" ++ show v]) o.leaseMax) ++ "}"

public export
jsonDHCPOptions : DHCPOptions -> String
jsonDHCPOptions o = "{" ++ join "," (
  maybe [] (\v => ["\"ignore\":" ++ boolJSON v]) o.ignore ++
  maybe [] (\v => ["\"lease-time\":" ++ jsonString v]) o.leaseTime ++
  maybe [] (\v => ["\"ipv4\":" ++ jsonString (showServerMode v)]) o.ipv4 ++
  maybe [] (\v => ["\"ipv6\":" ++ jsonString (showServerMode v)]) o.ipv6 ++
  maybe [] (\v => ["\"ra\":" ++ jsonString (showServerMode v)]) o.ra ++
  maybe [] (\v => ["\"ra-preference\":" ++ jsonString (showRAPreference v)]) o.raPreference) ++ "}"

public export
jsonODHCPOptions : ODHCPOptions -> String
jsonODHCPOptions o = "{" ++ join "," (
  maybe [] (\v => ["\"main-dhcp\":" ++ boolJSON v]) o.mainDhcp ++
  maybe [] (\v => ["\"leasefile\":" ++ jsonString v]) o.leasefile ++
  maybe [] (\v => ["\"lease-trigger\":" ++ jsonString v]) o.leaseTrigger ++
  maybe [] (\v => ["\"log-level\":" ++ show v]) o.logLevel ++
  maybe [] (\v => ["\"pio-directory\":" ++ jsonString v]) o.pioDirectory ++
  maybe [] (\v => ["\"hosts-directory\":" ++ jsonString v]) o.hostsDirectory) ++ "}"

public export
jsonFirewallDefaults : FirewallDefaults -> String
jsonFirewallDefaults o = "{" ++ join "," (
  maybe [] (\v => ["\"syn-flood\":" ++ boolJSON v]) o.synFlood ++
  maybe [] (\v => ["\"input\":" ++ jsonString (showVerdict v)]) o.input ++
  maybe [] (\v => ["\"output\":" ++ jsonString (showVerdict v)]) o.output ++
  maybe [] (\v => ["\"forward\":" ++ jsonString (showVerdict v)]) o.forward) ++ "}"

public export
jsonZoneOptions : ZoneOptions -> String
jsonZoneOptions o = "{" ++ join "," (
  maybe [] (\v => ["\"input\":" ++ jsonString (showVerdict v)]) o.input ++
  maybe [] (\v => ["\"output\":" ++ jsonString (showVerdict v)]) o.output ++
  maybe [] (\v => ["\"forward\":" ++ jsonString (showVerdict v)]) o.forward ++
  maybe [] (\v => ["\"masquerade\":" ++ boolJSON v]) o.masquerade ++
  maybe [] (\v => ["\"mss-adjust\":" ++ boolJSON v]) o.mssAdjust) ++ "}"

public export
jsonRuleOptions : RuleOptions -> String
jsonRuleOptions o = "{" ++ join "," (
  maybe [] (\v => ["\"family\":" ++ jsonString (showRuleFamily v)]) o.family ++
  maybe [] (\v => ["\"destination-port\":" ++ show v]) o.destinationPort ++
  maybe [] (\v => ["\"limit\":" ++ jsonString v]) o.limit ++
  maybe [] (\v => ["\"action\":" ++ jsonString (showVerdict v)]) o.action) ++ "}"

public export
jsonRadioOptions : RadioOptions -> String
jsonRadioOptions o = "{" ++ join "," (
  maybe [] (\v => ["\"driver\":" ++ jsonString (showRadioDriver v)]) o.driver ++
  maybe [] (\v => ["\"path\":" ++ jsonString v]) o.path ++
  maybe [] (\v => ["\"band\":" ++ jsonString (showBand v)]) o.band ++
  maybe [] (\v => ["\"channel\":" ++ show v]) o.channel ++
  maybe [] (\v => ["\"width\":" ++ jsonString (showChannelWidth v)]) o.width ++
  maybe [] (\v => ["\"country\":" ++ jsonString v]) o.country ++
  maybe [] (\v => ["\"cell-density\":" ++ show v]) o.cellDensity) ++ "}"

public export
jsonWiFiOptions : WiFiOptions -> String
jsonWiFiOptions o = "{" ++ join "," (
  maybe [] (\v => ["\"mode\":" ++ jsonString (showWiFiMode v)]) o.mode ++
  maybe [] (\v => ["\"ssid\":" ++ jsonString v]) o.ssid ++
  maybe [] (\v => ["\"security\":" ++ jsonString (showSecurity v)]) o.security ++
  maybe [] (\v => ["\"disabled\":" ++ boolJSON v]) o.disabled ++
  maybe [] (\v => ["\"ocv\":" ++ boolJSON v]) o.ocv ++
  maybe [] (\v => ["\"wds\":" ++ boolJSON v]) o.wds ++
  maybe [] (\v => ["\"hidden\":" ++ boolJSON v]) o.hidden ++
  maybe [] (\v => ["\"bssid\":" ++ jsonString v]) o.bssid ++
  maybe [] (\v => ["\"mac-address\":" ++ jsonString v]) o.macAddress) ++ "}"

