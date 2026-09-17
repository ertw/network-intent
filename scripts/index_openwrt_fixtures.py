#!/usr/bin/env python3
"""Deterministic inventory of committed OpenWrt fixture JSON files."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

KNOWN_RELEASES = ("24.10.8", "25.12.5")
FIXTURES_RELATIVE = "lab/openwrt/fixtures"
INDEX_RELATIVE = f"{FIXTURES_RELATIVE}/index.json"
REQUIRED_JSON = (
    "capabilities.json",
    "uci-network.json",
    "uci-changes.json",
    "netifd-interfaces.json",
    "netifd-devices.json",
)
WIRELESS_FILE = "netifd-wireless.json"
WIRELESS_OBJECT = "network.wireless"
INDEX_FILENAME = "index.json"


class FixtureIndexError(Exception):
    def __init__(self, path: str, message: str) -> None:
        self.path = path
        self.message = message
        super().__init__(f"{message}: {path}")


def repository_root() -> Path:
    return Path(__file__).resolve().parent.parent


def fixtures_dir(root: Path) -> Path:
    return root / "lab" / "openwrt" / "fixtures"


def index_path(root: Path) -> Path:
    return fixtures_dir(root) / INDEX_FILENAME


def repository_relative(root: Path, path: Path) -> str:
    return path.relative_to(root).as_posix()


def _reject_nonfinite_constant(name: str):
    raise ValueError(f"non-finite numeric constant {name}")


def _reject_duplicate_keys(pairs):
    parsed = {}
    for key, value in pairs:
        if key in parsed:
            raise ValueError(f"duplicate object key {key!r}")
        parsed[key] = value
    return parsed


def _assert_finite(value, rel: str) -> None:
    if isinstance(value, float) and not math.isfinite(value):
        raise FixtureIndexError(rel, "non-finite numeric constant")
    if isinstance(value, dict):
        for inner in value.values():
            _assert_finite(inner, rel)
    elif isinstance(value, list):
        for inner in value:
            _assert_finite(inner, rel)


def load_strict_json_bytes(data: bytes, rel: str):
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise FixtureIndexError(rel, "invalid UTF-8") from exc
    try:
        parsed = json.loads(
            text,
            parse_constant=_reject_nonfinite_constant,
            object_pairs_hook=_reject_duplicate_keys,
        )
    except json.JSONDecodeError as exc:
        raise FixtureIndexError(rel, f"invalid JSON ({exc.msg})") from exc
    except ValueError as exc:
        raise FixtureIndexError(rel, str(exc)) from exc
    _assert_finite(parsed, rel)
    return parsed


def read_regular_file(path: Path, rel: str) -> bytes:
    if path.is_symlink():
        raise FixtureIndexError(rel, "symlink rejected")
    if not path.is_file():
        raise FixtureIndexError(rel, "missing file")
    return path.read_bytes()


def load_strict_json(path: Path, rel: str):
    return load_strict_json_bytes(read_regular_file(path, rel), rel)


def serialize_index(index: dict) -> bytes:
    return (json.dumps(index, indent=2, ensure_ascii=False, allow_nan=False) + "\n").encode("utf-8")


def _validate_capabilities(data, release: str, rel: str) -> list[str]:
    if not isinstance(data, dict):
        raise FixtureIndexError(rel, "capabilities.json must be an object")
    if data.get("release") != release:
        raise FixtureIndexError(rel, "capabilities release does not match directory")
    if not isinstance(data.get("collection"), str):
        raise FixtureIndexError(rel, "capabilities collection must be a string")
    unavailable = data.get("unavailable_objects")
    if not isinstance(unavailable, list):
        raise FixtureIndexError(rel, "unavailable_objects must be an array")
    seen = set()
    for item in unavailable:
        if not isinstance(item, str):
            raise FixtureIndexError(rel, "unavailable_objects entries must be strings")
        if item in seen:
            raise FixtureIndexError(rel, "unavailable_objects contains duplicates")
        if item != WIRELESS_OBJECT:
            raise FixtureIndexError(rel, "only network.wireless may be unavailable")
        seen.add(item)
    return list(unavailable)


def _scan_release(root: Path, release_dir: Path, release: str) -> dict:
    rel_dir = repository_relative(root, release_dir)
    if release_dir.is_symlink():
        raise FixtureIndexError(rel_dir, "symlink rejected")
    if not release_dir.is_dir():
        raise FixtureIndexError(rel_dir, "missing release directory")

    json_paths = []
    for entry in sorted(release_dir.iterdir(), key=lambda item: item.name):
        rel = repository_relative(root, entry)
        if entry.is_symlink():
            raise FixtureIndexError(rel, "symlink rejected")
        if entry.is_dir():
            raise FixtureIndexError(rel, "unexpected nested directory")
        if entry.suffix == ".json":
            json_paths.append(entry)

    names = {path.name for path in json_paths}
    capabilities_rel = f"{rel_dir}/capabilities.json"
    if "capabilities.json" not in names:
        raise FixtureIndexError(capabilities_rel, "missing file")

    capabilities_path = release_dir / "capabilities.json"
    capabilities = load_strict_json(capabilities_path, capabilities_rel)
    unavailable = _validate_capabilities(capabilities, release, capabilities_rel)

    required = list(REQUIRED_JSON)
    wireless_listed = WIRELESS_OBJECT in unavailable
    if wireless_listed:
        if WIRELESS_FILE in names:
            raise FixtureIndexError(
                f"{rel_dir}/{WIRELESS_FILE}",
                "netifd-wireless.json must be absent when network.wireless is unavailable",
            )
    else:
        required.append(WIRELESS_FILE)

    for name in required:
        if name not in names:
            raise FixtureIndexError(f"{rel_dir}/{name}", "missing file")

    files = []
    for path in json_paths:
        rel = repository_relative(root, path)
        data = read_regular_file(path, rel)
        load_strict_json_bytes(data, rel)
        files.append(
            {
                "path": rel,
                "sha256": hashlib.sha256(data).hexdigest(),
                "bytes": len(data),
            }
        )
    files.sort(key=lambda item: item["path"])
    return {
        "release": release,
        "files": files,
        "unavailable_objects": unavailable,
    }


def build_index(root: Path) -> dict:
    root = Path(root)
    for relative in ("lab", "lab/openwrt", FIXTURES_RELATIVE):
        if (root / relative).is_symlink():
            raise FixtureIndexError(relative, "symlink rejected")
    fixtures = fixtures_dir(root)
    if not fixtures.is_dir():
        raise FixtureIndexError(FIXTURES_RELATIVE, "missing fixtures directory")

    found = {}
    for entry in sorted(fixtures.iterdir(), key=lambda item: item.name):
        rel = repository_relative(root, entry)
        if entry.is_symlink():
            raise FixtureIndexError(rel, "symlink rejected")
        if entry.is_dir():
            if entry.name not in KNOWN_RELEASES:
                raise FixtureIndexError(rel, "unknown release directory")
            found[entry.name] = entry
        elif entry.name == INDEX_FILENAME:
            continue
        elif entry.suffix == ".json":
            raise FixtureIndexError(rel, "unexpected JSON outside release directory")

    releases = []
    for name in KNOWN_RELEASES:
        if name not in found:
            raise FixtureIndexError(f"{FIXTURES_RELATIVE}/{name}", "missing release directory")
        releases.append(_scan_release(root, found[name], name))
    releases.sort(key=lambda item: item["release"])
    return {"version": 1, "releases": releases}


def canonical_index_bytes(root: Path) -> bytes:
    return serialize_index(build_index(root))


def _is_lowercase_sha256(value) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(char in "0123456789abcdef" for char in value)


def validate_index_document(document, rel: str) -> None:
    if not isinstance(document, dict):
        raise FixtureIndexError(rel, "index must be an object")
    if set(document) != {"version", "releases"}:
        raise FixtureIndexError(rel, "index keys must be version and releases")
    if document["version"] != 1 or isinstance(document["version"], bool):
        raise FixtureIndexError(rel, "index version must be 1")
    releases = document["releases"]
    if not isinstance(releases, list):
        raise FixtureIndexError(rel, "index releases must be an array")
    names = []
    for release in releases:
        if not isinstance(release, dict) or set(release) != {"release", "files", "unavailable_objects"}:
            raise FixtureIndexError(rel, "release entries must have release, files, unavailable_objects")
        name = release["release"]
        if not isinstance(name, str):
            raise FixtureIndexError(rel, "release name must be a string")
        names.append(name)
        files = release["files"]
        unavailable = release["unavailable_objects"]
        if not isinstance(files, list):
            raise FixtureIndexError(rel, "release files must be an array")
        if not isinstance(unavailable, list) or any(not isinstance(item, str) for item in unavailable):
            raise FixtureIndexError(rel, "unavailable_objects must be an array of strings")
        paths = []
        for entry in files:
            if not isinstance(entry, dict) or set(entry) != {"path", "sha256", "bytes"}:
                raise FixtureIndexError(rel, "file entries must have path, sha256, bytes")
            if not isinstance(entry["path"], str):
                raise FixtureIndexError(rel, "file path must be a string")
            if not _is_lowercase_sha256(entry["sha256"]):
                raise FixtureIndexError(rel, "file sha256 must be lowercase hex")
            size = entry["bytes"]
            if not isinstance(size, int) or isinstance(size, bool) or size < 0:
                raise FixtureIndexError(rel, "file bytes must be a non-negative integer")
            paths.append(entry["path"])
        if paths != sorted(paths):
            raise FixtureIndexError(rel, "file paths must be sorted lexically")
    if names != sorted(names):
        raise FixtureIndexError(rel, "releases must be sorted lexically")


def diagnose_mismatch(generated: dict, committed: dict) -> None:
    generated_releases = {item["release"]: item for item in generated["releases"]}
    committed_releases = {item["release"]: item for item in committed["releases"]}
    for name in sorted(set(generated_releases) | set(committed_releases)):
        rel_dir = f"{FIXTURES_RELATIVE}/{name}"
        if name not in committed_releases:
            raise FixtureIndexError(rel_dir, "extra release")
        if name not in generated_releases:
            raise FixtureIndexError(rel_dir, "missing release")
        generated_release = generated_releases[name]
        committed_release = committed_releases[name]
        if generated_release["unavailable_objects"] != committed_release["unavailable_objects"]:
            raise FixtureIndexError(f"{rel_dir}/capabilities.json", "unavailable_objects mismatch")
        generated_files = {item["path"]: item for item in generated_release["files"]}
        committed_files = {item["path"]: item for item in committed_release["files"]}
        for path in sorted(set(generated_files) | set(committed_files)):
            if path not in committed_files:
                raise FixtureIndexError(path, "extra file")
            if path not in generated_files:
                raise FixtureIndexError(path, "missing file")
            generated_file = generated_files[path]
            committed_file = committed_files[path]
            if generated_file["sha256"] != committed_file["sha256"] or generated_file["bytes"] != committed_file["bytes"]:
                raise FixtureIndexError(path, "changed file")
    raise FixtureIndexError(INDEX_RELATIVE, "index serialization mismatch")


def write_index(root: Path) -> bytes:
    data = canonical_index_bytes(root)
    dest = index_path(root)
    rel = INDEX_RELATIVE
    if dest.is_symlink():
        raise FixtureIndexError(rel, "symlink rejected")
    dest.write_bytes(data)
    return data


def check_index(root: Path) -> None:
    generated = build_index(root)
    canonical = serialize_index(generated)
    dest = index_path(root)
    rel = INDEX_RELATIVE
    if dest.is_symlink():
        raise FixtureIndexError(rel, "symlink rejected")
    if not dest.is_file():
        raise FixtureIndexError(rel, "missing index")
    existing = dest.read_bytes()
    if existing == canonical:
        return
    committed = load_strict_json_bytes(existing, rel)
    validate_index_document(committed, rel)
    diagnose_mismatch(generated, committed)


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description="Deterministic index of existing OpenWrt fixtures.")
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--write", action="store_true", help="Write lab/openwrt/fixtures/index.json")
    group.add_argument("--check", action="store_true", help="Check the committed index without writing")
    return parser.parse_args(argv)


def main(argv=None) -> int:
    args = parse_args(argv)
    root = repository_root()
    try:
        if args.write:
            write_index(root)
        else:
            check_index(root)
    except FixtureIndexError as exc:
        print(exc.path, file=sys.stderr)
        print(f"{exc.message}: {exc.path}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
