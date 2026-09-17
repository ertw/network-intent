# Isolated OpenWrt VM lab

This lab is a disposable x86/64 OpenWrt environment for validating native `ubus`, `rpcd`, UCI apply/confirm/rollback, and witness/apply-agent behavior. It is not a production deployment path and does not touch a physical router or its network.

## Pinned artifacts

The harness pins the official non-EFI `generic-ext4-combined.img.gz` x86/64 images in [manifest.tsv](../lab/openwrt/manifest.tsv). The official 25.12.5 index lists its ext4 image as SHA-256 `23e2538e8ab0eb52dfed1c65d608ecdb71ffd432dd54885da138ae67cd9e4461`; the official 24.10.8 index lists `23872c64fdb66d0765e0d17f71453a66e7a91e9ecb10606d5f738e6d166e14ae`. Each release index also publishes `sha256sums` and a detached `sha256sums.asc` signature.

OpenWrt's public-key page gives primary fingerprint `8A8BC12F46B836C0F9CDB36F1D53D1877742E911` for the Nitrokey3 release key and signing-subkey fingerprint `92C561DE55AE6552F3C736B82B0151090606D1D9`; the 25.12.5 detached signature identifies that subkey. The harness imports only the explicitly supplied key, isolates the keyring by release, and requires GnuPG's `VALIDSIG` primary fingerprint to match the pinned primary fingerprint. It never fetches a key from a keyserver.

## Workflow

```sh
./lab/openwrt/openwrt-lab prepare
./lab/openwrt/openwrt-lab fetch 25.12.5
./lab/openwrt/openwrt-lab verify 25.12.5 /secure/path/openwrt-release-key.asc FULL_40_HEX_FINGERPRINT
./lab/openwrt/openwrt-lab launch 25.12.5
./lab/openwrt/openwrt-lab stop
```

All mutable files, including release-separated downloads and manifests, per-release GnuPG homes, extracted disk, PID, monitor socket, and serial socket, remain under `lab/openwrt/state` with private permissions. The VM has QEMU user networking with `restrict=on`: it gets no host forwarding and cannot reach the LAN or Internet. No bridge, TAP device, privileged network helper, or device profile is created.

`verify` requires all four checks: the supplied key contains the full, independently authenticated fingerprint, the pinned SHA-256 matches the official manifest, the downloaded image matches that hash, and GnuPG verifies the detached manifest signature with that imported key. `launch` rehashes the compressed image and binds it to its verification marker before decompression. It uses Ruby's standard zlib reader for CRC/ISIZE validation. A nonempty gzip `unused` region is accepted only when it is a valid OpenWrt fwtool signature container: `FWx0`, type 0, zero padding, complete-size match, 1 KiB limit, and the fwtool CRC over the preceding file. It writes the extracted disk to a temporary file and renames it only after every check. `stop` validates both the PID and its QEMU command line before signalling it.

## Verified baseline capture (2026-09-16)

The ARM64 macOS host runs Homebrew QEMU 11.1.1. Both pinned images passed a fresh coordinator SHA-256 and GnuPG signature verification. The primary fingerprint was independently matched against the [OpenWrt public-key listing](https://openwrt.org/docs/guide-user/security/signatures?s%5B%5D=02); release hashes match the official [25.12.5](https://downloads.openwrt.org/releases/25.12.5/targets/x86/64/) and [24.10.8](https://downloads.openwrt.org/releases/24.10.8/targets/x86/64/) indexes.

The VM needs `virtio-rng-pci` backed by host `/dev/urandom`: without it the fresh image's random generator remained uninitialized and network startup stalled. With it, both releases initialized randomness and started netifd. The collector paces short serial lines to avoid UART input loss and requires real command-output markers, the exact release, parseable JSON, and an operational loopback before saving fixtures.

```sh
python3 lab/openwrt/capture-console.py 25.12.5
python3 lab/openwrt/capture-console.py 24.10.8
```

Both clean captures completed and their VMs stopped. Raw transcripts are retained in `docs/verification-runs/openwrt-<version>-baseline.log`; parsed responses are in `lab/openwrt/fixtures/<version>/`. Captured observations include ordered UCI network sections, empty staging deltas, interface addresses, bridge membership, method signatures, and installed package versions. `network.wireless` is absent on this 25.12.5 x86 image; 24.10.8 returns an empty object. Capability metadata preserves that distinction. All addresses, MACs and DUIDs in these fixtures belong to disposable default VMs.

These baseline captures use direct root ubus reads. They prove the real response shapes and boot behavior, but do **not** prove witness read-only identity enforcement or native adapter runtime acceptance. Separate nonroot uid/ubusd ACL and rpcd session denial tests have since passed on both releases, as recorded below. Remaining acceptance includes the compiled native adapter, staging separation, timed apply/confirm/rollback, restart recovery, and all seven primitives. Physical hardware acceptance remains open; the user has no dedicated device yet.

### Native boundary constraints

rpcd UCI explicitly consumes `ubus_rpc_session` for its per-session save directory and package permission checks. netifd does not consume that field; the deployed witness also requires a dedicated non-root uid with a read-only ubusd ACL. See the primary [rpcd UCI implementation](https://github.com/openwrt/rpcd/blob/master/uci.c) and [ubusd ACL implementation](https://github.com/openwrt/ubus/blob/master/ubusd_acl.c). Rust checks authorization before and after netifd collection. Native helper deadline isolation is implemented and host-tested because libubus object lookup does not take an invocation timeout, and its global buffers are unsuitable for concurrent calls in a multithreaded process.

## Nonroot permission acceptance

Both pinned releases passed `check-witness-permissions.sh` on 2026-09-16. The
collector verifies the transferred script SHA-256 before execution. The guest
creates a dedicated disposable uid, installs a root-owned mode-0644 ubusd ACL
(`ubusd` itself runs as user `ubus`), and runs checks through procd under that uid.
The checks cover successful UCI/interface/device reads, denied UCI mutation,
denied interface mutation and session grant, denial of all eleven UCI mutation
permissions, and failed UCI reads after session revocation. No test account is
installed on a physical device.

```sh
python3 lab/openwrt/capture-console.py 25.12.5 --guest-script lab/openwrt/check-witness-permissions.sh --require-marker INTENT_PERMISSION_PASS
python3 lab/openwrt/capture-console.py 24.10.8 --guest-script lab/openwrt/check-witness-permissions.sh --require-marker INTENT_PERMISSION_PASS
```

Successful transcripts are `docs/verification-runs/openwrt-<version>-permissions.log`.
These tests use the stock native ubus CLI as a lab test client. They establish OS
and rpcd permission behavior, not execution of our compiled C/Rust adapter.
The [native build experiment](../lab/openwrt/build-native/README.md) remains
incomplete and is reserved for Astra.
