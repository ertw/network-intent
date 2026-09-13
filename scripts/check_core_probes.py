#!/usr/bin/env python3
"""Run the independent direct-API and ill-typed certificate fixtures."""
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    with tempfile.TemporaryDirectory(prefix="netc-core-probes-") as directory:
        folder = Path(directory)
        shutil.copytree(ROOT / "src/NetDSL", folder / "NetDSL")
        for source in (ROOT / "scripts/core-probes").glob("*.idr"):
            shutil.copy(source, folder / source.name)
        for name in ("CoreAPI", "GraphProperties"):
            result = subprocess.run(["idris2", "-o", name, f"{name}.idr"], cwd=folder,
                                    text=True, capture_output=True, timeout=180)
            assert result.returncode == 0, result.stdout + result.stderr
            result = subprocess.run([str(folder / "build/exec" / name)], cwd=folder,
                                    text=True, capture_output=True, timeout=60)
            assert result.returncode == 0, result.stdout + result.stderr
            print(result.stdout, end="")
        negative = sorted(folder.glob("Forged*.idr")) + [folder / "WrongRefKind.idr"]
        for source in negative:
            result = subprocess.run(["idris2", "--check", source.name], cwd=folder,
                                    text=True, capture_output=True, timeout=60)
            output = result.stdout + result.stderr
            assert result.returncode != 0 and ("Mismatch between" in output or "Can't solve constraint" in output), output
            assert "Undefined name" not in output, output
            print("PASS compile-negative " + source.stem)


if __name__ == "__main__":
    main()
