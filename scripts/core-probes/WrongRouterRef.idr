module WrongRouterRef
import NetDSL.Router.Model
%default total
bad : RouterRef PhysicalEntity -> RouterRef InterfaceEntity
bad ref = ref
