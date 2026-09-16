# Native read-only observation worker

Build `intent-ubus-observe` with the `native-ubus` feature on Linux against the
target OpenWrt libubus/libubox headers and libraries. Install that binary at
`/usr/libexec/intent-ubus-observe`, owned by the administrator and not writable by
the witness service. The supervisor deliberately has no environment or assignment
override for its executable path. It does not invoke the ubus or UCI CLI.

Each transport invocation starts a fresh helper with `Command::spawn` (exec),
sends one length-prefixed JSON request over private socket-backed stdin/stdout,
and accepts exactly one status/length/payload frame and successful process exit.
The request includes the rpcd session in IPC, never in the command line. The helper
validates the read-only method allowlist again and makes exactly one C-shim call.
This keeps libubus global state out of concurrent calls in the witness process.

The parent starts a monotonic deadline before serialization and spawning, then
uses its remaining duration for every IPC operation and checks it after parsing.
It kills and reaps the helper on timeout, malformed/oversized output, and all other
return paths. The deadline includes native connect and object lookup, which the
libubus per-invocation timeout alone does not bound. Scheduling delays and kernel
process creation/termination latency may extend wall-clock return slightly beyond
the budget; this is not a hard real-time guarantee. Cleanup deliberately waits for
reaping before releasing capacity rather than leaving unbounded background work.

Hard ceilings are four concurrent helpers per supervisor process, 64 KiB encoded
request, 16 MiB response, and 30 seconds per invocation. Response limits must be
nonzero and at most that ceiling; lower caller limits are enforced. Longer caller
timeouts are clamped to 30 seconds. A full worker pool fails immediately. These
are application limits, not a cross-process cgroup/resource policy. The helper is
an internal executable: launching it directly bypasses supervisor deadlines.

Native status codes 7001 (too large), 7002 (timeout), 7003 (permission denied),
and 7004 (malformed/multipart/empty) retain their observation error meanings.
Missing executables, failed helper exit and transport failures cannot produce
successful evidence. Duplicate-key JSON remains rejected by the parent parser.

Host tests cover deadline expiry, stalled exit, slowly trickled bytes, oversize
headers, truncated/extra frames, unsuccessful exit, actual child reaping, capacity
release, request limits and native error mapping. They use shell fixtures only in
tests. Host tests do not verify Linux/OpenWrt ABI compatibility or live rpcd ACLs;
native-feature compilation and live worker execution must be verified separately
for each supported OpenWrt target.
