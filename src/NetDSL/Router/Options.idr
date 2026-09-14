module NetDSL.Router.Options

import NetDSL.Common
import NetDSL.Router.Address

%default total

public export
data BondPolicy = LACP

public export
Eq BondPolicy where
  LACP == LACP = True

public export
showBondPolicy : BondPolicy -> String
showBondPolicy LACP = "802.3ad"

public export
data HashPolicy = Layer2 | Layer23 | Layer34

public export
Eq HashPolicy where
  Layer2 == Layer2 = True
  Layer23 == Layer23 = True
  Layer34 == Layer34 = True
  _ == _ = False

public export
showHashPolicy : HashPolicy -> String
showHashPolicy Layer2 = "layer2"
showHashPolicy Layer23 = "layer2+3"
showHashPolicy Layer34 = "layer3+4"

public export
data BondSelection = Stable | Bandwidth | Count

public export
Eq BondSelection where
  Stable == Stable = True
  Bandwidth == Bandwidth = True
  Count == Count = True
  _ == _ = False

public export
showBondSelection : BondSelection -> String
showBondSelection Stable = "stable"
showBondSelection Bandwidth = "bandwidth"
showBondSelection Count = "count"

public export
data LACPRate = Fast | Slow

public export
Eq LACPRate where
  Fast == Fast = True
  Slow == Slow = True
  _ == _ = False

public export
showLACPRate : LACPRate -> String
showLACPRate Fast = "fast"
showLACPRate Slow = "slow"

public export
data InterfaceProtocol = Static | DHCPClient | DHCPv6Client | Unnumbered

public export
Eq InterfaceProtocol where
  Static == Static = True
  DHCPClient == DHCPClient = True
  DHCPv6Client == DHCPv6Client = True
  Unnumbered == Unnumbered = True
  _ == _ = False

public export
showInterfaceProtocol : InterfaceProtocol -> String
showInterfaceProtocol Static = "static"
showInterfaceProtocol DHCPClient = "dhcp"
showInterfaceProtocol DHCPv6Client = "dhcpv6"
showInterfaceProtocol Unnumbered = "none"

public export
data AddressRequest = TryAddress | ForceAddress | NoAddress

public export
Eq AddressRequest where
  TryAddress == TryAddress = True
  ForceAddress == ForceAddress = True
  NoAddress == NoAddress = True
  _ == _ = False

public export
showAddressRequest : AddressRequest -> String
showAddressRequest TryAddress = "try"
showAddressRequest ForceAddress = "force"
showAddressRequest NoAddress = "none"

public export
data PrefixRequest = AutoPrefix | NoPrefix

public export
Eq PrefixRequest where
  AutoPrefix == AutoPrefix = True
  NoPrefix == NoPrefix = True
  _ == _ = False

public export
showPrefixRequest : PrefixRequest -> String
showPrefixRequest AutoPrefix = "auto"
showPrefixRequest NoPrefix = "no"

public export
data ServerMode = Server | Disabled

public export
Eq ServerMode where
  Server == Server = True
  Disabled == Disabled = True
  _ == _ = False

public export
showServerMode : ServerMode -> String
showServerMode Server = "server"
showServerMode Disabled = "disabled"

public export
data RAPreference = Medium | High | Low

public export
Eq RAPreference where
  Medium == Medium = True
  High == High = True
  Low == Low = True
  _ == _ = False

public export
showRAPreference : RAPreference -> String
showRAPreference Medium = "medium"
showRAPreference High = "high"
showRAPreference Low = "low"

public export
data RAFlag = ManagedConfig | OtherConfig | NoFlags

public export
Eq RAFlag where
  ManagedConfig == ManagedConfig = True
  OtherConfig == OtherConfig = True
  NoFlags == NoFlags = True
  _ == _ = False

public export
showRAFlag : RAFlag -> String
showRAFlag ManagedConfig = "managed-config"
showRAFlag OtherConfig = "other-config"
showRAFlag NoFlags = "none"

public export
data Verdict = Accept | Reject | Drop

public export
Eq Verdict where
  Accept == Accept = True
  Reject == Reject = True
  Drop == Drop = True
  _ == _ = False

public export
showVerdict : Verdict -> String
showVerdict Accept = "ACCEPT"
showVerdict Reject = "REJECT"
showVerdict Drop = "DROP"

public export
data RuleFamily = Family4 | Family6 | BothFamilies

public export
Eq RuleFamily where
  Family4 == Family4 = True
  Family6 == Family6 = True
  BothFamilies == BothFamilies = True
  _ == _ = False

public export
showRuleFamily : RuleFamily -> String
showRuleFamily Family4 = "ipv4"
showRuleFamily Family6 = "ipv6"
showRuleFamily BothFamilies = "any"

public export
data IPProtocol = ProtoAll | ProtoTCP | ProtoUDP | ProtoICMP | ProtoICMP6 | ProtoIGMP | ProtoESP

public export
Eq IPProtocol where
  ProtoAll == ProtoAll = True
  ProtoTCP == ProtoTCP = True
  ProtoUDP == ProtoUDP = True
  ProtoICMP == ProtoICMP = True
  ProtoICMP6 == ProtoICMP6 = True
  ProtoIGMP == ProtoIGMP = True
  ProtoESP == ProtoESP = True
  _ == _ = False

public export
showIPProtocol : IPProtocol -> String
showIPProtocol ProtoAll = "all"
showIPProtocol ProtoTCP = "tcp"
showIPProtocol ProtoUDP = "udp"
showIPProtocol ProtoICMP = "icmp"
showIPProtocol ProtoICMP6 = "icmpv6"
showIPProtocol ProtoIGMP = "igmp"
showIPProtocol ProtoESP = "esp"

public export
data RadioDriver = Mac80211

public export
Eq RadioDriver where
  Mac80211 == Mac80211 = True

public export
showRadioDriver : RadioDriver -> String
showRadioDriver Mac80211 = "mac80211"

public export
data Band = Band2 | Band5 | Band6

public export
Eq Band where
  Band2 == Band2 = True
  Band5 == Band5 = True
  Band6 == Band6 = True
  _ == _ = False

public export
showBand : Band -> String
showBand Band2 = "2g"
showBand Band5 = "5g"
showBand Band6 = "6g"

public export
data ChannelWidth = HT20 | HT40 | HE20 | HE40 | HE80 | HE160 | VHT20 | VHT40 | VHT80 | VHT160

public export
Eq ChannelWidth where
  HT20 == HT20 = True
  HT40 == HT40 = True
  HE20 == HE20 = True
  HE40 == HE40 = True
  HE80 == HE80 = True
  HE160 == HE160 = True
  VHT20 == VHT20 = True
  VHT40 == VHT40 = True
  VHT80 == VHT80 = True
  VHT160 == VHT160 = True
  _ == _ = False

public export
showChannelWidth : ChannelWidth -> String
showChannelWidth HT20 = "HT20"
showChannelWidth HT40 = "HT40"
showChannelWidth HE20 = "HE20"
showChannelWidth HE40 = "HE40"
showChannelWidth HE80 = "HE80"
showChannelWidth HE160 = "HE160"
showChannelWidth VHT20 = "VHT20"
showChannelWidth VHT40 = "VHT40"
showChannelWidth VHT80 = "VHT80"
showChannelWidth VHT160 = "VHT160"

public export
data WiFiMode = AccessPoint | Station

public export
Eq WiFiMode where
  AccessPoint == AccessPoint = True
  Station == Station = True
  _ == _ = False

public export
showWiFiMode : WiFiMode -> String
showWiFiMode AccessPoint = "ap"
showWiFiMode Station = "sta"

public export
data Security = Open | WPA2 | WPA3

public export
Eq Security where
  Open == Open = True
  WPA2 == WPA2 = True
  WPA3 == WPA3 = True
  _ == _ = False

public export
showSecurity : Security -> String
showSecurity Open = "none"
showSecurity WPA2 = "psk2"
showSecurity WPA3 = "sae"

public export
record GlobalOptions where
  constructor MkGlobalOptions
  dhcpDefaultDuid : Maybe String
  ulaPrefix : Maybe IPv6Prefix
  packetSteering : Maybe Bool

public export
record BondOptions where
  constructor MkBondOptions
  mtu : Maybe Nat
  macAddress : Maybe String
  policy : Maybe BondPolicy
  hashPolicy : Maybe HashPolicy
  selection : Maybe BondSelection
  lacpRate : Maybe LACPRate
  minLinks : Maybe Nat
  monitorInterval : Maybe Nat

public export
record InterfaceOptions where
  constructor MkInterfaceOptions
  protocol : Maybe InterfaceProtocol
  ipv6Assignment : Maybe Nat
  multipath : Maybe Bool
  requestAddress : Maybe AddressRequest
  requestPrefix : Maybe PrefixRequest
  noRelease : Maybe Bool

public export
record DNSOptions where
  constructor MkDNSOptions
  domainneeded : Maybe Bool
  boguspriv : Maybe Bool
  filterwin2k : Maybe Bool
  localiseQueries : Maybe Bool
  rebindProtection : Maybe Bool
  rebindLocalhost : Maybe Bool
  expandhosts : Maybe Bool
  nonegcache : Maybe Bool
  authoritative : Maybe Bool
  readethers : Maybe Bool
  nonwildcard : Maybe Bool
  localservice : Maybe Bool
  filterAaaa : Maybe Bool
  filterA : Maybe Bool
  local : Maybe String
  domain : Maybe String
  leasefile : Maybe String
  resolvfile : Maybe String
  cacheSize : Maybe Nat
  ednsPacketMax : Maybe Nat
  leaseMax : Maybe Nat

public export
record DHCPOptions where
  constructor MkDHCPOptions
  ignore : Maybe Bool
  leaseTime : Maybe String
  ipv4 : Maybe ServerMode
  ipv6 : Maybe ServerMode
  ra : Maybe ServerMode
  raPreference : Maybe RAPreference

public export
record ODHCPOptions where
  constructor MkODHCPOptions
  mainDhcp : Maybe Bool
  leasefile : Maybe String
  leaseTrigger : Maybe String
  logLevel : Maybe Nat
  pioDirectory : Maybe String
  hostsDirectory : Maybe String

public export
record FirewallDefaults where
  constructor MkFirewallDefaults
  synFlood : Maybe Bool
  input : Maybe Verdict
  output : Maybe Verdict
  forward : Maybe Verdict

public export
record ZoneOptions where
  constructor MkZoneOptions
  input : Maybe Verdict
  output : Maybe Verdict
  forward : Maybe Verdict
  masquerade : Maybe Bool
  mssAdjust : Maybe Bool

public export
record RuleOptions where
  constructor MkRuleOptions
  family : Maybe RuleFamily
  destinationPort : Maybe Nat
  limit : Maybe String
  action : Maybe Verdict

public export
record RadioOptions where
  constructor MkRadioOptions
  driver : Maybe RadioDriver
  path : Maybe String
  band : Maybe Band
  channel : Maybe Nat
  width : Maybe ChannelWidth
  country : Maybe String
  cellDensity : Maybe Nat

public export
record WiFiOptions where
  constructor MkWiFiOptions
  mode : Maybe WiFiMode
  ssid : Maybe String
  security : Maybe Security
  disabled : Maybe Bool
  ocv : Maybe Bool
  wds : Maybe Bool
  hidden : Maybe Bool
  bssid : Maybe String
  macAddress : Maybe String

