#!/usr/bin/env python3
"""Structural and semantic validation of the committed `Area.Island` family.

python validate_islands.py [--root REPOSITORY_ROOT]

Checks schema, canonical bytes, contiguous shards and no stray file, sorted unique keys, the
snapshot / ground class / base map pins, page bindings against the snapshot (every snapshot
page is bound or excluded exactly once), City references (a listed city's temple lies in the
island's footprint on its floor), footprints and anchors inside the World bounds and floors,
the anchor inside its footprint and within 5 tiles of the snapshot coordinate (or equal to
the map correction or to its evidence anchor, `island-evidence-anchors.json`), additional
components, archipelago component links, `underground` (exactly the components below floor 7)
and the capture summary against the records.
It does not re-run the component search; `convert_islands.py --check` does that from the map.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

from jsonschema import Draft202012Validator

HERE = Path(__file__).resolve().parent
SCHEMA = json.loads((HERE / "island.schema.json").read_text(encoding="utf-8"))
DIRECTORY = "content/world/areas/islands"
STEM = "islands"
INDEX = f"{DIRECTORY}/index.json"
SUMMARY = "tools/content-schema/world-authoring/samples/islands-capture-v1.json"
SNAPSHOT = "imports/tibiawiki/islands/fandom-snapshot-v1.json"
GROUND_CLASSES = "tools/content-schema/world-authoring/island-ground-classes.json"
EVIDENCE = "tools/content-schema/world-authoring/island-evidence-anchors.json"
PLACEMENT_INDEX = "content/world/placements/index.json"
CITIES = "content/world/areas/cities"
WORLD_SHARD = "content/world/worlds/worlds-00000-00000.json"
SHARD_SIZE = 500
CAP = 400_000
ANCHOR_RADIUS = 5
SURFACE = 7
NAMESPACE = "tibiawiki-fandom/page-id"


class ValidationError(Exception):
    pass


def load(root: Path, path: str):
    try:
        return json.loads((root / path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValidationError(f"{path}: unreadable ({error})") from error


def canonical(value) -> str:
    return (
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        + "\n"
    )


def structural(kind: str, path: str, data, errors: list[str]) -> None:
    schema = {"$ref": f"#/$defs/{kind}", "$defs": SCHEMA["$defs"]}
    for error in Draft202012Validator(schema).iter_errors(data):
        errors.append(f"{path}: schema: {error.message[:200]}")


def sha256(root: Path, path: str) -> str:
    return hashlib.sha256((root / path).read_bytes()).hexdigest()


def read_family(root: Path, errors: list[str]) -> tuple[dict, list[dict]]:
    index = load(root, INDEX)
    structural("family_index", INDEX, index, errors)
    records: list[dict] = []
    expected = []
    for shard_index, path in enumerate(index.get("shards", [])):
        shard = load(root, path)
        structural("island_shard", path, shard, errors)
        start = len(records)
        chunk = shard.get("records", [])
        end = start + len(chunk) - 1
        name = f"{DIRECTORY}/{STEM}-{start:05d}-{end:05d}.json"
        expected.append(name)
        if path != name:
            errors.append(f"{path}: expected contiguous shard name {name}")
        if shard.get("shard") != {
            "count": len(chunk),
            "end": end,
            "index": shard_index,
            "start": start,
        }:
            errors.append(f"{path}: shard header does not match its records")
        if len(chunk) != SHARD_SIZE and shard_index != len(index["shards"]) - 1:
            errors.append(f"{path}: only the last shard may be partial")
        if (root / path).read_text(encoding="utf-8") != canonical(shard):
            errors.append(f"{path}: not canonical JSON")
        records.extend(chunk)
    if (root / INDEX).read_text(encoding="utf-8") != canonical(index):
        errors.append(f"{INDEX}: not canonical JSON")
    actual = sorted(str(p.relative_to(root)) for p in (root / DIRECTORY).iterdir())
    if actual != sorted([INDEX, *expected]):
        errors.append(
            f"{DIRECTORY}: files other than the index and its shards: "
            f"{sorted(set(actual) - {INDEX, *expected})}"
        )
    if index.get("record_count") != len(records):
        errors.append(f"{INDEX}: record_count differs from the shards")
    keys = [r.get("declaration", {}).get("identity", {}).get("key") for r in records]
    if keys != sorted(keys) or len(set(keys)) != len(keys):
        errors.append("Area.Island: keys must be unique and sorted")
    return index, records


def city_temples(root: Path) -> dict[str, dict | None]:
    """AREAS-1 city key -> hometown temple (x, y, floor), None without a hometown."""
    temples = {}
    for shard in sorted((root / CITIES).glob("areas-*.json")):
        for record in load(root, str(shard.relative_to(root)))["areas"]:
            hometown = record["hometown"]
            temple = hometown and hometown["temple"]
            temples[record["identity"]["key"]] = temple and {
                "x": temple["x"],
                "y": temple["y"],
                "floor": temple["z"],
            }
    return temples


def world_limits(root: Path) -> tuple[dict, set[int]]:
    declaration = load(root, WORLD_SHARD)["records"][0]["declaration"]
    return declaration["bounds"], set(declaration["floors"])


def inside_world(x: int, y: int, floor: int, bounds: dict, floors: set[int]) -> bool:
    return (
        bounds["min_x"] <= x < bounds["max_x_exclusive"]
        and bounds["min_y"] <= y < bounds["max_y_exclusive"]
        and floor in floors
    )


def footprints(declaration: dict) -> list[tuple[dict, dict]]:
    """(anchor, footprint) per component of a record, the primary component first."""
    if declaration["area_kind"] == "archipelago":
        return [(c["anchor"], c["footprint"]) for c in declaration["components"]]
    return [
        (declaration["anchor"], declaration["footprint"]),
        *(
            (c["anchor"], c["footprint"])
            for c in declaration.get("additional_components", [])
        ),
    ]


def check_footprint(
    key: str, anchor: dict, box: dict, bounds: dict, floors: set[int], errors: list[str]
) -> None:
    if box["min_x"] > box["max_x"] or box["min_y"] > box["max_y"]:
        errors.append(f"{key}: footprint bounding box is inverted")
        return
    area = (box["max_x"] - box["min_x"] + 1) * (box["max_y"] - box["min_y"] + 1)
    if box["tile_count"] > area:
        errors.append(f"{key}: tile_count exceeds the bounding box area")
    if box["tile_count"] >= CAP:
        errors.append(f"{key}: tile_count reaches the component cap")
    for x in (box["min_x"], box["max_x"]):
        for y in (box["min_y"], box["max_y"]):
            if not inside_world(x, y, box["floor"], bounds, floors):
                errors.append(f"{key}: footprint corner {x},{y} is outside the World")
    if anchor["floor"] != box["floor"] or not (
        box["min_x"] <= anchor["x"] <= box["max_x"]
        and box["min_y"] <= anchor["y"] <= box["max_y"]
    ):
        errors.append(f"{key}: anchor is outside its footprint")


def near(a: dict, b: dict) -> bool:
    return (
        a["floor"] == b["floor"]
        and abs(a["x"] - b["x"]) <= ANCHOR_RADIUS
        and abs(a["y"] - b["y"]) <= ANCHOR_RADIUS
    )


def validate_records(
    root: Path,
    records: list[dict],
    snapshot: dict,
    summary: dict,
    evidence: dict,
    errors: list[str],
) -> None:
    pages = {str(p["pageid"]): p for p in snapshot["pages"]}
    by_title = {p["title"]: p for p in snapshot["pages"]}
    temples = city_temples(root)
    bounds, floors = world_limits(root)
    bound_ids: list[str] = []
    by_key = {r["declaration"]["identity"]["key"]: r["declaration"] for r in records}
    for record in records:
        declaration = record["declaration"]
        key = declaration["identity"]["key"]
        kind = declaration["area_kind"]
        facts = declaration["source_facts"]
        rows = record["source_bindings"]
        ids = [row["external_id"] for row in rows]
        if ids != sorted(ids, key=int) or len(set(ids)) != len(ids):
            errors.append(f"{key}: bindings must be unique and ordered by page id")
        bound_ids.extend(ids)
        for row in rows:
            if row["target"] != {
                "family": "Area",
                "key": key,
                "revision": "definition-r1",
            }:
                errors.append(f"{key}: source binding target differs from identity")
            page = pages.get(row["external_id"])
            if page is None or str(page["revid"]) != row["source_revision"]:
                errors.append(f"{key}: page/revision is not in the pinned snapshot")
        bound_pages = [pages[i] for i in ids if i in pages]
        if not bound_pages:
            continue
        primary = next(
            (p for p in bound_pages if p["title"] == declaration["name"]), None
        )
        if primary is None:
            errors.append(f"{key}: no bound page is titled {declaration['name']!r}")
            continue
        aliases = sorted(p["title"] for p in bound_pages if p is not primary)
        if declaration.get("also_known_as", []) != aliases:
            errors.append(f"{key}: also_known_as differs from the merged pages")
        for page in bound_pages:
            if page is not primary and page.get("alias_of") != primary["title"]:
                errors.append(
                    f"{key}: page {page['title']} is not an alias of the name"
                )
        if declaration.get("event_only", False) != any(
            p["event_only"] for p in bound_pages
        ):
            errors.append(f"{key}: event_only differs from the snapshot")
        if (
            facts["evidence"] != primary["evidence"]
            or facts["wiki_class"] != primary["wiki_class"]
        ):
            errors.append(f"{key}: evidence or wiki_class differs from the snapshot")
        if facts.get("evidence_page") != primary.get("evidence_page"):
            errors.append(f"{key}: evidence_page differs from the snapshot")
        if facts.get("wiki_status") != primary.get("status"):
            errors.append(f"{key}: wiki_status differs from the snapshot")
        if facts.get("removed_from_game", False) != primary.get("removed", False):
            errors.append(f"{key}: removed_from_game differs from the snapshot")
        if kind == "continent" and primary.get("kind_hint") != "continent":
            errors.append(f"{key}: continent needs the wiki kind hint")
        if kind == "archipelago" and primary["wiki_class"] != "archipelago":
            errors.append(f"{key}: only a wiki archipelago page may be an archipelago")
        corrected = "map_correction" in primary
        if declaration.get("anchor_corrected_from_wiki", False) != corrected:
            errors.append(
                f"{key}: anchor_corrected_from_wiki differs from the snapshot"
            )
        for city in facts.get("wiki_cities", []):
            if city["key"] not in temples:
                errors.append(f"{key}: wiki city {city['key']} is not an AREAS-1 city")
        wiki_names = sorted(primary.get("wiki_cities", []))
        if len(facts.get("wiki_cities", [])) > len(wiki_names):
            errors.append(f"{key}: more wiki cities than the snapshot names")
        parts = footprints(declaration)
        entry = evidence.get(primary["pageid"])
        anchored = facts.get("anchor_origin") == "evidence_anchor"
        if anchored != (entry is not None) or anchored != ("anchor_source" in facts):
            errors.append(f"{key}: evidence anchor differs from the evidence file")
        if entry is not None:
            expected = (entry["x"], entry["y"], entry["floor"])
            got = (parts[0][0]["x"], parts[0][0]["y"], parts[0][0]["floor"])
            if got != expected or facts.get("anchor_source") != entry["anchor_source"]:
                errors.append(f"{key}: anchor differs from its evidence anchor")
            if facts.get("component_note") != entry.get("note"):
                errors.append(f"{key}: component_note differs from the evidence file")
        elif "component_note" in facts:
            errors.append(f"{key}: component_note needs an evidence anchor")
        if declaration.get("underground", False) != (
            parts[0][1]["floor"] > SURFACE
        ) or any(box["floor"] > SURFACE for _, box in parts[1:]):
            errors.append(f"{key}: underground differs from the footprint floors")
        for number, (anchor, box) in enumerate(parts):
            check_footprint(key, anchor, box, bounds, floors, errors)
            if number == 0 and anchored:
                continue
            if corrected:
                fix = primary["map_correction"]
                if (anchor["x"], anchor["y"], anchor["floor"]) != (
                    fix["x"],
                    fix["y"],
                    fix["floor"],
                ):
                    errors.append(f"{key}: anchor differs from the map correction")
            elif not any(near(anchor, c) for c in primary["coordinates"]):
                errors.append(
                    f"{key}: anchor is not within 5 tiles of a snapshot coordinate"
                )
        if kind == "archipelago":
            order = [(b["min_x"], b["min_y"]) for _, b in parts]
            if order != sorted(set(order)):
                errors.append(f"{key}: components must be distinct and ordered")
            for component in declaration["components"]:
                link = component.get("island")
                if link is None:
                    continue
                island = by_key.get(link["key"])
                if (
                    island is None
                    or island["area_kind"] == "archipelago"
                    or island["footprint"] != component["footprint"]
                ):
                    errors.append(f"{key}: component island link {link['key']} differs")
        listed = declaration.get("cities", [])
        listed_keys = [c["key"] for c in listed]
        if listed_keys != sorted(set(listed_keys)):
            errors.append(f"{key}: cities must be unique and sorted")
        for city in listed_keys:
            temple = temples.get(city)
            if city not in temples:
                errors.append(f"{key}: city {city} is not an AREAS-1 city")
            elif temple is None:
                errors.append(f"{key}: city {city} has no hometown temple")
            elif not any(
                temple["floor"] == box["floor"]
                and box["min_x"] <= temple["x"] <= box["max_x"]
                and box["min_y"] <= temple["y"] <= box["max_y"]
                for _, box in parts
            ):
                errors.append(f"{key}: temple of {city} is outside the footprint")
    excluded = summary["excluded"]
    excluded_ids = [str(row["pageid"]) for row in excluded]
    if len(set(bound_ids)) != len(bound_ids) or set(bound_ids) & set(excluded_ids):
        errors.append("Area.Island: a page is bound twice or both bound and excluded")
    if set(bound_ids) | set(excluded_ids) != set(pages) or len(excluded_ids) != len(
        set(excluded_ids)
    ):
        errors.append("Area.Island: bound plus excluded pages differ from the snapshot")
    for row in excluded:
        page = pages.get(str(row["pageid"]))
        if page is None or page["title"] != row["title"]:
            errors.append(f"excluded {row['title']}: not a snapshot page")
    for row in snapshot["pages"]:
        for name in ("alias_of", "place_within"):
            if name in row and row[name] not in by_title:
                errors.append(f"snapshot {row['title']}: {name} is not a snapshot page")


def validate_summary(
    records: list[dict], summary: dict, index: dict, errors: list[str]
) -> None:
    if summary["source"] != index.get("source"):
        errors.append(f"{INDEX}: source differs from the capture summary")
    if summary["families"] != {"Area.Island": len(records)}:
        errors.append("Area.Island: capture family count differs from the records")
    included = [
        {
            "area_kind": r["declaration"]["area_kind"],
            "components": len(footprints(r["declaration"])),
            "key": r["declaration"]["identity"]["key"],
            "tile_count": sum(b["tile_count"] for _, b in footprints(r["declaration"])),
            "title": r["declaration"]["name"],
        }
        for r in records
    ]
    listed = [
        {k: v for k, v in row.items() if k != "boundary_tiles"}
        for row in summary["included"]
    ]
    if listed != included:
        errors.append("Area.Island: capture included list differs from the records")
    kinds: dict[str, int] = {}
    for row in included:
        kinds[row["area_kind"]] = kinds.get(row["area_kind"], 0) + 1
    if summary["area_kinds"] != dict(sorted(kinds.items())):
        errors.append("Area.Island: capture area_kinds differ from the records")
    reasons: dict[str, int] = {}
    for row in summary["excluded"]:
        reasons[row["reason"]] = reasons.get(row["reason"], 0) + 1
    if summary["excluded_reasons"] != dict(sorted(reasons.items())):
        errors.append("Area.Island: capture excluded_reasons differ from the list")
    if [r["title"] for r in summary["excluded"]] != sorted(
        r["title"] for r in summary["excluded"]
    ):
        errors.append("Area.Island: excluded list must be sorted by title")
    declarations = [r["declaration"] for r in records]
    expected = {
        "additional_components": sum(
            "additional_components" in d for d in declarations
        ),
        "also_known_as": sum("also_known_as" in d for d in declarations),
        "anchor_corrected": sum(
            "anchor_corrected_from_wiki" in d for d in declarations
        ),
        "cities": sum("cities" in d for d in declarations),
        "component_island_links": sum(
            "island" in c for d in declarations for c in d.get("components", [])
        ),
        "components": sum(len(footprints(d)) for d in declarations),
        "event_only": sum("event_only" in d for d in declarations),
        "evidence_anchored": sum(
            d["source_facts"].get("anchor_origin") == "evidence_anchor"
            for d in declarations
        ),
        "underground": sum("underground" in d for d in declarations),
        "wiki_cities": sum("wiki_cities" in d["source_facts"] for d in declarations),
    }
    got = {k: v for k, v in summary["records_with"].items() if k in expected}
    if got != expected:
        errors.append(
            "Area.Island: capture records_with counts differ from the records"
        )
    moved = sum("source_coordinate" in d["source_facts"] for d in declarations)
    if summary["records_with"]["anchor_moved"] + expected["anchor_corrected"] != moved:
        errors.append("Area.Island: capture anchor_moved differs from the records")
    if summary["snapshot_pages"] != len(records) + len(summary["excluded"]) + sum(
        len(d.get("also_known_as", [])) for d in declarations
    ):
        errors.append(
            "Area.Island: capture snapshot_pages differs from included + excluded"
        )


def validate(root: Path) -> list[str]:
    errors: list[str] = []
    index, records = read_family(root, errors)
    summary = load(root, SUMMARY)
    structural("capture_summary", SUMMARY, summary, errors)
    snapshot = load(root, SNAPSHOT)
    structural("snapshot", SNAPSHOT, snapshot, errors)
    evidence = load(root, EVIDENCE)
    structural("evidence_anchors", EVIDENCE, evidence, errors)
    if errors:
        return errors
    if (root / SUMMARY).read_text(encoding="utf-8") != canonical(summary):
        errors.append(f"{SUMMARY}: not canonical JSON")
    if (root / SNAPSHOT).read_text(encoding="utf-8") != canonical(snapshot):
        errors.append(f"{SNAPSHOT}: not canonical JSON")
    if (root / EVIDENCE).read_text(encoding="utf-8") != canonical(evidence):
        errors.append(f"{EVIDENCE}: not canonical JSON")
    titles = {p["title"]: p["pageid"] for p in snapshot["pages"]}
    anchors = evidence["anchors"]
    if [a["title"] for a in anchors] != sorted(a["title"] for a in anchors) or any(
        titles.get(a["title"]) != a["pageid"] for a in anchors
    ):
        errors.append(f"{EVIDENCE}: anchors must be sorted snapshot pages")
    source = index["source"]
    for name, path in (
        ("snapshot", SNAPSHOT),
        ("ground_classes", GROUND_CLASSES),
        ("evidence_anchors", EVIDENCE),
        ("base_map", PLACEMENT_INDEX),
    ):
        if source[name] != {"path": path, "sha256": sha256(root, path)}:
            errors.append(f"{path}: differs from the sha256 pinned in the index")
    if summary["parameters"] != {"anchor_radius": ANCHOR_RADIUS, "component_cap": CAP}:
        errors.append(f"{SUMMARY}: search parameters differ from the documented ones")
    if summary["snapshot_pages"] != len(snapshot["pages"]):
        errors.append(f"{SUMMARY}: snapshot_pages differs from the snapshot")
    validate_records(
        root, records, snapshot, summary, {a["pageid"]: a for a in anchors}, errors
    )
    validate_summary(records, summary, index, errors)
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=HERE.parents[2])
    args = parser.parse_args()
    try:
        errors = validate(args.root.resolve())
    except (ValidationError, OSError, KeyError) as error:
        errors = [str(error)]
    for error in errors:
        print(f"FAIL {error}", file=sys.stderr)
    if errors:
        return 1
    print("PASS islands")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
