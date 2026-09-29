#!/usr/bin/env python3
"""Stage 15.30 client staticdata/staticmapdata House and Achievement observations.

The two inputs are the pinned, owner-provided official client files under
content/assets/files/. They are raw protobuf without a shipped schema, so a
small strict wire reader is used and every record shape is asserted: unknown
fields, wrong wire types, duplicate ids and count mismatches are rejected.
Field names are inferred from value distributions (see the README next to the
output); the staged data are source observations, not Oteryn definitions.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sys

INPUTS = {
    "staticdata": "62d3f5f761a4c8cab02c89bd1a351770aca3504a74f1b34f457f0a0451dcd128",
    "staticmapdata": "0967af2eacdd8f2a608e738b9042362676167d6c6455e60d08db7ae16cf7ea53",
}
DEFAULT_INPUT_DIR = Path("content/assets/files")
DEFAULT_OUTPUT = Path("imports/cipsoft-staticdata")
SHARDS = {"houses": 250, "achievements": 500}
HOUSE_TABLE, ACHIEVEMENT_TABLE = 4, 3
EXPECTED_COUNTS = {"houses": 995, "achievements": 368}
AUTHORITY = "SOURCE_OBSERVATIONS_ONLY; no Oteryn ProductionKeys or gameplay promotion"
ITEM_EXTRA_FIELDS = {101, 102}


def varint(data: bytes, i: int) -> tuple[int, int]:
    result = shift = 0
    while True:
        if i >= len(data) or shift > 63:
            raise ValueError("truncated or oversized varint")
        byte = data[i]
        i += 1
        result |= (byte & 0x7F) << shift
        if byte < 0x80:
            return result, i
        shift += 7


def parse(data: bytes) -> list[tuple[int, int, int | bytes]]:
    """Return (field, wire_type, value) triples; only varint and length-delimited are accepted."""
    i, out = 0, []
    while i < len(data):
        key, i = varint(data, i)
        field, wire = key >> 3, key & 7
        if field == 0:
            raise ValueError("field number 0")
        if wire == 0:
            value, i = varint(data, i)
        elif wire == 2:
            size, i = varint(data, i)
            if i + size > len(data):
                raise ValueError("truncated length-delimited field")
            value, i = data[i:i + size], i + size
        else:
            raise ValueError(f"unsupported wire type {wire}")
        out.append((field, wire, value))
    return out


def message(data: bytes, spec: dict[int, str], *, name: str) -> dict[int, int | bytes]:
    """Parse a message that must carry exactly the fields in spec ('v' varint, 'b' bytes), once each."""
    result: dict[int, int | bytes] = {}
    for field, wire, value in parse(data):
        kind = spec.get(field)
        if kind is None or field in result or (kind == "v") != (wire == 0):
            raise ValueError(f"{name}: unexpected field {field} (wire {wire})")
        result[field] = value
    if set(result) != set(spec):
        raise ValueError(f"{name}: missing fields {sorted(set(spec) - set(result))}")
    return result


def text(value: int | bytes) -> str:
    assert isinstance(value, bytes)
    return value.decode("utf-8")  # strict


def flag(value: int | bytes, name: str) -> bool:
    if value not in (0, 1):
        raise ValueError(f"{name}: non-boolean value {value!r}")
    return value == 1


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_input(directory: Path, role: str) -> bytes:
    data = (directory / f"{role}-{INPUTS[role]}.dat").read_bytes()
    if digest(data) != INPUTS[role]:
        raise ValueError(f"{role}: input digest mismatch")
    return data


def position(data: bytes, name: str) -> dict[str, int]:
    fields = message(data, {1: "v", 2: "v", 3: "v"}, name=name)
    return {"x": fields[1], "y": fields[2], "z": fields[3]}


def table(data: bytes, number: int) -> list[bytes]:
    tops = parse(data)
    if any(field > 6 or wire != 2 for field, wire, _ in tops):
        raise ValueError("staticdata: unexpected top-level field")
    return [value for field, _, value in tops if field == number]


def stage_houses(staticdata: bytes, staticmapdata: bytes) -> list[dict]:
    houses = {}
    for index, raw in enumerate(table(staticdata, HOUSE_TABLE)):
        f = message(raw, {1: "v", 2: "b", 3: "b", 4: "v", 5: "v", 6: "b", 7: "v", 8: "v", 9: "b", 10: "v"}, name="house")
        if f[1] in houses:
            raise ValueError(f"duplicate house id {f[1]}")
        houses[f[1]] = {
            "source_id": f[1], "source_index": index, "name": text(f[2]), "restrictions": text(f[3]),
            "rent_gold": f[4], "beds": f[5], "size_sqm": f[7], "town": text(f[9]),
            "guildhall": flag(f[8], "guildhall"), "shop": flag(f[10], "shop"),
            "entrance": position(f[6], "house entrance"), "staticdata_sha256": digest(raw),
        }
    layouts = {}
    for raw in (value for field, _, value in parse(staticmapdata) if field == 1):
        record = message(raw, {1: "v", 2: "b"}, name="house map record")
        if record[1] in layouts:
            raise ValueError(f"duplicate map house id {record[1]}")
        layouts[record[1]] = (raw, stage_layout(record[2]))
    if set(layouts) != set(houses):
        raise ValueError("staticdata and staticmapdata house id sets differ")
    for house_id, house in houses.items():
        raw, layout = layouts[house_id]
        layout["staticmapdata_sha256"] = digest(raw)
        house["layout"] = layout
    return list(houses.values())


def stage_layout(data: bytes) -> dict:
    fields = message(data, {1: "b", 2: "b", 3: "b"}, name="house layout")
    dims = message(fields[2], {1: "v", 2: "v", 3: "v"}, name="layout dimensions")
    (_, wire, body), = parse(fields[3])  # exactly one layout body
    if wire != 2:
        raise ValueError("layout body wire type")
    cells, expected_total, skips = [], dims[1] * dims[2] * dims[3], 0
    for cell_field, cell_wire, cell_raw in parse(body):
        if cell_field != 3 or cell_wire != 2:
            raise ValueError("unexpected layout body field")
        items, extras, cell = [], [], {}
        for field, wire, value in parse(cell_raw):
            if field == 1 and wire == 2:
                item = {"id": None}
                for item_field, item_wire, item_value in parse(value):
                    if item_field == 1 and item_wire == 0 and item["id"] is None:
                        item["id"] = item_value
                    elif item_field in ITEM_EXTRA_FIELDS and item_wire == 2:
                        extras.append({"item_index": len(items), "field": item_field, "hex": item_value.hex()})
                    else:
                        raise ValueError(f"unexpected item field {item_field}")
                if item["id"] is None:
                    raise ValueError("item without id")
                items.append(item["id"])
            elif field == 2 and wire == 0 and "skip" not in cell:
                cell["skip"] = value
                skips += value
            else:
                raise ValueError(f"unexpected cell field {field}")
        cell["items"] = items
        if extras:
            cell["item_extra_fields"] = extras
        cells.append(cell)
    if len(cells) + skips != expected_total:
        raise ValueError("cells plus skip counts do not equal width*height*floors")
    return {
        "origin": position(fields[1], "layout origin"),
        "dimensions": {"width": dims[1], "height": dims[2], "floors": dims[3]},
        "cell_count": len(cells), "cells": cells,
    }


def stage_achievements(staticdata: bytes) -> list[dict]:
    rows, seen = [], set()
    for index, raw in enumerate(table(staticdata, ACHIEVEMENT_TABLE)):
        f = message(raw, {1: "v", 2: "b", 3: "b", 4: "v"}, name="achievement")
        if f[1] in seen:
            raise ValueError(f"duplicate achievement id {f[1]}")
        seen.add(f[1])
        rows.append({
            "source_id": f[1], "source_index": index, "name": text(f[2]),
            "description": text(f[3]), "grade": f[4], "staticdata_sha256": digest(raw),
        })
    return rows


def dumps(payload: dict, *, indent: bool = False) -> bytes:
    if indent:
        return (json.dumps(payload, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode()
    return (json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()


def build(staticdata: bytes, staticmapdata: bytes) -> dict[str, bytes]:
    """Return {relative output path: bytes} for every generated file."""
    sets = {
        "houses": (stage_houses(staticdata, staticmapdata), "House", "houses"),
        "achievements": (stage_achievements(staticdata), "Achievement", "achievements"),
    }
    outputs: dict[str, bytes] = {}
    for directory, (rows, family, stem) in sets.items():
        if len(rows) != EXPECTED_COUNTS[directory]:
            raise ValueError(f"{directory}: expected {EXPECTED_COUNTS[directory]} records, found {len(rows)}")
        files = []
        for start in range(0, len(rows), SHARDS[directory]):
            chunk = rows[start:start + SHARDS[directory]]
            name = f"{stem}-{start:05d}-{start + len(chunk) - 1:05d}.json"
            data = dumps({"schema": f"OTERYN_CIPSOFT_STATICDATA_{family.upper()}_SOURCE_OBSERVATIONS/v1", "family": family, "records": chunk})
            outputs[f"{directory}/{name}"] = data
            files.append({"path": name, "records": len(chunk), "sha256": digest(data)})
        manifest = {
            "schema": f"OTERYN_CIPSOFT_STATICDATA_{family.upper()}_SOURCE_MANIFEST/v1",
            "source": "CIPSOFT_CLIENT_STATICDATA", "client_version": "15.30", "family": family,
            "identity_namespace": f"cipsoft/staticdata/{family.lower()}_id",
            "record_count": len(rows), "authority": AUTHORITY,
            "inputs": {role: {"sha256": sha, "file": f"content/assets/files/{role}-{sha}.dat"} for role, sha in INPUTS.items()
                       if directory == "houses" or role == "staticdata"},
            "files": files,
        }
        outputs[f"{directory}/manifest.json"] = dumps(manifest, indent=True)
    return outputs


def compare(output: Path, expected: dict[str, bytes]) -> list[str]:
    problems = []
    for directory in {rel.split("/")[0] for rel in expected}:
        base = output / directory
        present = {p.name for p in base.glob("*.json")} if base.is_dir() else set()
        wanted = {rel.split("/")[1] for rel in expected if rel.startswith(f"{directory}/")}
        problems += [f"unexpected file {directory}/{n}" for n in sorted(present - wanted)]
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
    expected = build(*(read_input(args.input_dir, role) for role in INPUTS))
    if args.check:
        problems = compare(args.output, expected)
        for problem in problems:
            print(problem, file=sys.stderr)
        print("staticdata staging check: " + ("FAIL" if problems else f"ok ({len(expected)} files)"))
        return 1 if problems else 0
    for rel, data in expected.items():
        path = args.output / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
