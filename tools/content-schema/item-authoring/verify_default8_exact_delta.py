"""Future read-only all-record comparison against actual Weapon103 predecessor."""

import argparse
import copy
import hashlib
import json
import subprocess
from pathlib import Path

IDS = {23229, 23230, 23231, 23232, 23295, 23299, 23335, 23339}
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--root", type=Path, required=True)
parser.add_argument("--parent", required=True)
parser.add_argument("--packet", required=True)
parser.add_argument("--packet-sha256", required=True)
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()
assert len(args.parent) == 40 and all(c in "0123456789abcdef" for c in args.parent)


def canonical(value):
    return json.dumps(
        value, sort_keys=True, ensure_ascii=False, separators=(",", ":")
    ).encode()


def old_bytes(path):
    return subprocess.check_output(
        ["git", "show", f"{args.parent}:{path}"], cwd=args.root
    )


def load(path, old=False):
    return json.loads(old_bytes(path) if old else (args.root / path).read_bytes())


raw = (args.root / args.packet).read_bytes()
assert hashlib.sha256(raw).hexdigest() == args.packet_sha256
packet = json.loads(raw)
rows = packet["promotions"]
assert len(rows) == 8 and {r["source_item_id"] for r in rows} == IDS
assert all(r["stackable"] is False for r in rows)
expected = {r["item_key"]: r["target"] for r in rows}
assert len(expected) == 8


def patched(row):
    out = copy.deepcopy(row)
    key = row["identity"]["key"]
    if key in expected:
        assert canonical(row["identity"]) == canonical(expected[key])
        group = out["semantics"]["stack"]
        if canonical(group) == canonical({"state": "UNKNOWN"}):
            out["semantics"]["stack"] = {
                "state": "KNOWN",
                "value": {
                    "stackable": {"state": "UNKNOWN"},
                    "stack_max": {"state": "UNKNOWN"},
                },
            }
        else:
            assert group["state"] == "KNOWN"
            assert canonical(group["value"]["stackable"]) == canonical(
                {"state": "UNKNOWN"}
            )
        out["semantics"]["stack"]["value"]["stackable"] = {
            "state": "KNOWN",
            "value": False,
        }
    return out


def false_census(rows):
    count = 0
    for row in rows:
        leaf = (
            row.get("semantics", {})
            .get("stack", {})
            .get("value", {})
            .get("stackable", {})
        )
        count += leaf.get("state") == "KNOWN" and leaf.get("value") is False
    return count


path = "content/world/definitions/reference.json"
before, after = load(path, True), load(path)
old = {r["identity"]["key"]: r for r in before["records"]}
new = {r["identity"]["key"]: r for r in after["records"]}
assert len(before["records"]) == len(after["records"]) == len(old) == len(new) == 57320
assert old.keys() == new.keys() and set(expected) <= old.keys()
assert sum(r["kind"] == "Item" for r in after["records"]) == 34031
assert {k for k in old if canonical(old[k]) != canonical(new[k])} == set(expected)
for key, row in old.items():
    assert canonical(new[key]) == canonical(patched(row)), (
        f"full native field drift: {key}"
    )
assert canonical({k: v for k, v in before.items() if k != "records"}) == canonical(
    {k: v for k, v in after.items() if k != "records"}
)
assert false_census(after["records"]) == false_census(before["records"]) + 8
changed_shards, seen = [], set()
shards = sorted(args.root.glob("content/items/definitions/items-*.json"))
assert len(shards) == 69
for file in shards:
    path = str(file.relative_to(args.root))
    before, after = load(path, True), load(path)
    projected = copy.deepcopy(before)
    for record in projected["records"]:
        definition = record["definition"]
        assert definition["identity"]["key"] not in seen
        seen.add(definition["identity"]["key"])
        record["definition"] = patched(definition)
    assert canonical(projected) == canonical(after), (
        f"canonical field/meta drift: {path}"
    )
    if canonical(before) != canonical(after):
        changed_shards.append(path)
assert len(seen) == 34031 and set(expected) <= seen
for area in ("content/world/objects", "content/world/terrain"):
    for file in (args.root / area).glob("*.json"):
        assert file.read_bytes() == old_bytes(str(file.relative_to(args.root)))
for path in (
    "content/world/definitions/declarations.json",
    "imports/crystalserver/bindings/items.json",
    "imports/tibiawiki/facts/items-stats.json",
    "content/items/taxonomy/items.json",
    "content/items/taxonomy/index.json",
    "content/items/relations/items.json",
    "content/items/relations/index.json",
):
    assert (args.root / path).read_bytes() == old_bytes(path)
report = {
    "status": "PASS",
    "baseline": args.parent,
    "native_records": 57320,
    "native_items": 34031,
    "canonical_shards": 69,
    "changed_items": 8,
    "only_change": "stack.stackable UNKNOWN to KNOWN(false)",
    "changed_keys": sorted(expected),
    "changed_shards": changed_shards,
    "other_fields_classes_admission_lifecycle": "UNCHANGED",
}
args.output.write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
