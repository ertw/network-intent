module WrongWirelessEndpoint
import NetDSL.Domain.Model
%default total
bad : Ref DeviceKind -> RouterRef RadioEntity -> WiFiRef
bad device radio = WiFiId device radio
