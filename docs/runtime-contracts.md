# Runtime contracts, version 1

Status: initial implementation contracts, not a completed acceptance claim. The
coordinator owns `runtime/protocol`, compiler admission, and this document.

## Boundaries

- `runtime/protocol`: serde wire records and validation without execution.
- `runtime/controller`: its own SQLite persistence, planning and orchestration.
- `runtime/witness-agent`: observation-only binary and execution implementation.
- `runtime/apply-agent`: separate binary, identity, database and deployment logic.
- Idris: authoritative parsing/validation, target compilation, semantic claim
  derivation and independent realization checking. A client-supplied `ok` flag
  is never sufficient for promotion. Bind checks to the exact source bytes,
  revision, target artifacts, current profile and complete assurance graph.

## Storage and delivery

Controller storage receives exact serialized bytes; hashes are SHA-256 of those
bytes, not of reserialized JSON. An ID collision with different bytes is an
error. Durably receive before acknowledgement, serialize writes, enforce queue
bounds, and persist fencing generations and epochs. A retry of an uncertain
apply must reconcile its journal; transport deduplication cannot prove that a
physical operation ran exactly once. Storage is not signature authorization.

## Probe interface

`ProbeSpec` is the agreed execution input; `ProbeResult` is its output. Every
network probe has a concrete IP endpoint, explicit IP family, witness identity,
location, source bind address, optional interface, expectation, deadline, byte
limit and bounded attempts. Device-local UCI/netifd probes have no IP endpoint.
Reject malformed or mismatched inputs, unsupported bindings and unsupported
claims. Do not silently fall back to a different source interface. TCP success
is transport reachability, not application health. Timeout is ambiguous and
cannot prove isolation. DNS transport and response errors remain distinct.

UCI values preserve ordered list items and section order. Observation must
identify committed configuration versus a particular session's staging; UCI
readback cannot establish that netifd has adopted the configuration. Redact
credentials before any serialization, database write or telemetry event.

## State and assurance

Configuration snapshots, operational snapshots, receipts and evidence have
different types. Collection includes scope, completeness, provenance, timestamps
and explicit freshness parameters. Unknown, missing, future-dated, stale or
partial evidence does not count as success. Health thresholds are three
consecutive violations to open an incident, two fresh successes to recover.
Intervals: netifd 15s, active probes 30s, UCI 60s, heartbeat 10s. Freshness is
three intervals plus invocation timeout. No enabled hardware profiles yet.

## Initial workers

Workers can implement the isolated controller storage and witness execution
packages against these records. They must not mutate shared schemas without
coordinator review. No interchangeable common runtime for witness/apply agents.
