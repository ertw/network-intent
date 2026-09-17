# Handoff verification — 2026-09-16

Runtime baseline: `d33310c`. This review covers the lab tooling and scoped Grok
instructions; it does not claim native adapter or full application acceptance.

| Check | Result | Evidence / limit |
|---|---|---|
| Isolated 24.10.8 permission capture | Exit 0 | `openwrt-24.10.8-permissions.log`; stock ubus CLI |
| Isolated 25.12.5 permission capture | Exit 0 | `openwrt-25.12.5-permissions.log`; stock ubus CLI |
| `cargo test -p intent-witness-agent --test openwrt_fixtures` | Exit 0; 2 passed | Existing fixture/parser tests |
| `sh -n` on each of openwrt-lab, check-witness-permissions.sh and build-native | Exit 0 | Shell syntax only |
| `python3 -m py_compile lab/openwrt/capture-console.py` | Exit 0 | Python syntax only |
| `cc -std=c11 -Wall -Wextra -Werror -fsyntax-only lab/openwrt/build-native/intent_ubus_smoke.c` | Exit 0 | Host smoke-harness syntax; no link/runtime evidence |
| `lab/openwrt/build-native/build-native build 25.12.5` | Expected exit 69 | Missing target development staging is explicitly rejected; build remains incomplete |
| Reparse both baseline logs via `capture-console.py VERSION --from-log LOG` | Exit 0 | Existing fixture bytes unchanged (`git diff --exit-code -- lab/openwrt/fixtures`) |
| Reparse both permission logs with guest script and required pass marker | Exit 0 | Guest status 0 plus release/readback/pass markers validated |
| Temporary negative copies of each permission transcript | Passed | Replacing guest exit status with 97, or removing its status line, makes parsing fail |
| New documentation links | Passed | Repository-local Markdown targets checked against filesystem |

The transcript negative checks used temporary files only; raw logs were unchanged.
No literal RPC session identifier was found in the permission transcripts. VM
images, signing keys, SDK downloads and mutable experimental state are excluded
by `lab/openwrt/.gitignore`. Grok tasks have not been executed.

Raw OpenWrt serial logs retain carriage returns and terminal whitespace. The local
`.gitattributes` disables line-ending normalization and whitespace lint only for
`openwrt-*.log`; source and documentation retain normal whitespace checks.
