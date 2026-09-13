module NetDSL.Domain.Model

import NetDSL.Common
import NetDSL.Domain.Address
import Data.List

%default total

public export
data EntityKind = VLAN | DeviceKind | HostKind | ServiceKind | RouteKind

public export
record Ref (kind : EntityKind) where
  constructor Id
  index : Nat

public export
Eq (Ref kind) where
  (==) a b = a.index == b.index

public export
record PortRef where
  constructor PortId
  owner : Ref DeviceKind
  index : Nat

public export
Eq PortRef where
  (==) a b = a.owner == b.owner && a.index == b.index

public export
data LinkEnd = DevicePort PortRef | HostPort (Ref HostKind)

public export
Eq LinkEnd where
  (==) (DevicePort a) (DevicePort b) = a == b
  (==) (HostPort a) (HostPort b) = a == b
  (==) _ _ = False

public export
record HostAssignment where
  constructor HostAt
  name : String
  address : Located IPv4
  source : SourceSpan

public export
record DHCPRange where
  constructor Pool
  first : Located IPv4
  last : Located IPv4

public export
record Vlan where
  constructor MkVlan
  name : String
  vid : VlanId
  subnet : Located IPv4Prefix
  gateway : Maybe (Located IPv4)
  dhcp : Maybe DHCPRange
  hosts : List HostAssignment
  source : SourceSpan

public export
data PortMode = Access (Ref VLAN) | Trunk (List (Ref VLAN))

public export
memberships : PortMode -> List (Ref VLAN)
memberships (Access v) = [v]
memberships (Trunk vs) = vs

public export
record Port where
  constructor MkPort
  owner : Ref DeviceKind
  name : String
  description : Maybe String
  mode : PortMode
  peer : Maybe (Located LinkEnd)
  source : SourceSpan

public export
data Driver = OpenWrt | CiscoIOS

public export
Eq Driver where
  OpenWrt == OpenWrt = True
  CiscoIOS == CiscoIOS = True
  _ == _ = False

public export
driverName : Driver -> String
driverName OpenWrt = "openwrt"
driverName CiscoIOS = "cisco-ios"

public export
record Device where
  constructor MkDevice
  name : String
  isRouter : Bool
  driver : Driver
  ports : List Port
  maxTagged : Nat
  source : SourceSpan

public export
data Transport = TCP | UDP

public export
Eq Transport where
  TCP == TCP = True
  UDP == UDP = True
  _ == _ = False

public export
transportName : Transport -> String
transportName TCP = "tcp"
transportName UDP = "udp"

public export
record Service where
  constructor MkService
  name : String
  transports : List (Transport, Integer)
  dependencies : List (Located (Ref ServiceKind))
  source : SourceSpan

public export
data PolicyEndpoint = Zone (Ref VLAN) | Internet | Gateway (Ref DeviceKind)

public export
Eq PolicyEndpoint where
  Zone a == Zone b = a == b
  Internet == Internet = True
  Gateway a == Gateway b = a == b
  _ == _ = False

public export
data Action = Allow | Deny

public export
actionName : Action -> String
actionName Allow = "allow"
actionName Deny = "deny"

public export
record Policy where
  constructor Rule
  from : Ref VLAN
  destination : PolicyEndpoint
  action : Action
  services : List (Ref ServiceKind)
  source : SourceSpan

public export
record Route where
  constructor MkRoute
  name : String
  destination : Located IPv4Prefix
  nextHop : Located IPv4
  vlan : Ref VLAN
  device : Ref DeviceKind
  metric : Integer
  dependencies : List (Located (Ref RouteKind))
  source : SourceSpan

public export
data SchemaVersion = V1

public export
record Network (version : SchemaVersion) where
  constructor MkNetwork
  name : String
  domain : Maybe String
  vlans : List Vlan
  devices : List Device
  services : List Service
  routes : List Route
  policies : List Policy
  enforcer : Maybe (Ref DeviceKind)
  source : SourceSpan

public export
allHosts : Network v -> List (Ref VLAN, HostAssignment)
allHosts n = concatMap (\(i,v) => map (\h => (Id i,h)) v.hosts) (indexed n.vlans)

public export
allPorts : Network v -> List (PortRef, Port)
allPorts n = concatMap (\(i,d) => map (\(j,p) => (PortId (Id i) j,p)) (indexed d.ports)) (indexed n.devices)

public export
getVlan : Network v -> Ref VLAN -> Maybe Vlan
getVlan n r = lookupAt r.index n.vlans

public export
getDevice : Network v -> Ref DeviceKind -> Maybe Device
getDevice n r = lookupAt r.index n.devices

public export
getPort : Network v -> PortRef -> Maybe Port
getPort n r = getDevice n r.owner >>= lookupAt r.index . ports

public export
serviceEdges : Network v -> List (Nat, Nat)
serviceEdges n = concatMap (\(i,s) => map (\d => (d.value.index, i)) s.dependencies) (indexed n.services)

public export
routeEdges : Network v -> List (Nat, Nat)
routeEdges n = concatMap (\(i,r) => map (\d => (d.value.index, i)) r.dependencies) (indexed n.routes)
