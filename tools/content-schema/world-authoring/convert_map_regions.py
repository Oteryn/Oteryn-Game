#!/usr/bin/env python3
"""Convert the official Tibia 15.30 client map file into `Area.Region` records.

Reads the committed `map-<sha256>.dat` and its `subarea-*` mask images from
`content/assets/files/` (offline; verified against `imports/official/client-assets/15.30/
manifest.json`) and the committed City Areas, and writes content/world/areas/regions/ plus
`samples/map-regions-capture-v1.json`. Only facts proven against the data are written;
everything else is omitted and counted.

    python convert_map_regions.py [--check]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

import client_map_reader as reader
import convert_world_metadata as base

ROOT = base.ROOT
FILES = "content/assets/files"
CATALOG = f"{FILES}/catalog-content.json"
MANIFEST = "imports/official/client-assets/15.30/manifest.json"
SUMMARY = "tools/content-schema/world-authoring/samples/map-regions-capture-v1.json"
CRYSTAL_SUMMARY = "tools/content-schema/world-authoring/samples/source-capture-v1.json"
GENERATOR = "tools/content-schema/world-authoring/convert_map_regions.py"
NAMESPACE = "tibia-client/map-area-id"
SOURCE_KEY = "oteryn:source.tibia_client"
CLIENT_VERSION = "15.30"
FAMILY = "Area.Region"
KEY_PREFIX = "oteryn:area.region."
PROJECTION = "temple_projected_to_floor_7"
PROJECTION_FLOOR = 7
KINDS = {1: "region", 2: "subregion"}
IMAGE_NAME = re.compile(r"^subarea-[0-9]{4}-([0-9a-f]{64})\.bmp\.lzma$")
MAP_NAME = re.compile(r"^map-([0-9a-f]{64})\.dat$")


class ConvertError(base.ConvertError):
    pass


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_json(root: Path, path: str):
    return json.loads((root / path).read_text(encoding="utf-8"))


def verified(root: Path, name: str, manifest: dict[str, str]) -> tuple[bytes, str]:
    """A committed client file's bytes and sha256, checked against the manifest."""
    data = (root / FILES / name).read_bytes()
    digest = sha256(data)
    if manifest.get(name) != digest:
        raise ConvertError(f"{name}: sha256 differs from the client asset manifest")
    return data, digest


def map_extent(root: Path) -> tuple[int, int]:
    summary = read_json(root, CRYSTAL_SUMMARY)
    return summary["map"]["width"], summary["map"]["height"]


def cities(root: Path) -> list[tuple[str, dict]]:
    """(key, temple position) of every committed City Area, sorted by key."""
    index = read_json(root, f"{base.FAMILIES['Area.City']['dir']}/index.json")
    found = []
    for shard in index["shards"]:
        for record in read_json(root, shard)["records"]:
            declaration = record["declaration"]
            found.append((declaration["identity"]["key"], declaration["temple"]))
    return sorted(found, key=lambda row: row[0])


def nearest_mask(masks: dict, by_area: dict, x: int, y: int, floor: int) -> dict:
    """The mask on `floor` nearest to tile (x, y) by Chebyshev distance; recorded only."""
    best = None
    for area_id, mask in masks.items():
        if mask.z != floor:
            continue
        for r, row in enumerate(mask.rows):
            for c, cell in enumerate(row):
                if cell:
                    d = max(abs(mask.x + c - x), abs(mask.y + r - y))
                    if best is None or (d, by_area[area_id]) < best:
                        best = (d, by_area[area_id])
    return {} if best is None else {"nearest_mask": best[1], "distance": best[0]}


def check_hierarchy(facts: reader.MapFile) -> dict[int, reader.Area]:
    areas: dict[int, reader.Area] = {}
    for area in facts.areas:
        if area.id < 1 or area.id in areas:
            raise ConvertError(f"area id {area.id} is invalid or repeated")
        if area.kind not in KINDS:
            raise ConvertError(f"area {area.id}: unknown kind {area.kind}")
        if not 1 <= len(area.name) <= 64 or area.name != area.name.strip():
            raise ConvertError(f"area {area.id}: name is not usable")
        areas[area.id] = area
    parents: dict[int, list[int]] = {}
    for area in facts.areas:
        if (area.kind == 1) != bool(area.children):
            raise ConvertError(f"area {area.id}: kind and child list disagree")
        if len(set(area.children)) != len(area.children):
            raise ConvertError(f"area {area.id}: repeated child")
        for child in area.children:
            if child not in areas or areas[child].kind != 2:
                raise ConvertError(f"area {area.id}: child {child} is not a subarea")
            parents.setdefault(child, []).append(area.id)
    orphans = [a.id for a in facts.areas if a.kind == 2 and a.id not in parents]
    if orphans:
        raise ConvertError(f"subareas without a parent: {orphans[:5]}")
    return areas


def check_images(
    facts: reader.MapFile, areas: dict[int, reader.Area]
) -> dict[int, reader.SubareaImage]:
    images: dict[int, reader.SubareaImage] = {}
    for image in facts.subareas:
        if image.area in images or areas.get(image.area, None) is None:
            raise ConvertError(f"{image.file}: area is unknown or has two images")
        if areas[image.area].kind != 2 or not IMAGE_NAME.fullmatch(image.file):
            raise ConvertError(f"{image.file}: not a subarea mask of a child area")
        if not 0 <= image.z <= base.MAX_FLOOR or min(image.width, image.height) < 1:
            raise ConvertError(f"{image.file}: floor or size is invalid")
        images[image.area] = image
    return images


def key_names(areas: dict[int, reader.Area]) -> list[tuple[str, str]]:
    """(source id, name to slug). A subregion whose slug is shared gets a `subregion` suffix."""
    shared: dict[str, int] = {}
    for area in areas.values():
        shared[base.slug(area.name)] = shared.get(base.slug(area.name), 0) + 1
    return [
        (
            str(area.id),
            f"{area.name} subregion"
            if area.kind == 2 and shared[base.slug(area.name)] > 1
            else area.name,
        )
        for area in areas.values()
    ]


def build(root: Path = ROOT) -> dict[str, bytes]:
    manifest_doc = read_json(root, MANIFEST)
    manifest = {row["name"]: row["sha256"] for row in manifest_doc["files"]}
    entries = [row for row in read_json(root, CATALOG) if row["type"] == "map"]
    if len(entries) != 1 or not MAP_NAME.fullmatch(entries[0]["file"]):
        raise ConvertError("the catalog must list exactly one map file")
    map_data, map_sha = verified(root, entries[0]["file"], manifest)
    if MAP_NAME.fullmatch(entries[0]["file"]).group(1) != map_sha:  # type: ignore[union-attr]
        raise ConvertError("map file name does not carry its sha256")
    facts = reader.read_map(map_data)
    areas = check_hierarchy(facts)
    images = check_images(facts, areas)
    width, height = map_extent(root)
    keys = base.assign_keys(
        key_names(areas),
        base.committed_keys(root, FAMILY, NAMESPACE),
        KEY_PREFIX,
        FAMILY,
    )
    by_area = {area_id: keys[str(area_id)] for area_id in areas}
    parents: dict[int, list[int]] = {}
    for area in facts.areas:
        for child in area.children:
            parents.setdefault(child, []).append(area.id)

    def inside(x: int, y: int) -> bool:
        return 0 <= x < width and 0 <= y < height

    footprints, masks = {}, {}
    for area_id, image in sorted(images.items()):
        raw, digest = verified(root, image.file, manifest)
        mask = reader.read_mask(raw, image)
        min_x, min_y, max_x, max_y = mask.bounds()
        if not (inside(min_x, min_y) and inside(max_x, max_y)):
            raise ConvertError(f"{image.file}: footprint outside the map extent")
        masks[area_id] = mask
        footprints[area_id] = {
            "coordinate_frame": base.COORDINATE_FRAME,
            "floor": image.z,
            "image": {"path": f"{FILES}/{image.file}", "sha256": digest},
            "max_x": max_x,
            "max_y": max_y,
            "min_x": min_x,
            "min_y": min_y,
            "tile_count": mask.tile_count,
        }
    city_rows = cities(root)
    linked: dict[int, list[str]] = {}
    projected, unlinked = [], []
    floor_differs = outside = 0
    for city_key, temple in city_rows:
        x, y, floor = temple["x"], temple["y"], temple["floor"]
        hits = [a for a, mask in masks.items() if mask.contains(x, y, floor)]
        if not hits and floor != PROJECTION_FLOOR:
            hits = [a for a, m in masks.items() if m.contains(x, y, PROJECTION_FLOOR)]
            if len(hits) == 1:
                projected.append(
                    {
                        "city": city_key,
                        "method": PROJECTION,
                        "subregion": by_area[hits[0]],
                    }
                )
            else:
                reason = "projection_ambiguous" if hits else "projection_outside"
                unlinked.append({"city": city_key, "reason": reason})
                hits = []
        for area_id in hits:
            linked.setdefault(area_id, []).append(city_key)
        if not hits:
            if any(m.z == floor for m in masks.values()):
                outside += 1
                unlinked.append(
                    {"city": city_key, "reason": "outside_every_mask"}
                    | nearest_mask(masks, by_area, x, y, floor)
                )
            else:
                floor_differs += 1
    records, counts = (
        [],
        {"anchor": 0, "cities": 0, "footprint": 0, "parent_regions": 0},
    )
    for area in facts.areas:
        key = by_area[area.id]
        declaration: dict = {
            "area_kind": KINDS[area.kind],
            "identity": {"key": key, "revision": base.REVISION},
            "kind": "Area",
            "name": area.name,
        }
        if area.anchor is not None:
            if not inside(area.anchor[0], area.anchor[1]) or area.anchor[2] > 15:
                raise ConvertError(f"{key}: anchor outside the map extent")
            declaration["anchor"] = base.position(*area.anchor)
            counts["anchor"] += 1
        if area.kind == 2:
            declaration["parent_regions"] = sorted(
                (base.ref("Area", by_area[p]) for p in parents[area.id]),
                key=lambda row: row["key"],
            )
            counts["parent_regions"] += 1
            if area.id in footprints:
                declaration["footprint"] = footprints[area.id]
                counts["footprint"] += 1
            city_keys = linked.get(area.id, [])
        else:
            city_keys = sorted(
                {c for child in area.children for c in linked.get(child, [])}
            )
        if city_keys:
            declaration["cities"] = [base.ref("Area", c) for c in sorted(city_keys)]
            counts["cities"] += 1
        records.append(
            {
                "declaration": declaration,
                "source_bindings": [
                    {
                        "disposition": "EXACT",
                        "external_id": str(area.id),
                        "identity_namespace": NAMESPACE,
                        "source_key": SOURCE_KEY,
                        "source_revision": map_sha,
                        "target": base.ref("Area", key),
                    }
                ],
            }
        )
    records = base.unique(records, FAMILY)
    source = {
        "client_version": CLIENT_VERSION,
        "evidence": "OfficialClient",
        "files": [{"path": f"{FILES}/{entries[0]['file']}", "sha256": map_sha}],
        "manifest": {
            "archive_sha256": manifest_doc["archive_sha256"],
            "path": MANIFEST,
        },
        "source_key": SOURCE_KEY,
    }
    out = base.shard_files(FAMILY, records, source, GENERATOR)
    summary = {
        "cities": {
            "linked": len({c for row in linked.values() for c in row}),
            "projected": sorted(projected, key=lambda row: row["city"]),
            "total": len(city_rows),
            "unlinked": sorted(unlinked, key=lambda row: row["city"]),
        },
        "families": {FAMILY: len(records)},
        "not_imported": {
            "area_flag_field": sum(a.flag is not None for a in facts.areas),
            "areas_with_secondary_names": sum(
                bool(a.secondary_names) for a in facts.areas
            ),
            "cities_outside_every_footprint": outside,
            "cities_with_temple_floor_absent_from_footprints": floor_differs,
            "map_markers": facts.markers,
            "minimap_images": facts.other_images.get("minimap", 0),
            "satellite_images": facts.other_images.get("satellite", 0),
            "subregions_without_footprint": sum(
                a.kind == 2 and a.id not in footprints for a in facts.areas
            ),
        },
        "records_by_kind": {
            name: sum(a.kind == kind for a in facts.areas)
            for kind, name in KINDS.items()
        },
        "records_with": counts,
        "schema": "OTERYN_MAP_REGIONS_SOURCE_CAPTURE/v1",
        "source": source,
    }
    out[SUMMARY] = base.canonical(summary)
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="fail instead of writing")
    args = parser.parse_args()
    try:
        out = build()
    except (
        base.ConvertError,
        reader.ClientMapError,
        OSError,
        KeyError,
        json.JSONDecodeError,
    ) as error:
        print(f"FAIL {error!r}", file=sys.stderr)
        return 1
    stale = [
        path
        for path, data in out.items()
        if not (ROOT / path).is_file() or (ROOT / path).read_bytes() != data
    ]
    if args.check:
        for path in stale:
            print(f"STALE {path}", file=sys.stderr)
        return 1 if stale else 0
    for path in stale:
        (ROOT / path).parent.mkdir(parents=True, exist_ok=True)
        (ROOT / path).write_bytes(out[path])
    print(f"wrote {len(stale)} of {len(out)} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
