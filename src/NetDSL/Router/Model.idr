module NetDSL.Router.Model

import public NetDSL.Router.Options
import public NetDSL.Router.Address
import NetDSL.Common
import NetDSL.Domain.Address
import NetDSL.AAA

%default total

public export
data RouterEntity = PhysicalEntity | BridgeEntity | BondEntity | InterfaceEntity | ZoneEntity | RadioEntity | WiFiEntity

public export
record RouterRef (kind : RouterEntity) where
  constructor RRef
  index : Nat

public export
Eq (RouterRef k) where
  a == b = a.index == b.index

public export
data Attachment = Physical (RouterRef PhysicalEntity) | BridgeDevice (RouterRef BridgeEntity) | BondDevice (RouterRef BondEntity) | Loopback | Unattached

public export
record PhysicalPort where
  constructor Ethernet
  name : String
  source : SourceSpan

public export
record Bridge where
  constructor MkBridge
  name : String
  members : List (Located Attachment)
  stp : Maybe Bool
  source : SourceSpan

public export
record Bond where
  constructor MkBond
  name : String
  members : List (Located (RouterRef PhysicalEntity))
  settings : BondOptions
  source : SourceSpan

public export
record LogicalInterface where
  constructor MkInterface
  name : String
  attachment : Located Attachment
  addresses : List (Located Address4)
  addresses6 : List (Located Address6)
  gateway : Maybe (Located IPv4)
  dnsServers : List (Located IPv4)
  settings : InterfaceOptions
  source : SourceSpan

public export
record DHCPServer where
  constructor MkDHCPServer
  name : String
  ifaceRef : Located (RouterRef InterfaceEntity)
  pool : Maybe (Located IPv4, Located IPv4)
  flags : List RAFlag
  settings : DHCPOptions
  source : SourceSpan

public export
record FirewallZone where
  constructor MkZone
  name : String
  interfaces : List (Located (RouterRef InterfaceEntity))
  settings : ZoneOptions
  source : SourceSpan

public export
data RuleDestination = LocalInput | AnyZone | NamedZone (RouterRef ZoneEntity)

public export
data SourcePrefix = Source4 IPv4Prefix | Source6 IPv6Prefix

public export
record FirewallRule where
  constructor MkFirewallRule
  name : String
  from : Located (RouterRef ZoneEntity)
  destination : Located RuleDestination
  protocols : List IPProtocol
  sourcePrefix : Maybe SourcePrefix
  icmpTypes : List String
  settings : RuleOptions
  source : SourceSpan

public export
record Forwarding where
  constructor MkForwarding
  from : Located (RouterRef ZoneEntity)
  destination : Located (RouterRef ZoneEntity)
  source : SourceSpan

public export
record Radio where
  constructor MkRadio
  name : String
  settings : RadioOptions
  source : SourceSpan

public export
record WiFiInterface where
  constructor MkWiFiInterface
  name : String
  radio : Located (RouterRef RadioEntity)
  ifaceRef : Located (RouterRef InterfaceEntity)
  credential : Maybe (SecretRef WiFiCredential)
  settings : WiFiOptions
  source : SourceSpan

public export
record RouterConfig where
  constructor MkRouterConfig
  physicals : List PhysicalPort
  bridges : List Bridge
  bonds : List Bond
  interfaces : List LogicalInterface
  globals : Maybe (Located GlobalOptions)
  dns : Maybe (Located DNSOptions)
  dhcpServers : List DHCPServer
  odhcp : Maybe (Located ODHCPOptions)
  defaults : Located FirewallDefaults
  zones : List FirewallZone
  forwardings : List Forwarding
  rules : List FirewallRule
  radios : List Radio
  wifiInterfaces : List WiFiInterface
  source : SourceSpan
