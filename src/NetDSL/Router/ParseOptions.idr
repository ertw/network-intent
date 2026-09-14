module NetDSL.Router.ParseOptions

import NetDSL.Common
import NetDSL.Syntax.Parser
import NetDSL.Router.Fields
import NetDSL.Router.Options
import NetDSL.Router.Address

%default total

public export
globalOptionsKeys : List String
globalOptionsKeys = ["dhcp-default-duid", "ula-prefix", "packet-steering"]

public export
parseGlobalOptions : Statement -> Either Diagnostic GlobalOptions
parseGlobalOptions s = do
  dhcpDefaultDuid <- optional safeString "dhcp-default-duid" s
  ulaPrefix <- optional (\t => parsePrefix6 t.source t.text) "ula-prefix" s
  packetSteering <- optional boolean "packet-steering" s
  Right (MkGlobalOptions dhcpDefaultDuid ulaPrefix packetSteering)

public export
bondOptionsKeys : List String
bondOptionsKeys = ["mtu", "mac-address", "policy", "hash-policy", "selection", "lacp-rate", "min-links", "monitor-interval"]

public export
parseBondOptions : Statement -> Either Diagnostic BondOptions
parseBondOptions s = do
  mtu <- optional (natural 68 65535) "mtu" s
  macAddress <- optional safeString "mac-address" s
  policy <- optional (choice [("802.3ad",LACP)]) "policy" s
  hashPolicy <- optional (choice [("layer2",Layer2), ("layer2+3",Layer23), ("layer3+4",Layer34)]) "hash-policy" s
  selection <- optional (choice [("stable",Stable), ("bandwidth",Bandwidth), ("count",Count)]) "selection" s
  lacpRate <- optional (choice [("fast",Fast), ("slow",Slow)]) "lacp-rate" s
  minLinks <- optional (natural 1 65535) "min-links" s
  monitorInterval <- optional (natural 1 4294967295) "monitor-interval" s
  Right (MkBondOptions mtu macAddress policy hashPolicy selection lacpRate minLinks monitorInterval)

public export
interfaceOptionsKeys : List String
interfaceOptionsKeys = ["protocol", "ipv6-assignment", "multipath", "request-address", "request-prefix", "no-release"]

public export
parseInterfaceOptions : Statement -> Either Diagnostic InterfaceOptions
parseInterfaceOptions s = do
  protocol <- optional (choice [("static",Static), ("dhcp",DHCPClient), ("dhcpv6",DHCPv6Client), ("none",Unnumbered)]) "protocol" s
  ipv6Assignment <- optional (natural 0 128) "ipv6-assignment" s
  multipath <- optional boolean "multipath" s
  requestAddress <- optional (choice [("try",TryAddress), ("force",ForceAddress), ("none",NoAddress)]) "request-address" s
  requestPrefix <- optional (choice [("auto",AutoPrefix), ("no",NoPrefix)]) "request-prefix" s
  noRelease <- optional boolean "no-release" s
  Right (MkInterfaceOptions protocol ipv6Assignment multipath requestAddress requestPrefix noRelease)

public export
dnsOptionsKeys : List String
dnsOptionsKeys = ["domainneeded", "boguspriv", "filterwin2k", "localise-queries", "rebind-protection", "rebind-localhost", "expandhosts", "nonegcache", "authoritative", "readethers", "nonwildcard", "localservice", "filter-aaaa", "filter-a", "local", "domain", "leasefile", "resolvfile", "cache-size", "edns-packet-max", "lease-max"]

public export
parseDNSOptions : Statement -> Either Diagnostic DNSOptions
parseDNSOptions s = do
  domainneeded <- optional boolean "domainneeded" s
  boguspriv <- optional boolean "boguspriv" s
  filterwin2k <- optional boolean "filterwin2k" s
  localiseQueries <- optional boolean "localise-queries" s
  rebindProtection <- optional boolean "rebind-protection" s
  rebindLocalhost <- optional boolean "rebind-localhost" s
  expandhosts <- optional boolean "expandhosts" s
  nonegcache <- optional boolean "nonegcache" s
  authoritative <- optional boolean "authoritative" s
  readethers <- optional boolean "readethers" s
  nonwildcard <- optional boolean "nonwildcard" s
  localservice <- optional boolean "localservice" s
  filterAaaa <- optional boolean "filter-aaaa" s
  filterA <- optional boolean "filter-a" s
  local <- optional safeString "local" s
  domain <- optional safeString "domain" s
  leasefile <- optional safeString "leasefile" s
  resolvfile <- optional safeString "resolvfile" s
  cacheSize <- optional (natural 0 1000000) "cache-size" s
  ednsPacketMax <- optional (natural 512 65535) "edns-packet-max" s
  leaseMax <- optional (natural 1 65535) "lease-max" s
  Right (MkDNSOptions domainneeded boguspriv filterwin2k localiseQueries rebindProtection rebindLocalhost expandhosts nonegcache authoritative readethers nonwildcard localservice filterAaaa filterA local domain leasefile resolvfile cacheSize ednsPacketMax leaseMax)

public export
dhcpOptionsKeys : List String
dhcpOptionsKeys = ["ignore", "lease-time", "ipv4", "ipv6", "ra", "ra-preference"]

public export
parseDHCPOptions : Statement -> Either Diagnostic DHCPOptions
parseDHCPOptions s = do
  ignore <- optional boolean "ignore" s
  leaseTime <- optional safeString "lease-time" s
  ipv4 <- optional (choice [("server",Server), ("disabled",Disabled)]) "ipv4" s
  ipv6 <- optional (choice [("server",Server), ("disabled",Disabled)]) "ipv6" s
  ra <- optional (choice [("server",Server), ("disabled",Disabled)]) "ra" s
  raPreference <- optional (choice [("medium",Medium), ("high",High), ("low",Low)]) "ra-preference" s
  Right (MkDHCPOptions ignore leaseTime ipv4 ipv6 ra raPreference)

public export
odhcpOptionsKeys : List String
odhcpOptionsKeys = ["main-dhcp", "leasefile", "lease-trigger", "log-level", "pio-directory", "hosts-directory"]

public export
parseODHCPOptions : Statement -> Either Diagnostic ODHCPOptions
parseODHCPOptions s = do
  mainDhcp <- optional boolean "main-dhcp" s
  leasefile <- optional safeString "leasefile" s
  leaseTrigger <- optional safeString "lease-trigger" s
  logLevel <- optional (natural 0 7) "log-level" s
  pioDirectory <- optional safeString "pio-directory" s
  hostsDirectory <- optional safeString "hosts-directory" s
  Right (MkODHCPOptions mainDhcp leasefile leaseTrigger logLevel pioDirectory hostsDirectory)

public export
firewallDefaultsKeys : List String
firewallDefaultsKeys = ["syn-flood", "input", "output", "forward"]

public export
parseFirewallDefaults : Statement -> Either Diagnostic FirewallDefaults
parseFirewallDefaults s = do
  synFlood <- optional boolean "syn-flood" s
  input <- optional (choice [("ACCEPT",Accept), ("REJECT",Reject), ("DROP",Drop)]) "input" s
  output <- optional (choice [("ACCEPT",Accept), ("REJECT",Reject), ("DROP",Drop)]) "output" s
  forward <- optional (choice [("ACCEPT",Accept), ("REJECT",Reject), ("DROP",Drop)]) "forward" s
  Right (MkFirewallDefaults synFlood input output forward)

public export
zoneOptionsKeys : List String
zoneOptionsKeys = ["input", "output", "forward", "masquerade", "mss-adjust"]

public export
parseZoneOptions : Statement -> Either Diagnostic ZoneOptions
parseZoneOptions s = do
  input <- optional (choice [("ACCEPT",Accept), ("REJECT",Reject), ("DROP",Drop)]) "input" s
  output <- optional (choice [("ACCEPT",Accept), ("REJECT",Reject), ("DROP",Drop)]) "output" s
  forward <- optional (choice [("ACCEPT",Accept), ("REJECT",Reject), ("DROP",Drop)]) "forward" s
  masquerade <- optional boolean "masquerade" s
  mssAdjust <- optional boolean "mss-adjust" s
  Right (MkZoneOptions input output forward masquerade mssAdjust)

public export
ruleOptionsKeys : List String
ruleOptionsKeys = ["family", "destination-port", "limit", "action"]

public export
parseRuleOptions : Statement -> Either Diagnostic RuleOptions
parseRuleOptions s = do
  family <- optional (choice [("ipv4",Family4), ("ipv6",Family6), ("any",BothFamilies)]) "family" s
  destinationPort <- optional (natural 1 65535) "destination-port" s
  limit <- optional safeString "limit" s
  action <- optional (choice [("ACCEPT",Accept), ("REJECT",Reject), ("DROP",Drop)]) "action" s
  Right (MkRuleOptions family destinationPort limit action)

public export
radioOptionsKeys : List String
radioOptionsKeys = ["driver", "path", "band", "channel", "width", "country", "cell-density"]

public export
parseRadioOptions : Statement -> Either Diagnostic RadioOptions
parseRadioOptions s = do
  driver <- optional (choice [("mac80211",Mac80211)]) "driver" s
  path <- optional safeString "path" s
  band <- optional (choice [("2g",Band2), ("5g",Band5), ("6g",Band6)]) "band" s
  channel <- optional (natural 1 233) "channel" s
  width <- optional (choice [("HT20",HT20), ("HT40",HT40), ("HE20",HE20), ("HE40",HE40), ("HE80",HE80), ("HE160",HE160), ("VHT20",VHT20), ("VHT40",VHT40), ("VHT80",VHT80), ("VHT160",VHT160)]) "width" s
  country <- optional safeString "country" s
  cellDensity <- optional (natural 0 3) "cell-density" s
  Right (MkRadioOptions driver path band channel width country cellDensity)

public export
wifiOptionsKeys : List String
wifiOptionsKeys = ["mode", "ssid", "security", "disabled", "ocv", "wds", "hidden", "bssid", "mac-address"]

public export
parseWiFiOptions : Statement -> Either Diagnostic WiFiOptions
parseWiFiOptions s = do
  mode <- optional (choice [("ap",AccessPoint), ("sta",Station)]) "mode" s
  ssid <- optional safeString "ssid" s
  security <- optional (choice [("none",Open), ("psk2",WPA2), ("sae",WPA3)]) "security" s
  disabled <- optional boolean "disabled" s
  ocv <- optional boolean "ocv" s
  wds <- optional boolean "wds" s
  hidden <- optional boolean "hidden" s
  bssid <- optional safeString "bssid" s
  macAddress <- optional safeString "mac-address" s
  Right (MkWiFiOptions mode ssid security disabled ocv wds hidden bssid macAddress)

