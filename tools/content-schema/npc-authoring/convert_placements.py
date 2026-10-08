#!/usr/bin/env python3
"""Write the `Npc.Placement` family (NPC-PLACE-1 §4) from the committed promotion candidates.

Writes content/world/npc-placements/ (index.json, held.json and placements-*.json shards) for the
NPC keys of npc_placement_scope.json, and only those that the NPC catalogue admits. A cell shared
with another placement is held (SCHEDULE_VARIANT, SHARED_CELL) instead of written; one NPC listed
twice at one cell and direction is one record. A wiki-origin placement has no direction in the
source and is written as north. The source is OTS_HYPOTHESIS_ONLY migration evidence: only
normalized facts are written. Fails closed on an unadmitted key or an out-of-range position.

    python convert_placements.py [--check]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
OUT = ROOT / "content/world/npc-placements"
CANDIDATES = HERE / "samples/promotion-candidates-v1.json"
SCOPE = HERE / "npc_placement_scope.json"
NPCS = ROOT / "content/npcs/definitions"
FRAME = "global-target-2026-09-27"
SHARD_SIZE = 500
SOURCES = {
    "canary": ("opentibiabr/canary", "47dfd51f45280a59a1d3e50ba7edd573d7234446", "oteryn:source.canary"),
    "crystal": ("zimbadev/crystalserver", "ff7ede593c69d4c658b382c97443e8155926924a", "oteryn:source.crystalserver"),
}
WIKI_REPOSITORY = "tibia.fandom.com"
REASONS = ("NO_PLACEMENT_SOURCE", "POSITION_CONFLICT", "SCHEDULE_VARIANT", "SHARED_CELL")


def dump(value) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def admitted_keys() -> set[str]:
    keys = set()
    for path in sorted(NPCS.glob("npcs-*.json")):
        for record in json.loads(path.read_text())["records"]:
            keys.add(record.get("definition", record.get("declaration"))["identity"]["key"])
    return keys


def local_name(npc: str) -> str:
    return npc.removeprefix("oteryn:npc.")


def cell_of(position: dict) -> tuple[int, int, int]:
    x, y, z = position["x"], position["y"], position["z"]
    if not (0 <= x <= 0xFFFF and 0 <= y <= 0xFFFF and 0 <= z <= 15):
        sys.exit(f"position ({x}, {y}, {z}) is outside the project frame")
    return x, y, z


def provenance(candidate: dict, placement: dict) -> list[dict]:
    arbitration = sorted(candidate.get("arbitration", []), key=dump)
    if placement.get("origin") == "wiki":
        return [
            {
                "arbitration": arbitration,
                "direction_note": "no direction in the source; written as north (engine default)",
                "origin": "wiki",
                "pageid": candidate["wiki"]["pageid"],
                "repository": WIKI_REPOSITORY,
                "revid": candidate["wiki"]["revid"],
            }
        ]
    rows = []
    for origin in ("canary", "crystal"):
        source = candidate["provenance"].get(origin)
        if source is None:
            continue
        repository, revision, source_key = SOURCES[origin]
        rows.append(
            {
                "arbitration": arbitration,
                "definition_sha256": source["sha256"],
                "origin": origin,
                "repository": repository,
                "revision": revision,
                "source_key": source_key,
                "source_row": source["key"],
            }
        )
    return rows


def convert() -> dict[str, str]:
    report = json.loads(CANDIDATES.read_text())
    candidates = {c["identity"]["key"]: c for c in report["candidates"]}
    scope = json.loads(SCOPE.read_text())
    admitted = admitted_keys()
    npcs = sorted(scope["npcs"])
    if len(set(npcs)) != len(npcs):
        sys.exit("the scope lists an NPC twice")
    unadmitted = [n for n in npcs if n not in admitted]
    if unadmitted:
        sys.exit(f"not an admitted NPC definition: {unadmitted}")

    # Every candidate placement counts for cell sharing, scoped or not.
    occupancy = defaultdict(list)
    for key, candidate in candidates.items():
        for placement in candidate["placements"]:
            direction = placement["direction"] or "NORTH"
            occupancy[cell_of(placement["position"])].append((key, direction))

    conflicts = {row["npc"]: row for row in scope["position_conflicts"]}
    held, records = [], {}
    for npc in npcs:
        candidate = candidates.get(npc)
        if npc in conflicts:
            held.append({"evidence": conflicts[npc]["evidence"], "npc": npc, "reason": "POSITION_CONFLICT"})
            continue
        if candidate is None or not candidate["placements"]:
            held.append({"evidence": [], "npc": npc, "reason": "NO_PLACEMENT_SOURCE"})
            continue
        for placement in candidate["placements"]:
            cell = cell_of(placement["position"])
            direction = (placement["direction"] or "NORTH").lower()
            sharers = set(occupancy[cell])
            if len(sharers) > 1:
                bases = {k.removesuffix("_day").removesuffix("_night") for k, _ in sharers}
                variants = len({k for k, _ in sharers}) > 1 and len(bases) == 1
                reason = "SCHEDULE_VARIANT" if variants else "SHARED_CELL"
                entry = {
                    "evidence": [{"cell": {"floor": cell[2], "x": cell[0], "y": cell[1]}, "npc": k, "direction": d.lower()} for k, d in sorted(sharers)],
                    "npc": npc,
                    "reason": reason,
                }
                if entry not in held:
                    held.append(entry)
                continue
            x, y, z = cell
            key = f"oteryn:npc_placement.{local_name(npc)}.x{x}_y{y}_z{z}"
            rows = provenance(candidate, placement)
            if key in records:
                existing = records[key]["declaration"]
                if existing["direction"] != direction:
                    sys.exit(f"{key}: two directions at one cell")
                for row in rows:
                    if row not in existing["provenance"]:
                        existing["provenance"].append(row)
                continue
            records[key] = {
                "declaration": {
                    "cell": {"floor": z, "x": x, "y": y},
                    "direction": direction,
                    "identity": {"key": key, "revision": "definition-r1"},
                    "npc": npc,
                    "provenance": rows,
                }
            }
    placed = {r["declaration"]["npc"] for r in records.values()}
    for npc in npcs:
        if npc not in placed and not any(h["npc"] == npc for h in held):
            held.append({"evidence": [], "npc": npc, "reason": "NO_PLACEMENT_SOURCE"})
    assert all(h["reason"] in REASONS for h in held)
    held.sort(key=dump)

    ordered = [records[k] for k in sorted(records)]
    files, shards = {}, []
    for start in range(0, len(ordered), SHARD_SIZE):
        part = ordered[start : start + SHARD_SIZE]
        name = f"placements-{start:05d}-{start + len(part) - 1:05d}.json"
        shards.append(f"content/world/npc-placements/{name}")
        files[name] = dump({"coordinate_frame": FRAME, "family": "Npc.Placement", "records": part})
    held_text = dump({"coordinate_frame": FRAME, "family": "Npc.Placement", "held": held, "schema": "OTERYN_NPC_PLACEMENT_HELD/v1"})
    files["held.json"] = held_text
    files["index.json"] = dump(
        {
            "coordinate_frame": FRAME,
            "family": "Npc.Placement",
            "generator": "tools/content-schema/npc-authoring/convert_placements.py",
            "held": {
                "count": len(held),
                "path": "content/world/npc-placements/held.json",
                "sha256": hashlib.sha256((held_text + "\n").encode()).hexdigest(),
            },
            "inputs": {
                "candidates_sha256": hashlib.sha256(CANDIDATES.read_bytes()).hexdigest(),
                "scope_sha256": hashlib.sha256(SCOPE.read_bytes()).hexdigest(),
            },
            "npc_count": len(placed),
            "population_state": "POPULATED",
            "record_count": len(ordered),
            "schema": "OTERYN_FAMILY_INDEX/v1",
            "shard_size": SHARD_SIZE,
            "shards": shards,
            "source_evidence": "OtsHypothesisOnly",
        }
    )
    return {name: text + "\n" for name, text in files.items()}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    files = convert()
    if args.check:
        have = {p.name: p.read_text() for p in OUT.glob("*.json")}
        if have != files:
            sys.exit("content/world/npc-placements is not the conversion of the committed candidates")
        return
    OUT.mkdir(parents=True, exist_ok=True)
    for stale in OUT.glob("*.json"):
        stale.unlink()
    for name, text in files.items():
        (OUT / name).write_text(text)


if __name__ == "__main__":
    main()
