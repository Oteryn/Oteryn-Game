"""Build the Terrain and WorldObject catalogues (WO-2) from the pinned Crystal sources.

Writes every record `world_objects.build_census` validates into `content/world/terrain/`
and `content/world/objects/`, in shards of 500 records ordered by source (Tibia) id, and
marks both directories POPULATED. Each record's key is the A12 §4.6 family key of its
Tibia Item key; the routed Item keeps its key (the `routed_to` pointer on the Item record
and the typed relation references are WO-2b).

`--check` rebuilds every file in memory and fails on any byte difference, missing or
extra shard. The census sample is rebuilt from the same run, so both always agree.

Architecture: docs/architecture/reviews/
OTERYN_GAME_WO0_WORLD_OBJECT_AND_TERRAIN_AUTHORING_FORMAT_DECISION_2026-09-28.md
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import world_objects

ROOT = Path(__file__).resolve().parents[3]
SHARD = 500
CATALOGUES = {
    # family: (directory, shard prefix, marker notes)
    "Terrain": ("content/world/terrain", "terrain", "Terrain definitions"),
    "WorldObject": (
        "content/world/objects",
        "objects",
        "Doors, ladders, beds, corpses, furniture, decorations and other world objects",
    ),
}
BUILDER = "tools/content-schema/world-object-authoring/build_catalogue.py"


def marker(directory, family, count, source):
    notes = (
        f"{CATALOGUES[family][2]}: {count} records, one per Tibia id routed to {family} "
        f"by the item converter at {source['repository']}@{source['revision'][:7]} "
        f"(WO-0 D93/D94, A12 §4.6), built by {BUILDER}."
    )
    return {
        "schema": "OTERYN_GAME_TREE_DIRECTORY/v1",
        "path": f"{directory}/",
        "kind": "static_content",
        "owner": "WorldObject/LocalObject" if family == "WorldObject" else "Terrain",
        "repository": "Oteryn/Oteryn-Game",
        "population_state": "POPULATED",
        "contract": "docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md",
        "notes": notes,
    }


def outputs(sources):
    """Return ({relative path: bytes} for both catalogues, census bytes)."""
    records = {family: [] for family in CATALOGUES}
    result = world_objects.build_census(
        sources, on_record=lambda family, record: records[family].append(record)
    )
    files = {}
    for family, (directory, prefix, _notes) in CATALOGUES.items():
        rows = records[family]
        for start in range(0, len(rows), SHARD):
            chunk = rows[start : start + SHARD]
            name = f"{prefix}-{start:05d}-{start + len(chunk) - 1:05d}.json"
            document = {"family": family, "records": chunk}
            files[f"{directory}/{name}"] = (
                world_objects.canonical_bytes(document) + b"\n"
            )
        index = marker(directory, family, len(rows), result["source"])
        files[f"{directory}/index.json"] = (
            json.dumps(index, indent=2, ensure_ascii=False) + "\n"
        ).encode("utf-8")
    return files, world_objects.census_document_bytes(result)


def committed_files():
    found = set()
    for directory, _prefix, _notes in CATALOGUES.values():
        for path in (ROOT / directory).iterdir():
            found.add(path.relative_to(ROOT).as_posix())
    return found


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--source", type=Path, required=True, help="pinned Crystal checkout (ff7ede5)"
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="rebuild in memory and fail on any difference from the committed files",
    )
    args = parser.parse_args(argv)

    sources = world_objects.engine_items.load_engine_sources("crystal", args.source)
    files, census = outputs(sources)
    census_path = world_objects.DEFAULT_SAMPLE
    if args.check:
        drift = sorted(
            path
            for path, data in files.items()
            if not (ROOT / path).is_file() or (ROOT / path).read_bytes() != data
        )
        extra = sorted(committed_files() - set(files))
        if census_path.read_bytes() != census:
            drift.append(census_path.relative_to(ROOT).as_posix())
        if drift or extra:
            print(
                json.dumps({"drift": drift[:10], "extra": extra[:10]}), file=sys.stderr
            )
            return 1
        print(json.dumps({"check": "PASS", "files": len(files)}))
        return 0

    for path in committed_files() - set(files):
        (ROOT / path).unlink()
    for path, data in files.items():
        (ROOT / path).write_bytes(data)
    census_path.write_bytes(census)
    counts = {family: 0 for family in CATALOGUES}
    for path in files:
        if not path.endswith("index.json"):
            family = "Terrain" if "/terrain/" in path else "WorldObject"
            counts[family] += len(json.loads(files[path])["records"])
    print(json.dumps({"records": counts, "files": len(files)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
