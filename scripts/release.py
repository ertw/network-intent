#!/usr/bin/env python3
"""Conservative release compatibility checks over compiler-emitted manifests.

This is release tooling, not an implementation of network semantics.
"""
import argparse
import json
import re
from pathlib import Path

BUMP = {"none": 0, "patch": 1, "minor": 2, "major": 3}


def version(value):
    if not isinstance(value, str) or not re.fullmatch(r"\d+\.\d+(?:\.\d+)?", value):
        raise ValueError(f"Invalid numeric language version: {value!r}")
    numbers = tuple(map(int, value.split(".")))
    return numbers + (0,) * (3 - len(numbers))


def constructs(document):
    items = document["constructs"]
    if not isinstance(items, list):
        raise ValueError("constructs must be a list")
    result = {}
    for item in items:
        if not isinstance(item, dict) or not isinstance(item.get("name"), str) or not isinstance(item.get("required"), bool):
            raise ValueError("Each construct requires name and boolean required")
        if item["name"] in result:
            raise ValueError("Duplicate construct: " + item["name"])
        result[item["name"]] = item
    return result


def compare(old, new):
    if old.get("manifestVersion") != 1 or new.get("manifestVersion") != 1:
        raise ValueError("Only manifestVersion 1 is supported")
    old_version, new_version = version(old["languageVersion"]), version(new["languageVersion"])
    before, after = constructs(old), constructs(new)
    minimum, reasons = 0, []

    def require(level, reason):
        nonlocal minimum
        minimum = max(minimum, BUMP[level])
        reasons.append(reason)

    for name in sorted(before.keys() - after.keys()):
        require("major", f"Removed construct {name}")
    for name in sorted(after.keys() - before.keys()):
        require("major" if after[name]["required"] else "minor", f"Added construct {name}")
    for name in sorted(before.keys() & after.keys()):
        if before[name] != after[name]:
            require("major", f"Changed construct contract {name}")
    independent = {"languageVersion", "compilerVersion", "backendApiVersion", "exportVersion", "semanticBump", "constructs"}
    for key in sorted((old.keys() | new.keys()) - independent):
        if old.get(key) != new.get(key):
            require("major", f"Changed {key}; conservative structural/semantic classification")
    declaration = new.get("semanticBump", "none")
    if declaration not in BUMP:
        raise ValueError("semanticBump must be none, patch, minor or major")
    if declaration != "none":
        require(declaration, "Explicit semantic change declaration")
    actual = next((level for index, level in enumerate((3, 2, 1)) if new_version[index] > old_version[index]), 0)
    if new_version < old_version:
        actual = -1
    minimum_name = next(name for name, rank in BUMP.items() if rank == minimum)
    return {"minimumBump": minimum_name, "accepted": actual >= minimum, "reasons": reasons,
            "oldVersion": old["languageVersion"], "newVersion": new["languageVersion"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("diff", "check"))
    parser.add_argument("old", type=Path)
    parser.add_argument("new", type=Path)
    args = parser.parse_args()
    try:
        result = compare(json.loads(args.old.read_text()), json.loads(args.new.read_text()))
    except (ValueError, KeyError, TypeError, OSError) as exc:
        parser.error(str(exc))
    print(json.dumps(result, sort_keys=True))
    return 1 if args.mode == "check" and not result["accepted"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
