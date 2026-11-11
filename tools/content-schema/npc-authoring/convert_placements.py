#!/usr/bin/env python3
"""Write the `Npc.Placement` family (NPC-PLACE-1 §4) from the committed promotion candidates.

Writes content/world/npc-placements/ (index.json, held.json and placements-*.json shards) for the
NPC keys of npc_placement_scope.json, and only those that the NPC catalogue admits. A cell shared
with another placement is held (SCHEDULE_VARIANT, SHARED_CELL) instead of written; one NPC listed
twice at one cell and direction is one record. A wiki-origin placement has no direction in the
source and is written as north. Optional placement_cells narrows a scoped NPC to
a distinct subset of candidate cells; it never introduces coordinates or bypasses holds.
Optional block_solid_contacts admits only source-backed solid-object contacts;
the compiler still checks walls, floor changes and other terrain hazards.
Explicit placement_adjustments choose adjacent clear native ground while retaining
the donor cell. Pinned placement overrides preserve their own evidence and replace
only a scoped candidate's placement hypothesis. The source is OTS_HYPOTHESIS_ONLY
migration evidence: only normalized facts are written. Unadmitted keys and
out-of-range positions fail closed.

    python convert_placements.py [--check]
"""

from __future__ import annotations

import argparse
import copy
import re
import hashlib
import json
import sys
from collections import defaultdict
from pathlib import Path

import native_contacts

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
OUT = ROOT / "content/world/npc-placements"
CANDIDATES = HERE / "samples/promotion-candidates-v1.json"
SCOPE = HERE / "npc_placement_scope.json"
NPCS = ROOT / "content/npcs/definitions"
FRAME = "global-target-2026-09-27"
from chosen_placement_adjustments import choose_cell
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


def apply_placement_overrides(candidates: dict, scope: dict) -> dict:
    """Explicit pinned schedules supersede migration placement hypotheses, not definitions."""
    result = copy.deepcopy(candidates)
    for key, override in scope.get("placement_overrides", {}).items():
        if key not in scope["npcs"] or key not in result:
            raise ValueError("placement override requires a scoped candidate")
        source, placements = override["source"], override["placements"]
        if (source.get("origin") not in SOURCES
                or not re.fullmatch(r"[0-9a-f]{40}", source.get("revision", ""))
                or not re.fullmatch(r"[0-9a-f]{64}", source.get("sha256", ""))
                or not isinstance(source.get("path"), str)
                or source["path"].startswith("/") or ".." in source["path"].split("/")
                or not source["path"].endswith((".lua", ".xml"))
                or not override.get("basis") or not 1 <= len(placements) <= 16):
            raise ValueError("placement override needs bounded positions and immutable source evidence")
        cells = set()
        for placement in placements:
            position = placement["position"]
            if (placement.get("direction") not in ("NORTH", "SOUTH", "EAST", "WEST")
                    or any(type(position.get(k)) is not int for k in ("x", "y", "z"))
                    or not 0 <= position["x"] <= 65535 or not 0 <= position["y"] <= 65535
                    or not 0 <= position["z"] <= 15):
                raise ValueError("invalid native placement override")
            cell = cell_of(position)
            if cell in cells:
                raise ValueError("ambiguous native placement override")
            cells.add(cell)
        repository, _, source_key = SOURCES[source["origin"]]
        result[key]["placements"] = copy.deepcopy(placements)
        result[key]["placement_override_provenance"] = [{
            "origin": source["origin"], "repository": repository,
            "revision": source["revision"], "source_key": source_key,
            "source_row": source["path"], "source_sha256": source["sha256"],
            "url": f"https://github.com/{repository}/blob/{source['revision']}/{source['path']}",
            "arbitration": [{"fact": "placements", "chosen": source["origin"],
                             "rule": "EXPLICIT_PINNED_NATIVE_PLACEMENTS", "basis": override["basis"]}],
        }]
    return result


def apply_authored_placements(candidates: dict, scope: dict, admitted: set[str], root: Path = ROOT) -> dict:
    """Add explicit recipe NPC cells without relabelling or replacing donor evidence."""
    result = copy.deepcopy(candidates)
    for key, row in scope.get("authored_placements", {}).items():
        if key not in scope["npcs"] or key not in admitted or key in result:
            raise ValueError("authored placement requires a new scoped admitted NPC")
        path = Path(row["recipe_path"])
        if path.is_absolute() or ".." in path.parts:
            raise ValueError("authored placement recipe path")
        raw = (root / path).read_bytes()
        if (row.get("basis") != "CHOSEN_OTERYN_APPROXIMATION"
                or hashlib.sha256(raw).hexdigest() != row.get("recipe_sha256")):
            raise ValueError("authored placement recipe pin")
        recipes = [recipe for recipe in json.loads(raw).get("recipes", [record["recipe"] for record in json.loads(raw).get("records", [])])
                   if recipe["wiki_title"] == row["recipe_title"]]
        normalize = lambda value: re.sub(r"[^a-z0-9]+", "_", value.lower()).strip("_")
        if (len(recipes) != 1 or not any(
                normalize(target) == local_name(key)
                for stage in recipes[0]["stages"] if stage["kind"] == "talk"
                for target in stage["targets"])):
            raise ValueError("authored placement NPC is not a recipe talk target")
        placements = row["placements"]
        if not isinstance(placements, list) or not 1 <= len(placements) <= 8:
            raise ValueError("authored placement count")
        seen = set()
        for placement in placements:
            cell = cell_of(placement["position"])
            if placement["direction"] not in {"NORTH", "EAST", "SOUTH", "WEST"} or cell in seen:
                raise ValueError("authored placement direction or duplicate")
            seen.add(cell)
        result[key] = {
            "identity": {"key": key}, "placements": copy.deepcopy(placements),
            "placement_override_provenance": [{
                "origin": "oteryn", "repository": "Oteryn/Oteryn-Game",
                "basis": row["basis"], "source_row": str(path) + "#" + row["recipe_title"],
                "definition_sha256": row["recipe_sha256"],
                "direction_note": "Explicit chosen placement; no donor coordinate claim.",
            }],
        }
    return result



def provenance(candidate: dict, placement: dict) -> list[dict]:
    if "placement_override_provenance" in candidate:
        return copy.deepcopy(candidate["placement_override_provenance"])
    if placement.get("origin") == "native_wiki_br":
        return [candidate["native_provenance"]]
    # Only the placement decisions are evidence for the cell; offer or trade arbitration is not.
    arbitration = sorted(
        (row for row in candidate.get("arbitration", []) if row.get("fact") == "placements"), key=dump
    )
    chosen = {row["chosen"] for row in arbitration}
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
        # An arbitrated cell is attributed to the chosen source only; agreeing sources keep both rows.
        if source is None or (chosen and origin not in chosen):
            continue
        repository, revision, source_key = SOURCES[origin]
        # Supplement NPCs already carry their own immutable donor revision. Do not
        # attribute their definition digest to the older baseline source commit.
        revision = source.get("revision", revision)
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
    native = native_contacts.candidates(NPCS, scope.get("native_contacts", []))
    if set(native) & set(candidates):
        raise ValueError("native contacts cannot replace donor candidates")
    if not set(native) <= set(scope["npcs"]):
        raise ValueError("native_contacts names an unscoped NPC")
    candidates.update(native)
    candidates = apply_placement_overrides(candidates, scope)
    admitted = admitted_keys()
    candidates = apply_authored_placements(candidates, scope, admitted)
    npcs = sorted(scope["npcs"])
    if len(set(npcs)) != len(npcs):
        sys.exit("the scope lists an NPC twice")
    unadmitted = [n for n in npcs if n not in admitted]
    if unadmitted:
        sys.exit(f"not an admitted NPC definition: {unadmitted}")

    # A candidate can include both the Global world and an example map. Scope
    # may select known cells, but never introduce a cell or bypass sharing rules.
    contacts = scope.get("block_solid_contacts", [])
    if not isinstance(contacts, list) or not set(contacts) <= set(npcs):
        sys.exit("block_solid_contacts names an unscoped NPC")
    selections = scope.get("placement_cells", {})
    if not isinstance(selections, dict) or not set(selections) <= set(npcs):
        sys.exit("placement_cells names an unscoped NPC")
    selected_cells = {}
    for npc, cells in selections.items():
        if not isinstance(cells, list) or not cells:
            sys.exit(f"{npc}: placement selection is empty")
        selected = {cell_of(cell) for cell in cells}
        available = {cell_of(p["position"]) for p in candidates.get(npc, {}).get("placements", [])}
        if len(selected) != len(cells) or not selected <= available:
            sys.exit(f"{npc}: placement selection is not a distinct subset of source cells")
        selected_cells[npc] = selected

    adjustments = scope.get("placement_adjustments", {})
    if not isinstance(adjustments, dict) or not set(adjustments) <= set(npcs):
        sys.exit("placement_adjustments names an unscoped NPC")
    if any(len(candidates[npc]["placements"]) != 1 for npc in adjustments):
        sys.exit("chosen adjacent placement requires a unique retained source cell")
    adjusted_cells = set()

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
            if npc in selected_cells and cell not in selected_cells[npc]:
                continue
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
            choice_evidence = None
            if npc in adjustments:
                cell, choice_evidence = choose_cell(ROOT, cell, adjustments[npc], set(occupancy) | adjusted_cells)
                adjusted_cells.add(cell)
            x, y, z = cell
            key = f"oteryn:npc_placement.{local_name(npc)}.x{x}_y{y}_z{z}"
            rows = provenance(candidate, placement)
            if choice_evidence is not None:
                rows.append(choice_evidence)
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
    for record in records.values():
        if record["declaration"]["npc"] in contacts:
            record["declaration"]["block_solid_contact"] = True
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
        if start == 0:
            # Keep Main's published shard path as admitted placements grow.
            name = "placements-00000-00055.json"
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
                **({"native_contacts_sha256": hashlib.sha256(dump(native).encode()).hexdigest()}
                   if native else {}),
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
