#!/usr/bin/env python3
"""Structural and semantic validation of the committed world metadata families.

python validate_world_metadata.py [--root REPOSITORY_ROOT]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

from jsonschema import Draft202012Validator

HERE = Path(__file__).resolve().parent
SCHEMA = json.loads((HERE / "world-metadata.schema.json").read_text(encoding="utf-8"))
SUMMARY = "tools/content-schema/world-authoring/samples/source-capture-v1.json"
HUNTING_SUMMARY = (
    "tools/content-schema/world-authoring/samples/hunting-places-capture-v1.json"
)
REGION_SUMMARY = (
    "tools/content-schema/world-authoring/samples/map-regions-capture-v1.json"
)
CITY_SUMMARY = "tools/content-schema/world-authoring/samples/cities-capture-v1.json"
SNAPSHOT = "imports/tibiawiki/hunting-places/fandom-snapshot-v1.json"
CITY_SNAPSHOT = "imports/tibiawiki/cities/fandom-snapshot-v1.json"
NPC_INDEX = "content/npcs/definitions/index.json"
WIKI_NAMESPACE = "tibiawiki-fandom/page-id"
CLIENT_MANIFEST = "imports/official/client-assets/15.30/manifest.json"
ITEM_BINDINGS = "imports/crystalserver/bindings/items.json"
GENERATORS = {
    "Area.HuntingPlace": "tools/content-schema/world-authoring/convert_hunting_places.py",
    "Area.Region": "tools/content-schema/world-authoring/convert_map_regions.py",
}
OWN_SOURCE_REVISION = {"Area.HuntingPlace", "Area.Region"}
DEFAULT_GENERATOR = "tools/content-schema/world-authoring/convert_world_metadata.py"
FAMILIES = {
    "Area.City": ("content/world/areas/cities", "cities", "Area"),
    "Area.HuntingPlace": (
        "content/world/areas/hunting-places",
        "hunting-places",
        "Area",
    ),
    "Area.Region": ("content/world/areas/regions", "regions", "Area"),
    "House": ("content/houses", "houses", "House"),
    "Transition.Teleport": ("content/world/transitions", "teleports", "Transition"),
}
SHARD_SIZE = 500


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


def structural(path: str, data, errors: list[str]) -> None:
    for error in Draft202012Validator(SCHEMA).iter_errors(data):
        errors.append(f"{path}: schema: {error.message[:200]}")


def positions(declaration: dict):
    for name in ("temple", "entry", "from", "to", "position", "anchor"):
        if name in declaration:
            yield declaration[name]
    for door in declaration.get("doors", []):
        yield door["position"]


def family_records(
    root: Path, family: str, summary: dict, map_extent: dict, errors: list[str]
) -> list[dict]:
    directory, stem, identity_family = FAMILIES[family]
    index_path = f"{directory}/index.json"
    index = load(root, index_path)
    structural(index_path, index, errors)
    if index.get("family") != family:
        errors.append(f"{index_path}: family must be {family}")
    if index.get("source") != summary["source"]:
        errors.append(f"{index_path}: source differs from the capture summary")
    if index.get("generator") != GENERATORS.get(family, DEFAULT_GENERATOR):
        errors.append(f"{index_path}: generator does not belong to {family}")
    records: list[dict] = []
    expected = []
    for shard_index, shard_path in enumerate(index.get("shards", [])):
        shard = load(root, shard_path)
        structural(shard_path, shard, errors)
        start = len(records)
        chunk = shard.get("records", [])
        end = start + len(chunk) - 1
        name = f"{directory}/{stem}-{start:05d}-{end:05d}.json"
        expected.append(name)
        if shard_path != name:
            errors.append(f"{shard_path}: expected contiguous shard name {name}")
        if shard.get("shard") != {
            "count": len(chunk),
            "end": end,
            "index": shard_index,
            "start": start,
        }:
            errors.append(f"{shard_path}: shard header does not match its records")
        if len(chunk) != SHARD_SIZE and shard_index != len(index["shards"]) - 1:
            errors.append(f"{shard_path}: only the last shard may be partial")
        if (root / shard_path).read_text(encoding="utf-8") != canonical(shard):
            errors.append(f"{shard_path}: not canonical JSON")
        records.extend(chunk)
    if (root / index_path).read_text(encoding="utf-8") != canonical(index):
        errors.append(f"{index_path}: not canonical JSON")
    actual = sorted(str(p.relative_to(root)) for p in (root / directory).iterdir())
    if actual != sorted([index_path, *expected]):
        errors.append(
            f"{directory}: files other than the index and its shards: {sorted(set(actual) - {index_path, *expected})}"
        )
    if index.get("record_count") != len(records) or summary["families"].get(
        family
    ) != len(records):
        errors.append(
            f"{index_path}: record_count differs from shards or capture summary"
        )
    keys = [r.get("declaration", {}).get("identity", {}).get("key") for r in records]
    if keys != sorted(keys) or len(set(keys)) != len(keys):
        errors.append(f"{family}: keys must be unique and sorted")
    for record in records:
        declaration = record.get("declaration", {})
        key = declaration.get("identity", {}).get("key")
        for bound in record.get("source_bindings", []):
            if bound.get("target") != {
                "family": identity_family,
                "key": key,
                "revision": "definition-r1",
            }:
                errors.append(f"{key}: source binding target differs from identity")
            if (
                family not in OWN_SOURCE_REVISION
                and bound.get("identity_namespace") != WIKI_NAMESPACE
                and bound.get("source_revision") != summary["source"].get("revision")
            ):
                errors.append(
                    f"{key}: source binding revision differs from the pinned source"
                )
        for pos in positions(declaration):
            if not (pos["x"] < map_extent["width"] and pos["y"] < map_extent["height"]):
                errors.append(f"{key}: position outside the source map extent")
    return records


def hunting_places(
    root: Path, summary: dict, families: dict[str, list[dict]], errors: list[str]
) -> None:
    """HuntingPlace semantics: city refs, snapshot pin, page/revision bindings, capture counts."""
    records = families["Area.HuntingPlace"]
    try:
        raw = (root / SNAPSHOT).read_bytes()
        snapshot = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        errors.append(f"{SNAPSHOT}: unreadable ({error})")
        return
    if summary["source"].get("snapshot") != {
        "path": SNAPSHOT,
        "sha256": hashlib.sha256(raw).hexdigest(),
    }:
        errors.append(f"{SNAPSHOT}: differs from the sha256 pinned in the capture")
    if raw.decode("utf-8") != canonical(snapshot):
        errors.append(f"{SNAPSHOT}: not canonical JSON")
    pinned = {str(row["pageid"]): str(row["revid"]) for row in snapshot["pages"]}
    city_keys = {r["declaration"]["identity"]["key"] for r in families["Area.City"]}
    bound, with_ = set(), {"city": 0, "creatures": 0, "levels": 0, "position": 0}
    for record in records:
        declaration = record["declaration"]
        key = declaration["identity"]["key"]
        for row in record["source_bindings"]:
            if pinned.get(row["external_id"]) != row["source_revision"]:
                errors.append(f"{key}: page/revision is not in the pinned snapshot")
            bound.add(row["external_id"])
        if "city" in declaration:
            with_["city"] += 1
            if declaration["city"]["key"] not in city_keys:
                errors.append(
                    f"{key}: city {declaration['city']['key']} is not a City Area"
                )
        with_["position"] += "position" in declaration
        with_["levels"] += "recommended_levels" in declaration
        with_["creatures"] += "creature_names" in declaration.get("source_facts", {})
        names = declaration.get("source_facts", {}).get("creature_names", [])
        if names != sorted(names):
            errors.append(f"{key}: creature_names must be sorted")
    if bound != set(pinned) or len(bound) != len(records):
        errors.append(
            "Area.HuntingPlace: records must bind each snapshot page exactly once"
        )
    if summary.get("records_with") != with_:
        errors.append("Area.HuntingPlace: capture counts differ from the records")


def cities(
    root: Path, summary: dict, families: dict[str, list[dict]], errors: list[str]
) -> None:
    """City semantics: snapshot pin, page/revision bindings, NPC references, capture counts."""
    records = families["Area.City"]
    try:
        raw = (root / CITY_SNAPSHOT).read_bytes()
        snapshot = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        errors.append(f"{CITY_SNAPSHOT}: unreadable ({error})")
        return
    if summary["source"].get("snapshot") != {
        "path": CITY_SNAPSHOT,
        "sha256": hashlib.sha256(raw).hexdigest(),
    }:
        errors.append(f"{CITY_SNAPSHOT}: differs from the sha256 pinned in the capture")
    if raw.decode("utf-8") != canonical(snapshot):
        errors.append(f"{CITY_SNAPSHOT}: not canonical JSON")
    index_path = f"{FAMILIES['Area.City'][0]}/index.json"
    if load(root, index_path).get("enrichment") != summary["source"]:
        errors.append(f"{index_path}: enrichment differs from the capture summary")
    pages = {str(row["pageid"]): row for row in snapshot["pages"]}
    unmatched = {row["name"] for row in snapshot["unmatched"]}
    npc_keys = set()
    for shard in load(root, NPC_INDEX)["shards"]:
        npc_keys.update(
            r["declaration"]["identity"]["key"] for r in load(root, shard)["records"]
        )
    bound, apart = set(), set()
    with_ = {"implemented": 0, "npcs": 0, "source_facts": 0, "wiki_binding": 0}
    linked = 0
    for record in records:
        declaration = record["declaration"]
        key = declaration["identity"]["key"]
        name = declaration["name"]
        wiki = [
            b
            for b in record["source_bindings"]
            if b["identity_namespace"] == WIKI_NAMESPACE
        ]
        if not wiki:
            apart.add(name)
            if any(f in declaration for f in ("implemented", "npcs", "source_facts")):
                errors.append(f"{key}: wiki facts without a wiki binding")
            continue
        row = pages.get(wiki[0]["external_id"])
        if row is None or str(row["revid"]) != wiki[0]["source_revision"]:
            errors.append(f"{key}: page/revision is not in the pinned snapshot")
            continue
        if row["title"] != name:
            errors.append(f"{key}: bound page {row['title']!r} is not the exact name")
        bound.add(wiki[0]["external_id"])
        with_["wiki_binding"] += 1
        facts = declaration.get("source_facts", {})
        with_["implemented"] += "implemented" in declaration
        with_["npcs"] += "npcs" in declaration
        with_["source_facts"] += "source_facts" in declaration
        if "implemented" in declaration and (
            facts.get("implemented") != declaration["implemented"]
            or row["facts"].get("implemented", "").strip() != declaration["implemented"]
        ):
            errors.append(f"{key}: implemented differs from the pinned snapshot")
        for field in ("ruler", "near"):
            if (field in facts) != bool(row["facts"].get(field, "").strip()):
                errors.append(f"{key}: source_facts.{field} differs from the snapshot")
        npc_refs = [ref["key"] for ref in declaration.get("npcs", [])]
        if npc_refs != sorted(set(npc_refs)):
            errors.append(f"{key}: npcs must be unique and sorted")
        for npc in npc_refs:
            if npc not in npc_keys:
                errors.append(f"{key}: npc {npc} is not an NPC definition")
        names = facts.get("npc_names_unmatched", [])
        snapshot_names = set(row["facts"]["npc_names"])
        if names != sorted(names) or not set(names) <= snapshot_names:
            errors.append(f"{key}: unmatched npc names differ from the snapshot")
        if len(npc_refs) + len(names) != len(snapshot_names):
            errors.append(f"{key}: npcs and unmatched names do not cover the snapshot")
        linked += len(npc_refs)
    if bound != set(pages) or apart != unmatched:
        errors.append("Area.City: records must bind each snapshot page exactly once")
    if (
        summary.get("records_with") != with_
        or summary.get("npc_names_linked") != linked
    ):
        errors.append("Area.City: capture counts differ from the records")


def file_sha256(root: Path, path: str, errors: list[str]) -> str | None:
    try:
        return hashlib.sha256((root / path).read_bytes()).hexdigest()
    except OSError:
        errors.append(f"{path}: unreadable")
        return None


def regions(
    root: Path,
    summary: dict,
    families: dict[str, list[dict]],
    map_extent: dict,
    errors: list[str],
) -> None:
    """Region semantics: source pin, hierarchy, footprint files, city links, capture counts."""
    records = families["Area.Region"]
    source_file = summary["source"]["files"][0]
    manifest_doc = load(root, CLIENT_MANIFEST)
    manifest = {row["name"]: row["sha256"] for row in manifest_doc["files"]}
    if (
        summary["source"]["manifest"]["archive_sha256"]
        != manifest_doc["archive_sha256"]
    ):
        errors.append(f"{CLIENT_MANIFEST}: archive sha256 differs from the pinned one")
    map_sha = source_file["sha256"]
    if file_sha256(root, source_file["path"], errors) != map_sha or (
        manifest.get(source_file["path"].rsplit("/", 1)[-1]) != map_sha
        or not source_file["path"].endswith(f"map-{map_sha}.dat")
    ):
        errors.append(f"{source_file['path']}: differs from the pinned map file")
    cities = {
        r["declaration"]["identity"]["key"]: r["declaration"]["temple"]
        for r in families["Area.City"]
    }
    declarations = {
        r["declaration"]["identity"]["key"]: r["declaration"] for r in records
    }
    external_ids, children = set(), {}
    with_ = {"anchor": 0, "cities": 0, "footprint": 0, "parent_regions": 0}
    by_kind = {"region": 0, "subregion": 0}
    for record in records:
        declaration = record["declaration"]
        key = declaration["identity"]["key"]
        by_kind[declaration["area_kind"]] += 1
        for row in record["source_bindings"]:
            if row["source_revision"] != map_sha:
                errors.append(
                    f"{key}: binding revision differs from the pinned map file"
                )
            if row["external_id"] in external_ids:
                errors.append(f"{key}: area id {row['external_id']} is bound twice")
            external_ids.add(row["external_id"])
        for name in ("anchor", "cities", "footprint", "parent_regions"):
            with_[name] += name in declaration
        parents = declaration.get("parent_regions", [])
        parent_keys = [ref["key"] for ref in parents]
        if parent_keys != sorted(set(parent_keys)):
            errors.append(f"{key}: parent_regions must be unique and sorted")
        for parent in parent_keys:
            if declarations.get(parent, {}).get("area_kind") != "region":
                errors.append(f"{key}: parent {parent} is not a region")
            children.setdefault(parent, []).append(key)
        city_keys = [ref["key"] for ref in declaration.get("cities", [])]
        if city_keys != sorted(set(city_keys)):
            errors.append(f"{key}: cities must be unique and sorted")
        for city in city_keys:
            if city not in cities:
                errors.append(f"{key}: city {city} is not a City Area")
        footprint = declaration.get("footprint")
        if footprint is None:
            if declaration["area_kind"] == "subregion" and city_keys:
                errors.append(f"{key}: city links need a footprint")
            continue
        box = (
            footprint["min_x"],
            footprint["min_y"],
            footprint["max_x"],
            footprint["max_y"],
        )
        area = (box[2] - box[0] + 1) * (box[3] - box[1] + 1)
        if (
            box[0] > box[2]
            or box[1] > box[3]
            or not (box[2] < map_extent["width"] and box[3] < map_extent["height"])
        ):
            errors.append(
                f"{key}: footprint is inverted or outside the source map extent"
            )
        if not 1 <= footprint["tile_count"] <= area:
            errors.append(f"{key}: footprint tile_count does not fit its bounding box")
        image = footprint["image"]
        name = image["path"].rsplit("/", 1)[-1]
        if int(name.split("-")[1]) != int(record["source_bindings"][0]["external_id"]):
            errors.append(f"{key}: footprint image belongs to another area id")
        if file_sha256(root, image["path"], errors) != image["sha256"] or (
            manifest.get(name) != image["sha256"]
        ):
            errors.append(f"{key}: footprint image differs from the pinned file")
        for city in city_keys:
            temple = cities.get(city)
            if temple is not None and not (
                temple["floor"] == footprint["floor"]
                and box[0] <= temple["x"] <= box[2]
                and box[1] <= temple["y"] <= box[3]
            ):
                errors.append(f"{key}: city {city} temple is outside the footprint")
    for key, declaration in declarations.items():
        if declaration["area_kind"] != "region":
            continue
        kids = children.get(key, [])
        if not kids:
            errors.append(f"{key}: region without subregions")
        expected = sorted(
            {ref["key"] for kid in kids for ref in declarations[kid].get("cities", [])}
        )
        if [ref["key"] for ref in declaration.get("cities", [])] != expected:
            errors.append(f"{key}: cities differ from the cities of its subregions")
    linked = {ref["key"] for d in declarations.values() for ref in d.get("cities", [])}
    if (
        summary.get("records_with") != with_
        or summary.get("records_by_kind") != by_kind
        or summary.get("cities") != {"linked": len(linked), "total": len(cities)}
    ):
        errors.append("Area.Region: capture counts differ from the records")


def validate(root: Path) -> list[str]:
    errors: list[str] = []
    summary = load(root, SUMMARY)
    hunting_summary = load(root, HUNTING_SUMMARY)
    region_summary = load(root, REGION_SUMMARY)
    city_summary = load(root, CITY_SUMMARY)
    own_summaries = {
        "Area.HuntingPlace": hunting_summary,
        "Area.Region": region_summary,
    }
    families = {
        family: family_records(
            root,
            family,
            own_summaries.get(family, summary),
            summary["map"],
            errors,
        )
        for family in FAMILIES
    }
    if any(": schema:" in error for error in errors):
        return errors  # semantic checks assume schema-valid records
    cities(root, city_summary, families, errors)
    hunting_places(root, hunting_summary, families, errors)
    regions(root, region_summary, families, summary["map"], errors)
    city_keys = {r["declaration"]["identity"]["key"] for r in families["Area.City"]}
    item_keys = {row["target"]["key"] for row in load(root, ITEM_BINDINGS)["bindings"]}
    for record in families["House"]:
        declaration = record["declaration"]
        key = declaration["identity"]["key"]
        if declaration["city"]["key"] not in city_keys:
            errors.append(
                f"{key}: city {declaration['city']['key']} is not a City Area"
            )
        footprint = declaration["footprint"]
        if sum(row["tiles"] for row in footprint["floors"]) != footprint["tile_count"]:
            errors.append(f"{key}: footprint floors do not sum to tile_count")
        floors = [row["floor"] for row in footprint["floors"]]
        if floors != sorted(set(floors)):
            errors.append(f"{key}: footprint floors must be unique and ascending")
        for door in declaration["doors"]:
            pos = door["position"]
            inside = (
                footprint["min_x"] <= pos["x"] <= footprint["max_x"]
                and footprint["min_y"] <= pos["y"] <= footprint["max_y"]
            )
            if not inside or pos["floor"] not in floors:
                errors.append(
                    f"{key}: door {door['door_id']} outside the house footprint"
                )
    for record in families["Transition.Teleport"]:
        declaration = record["declaration"]
        if "object" in declaration and declaration["object"]["key"] not in item_keys:
            errors.append(
                f"{declaration['identity']['key']}: object is not a bound Item"
            )
        if declaration["from"] == declaration["to"]:
            errors.append(f"{declaration['identity']['key']}: teleport to its own tile")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=HERE.parents[2])
    args = parser.parse_args()
    try:
        errors = validate(args.root.resolve())
    except ValidationError as error:
        errors = [str(error)]
    for error in errors:
        print(f"FAIL {error}", file=sys.stderr)
    if errors:
        return 1
    print("PASS world metadata families")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
