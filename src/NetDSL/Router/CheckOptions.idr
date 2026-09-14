module NetDSL.Router.CheckOptions

import NetDSL.Common
import NetDSL.Router.Options
import NetDSL.Backend.AST
import Data.List

%default total

public export
checkGlobalOptions : SourceSpan -> GlobalOptions -> List Diagnostic
checkGlobalOptions at o =
  maybe [] (\v => if safeLine v && length v <= 1024 then [] else [failure "router.invalid-setting" at "Invalid dhcp-default-duid"]) o.dhcpDefaultDuid

public export
checkBondOptions : SourceSpan -> BondOptions -> List Diagnostic
checkBondOptions at o =
  maybe [] (\v => if v >= 68 && v <= 65535 then [] else [failure "router.invalid-setting" at "Invalid mtu"]) o.mtu ++
  maybe [] (\v => if safeLine v && length v <= 1024 then [] else [failure "router.invalid-setting" at "Invalid mac-address"]) o.macAddress ++
  maybe [] (\v => if v >= 1 && v <= 65535 then [] else [failure "router.invalid-setting" at "Invalid min-links"]) o.minLinks ++
  maybe [] (\v => if v >= 1 && v <= 4294967295 then [] else [failure "router.invalid-setting" at "Invalid monitor-interval"]) o.monitorInterval

public export
checkInterfaceOptions : SourceSpan -> InterfaceOptions -> List Diagnostic
checkInterfaceOptions at o =
  maybe [] (\v => if v >= 0 && v <= 128 then [] else [failure "router.invalid-setting" at "Invalid ipv6-assignment"]) o.ipv6Assignment

public export
checkDNSOptions : SourceSpan -> DNSOptions -> List Diagnostic
checkDNSOptions at o =
  maybe [] (\v => if safeLine v && length v <= 1024 then [] else [failure "router.invalid-setting" at "Invalid local"]) o.local ++
  maybe [] (\v => if safeLine v && length v <= 1024 then [] else [failure "router.invalid-setting" at "Invalid domain"]) o.domain ++
  maybe [] (\v => if safeLine v && length v <= 1024 then [] else [failure "router.invalid-setting" at "Invalid leasefile"]) o.leasefile ++
  maybe [] (\v => if safeLine v && length v <= 1024 then [] else [failure "router.invalid-setting" at "Invalid resolvfile"]) o.resolvfile ++
  maybe [] (\v => if v >= 0 && v <= 1000000 then [] else [failure "router.invalid-setting" at "Invalid cache-size"]) o.cacheSize ++
  maybe [] (\v => if v >= 512 && v <= 65535 then [] else [failure "router.invalid-setting" at "Invalid edns-packet-max"]) o.ednsPacketMax ++
  maybe [] (\v => if v >= 1 && v <= 65535 then [] else [failure "router.invalid-setting" at "Invalid lease-max"]) o.leaseMax

public export
checkDHCPOptions : SourceSpan -> DHCPOptions -> List Diagnostic
checkDHCPOptions at o =
  maybe [] (\v => if safeLine v && length v <= 1024 then [] else [failure "router.invalid-setting" at "Invalid lease-time"]) o.leaseTime

public export
checkODHCPOptions : SourceSpan -> ODHCPOptions -> List Diagnostic
checkODHCPOptions at o =
  maybe [] (\v => if safeLine v && length v <= 1024 then [] else [failure "router.invalid-setting" at "Invalid leasefile"]) o.leasefile ++
  maybe [] (\v => if safeLine v && length v <= 1024 then [] else [failure "router.invalid-setting" at "Invalid lease-trigger"]) o.leaseTrigger ++
  maybe [] (\v => if v >= 0 && v <= 7 then [] else [failure "router.invalid-setting" at "Invalid log-level"]) o.logLevel ++
  maybe [] (\v => if safeLine v && length v <= 1024 then [] else [failure "router.invalid-setting" at "Invalid pio-directory"]) o.pioDirectory ++
  maybe [] (\v => if safeLine v && length v <= 1024 then [] else [failure "router.invalid-setting" at "Invalid hosts-directory"]) o.hostsDirectory

public export
checkFirewallDefaults : SourceSpan -> FirewallDefaults -> List Diagnostic
checkFirewallDefaults at o =
  []

public export
checkZoneOptions : SourceSpan -> ZoneOptions -> List Diagnostic
checkZoneOptions at o =
  []

public export
checkRuleOptions : SourceSpan -> RuleOptions -> List Diagnostic
checkRuleOptions at o =
  maybe [] (\v => if v >= 1 && v <= 65535 then [] else [failure "router.invalid-setting" at "Invalid destination-port"]) o.destinationPort ++
  maybe [] (\v => if safeLine v && length v <= 1024 then [] else [failure "router.invalid-setting" at "Invalid limit"]) o.limit

public export
checkRadioOptions : SourceSpan -> RadioOptions -> List Diagnostic
checkRadioOptions at o =
  maybe [] (\v => if safeLine v && length v <= 1024 then [] else [failure "router.invalid-setting" at "Invalid path"]) o.path ++
  maybe [] (\v => if v >= 1 && v <= 233 then [] else [failure "router.invalid-setting" at "Invalid channel"]) o.channel ++
  maybe [] (\v => if safeLine v && length v <= 1024 then [] else [failure "router.invalid-setting" at "Invalid country"]) o.country ++
  maybe [] (\v => if v >= 0 && v <= 3 then [] else [failure "router.invalid-setting" at "Invalid cell-density"]) o.cellDensity

public export
checkAPOptions : SourceSpan -> APOptions -> List Diagnostic
checkAPOptions at o =
  maybe [] (\v => if safeLine v && length v <= 1024 then [] else [failure "router.invalid-setting" at "Invalid ssid"]) o.ssid

