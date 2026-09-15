#!/usr/bin/env python3
"""Exercise the Idris browser build in a browser-like VM against native Idris.

The temporary native wrapper imports NetDSL.Compiler, so this checks one pure
implementation rather than maintaining a second compiler in Python or JS.
"""

from __future__ import annotations

import json
import pathlib
import subprocess
import tempfile


ROOT = pathlib.Path(__file__).resolve().parents[1]
JS = ROOT / "build" / "browser" / "exec" / "netc-browser"

NATIVE_WRAPPER = """module BrowserParityTmp
import NetDSL.Compiler
import System

main : IO ()
main = do
  args <- getArgs
  case args of
    _ :: filename :: source :: [] => putStrLn (evaluateSource filename source)
    _ => putStrLn "{}"
"""

# The host only supplies source to the VM.  The evaluated compiler gets a
# browser-like global object with no process, module, require, or filesystem.
NODE_VM = r"""
const fs = require('fs');
const vm = require('vm');
const sandbox = Object.create(null);
sandbox.globalThis = sandbox;
vm.runInNewContext(fs.readFileSync(process.argv[1], 'utf8'), sandbox, { filename: 'netc-browser.js' });
if (typeof sandbox.netcEvaluate !== 'function') throw new Error('netcEvaluate was not registered');
process.stdout.write(sandbox.netcEvaluate(process.argv[2]));
"""


def run(command: list[str], *, cwd: pathlib.Path = ROOT, timeout: float = 120) -> str:
    try:
        completed = subprocess.run(
            command, cwd=cwd, text=True, capture_output=True, timeout=timeout
        )
    except subprocess.TimeoutExpired as error:
        raise RuntimeError(f"{' '.join(command)} timed out after {timeout:g}s") from error
    if completed.returncode:
        raise RuntimeError(f"{' '.join(command)} failed:\n{completed.stderr}{completed.stdout}")
    return completed.stdout


def build_native(directory: pathlib.Path) -> pathlib.Path:
    wrapper = directory / "BrowserParityTmp.idr"
    wrapper.write_text(NATIVE_WRAPPER, encoding="utf-8")
    (directory / "NetDSL").symlink_to(ROOT / "src" / "NetDSL", target_is_directory=True)
    run([
        "idris2", "--source-dir", str(directory), "--build-dir", str(directory / "ttc"),
        "--cg", "chez", "-o", "native-parity", str(wrapper),
    ], cwd=directory)
    return directory / "ttc" / "exec" / "native-parity"


def browser(source: str) -> object:
    output = run(["node", "-e", NODE_VM, str(JS), source])
    return json.loads(output)


def native(executable: pathlib.Path, filename: str, source: str) -> object:
    return json.loads(run(["./" + executable.name, filename, source], cwd=executable.parent, timeout=30))


def main() -> None:
    run(["idris2", "--build-dir", "build/browser", "--build", "browser.ipkg"])
    fixtures = [
        ("minimal.net", (ROOT / "examples" / "minimal.net").read_text(encoding="utf-8")),
        ("home.net", (ROOT / "examples" / "home.net").read_text(encoding="utf-8")),
        ("core-router.net", (ROOT / "examples" / "core-router.net").read_text(encoding="utf-8")),
        ("wds-network.net", (ROOT / "examples" / "wds-network.net").read_text(encoding="utf-8")),
        ("bad-cidr.net", (ROOT / "tests" / "fixtures" / "errors" / "bad-cidr.net").read_text(encoding="utf-8")),
        ("unknown-vlan.net", (ROOT / "tests" / "fixtures" / "errors" / "unknown-vlan.net").read_text(encoding="utf-8")),
        ("unicode-comments.net", "# café: parser columns are Unicode characters\nnetwork-language 3.0\nnetwork café {}\n"),
        ("malformed.net", "network-language 3.0\nnetwork broken { vlan x 2 {\n"),
    ]
    with tempfile.TemporaryDirectory(prefix="netc-browser-parity-") as temp:
        executable = build_native(pathlib.Path(temp))
        for filename, source in fixtures:
            actual = browser(source)
            expected = native(executable, "browser.net", source)
            if actual != expected:
                raise AssertionError(f"browser/native mismatch for {filename}:\n{actual!r}\n!=\n{expected!r}")
    print(f"browser compiler parity passed for {len(fixtures)} fixtures")


if __name__ == "__main__":
    main()
