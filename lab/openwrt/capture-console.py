#!/usr/bin/env python3
"""Boot one isolated pinned VM, capture real console diagnostics, then stop it."""
import argparse
import hashlib
import json
import re
from pathlib import Path
import socket
import subprocess
import time

ROOT = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument("version", choices=["25.12.5", "24.10.8"])
parser.add_argument("--from-log", type=Path, help="validate a previously captured transcript without booting")
parser.add_argument("--guest-script", type=Path, help="run additional acceptance checks inside the disposable VM")
parser.add_argument("--require-marker", help="require this exact output line from the guest checks")
args = parser.parse_args()
run = ROOT / "state" / "run"
label = args.guest_script.stem if args.guest_script else "diagnostic"
log = run / f"{args.version}-{label}.log"
reads = {
    "uci-network": "ubus call uci get '{\"config\":\"network\"}'",
    "uci-changes": "ubus call uci changes '{\"config\":\"network\"}'",
    "netifd-interfaces": "ubus call network.interface dump",
    "netifd-devices": "ubus call network.device status",
    "netifd-wireless": "ubus call network.wireless status",
}
commands = (
    "n=0; while [ \"$n\" -lt 90 ]; do "
    "[ \"$(ubus list network.interface 2>/dev/null)\" = network.interface ] && break; "
    "n=$((n+1)); sleep 1; done\n"
)
if args.guest_script:
    digest = hashlib.sha256(args.guest_script.read_bytes()).hexdigest()
    commands += f"[ \"$(sha256sum /tmp/intent-guest.sh | cut -d ' ' -f 1)\" = {digest} ] || {{ printf '\\n%s%s\\n' INTENT_GUEST_STATUS_ 97; exit 97; }}\n"
    commands += "sh /tmp/intent-guest.sh; guest_status=$?\n"
    commands += "printf '\\n%s%s\\n' INTENT_GUEST_STATUS_ \"$guest_status\"\n"
    commands += "[ \"$guest_status\" -eq 0 ] || exit \"$guest_status\"\n"
commands += (
    "cat /etc/openwrt_release; uptime; ubus list; /etc/init.d/network status\n"
    "ps w; logread -e netifd\n"
    "ubus -v list uci; ubus -v list network.interface\n"
    "ubus -v list network.device; ubus -v list network.wireless\n"
    "if command -v apk >/dev/null; then apk list --installed; else opkg list-installed; fi\n"
)
for label, command in reads.items():
    commands += (
        f"printf '\\n%s%s\\n' INTENT_BEGIN_ {label}; {command}; "
        f"printf '\\n%s%s\\n' INTENT_END_ {label}\n"
    )
commands += "printf '\\n%s%s\\n' INTENT_ LAB_DONE\n"

def extract_fixtures(raw):
    transcript = raw.decode(errors="strict").replace("\r", "")
    if f"DISTRIB_RELEASE='{args.version}'" not in transcript or "\nINTENT_LAB_DONE\n" not in transcript:
        raise RuntimeError("transcript lacks actual release output or completion marker")
    if args.require_marker and f"\n{args.require_marker}\n" not in transcript:
        raise RuntimeError("guest acceptance marker missing")
    if args.guest_script:
        statuses = re.findall(r"^INTENT_GUEST_STATUS_([0-9]+)$", transcript, re.MULTILINE)
        if statuses != ["0"]:
            raise RuntimeError("transcript lacks a single successful guest exit status")
    parsed = {}
    unavailable = []
    for label in reads:
        match = re.search(
            rf"^INTENT_BEGIN_{label}\n(.*?)^INTENT_END_{label}$",
            transcript, re.MULTILINE | re.DOTALL,
        )
        if not match:
            raise RuntimeError(f"missing real observation: {label}")
        response = match.group(1).strip()
        if label == "netifd-wireless" and response == "Command failed: Not found":
            unavailable.append("network.wireless")
            continue
        parsed[label] = json.loads(response)
    interfaces = parsed["netifd-interfaces"].get("interface", [])
    if not any(i.get("interface") == "loopback" and i.get("up") is True for i in interfaces):
        raise RuntimeError("netifd did not report loopback up")
    if not parsed["uci-network"].get("values"):
        raise RuntimeError("no actual UCI network sections")
    if args.guest_script:
        print(f"Guest acceptance and {len(parsed)} real readback responses validated for {args.version}")
        return
    fixtures = ROOT / "fixtures" / args.version
    fixtures.mkdir(parents=True, exist_ok=True)
    for label, value in parsed.items():
        (fixtures / f"{label}.json").write_text(json.dumps(value, indent=2) + "\n")
    (fixtures / "capabilities.json").write_text(json.dumps({
        "release": args.version, "unavailable_objects": unavailable,
        "collection": "direct root ubus baseline; session permission acceptance is separate",
    }, indent=2) + "\n")
    print(f"Captured {len(parsed)} real ubus response fixtures for {args.version}")

if args.from_log:
    extract_fixtures(args.from_log.read_bytes())
    raise SystemExit(0)

subprocess.run([str(ROOT / "openwrt-lab"), "launch", args.version], check=True)
try:
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as console, log.open("wb") as output:
        console.settimeout(1)
        console.connect(str(run / "serial.sock"))
        pending = b""
        sent = False
        last_enter = 0.0
        until = time.monotonic() + 300
        while time.monotonic() < until:
            try:
                chunk = console.recv(65536)
                if not chunk:
                    raise RuntimeError("serial connection closed before acceptance marker")
                output.write(chunk)
                output.flush()
                pending = (pending + chunk)[-65536:]
            except socket.timeout:
                pass
            if not sent and (b"root@OpenWrt:" in pending or b"root@(none):" in pending):
                # Pace short shell lines: a single long serial write can
                # overrun the emulated UART and leave ash in a quoted prompt.
                transfer = ""
                if args.guest_script:
                    guest = args.guest_script.read_text()
                    if "INTENT_GUEST" in guest.splitlines():
                        raise ValueError("guest script collides with transfer delimiter")
                    transfer += "cat > /tmp/intent-guest.sh <<'INTENT_GUEST'\n"
                    transfer += guest.rstrip("\n") + "\nINTENT_GUEST\n"
                transfer = (transfer + "cat > /tmp/intent-capture.sh <<'INTENT_SCRIPT'\n"
                    + commands + "INTENT_SCRIPT\nsh /tmp/intent-capture.sh\n"
                ).replace("\n", "\r").encode()
                for offset in range(0, len(transfer), 64):
                    console.sendall(transfer[offset:offset + 64])
                    time.sleep(0.05)
                pending = b""
                sent = True
            elif not sent and b"Please press Enter" in pending and time.monotonic() - last_enter > 3:
                console.sendall(b"\r")
                last_enter = time.monotonic()
            guest_status = re.search(rb"\r\nINTENT_GUEST_STATUS_([0-9]+)\r\n", pending)
            if guest_status and guest_status.group(1) != b"0":
                raise RuntimeError(f"guest acceptance failed with status {guest_status.group(1).decode()}; see {log}")
            if sent and b"\r\nINTENT_LAB_DONE\r\n" in pending:
                if f"DISTRIB_RELEASE='{args.version}'".encode() not in pending:
                    raise RuntimeError("console command output did not verify the requested release")
                extract_fixtures(pending)
                print(f"Completed console capture: {log}")
                break
        else:
            raise TimeoutError(f"console acceptance marker not received; see {log}")
finally:
    subprocess.run([str(ROOT / "openwrt-lab"), "stop"], check=True, timeout=20)
