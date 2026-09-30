#!/usr/bin/env python3
"""Stage the 15.30 client map file (map areas, named markers, layer geometry) as source observations.

One pinned, owner-provided official client file under content/assets/files/: map-<sha256>.dat, a
protobuf with top-level field 1 = map areas, 2 = named map markers, 3 = layer records (subarea
overlays and satellite tiles) and 4/5 = bounds corners. It is read with the strict wire reader of
stage_proficiencies.py (varint and length-delimited only). The one fixed64 field (layer field 7) is
split off the record tail and the remainder goes through the same reader. Each layer's image file is
joined by file name and the SHA-256 of its stored bytes (the 64-hex token in the name is kept as
`name_hash`; it is not the digest of the stored file); images are never decoded. Unknown fields or wire types, duplicate ids, dangling ids and wrong counts
are rejected. The staged data are source observations, not Oteryn definitions (see the READMEs).
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from stage_proficiencies import parse  # noqa: E402  (shared strict wire reader; no third parser)

MAP_SHA = "c54dfeb8f3880e9534f1330fa547f06d99c5afa55f641b875ceee23cc04e3496"
MAP_FILE = f"map-{MAP_SHA}.dat"
DEFAULT_INPUT_DIR = Path("content/assets/files")
DEFAULT_OUTPUT = Path("imports/cipsoft-staticdata/map")
EXPECTED_COUNTS = {"areas": 465, "markers": 1270, "subareas": 1157}
EXPECTED_LAYER_KINDS = {0: 209, 1: 741, 2: 207}
LAYER_PREFIX = {0: "subarea", 1: "satellite", 2: "minimap"}  # observed 1:1 for all 1157 records
SHARD = 150
AUTHORITY = "SOURCE_OBSERVATIONS_ONLY; no Oteryn ProductionKeys or gameplay promotion"
NAME_RE = re.compile(r"^(?P<role>subarea-(?P<area>\d{4})|(?:satellite|minimap)-\d+-\d{4}-\d{4}-\d{2})-(?P<sha>[0-9a-f]{64})\.bmp\.lzma$")
FAMILIES = {
    "areas": ("MapArea", "cipsoft/map_area"),
    "markers": ("MapMarker", "cipsoft/map_marker"),
    "subareas": ("MapLayer", "cipsoft/map_layer"),
}


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def dumps(payload: object, *, indent: bool = False) -> bytes:
    if indent:
        return (json.dumps(payload, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode()
    return (json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()


def read_map(directory: Path) -> bytes:
    data = (directory / MAP_FILE).read_bytes()
    if digest(data) != MAP_SHA:
        raise ValueError("map: input digest mismatch")
    return data


def message(raw: bytes, spec: dict[int, tuple[int, bool]], name: str) -> dict[int, list[int | bytes]]:
    """Fields of one record by number; spec is {field: (wire, repeated)}. Unknown or repeated-once violations reject."""
    out: dict[int, list[int | bytes]] = {}
    for field, wire, value in parse(raw):
        if field not in spec or spec[field][0] != wire:
            raise ValueError(f"{name}: unexpected field {field} wire {wire}")
        out.setdefault(field, []).append(value)
        if len(out[field]) > 1 and not spec[field][1]:
            raise ValueError(f"{name}: repeated field {field}")
    return out


def position(raw: object, name: str) -> dict[str, int]:
    """x, y, z: every position message in the file has exactly varint fields 1, 2, 3 in that order."""
    if not isinstance(raw, bytes):
        raise ValueError(f"{name}: position is not a message")
    fields = parse(raw)
    if [(f, w) for f, w, _ in fields] != [(1, 0), (2, 0), (3, 0)]:
        raise ValueError(f"{name}: position shape")
    x, y, z = (int(v) for _, _, v in fields)
    if z > 15:
        raise ValueError(f"{name}: position z {z} out of range")
    return {"x": x, "y": y, "z": z}


def text(raw: object, name: str) -> str:
    if not isinstance(raw, bytes):
        raise ValueError(f"{name}: not a string")
    return raw.decode("utf-8")  # strict


def split_tables(data: bytes) -> tuple[list[bytes], list[bytes], list[bytes], dict[str, dict[str, int]]]:
    tables: dict[int, list[bytes]] = {1: [], 2: [], 3: []}
    bounds: dict[int, dict[str, int]] = {}
    for field, wire, value in parse(data):
        if wire != 2 or not isinstance(value, bytes):
            raise ValueError(f"map: unexpected top-level field {field} wire {wire}")
        if field in tables:
            tables[field].append(value)
        elif field in (4, 5) and field not in bounds:
            bounds[field] = position(value, f"bounds {field}")
        else:
            raise ValueError(f"map: unexpected or repeated top-level field {field}")
    if set(bounds) != {4, 5}:
        raise ValueError("map: bounds corners missing")
    return tables[1], tables[2], tables[3], {"min_corner": bounds[4], "max_corner": bounds[5]}


def stage_layers(raws: list[bytes], area_ids: set[int], directory: Path) -> list[dict]:
    rows, seen_area, seen_files = [], set(), set()
    for index, raw in enumerate(raws):
        name = f"layer #{index}"
        scale = None
        if len(raw) > 9 and raw[-9] == 0x39:  # field 7, wire type 1 (fixed64), always the last field
            (scale,), raw_fields = struct.unpack("<d", raw[-8:]), raw[:-9]
        else:
            raw_fields = raw
        fields = message(raw_fields, {1: (0, False), 2: (2, False), 3: (2, False), 4: (0, False), 5: (0, False), 6: (0, False)}, name)
        if not {1, 2, 3, 4, 5} <= set(fields):
            raise ValueError(f"{name}: missing field")
        kind = int(fields[1][0])
        file_name = text(fields[3][0], name)
        match = NAME_RE.match(file_name)
        if match is None:
            raise ValueError(f"{name}: unexpected image file name {file_name!r}")
        is_subarea = kind == 0
        if file_name.split("-")[0] != LAYER_PREFIX.get(kind):
            raise ValueError(f"{name}: layer kind {kind} does not match file name {file_name!r}")
        if is_subarea != (6 in fields) or is_subarea != file_name.startswith("subarea-") or is_subarea != (scale is None):
            raise ValueError(f"{name}: kind/field-6/file-name/scale mismatch")
        area_id = None
        if is_subarea:
            area_id = int(fields[6][0])
            if area_id not in area_ids:
                raise ValueError(f"{name}: dangling subarea id {area_id}")
            if area_id in seen_area:
                raise ValueError(f"duplicate subarea id {area_id}")
            if int(match["area"]) != area_id:
                raise ValueError(f"{name}: file name subarea number differs from field 6")
            seen_area.add(area_id)
        if file_name in seen_files:
            raise ValueError(f"duplicate image file {file_name}")
        seen_files.add(file_name)
        image = directory / file_name
        if not image.is_file():
            raise ValueError(f"{name}: image file {file_name} missing")
        rows.append({
            "source_index": index, "layer_kind": kind, "area_id": area_id, "position": position(fields[2][0], name),
            "field_4": int(fields[4][0]), "field_5": int(fields[5][0]), "scale": scale,
            "image": {"file": file_name, "file_sha256": digest(image.read_bytes()), "name_hash": match["sha"]}, "source_record_sha256": digest(raw),
        })
    return rows


def stage_areas(raws: list[bytes], images: dict[int, dict]) -> list[dict]:
    spec = {1: (0, False), 2: (2, False), 3: (0, False), 4: (0, True), 5: (2, False), 6: (0, False), 7: (2, False)}
    parsed, ids = [], set()
    for index, raw in enumerate(raws):
        fields = message(raw, spec, f"area #{index}")
        if not {1, 2, 3} <= set(fields):
            raise ValueError(f"area #{index}: missing field")
        area_id = int(fields[1][0])
        if area_id in ids:
            raise ValueError(f"duplicate area id {area_id}")
        ids.add(area_id)
        parsed.append((index, raw, fields))
    rows = []
    for index, raw, fields in parsed:
        area_id = int(fields[1][0])
        members = [int(v) for v in fields.get(4, [])]
        if len(set(members)) != len(members):
            raise ValueError(f"area {area_id}: duplicate subarea id in list")
        dangling = [m for m in members if m not in ids]
        if dangling:
            raise ValueError(f"area {area_id}: dangling subarea id {dangling[0]}")
        rows.append({
            "source_id": area_id, "source_index": index, "name": text(fields[2][0], f"area {area_id}"),
            "flag": int(fields[3][0]), "subarea_ids": members,
            "position": position(fields[5][0], f"area {area_id}") if 5 in fields else None,
            "field_6": int(fields[6][0]) if 6 in fields else None,
            "field_7_text": text(fields[7][0], f"area {area_id}") if 7 in fields else None,
            "subarea_image": images.get(area_id), "source_record_sha256": digest(raw),
        })
    return rows


def stage_markers(raws: list[bytes]) -> list[dict]:
    rows = []
    for index, raw in enumerate(raws):
        fields = message(raw, {1: (2, False), 2: (2, False), 3: (0, False)}, f"marker #{index}")
        if set(fields) != {1, 2, 3}:
            raise ValueError(f"marker #{index}: missing field")
        rows.append({
            "source_index": index, "name": text(fields[1][0], f"marker #{index}"),
            "position": position(fields[2][0], f"marker #{index}"), "icon_id": int(fields[3][0]),
            "source_record_sha256": digest(raw),
        })
    return rows


def build(data: bytes, directory: Path) -> dict[str, bytes]:
    """Return {relative output path: bytes} for every generated file."""
    area_raws, marker_raws, layer_raws, bounds = split_tables(data)
    area_ids = set()
    for raw in area_raws:
        first = parse(raw)[:1]
        if not first or first[0][:2] != (1, 0):
            raise ValueError("area without leading id")
        area_ids.add(first[0][2])
    layers = stage_layers(layer_raws, area_ids, directory)
    images = {row["area_id"]: row["image"] for row in layers if row["layer_kind"] == 0}
    kinds = {k: sum(1 for r in layers if r["layer_kind"] == k) for k in sorted({r["layer_kind"] for r in layers})}
    if kinds != EXPECTED_LAYER_KINDS:
        raise ValueError(f"layer kinds {kinds} differ from expected {EXPECTED_LAYER_KINDS}")
    sets = {"areas": stage_areas(area_raws, images), "markers": stage_markers(marker_raws), "subareas": layers}
    outputs: dict[str, bytes] = {}
    for directory_name, rows in sets.items():
        if len(rows) != EXPECTED_COUNTS[directory_name]:
            raise ValueError(f"{directory_name}: expected {EXPECTED_COUNTS[directory_name]} records, found {len(rows)}")
        family, namespace = FAMILIES[directory_name]
        files = []
        for start in range(0, len(rows), SHARD):
            chunk = rows[start:start + SHARD]
            name = f"{directory_name}-{start:05d}-{start + len(chunk) - 1:05d}.json"
            payload = dumps({"schema": f"OTERYN_CIPSOFT_{family.upper()}_SOURCE_OBSERVATIONS/v1", "family": family, "records": chunk})
            outputs[f"{directory_name}/{name}"] = payload
            files.append({"path": name, "records": len(chunk), "sha256": digest(payload)})
        manifest = {
            "schema": f"OTERYN_CIPSOFT_{family.upper()}_SOURCE_MANIFEST/v1",
            "source": "CIPSOFT_CLIENT_ASSETS", "client_version": "15.30", "family": family,
            "identity_namespace": namespace, "record_count": len(rows), "authority": AUTHORITY,
            "inputs": {"map": {"sha256": MAP_SHA, "file": f"content/assets/files/{MAP_FILE}"}},
            "files": files,
        }
        if directory_name == "subareas":
            manifest["layer_kind_counts"] = {str(k): v for k, v in kinds.items()}
        outputs[f"{directory_name}/manifest.json"] = dumps(manifest, indent=True)
    outputs["bounds.json"] = dumps({
        "schema": "OTERYN_CIPSOFT_MAP_BOUNDS_SOURCE_OBSERVATIONS/v1", "source_sha256": MAP_SHA,
        "authority": AUTHORITY, **bounds}, indent=True)
    return outputs


def compare(output: Path, expected: dict[str, bytes]) -> list[str]:
    problems = []
    present = {p.relative_to(output).as_posix() for p in output.rglob("*.json")} if output.is_dir() else set()
    problems += [f"unexpected file {n}" for n in sorted(present - set(expected))]
    for rel, data in expected.items():
        path = output / rel
        if not path.is_file() or path.read_bytes() != data:
            problems.append(f"differs or missing: {rel}")
    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input-dir", type=Path, default=DEFAULT_INPUT_DIR)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true", help="verify committed bytes equal regeneration; write nothing")
    args = parser.parse_args(argv)
    expected = build(read_map(args.input_dir), args.input_dir)
    if args.check:
        problems = compare(args.output, expected)
        for problem in problems:
            print(problem, file=sys.stderr)
        print("map staging check: " + ("FAIL" if problems else f"ok ({len(expected)} files)"))
        return 1 if problems else 0
    for rel, data in expected.items():
        path = args.output / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
