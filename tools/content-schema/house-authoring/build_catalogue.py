"""Build the House catalogue in content/houses/ (OTERYN_HOUSE_CATALOGUE_OWNER_CONTRACT_V1 §3).

The records come from `convert_houses.convert` over the committed client 15.30 staging, the
pinned CrystalServer sample and the door item ids. Keys are allocated there once from the
official name; an existing catalogue record keeps its key and revision, joined by
`provenance.source_id` (contract §2.1). The whole catalogue is validated before it is written.

Usage: python build_catalogue.py [--check]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import convert_houses as c
from validate_houses import validate

HERE = Path(__file__).resolve().parent
OUT = HERE.parents[2] / "content" / "houses"
SHARD = 500
SCHEMA = "OTERYN_HOUSE_AUTHORING/candidate-1"


def canonical(value: object) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True) + "\n").encode()


def existing_identities() -> dict[int, dict]:
    """source_id -> identity of the committed catalogue, so a rebuild never re-derives a key."""
    kept = {}
    for path in sorted(OUT.glob("houses-*.json")):
        for house in json.loads(path.read_text(encoding="utf-8"))["houses"]:
            kept[house["provenance"]["source_id"]] = house["identity"]
    return kept


def build() -> list[dict]:
    crystal = json.loads(c.CRYSTAL_SAMPLE.read_text(encoding="utf-8"))
    doors = set(json.loads(c.DOOR_ITEMS.read_text(encoding="utf-8"))["door_item_ids"])
    catalog, _ = c.convert(c.load_staged(), crystal, doors)
    kept = existing_identities()
    houses = catalog["houses"]
    for house in houses:
        house["identity"] = kept.get(
            house["provenance"]["source_id"], house["identity"]
        )
    missing = set(kept) - {h["provenance"]["source_id"] for h in houses}
    if missing:
        raise SystemExit(f"catalogue houses missing from the source: {sorted(missing)}")
    return sorted(houses, key=lambda h: h["identity"]["key"])


def outputs(houses: list[dict]) -> dict[str, bytes]:
    files = {}
    for start in range(0, len(houses), SHARD):
        chunk = houses[start : start + SHARD]
        name = f"houses-{start:05d}-{start + len(chunk) - 1:05d}.json"
        files[name] = canonical({"schema": SCHEMA, "houses": chunk})
    return files


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    houses = build()
    errors = validate({"schema": SCHEMA, "houses": houses})
    if errors:
        print("\n".join(errors[:20]), file=sys.stderr)
        return 1
    files = outputs(houses)
    committed = {p.name: p.read_bytes() for p in OUT.glob("houses-*.json")}
    if args.check:
        if committed != files:
            print("stale or tampered catalogue", file=sys.stderr)
            return 1
        print(f"ok ({len(houses)} houses, {len(files)} files)")
        return 0
    for name in set(committed) - set(files):
        (OUT / name).unlink()
    for name, data in files.items():
        (OUT / name).write_bytes(data)
    print(f"wrote {len(houses)} houses in {len(files)} files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
