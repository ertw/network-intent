# Independent semantic API probes

These are test sources, not application modules. Build them in a temporary source
root containing a `NetDSL` symlink to the repository's `src/NetDSL` directory;
use separate temporary build and executable directories. Do not include every
file in one package: the negative fixtures are intentionally ill-typed.

- `CoreAPI.idr`: 34 direct API assertions covering valid models, malformed
  programmatically constructed networks, reference kinds and bounds, source
  value checks, route/service dependencies, diagnostic codes, graph input
  rejection, and directed cycle witnesses. Compile and run; exit status must be 0.
- `GraphProperties.idr`: 640 graph cases, including all 512 directed graphs on
  three vertices and 128 four-vertex graphs selected with Python's
  `random.Random(912).sample(range(65536), 128)`. Expected acyclicity decisions
  were computed independently with Python Kahn traversal. Each successful
  certificate and rejected graph's closed directed cycle witness is also
  checked. Compile and run; both reported totals must be 640/640.
- `Forged*.idr` and `WrongRefKind.idr`: nine negative fixtures. Type-check each
  individually and require a nonzero exit status with a type mismatch or
  unsolved proof constraint. These protect IPv4/VLAN bounds, canonical prefix
  addresses, prefix width and allocation-size consistency, graph inventory
  uniqueness, graph coverage and edge order, and reference-kind separation.

The last isolated Idris 2 version 0.8.0 run passed all 34 direct assertions, all 640
acyclicity decisions and witnesses/certificates, and all nine compile-negative
fixtures. No trusted-core escape hatches are used in these probes.


`RouterAPI.idr` elaborates the sanitized core-router example and mutates its
public model before recertification. It checks references, attachment cycles,
bond bounds/membership, DHCP/firewall constraints, radio and credential validation,
and synthetic UCI quoting. `ForgedIPv6` and `ForgedPrefix6` must fail at compile
time; `WrongRouterRef` and `WrongWiFiRef` reject reference-kind substitution.
