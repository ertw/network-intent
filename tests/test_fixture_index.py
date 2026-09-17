#!/usr/bin/env python3
"""Acceptance tests for the OpenWrt fixture inventory indexer."""
from __future__ import annotations

import hashlib
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import index_openwrt_fixtures as idx  # noqa: E402

REAL_FIXTURES = ROOT / "lab" / "openwrt" / "fixtures"
KNOWN_RELEASES = ("24.10.8", "25.12.5")


def snapshot_bytes(directory: Path) -> dict[str, bytes]:
    recorded = {}
    for path in sorted(directory.rglob("*")):
        rel = path.relative_to(directory).as_posix()
        if path.is_symlink():
            recorded[rel] = b"symlink:" + os.fsencode(os.readlink(path))
        elif path.is_file():
            recorded[rel] = path.read_bytes()
    return recorded


def fixture_file_hashes(root: Path) -> dict[str, str]:
    fixtures = root / "lab" / "openwrt" / "fixtures"
    hashes = {}
    for path in sorted(fixtures.rglob("*.json")):
        if path.name == "index.json":
            continue
        rel = path.relative_to(root).as_posix()
        hashes[rel] = hashlib.sha256(path.read_bytes()).hexdigest()
    return hashes


class FixtureIndexTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory(prefix="openwrt-fixture-index-")
        self.root = Path(self.tmp.name)
        fixtures = self.root / "lab" / "openwrt" / "fixtures"
        fixtures.mkdir(parents=True)
        for release in KNOWN_RELEASES:
            shutil.copytree(REAL_FIXTURES / release, fixtures / release)

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def path(self, relative: str) -> Path:
        return self.root / relative

    def test_two_consecutive_generations_identical(self) -> None:
        first = idx.canonical_index_bytes(self.root)
        second = idx.canonical_index_bytes(self.root)
        self.assertEqual(first, second)
        written = idx.write_index(self.root)
        rewritten = idx.write_index(self.root)
        self.assertEqual(written, rewritten)
        self.assertEqual(written, first)
        real_first = idx.canonical_index_bytes(ROOT)
        real_second = idx.canonical_index_bytes(ROOT)
        self.assertEqual(real_first, real_second)

    def test_modified_byte_fails_check_with_path(self) -> None:
        idx.write_index(self.root)
        target = self.path("lab/openwrt/fixtures/24.10.8/uci-network.json")
        data = bytearray(target.read_bytes())
        data[-1] = data[-1] ^ 0x01
        target.write_bytes(bytes(data))
        with self.assertRaises(idx.FixtureIndexError) as caught:
            idx.check_index(self.root)
        self.assertEqual(caught.exception.path, "lab/openwrt/fixtures/24.10.8/uci-network.json")

    def test_valid_json_byte_change_fails_digest_check(self) -> None:
        idx.write_index(self.root)
        target = self.path("lab/openwrt/fixtures/24.10.8/uci-network.json")
        target.write_bytes(target.read_bytes() + b" ")
        with self.assertRaises(idx.FixtureIndexError) as caught:
            idx.check_index(self.root)
        self.assertEqual(caught.exception.path, "lab/openwrt/fixtures/24.10.8/uci-network.json")
        self.assertEqual(caught.exception.message, "changed file")

    def test_symlink_ancestor_rejected_before_write(self) -> None:
        for relative in ("lab", "lab/openwrt"):
            with self.subTest(relative=relative):
                target = self.path(relative)
                moved = target.with_name(target.name + "-real")
                target.rename(moved)
                target.symlink_to(moved, target_is_directory=True)
                before = snapshot_bytes(moved)
                try:
                    with self.assertRaises(idx.FixtureIndexError) as caught:
                        idx.write_index(self.root)
                    self.assertEqual(caught.exception.path, relative)
                    self.assertEqual(snapshot_bytes(moved), before)
                finally:
                    target.unlink()
                    moved.rename(target)

    def test_unknown_and_nested_directories_rejected(self) -> None:
        for relative in ("lab/openwrt/fixtures/99.0", "lab/openwrt/fixtures/24.10.8/nested"):
            with self.subTest(relative=relative):
                target = self.path(relative)
                target.mkdir()
                try:
                    with self.assertRaises(idx.FixtureIndexError) as caught:
                        idx.build_index(self.root)
                    self.assertEqual(caught.exception.path, relative)
                finally:
                    target.rmdir()

    def test_missing_file_fails_check_with_path(self) -> None:
        idx.write_index(self.root)
        missing = self.path("lab/openwrt/fixtures/24.10.8/uci-changes.json")
        missing.unlink()
        with self.assertRaises(idx.FixtureIndexError) as caught:
            idx.check_index(self.root)
        self.assertEqual(caught.exception.path, "lab/openwrt/fixtures/24.10.8/uci-changes.json")

    def test_extra_json_fails_check_with_path(self) -> None:
        idx.write_index(self.root)
        extra = self.path("lab/openwrt/fixtures/24.10.8/extra.json")
        extra.write_text("{}\n", encoding="utf-8")
        with self.assertRaises(idx.FixtureIndexError) as caught:
            idx.check_index(self.root)
        self.assertEqual(caught.exception.path, "lab/openwrt/fixtures/24.10.8/extra.json")

    def test_wrong_release_label_fails_check_with_path(self) -> None:
        idx.write_index(self.root)
        capabilities = self.path("lab/openwrt/fixtures/24.10.8/capabilities.json")
        capabilities.write_text(
            '{\n  "release": "99.0.0",\n  "unavailable_objects": [],\n  "collection": "wrong"\n}\n',
            encoding="utf-8",
        )
        with self.assertRaises(idx.FixtureIndexError) as caught:
            idx.check_index(self.root)
        self.assertEqual(caught.exception.path, "lab/openwrt/fixtures/24.10.8/capabilities.json")
        self.assertIn("release", caught.exception.message)

    def test_malformed_capabilities_fails_check_with_path(self) -> None:
        idx.write_index(self.root)
        capabilities = self.path("lab/openwrt/fixtures/25.12.5/capabilities.json")
        capabilities.write_text(
            '{\n  "release": "25.12.5",\n  "unavailable_objects": "network.wireless",\n  "collection": "bad"\n}\n',
            encoding="utf-8",
        )
        with self.assertRaises(idx.FixtureIndexError) as caught:
            idx.check_index(self.root)
        self.assertEqual(caught.exception.path, "lab/openwrt/fixtures/25.12.5/capabilities.json")

    def test_duplicate_json_keys_rejected(self) -> None:
        target = self.path("lab/openwrt/fixtures/24.10.8/uci-changes.json")
        target.write_text('{"changes": [], "changes": []}\n', encoding="utf-8")
        with self.assertRaises(idx.FixtureIndexError) as caught:
            idx.build_index(self.root)
        self.assertEqual(caught.exception.path, "lab/openwrt/fixtures/24.10.8/uci-changes.json")
        self.assertIn("duplicate", caught.exception.message)

    def test_nan_rejected(self) -> None:
        target = self.path("lab/openwrt/fixtures/24.10.8/uci-changes.json")
        target.write_text('{"changes": NaN}\n', encoding="utf-8")
        with self.assertRaises(idx.FixtureIndexError) as caught:
            idx.build_index(self.root)
        self.assertEqual(caught.exception.path, "lab/openwrt/fixtures/24.10.8/uci-changes.json")
        self.assertIn("non-finite", caught.exception.message)

    def test_symlink_input_rejected(self) -> None:
        target = self.path("lab/openwrt/fixtures/24.10.8/uci-changes.json")
        target.unlink()
        target.symlink_to("uci-network.json")
        self.assertTrue(target.is_symlink())
        with self.assertRaises(idx.FixtureIndexError) as caught:
            idx.build_index(self.root)
        self.assertEqual(caught.exception.path, "lab/openwrt/fixtures/24.10.8/uci-changes.json")
        self.assertIn("symlink", caught.exception.message)

    def test_wireless_capability_cases_remain_distinct(self) -> None:
        index = idx.build_index(ROOT)
        releases = {item["release"]: item for item in index["releases"]}
        self.assertEqual(set(releases), {"24.10.8", "25.12.5"})

        wireless_24 = "lab/openwrt/fixtures/24.10.8/netifd-wireless.json"
        wireless_25 = "lab/openwrt/fixtures/25.12.5/netifd-wireless.json"
        paths_24 = {item["path"] for item in releases["24.10.8"]["files"]}
        paths_25 = {item["path"] for item in releases["25.12.5"]["files"]}

        self.assertEqual(releases["24.10.8"]["unavailable_objects"], [])
        self.assertIn(wireless_24, paths_24)
        self.assertTrue((ROOT / wireless_24).is_file())
        self.assertEqual((ROOT / wireless_24).read_bytes().strip(), b"{}")

        self.assertEqual(releases["25.12.5"]["unavailable_objects"], ["network.wireless"])
        self.assertNotIn(wireless_25, paths_25)
        self.assertFalse((ROOT / wireless_25).exists())

        copied = idx.build_index(self.root)
        copied_releases = {item["release"]: item for item in copied["releases"]}
        self.assertEqual(copied_releases["24.10.8"]["unavailable_objects"], [])
        self.assertEqual(copied_releases["25.12.5"]["unavailable_objects"], ["network.wireless"])

    def test_check_does_not_modify_fixtures_or_index(self) -> None:
        idx.write_index(self.root)
        before = snapshot_bytes(self.root)
        idx.check_index(self.root)
        after = snapshot_bytes(self.root)
        self.assertEqual(before, after)
        real_before = snapshot_bytes(REAL_FIXTURES)
        idx.build_index(ROOT)
        if idx.index_path(ROOT).is_file() and not idx.index_path(ROOT).is_symlink():
            idx.check_index(ROOT)
        real_after = snapshot_bytes(REAL_FIXTURES)
        self.assertEqual(real_before, real_after)

    def test_original_fixture_hashes_unchanged(self) -> None:
        before = fixture_file_hashes(ROOT)
        expected = {
            "lab/openwrt/fixtures/24.10.8/capabilities.json",
            "lab/openwrt/fixtures/24.10.8/netifd-devices.json",
            "lab/openwrt/fixtures/24.10.8/netifd-interfaces.json",
            "lab/openwrt/fixtures/24.10.8/netifd-wireless.json",
            "lab/openwrt/fixtures/24.10.8/uci-changes.json",
            "lab/openwrt/fixtures/24.10.8/uci-network.json",
            "lab/openwrt/fixtures/25.12.5/capabilities.json",
            "lab/openwrt/fixtures/25.12.5/netifd-devices.json",
            "lab/openwrt/fixtures/25.12.5/netifd-interfaces.json",
            "lab/openwrt/fixtures/25.12.5/uci-changes.json",
            "lab/openwrt/fixtures/25.12.5/uci-network.json",
        }
        self.assertEqual(set(before), expected)
        idx.canonical_index_bytes(ROOT)
        idx.build_index(self.root)
        idx.write_index(self.root)
        idx.check_index(self.root)
        after = fixture_file_hashes(ROOT)
        self.assertEqual(before, after)
        self.assertNotIn("lab/openwrt/fixtures/25.12.5/netifd-wireless.json", after)


if __name__ == "__main__":
    unittest.main()
