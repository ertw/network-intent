#!/usr/bin/env python3
"""Idris must reject migration debt at stable-only API boundaries."""
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PROBES = {
    "DebtCannotFinish": '''module DebtCannotFinish
import NetDSL.Common
import NetDSL.Domain.Model
import NetDSL.Validate
import NetDSL.Migration
%default total
bad : MigrationState [Owed (DNSAvailable (Id 0)) KnownViolation "pending"] -> Either (List Diagnostic) StableNetwork
bad state = finish state
''',
    "DebtCannotCompile": '''module DebtCannotCompile
import NetDSL.Common
import NetDSL.Domain.Model
import NetDSL.Migration
import NetDSL.Backend.AST
import NetDSL.Backend.Compile
%default total
bad : MigrationState [Owed (DNSAvailable (Id 0)) KnownViolation "pending"] -> Either (List Diagnostic) Realization
bad state = compileTarget state "gateway" 4094
''',
    "ObservationCannotUseRepair": '''module ObservationCannotUseRepair
import NetDSL.Domain.Model
import NetDSL.Migration
%default total
bad : DischargeEvidence (Owed (DNSAvailable (Id 0)) RequiresObservation "pending")
bad = RepairEvidence "just assume healthy"
''',
}


def main():
    with tempfile.TemporaryDirectory(prefix="netc-typestate-") as directory:
        folder = Path(directory)
        shutil.copytree(ROOT / "src/NetDSL", folder / "NetDSL")
        for name, source in PROBES.items():
            (folder / f"{name}.idr").write_text(source)
            result = subprocess.run(["idris2", "--check", f"{name}.idr"], cwd=folder,
                                    text=True, capture_output=True, timeout=120)
            output = result.stdout + result.stderr
            assert result.returncode != 0 and "Mismatch" in output, output
            assert "Undefined name" not in output, output
            print("PASS compile-negative " + name)


if __name__ == "__main__":
    main()
