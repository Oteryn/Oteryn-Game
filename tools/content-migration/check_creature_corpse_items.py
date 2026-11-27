#!/usr/bin/env python3
"""Check exact canonical creature corpse references against Item admission facts."""

import argparse
import json
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def records(path):
    index = json.loads(path.read_bytes())
    for shard in index["shards"]:
        yield from json.loads((ROOT / shard).read_bytes())["records"]


def identity(reference):
    return reference["family"], reference["key"], reference["revision"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--creature", help="Check a single canonical creature key")
    args = parser.parse_args()

    items = {
        identity(row["definition"]["identity"]): row["definition"]
        for row in records(ROOT / "content/items/index.json")
    }
    corpses = defaultdict(list)
    for row in records(ROOT / "content/creatures/definitions/index.json"):
        creature = row["definition"]["identity"]["key"]
        if args.creature and creature != args.creature:
            continue
        details = row.get("authoring", {}).get("profile", {}).get("details") or {}
        if corpse := details.get("corpse_item"):
            corpses[identity(corpse)].append(creature)

    failures = 0
    for ref, creatures in sorted(corpses.items()):
        definition = items.get(ref, {})
        container = definition.get("semantics", {}).get("container", {})
        capacity = container.get("value", {}).get("capacity", {})
        problems = []
        if definition.get("materializable") is not True:
            problems.append("non-materializable")
        if definition.get("stack_class") not in ("NonStackable", "StackCapable"):
            problems.append("unknown stack class")
        if (container.get("state") != "KNOWN" or capacity.get("state") != "KNOWN"
                or type(capacity.get("value")) is not int or capacity["value"] <= 0):
            problems.append("missing positive container capacity")
        if problems:
            failures += 1
            print(f"{ref[1]}@{ref[2]}: {', '.join(problems)} ({', '.join(sorted(creatures))})")
    print(f"corpse items checked={len(corpses)} failed={failures}")
    return int(not corpses or failures > 0)


if __name__ == "__main__":
    raise SystemExit(main())
