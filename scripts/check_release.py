#!/usr/bin/env python3
"""Release-gate regression tests; no network semantic implementation here."""
import copy
import json
from pathlib import Path
import unittest

from release import compare

ROOT = Path(__file__).resolve().parents[1]


class ReleaseChecks(unittest.TestCase):
    def setUp(self):
        self.old = json.loads((ROOT / "docs/schema/2.0.json").read_text())
        self.new = copy.deepcopy(self.old)

    def test_current_major_release_gate(self):
        current = json.loads((ROOT / "docs/schema/3.0.json").read_text())
        result = compare(self.old, current)
        self.assertTrue(result["accepted"], result)
        self.assertEqual(result["minimumBump"], "major")

    def test_unchanged_compatible(self):
        self.assertTrue(compare(self.old, self.new)["accepted"])

    def test_optional_addition_requires_minor(self):
        self.new["constructs"].append({"name": "future", "required": False, "introduced": "2.1"})
        self.assertEqual(compare(self.old, self.new)["minimumBump"], "minor")
        self.assertFalse(compare(self.old, self.new)["accepted"])
        self.new["languageVersion"] = "2.1"
        self.assertTrue(compare(self.old, self.new)["accepted"])

    def test_required_or_removed_field_requires_major(self):
        self.new["constructs"].append({"name": "required", "required": True, "introduced": "3.0"})
        self.new["languageVersion"] = "2.1"
        self.assertFalse(compare(self.old, self.new)["accepted"])
        self.new["languageVersion"] = "3.0"
        self.assertTrue(compare(self.old, self.new)["accepted"])
        self.new = copy.deepcopy(self.old)
        self.new["constructs"].pop()
        self.assertEqual(compare(self.old, self.new)["minimumBump"], "major")

    def test_semantics_raise_never_lower_requirement(self):
        self.new["constraints"]["vlanMax"] = 100
        self.new["semanticBump"] = "patch"
        self.new["languageVersion"] = "2.0.1"
        self.assertFalse(compare(self.old, self.new)["accepted"])
        self.new = copy.deepcopy(self.old)
        self.new["semanticBump"] = "major"
        self.assertFalse(compare(self.old, self.new)["accepted"])

    def test_backend_version_independent(self):
        self.new["backendApiVersion"] = "3.0"
        self.assertTrue(compare(self.old, self.new)["accepted"])

    def test_downgrades_and_malformed_manifests_fail(self):
        self.new["languageVersion"] = "0.9"
        self.assertFalse(compare(self.old, self.new)["accepted"])
        self.new["constructs"].append(self.new["constructs"][0])
        with self.assertRaises(ValueError):
            compare(self.old, self.new)


if __name__ == "__main__":
    unittest.main()
