"""WO-2 content emitter: write one family's catalog records under `content/world/`.

`world_objects.py` (WO-1) builds and validates the records but writes nothing under
`content/`. This driver reuses its builders, routing, D94 exclusions and validator
unchanged and only adds the physical layout and the holds:

- Records are sharded by ascending source id, 500 per shard, in the same envelope the
  other family trees use (`family`, `shard {start, end, count}`, `records`).
- A family key is the D93 pure function of the frozen CW2-B1 Item key. It is also
  cross-checked here against `imports/crystalserver/bindings/items.json`.
- A **held** id gets no family key. It is listed by its existing Item key in
  `held.json` with the reason. Holds are `no_client_appearance` (D94 excludes an id
  with no client appearance) and the WO-1 census "contested route"
  `type_outside_family` (an `items.xml` `type` that the routed family's kind set does
  not express).
- An unresolved `kind` is not a hold. The D93 key is a pure function of the route, so
  the id is minted and its `kind` stays `{"state": "UNKNOWN"}`.

Generation is deterministic; `--check` regenerates in memory and diffs the committed
files byte for byte.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from collections import Counter
from pathlib import Path

import world_objects as wo

REPO_ROOT = Path(__file__).resolve().parents[3]
BINDINGS = REPO_ROOT / "imports" / "crystalserver" / "bindings" / "items.json"
SHARD_SIZE = 500
FAMILY_DIRECTORY = {
    "Terrain": ("content/world/terrain/definitions", "terrain"),
    "WorldObject": ("content/world/objects/definitions", "objects"),
}
HOLD_REASONS = {
    "no_client_appearance": (
        "The id has no client appearance, so D94 excludes it from the family; it stays "
        "a non-materializable Item record."
    ),
    "type_outside_family": (
        "The items.xml type is one the routed family's kind set does not express, so the "
        "family itself is contested (WO-0 remaining_unknowns: contested routes)."
    ),
}
DECISION = "WO0-WORLD-OBJECT-TERRAIN-FORMAT-V1 (D93, D94)"


def dumps(value):
    return wo.canonical_bytes(value) + b"\n"


def pretty_dumps(value):
    return (
        json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    ).encode("utf-8")


def frozen_item_keys():
    """Map source id (int) to the frozen CW2-B1 Item key from the binding table."""
    document = json.loads(BINDINGS.read_text(encoding="utf-8"))
    keys = {}
    for row in document["bindings"]:
        if row["disposition"] == "EXACT" and row["target"]["family"] == "Item":
            keys[int(row["external_id"])] = row["target"]["key"]
    return keys


def hold_reasons(family, attrs, appearance):
    reasons = []
    if appearance is None:
        reasons.append("no_client_appearance")
    if wo.type_outside_family(family, attrs):
        reasons.append("type_outside_family")
    return reasons


def collect(sources, family):
    """Return (records, held) for one family, both in ascending source id order."""
    meta = wo.source_meta_of(sources)
    bindings = frozen_item_keys()
    records = []
    held = []
    keys = set()
    for item_id, item_key, owner, reason in wo.iter_routed(sources):
        routed_family = wo.family_for_route(owner, reason)
        if routed_family != family:
            continue
        if bindings.get(item_id) != item_key:
            raise SystemExit(f"id {item_id}: {item_key} is not its frozen binding")
        xml_record = sources["items"].get(item_id)
        appearance = sources["appearances"].get(item_id)
        record = wo.build_record(
            family, item_id, item_key, reason, xml_record, appearance, meta
        )
        errors = wo.validate_record(record)
        if errors:
            raise SystemExit(f"id {item_id}: invalid {family} record: {errors[:3]}")
        attrs = dict(xml_record["attrs"]) if xml_record else {}
        reasons = hold_reasons(family, attrs, appearance)
        if reasons:
            held.append(
                {
                    "item_key": item_key,
                    "reasons": reasons,
                    "route": f"{owner}:{reason}",
                    "source_item_id": item_id,
                }
            )
            continue
        key = record["identity"]["key"]
        if key in keys:
            raise SystemExit(f"family key collision: {key}")
        keys.add(key)
        records.append(record)
    return records, held


def render(sources, family):
    """Return {repo-relative path: bytes} for one family and the summary counts."""
    directory, stem = FAMILY_DIRECTORY[family]
    records, held = collect(sources, family)
    meta = wo.source_meta_of(sources)
    files = {}
    shard_paths = []
    digest = hashlib.sha256()
    for index, start in enumerate(range(0, len(records), SHARD_SIZE)):
        chunk = records[start : start + SHARD_SIZE]
        end = start + len(chunk) - 1
        path = f"{directory}/{stem}-{start:05d}-{end:05d}.json"
        shard_paths.append(path)
        body = dumps(
            {
                "family": family,
                "records": chunk,
                "schema": "OTERYN_WORLD_OBJECT_TERRAIN_SHARD/v1",
                "shard": {
                    "count": len(chunk),
                    "end": end,
                    "index": index,
                    "start": start,
                },
            }
        )
        digest.update(body)
        files[path] = body
    by_kind = Counter(
        record["kind"]["value"] if record["kind"].get("state") == "KNOWN" else "UNKNOWN"
        for record in records
    )
    held_reasons = Counter(reason for row in held for reason in row["reasons"])
    files[f"{directory}/held.json"] = pretty_dumps(
        {
            "decision": DECISION,
            "family": family,
            "held": held,
            "held_count": len(held),
            "reasons": {name: HOLD_REASONS[name] for name in sorted(held_reasons)},
            "schema": "OTERYN_WORLD_OBJECT_TERRAIN_HELD/v1",
            "source": meta,
            "statement": (
                "A held id has no family key. It is listed by its existing frozen Item "
                "key only and stays a non-materializable Item record until an owner or "
                "architect call resolves it."
            ),
        }
    )
    files[f"{directory}/index.json"] = dumps(
        {
            "decision": DECISION,
            "family": family,
            "held": f"{directory}/held.json",
            "held_count": len(held),
            "kind_counts": dict(sorted(by_kind.items())),
            "record_count": len(records),
            "schema": "OTERYN_FAMILY_INDEX/v1",
            "shard_size": SHARD_SIZE,
            "shards": shard_paths,
            "shards_sha256": digest.hexdigest(),
            "source": meta,
        }
    )
    summary = {
        "family": family,
        "held": len(held),
        "held_by_reason": dict(sorted(held_reasons.items())),
        "minted": len(records),
        "minted_by_kind": dict(sorted(by_kind.items())),
        "shards": len(shard_paths),
    }
    return files, summary


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--family", choices=sorted(FAMILY_DIRECTORY), required=True)
    parser.add_argument(
        "--root", type=Path, default=REPO_ROOT, help="repository root to write into"
    )
    parser.add_argument("--check", action="store_true", help="diff, do not write")
    args = parser.parse_args()

    sources = wo.engine_items.load_engine_sources("crystal", args.source)
    files, summary = render(sources, args.family)
    if args.check:
        drift = [
            path
            for path, body in sorted(files.items())
            if not (args.root / path).is_file()
            or (args.root / path).read_bytes() != body
        ]
        directory = FAMILY_DIRECTORY[args.family][0]
        on_disk = {
            path.relative_to(args.root).as_posix()
            for path in (args.root / directory).glob("*.json")
        }
        drift += sorted(on_disk - set(files))
        if drift:
            print(json.dumps({"check": "DRIFT", "paths": drift[:10]}))
            raise SystemExit(1)
        print(json.dumps({"check": "PASS", **summary}, sort_keys=True))
        return
    for path, body in files.items():
        target = args.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(body)
    print(json.dumps(summary, sort_keys=True))


if __name__ == "__main__":
    sys.exit(main())
