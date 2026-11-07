#!/usr/bin/env python3
"""Stage the non-object tables of the pinned 15.30 client appearances file.

Top-level fields 2, 3 and 4 of the owner-provided appearances protobuf (inferred: outfits, effects,
missiles) become source observations. Field 1 (objects) belongs to other lanes and is never read
here beyond counting; field 5 (a single record) is only hashed. The wire reader is the one from
stage_proficiencies.py. Flags are kept as field number -> raw value, with a message decoded only
when its bytes are varint-only. Frame groups are reduced to counts and sprite-id lists; no image
decoding. Unknown fields, wire types, duplicate ids and wrong counts are rejected.
"""
from __future__ import annotations

import argparse
import importlib.util
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
_spec = importlib.util.spec_from_file_location("stage_proficiencies", HERE / "stage_proficiencies.py")
if _spec is None or _spec.loader is None:
    raise RuntimeError("stage_proficiencies import failed")
base = importlib.util.module_from_spec(_spec)
sys.modules.setdefault("stage_proficiencies", base)
_spec.loader.exec_module(base)
parse, digest, dumps = base.parse, base.digest, base.dumps

INPUT_ROLE = "appearances"
DEFAULT_OUTPUT = Path("imports/cipsoft-appearances")
# top-level field -> (directory, family, expected record count); meanings are inferred, see README
TABLES = {2: ("outfits", "Outfit", 1480), 3: ("effects", "Effect", 243), 4: ("missiles", "Missile", 76)}
OBJECT_FIELD, EXTRA_FIELD, EXPECTED_OBJECTS, EXPECTED_EXTRA = 1, 5, 43516, 1
SHARD = 150
AUTHORITY = "SOURCE_OBSERVATIONS_ONLY; no Oteryn ProductionKeys or gameplay promotion"
RECORD_FIELDS, GROUP_FIELDS = {1, 2, 3}, {1, 2, 3}
SPRITE_INFO_FIELDS = set(range(1, 10))
SPRITE_ID_FIELD = 5


def entry(field: int, wire: int, value: int | bytes) -> dict:
    """Verbatim observation: varint as int; length-delimited as hex, plus decoded varint fields if all varint."""
    if isinstance(value, int):
        return {"field": field, "wire": wire, "value": value}
    out: dict = {"field": field, "wire": wire, "hex": value.hex()}
    try:
        inner = parse(value)
    except ValueError:
        return out
    if inner and all(w == 0 for _, w, _ in inner):
        out["varint_fields"] = [[f, v] for f, _, v in inner]
    return out


def stage_group(raw: bytes, owner: str) -> dict:
    fields: dict[int, tuple[int, int | bytes]] = {}
    for f, w, v in parse(raw):
        if f not in GROUP_FIELDS or f in fields:
            raise ValueError(f"{owner}: unexpected or repeated frame group field {f}")
        fields[f] = (w, v)
    if set(fields) != GROUP_FIELDS or fields[1][0] != 0 or fields[2][0] != 0 or fields[3][0] != 2:
        raise ValueError(f"{owner}: frame group shape")
    info = fields[3][1]
    assert isinstance(info, bytes)
    sprites, other = [], []
    for f, w, v in parse(info):
        if f not in SPRITE_INFO_FIELDS:
            raise ValueError(f"{owner}: unknown sprite info field {f}")
        if f == SPRITE_ID_FIELD:
            if w != 0:
                raise ValueError(f"{owner}: sprite id wire type {w}")
            sprites.append(v)
        else:
            other.append(entry(f, w, v))
    return {
        "fixed_frame_group": fields[1][1], "group_id": fields[2][1],
        "sprite_count": len(sprites), "sprite_ids": sprites, "sprite_info_fields": other,
    }


def stage_record(index: int, raw: bytes, table: str) -> dict:
    record_id, groups, flags = None, [], []
    for f, w, v in parse(raw):
        if f not in RECORD_FIELDS:
            raise ValueError(f"{table} #{index}: unknown record field {f}")
        if f == 1 and w == 0 and record_id is None:
            record_id = v
        elif f == 2 and w == 2:
            assert isinstance(v, bytes)
            groups.append(v)
        elif f == 3 and w == 2 and not flags:
            assert isinstance(v, bytes)
            flags = [entry(a, b, c) for a, b, c in parse(v)]
        else:
            raise ValueError(f"{table} #{index}: unexpected wire shape for field {f}")
    if record_id is None:
        raise ValueError(f"{table} #{index}: record without id")
    return {
        "source_id": record_id, "source_index": index, "flags": flags,
        "frame_groups": [stage_group(g, f"{table} {record_id}") for g in groups],
        "source_record_sha256": digest(raw),
    }


def stage_tables(appearances: bytes) -> tuple[dict[int, list[dict]], dict]:
    raws: dict[int, list[bytes]] = {f: [] for f in TABLES}
    counts = {OBJECT_FIELD: 0, EXTRA_FIELD: 0}
    extra = b""
    for field, wire, raw in parse(appearances):
        if field not in TABLES and field not in counts:
            raise ValueError(f"unknown top-level field {field}")
        if wire != 2:
            raise ValueError(f"top-level field {field}: wire type {wire}")
        assert isinstance(raw, bytes)
        if field in TABLES:
            raws[field].append(raw)
        else:
            counts[field] += 1
            if field == EXTRA_FIELD:
                extra = raw
    if counts[OBJECT_FIELD] != EXPECTED_OBJECTS or counts[EXTRA_FIELD] != EXPECTED_EXTRA:
        raise ValueError(f"unexpected excluded top-level counts {counts}")
    tables = {}
    for field, (directory, _, expected) in TABLES.items():
        rows = [stage_record(i, raw, directory) for i, raw in enumerate(raws[field])]
        ids = [r["source_id"] for r in rows]
        if len(set(ids)) != len(ids):
            raise ValueError(f"{directory}: duplicate id")
        if len(rows) != expected:
            raise ValueError(f"{directory}: expected {expected} records, found {len(rows)}")
        tables[field] = rows
    return tables, {"count": 1, "sha256": digest(extra), "note": "top-level field 5, not staged"}


README = """# CipSoft appearances non-object tables (15.30)

Source observations from top-level fields 2, 3 and 4 of the pinned client appearances file
`content/assets/files/appearances-{sha}.dat`. Authority: {authority}.

| Directory | Top-level field | Records | Inferred meaning |
|---|---|---|---|
| outfits | 2 | 1480 | outfits (inferred) |
| effects | 3 | 243 | magic effects (inferred) |
| missiles | 4 | 76 | distance missiles (inferred) |

Field 1 (objects) is other lanes' scope; field 5 (one record) is only hashed in each manifest.

Per record: `source_id` (field 1), `source_index`, `flags` (field 3 as verbatim field number -> value;
length-delimited flags carry `hex` and, when varint-only, `varint_fields`), `frame_groups` (field 2:
`fixed_frame_group`, `group_id`, `sprite_count`, `sprite_ids`, other sprite-info fields verbatim) and
`source_record_sha256` of the raw record bytes. No image decoding.

Inferred meanings, not proven by the data (all marked inferred): flags 23 looks like light
(intensity, colour), 26 like a draw offset (x, y), the varint-only flags (29, 39, 49-52) like booleans or
small enums; sprite-info fields 1-4 like pattern width, height, depth and layers, 6 like animation
phases, 7 like bounding square, 8 like opaque, 9 like per-direction bounding boxes. Outfit records
with two frame groups are inferred as idle and walking groups.

Regenerate with `python tools/content-census/stage_appearance_tables.py`; `--check` compares bytes.
"""


def build(appearances: bytes) -> dict[str, bytes]:
    tables, extra = stage_tables(appearances)
    sha = digest(appearances)
    outputs: dict[str, bytes] = {"README.md": README.format(sha=sha, authority=AUTHORITY).encode()}
    for field, (directory, family, _) in TABLES.items():
        rows = tables[field]
        files = []
        for start in range(0, len(rows), SHARD):
            chunk = rows[start:start + SHARD]
            name = f"{directory}-{start:05d}-{start + len(chunk) - 1:05d}.json"
            data = dumps({"schema": f"OTERYN_CIPSOFT_APPEARANCE_{family.upper()}_SOURCE_OBSERVATIONS/v1",
                          "family": family, "records": chunk})
            outputs[f"{directory}/{name}"] = data
            files.append({"path": name, "records": len(chunk), "sha256": digest(data)})
        outputs[f"{directory}/manifest.json"] = dumps({
            "schema": f"OTERYN_CIPSOFT_APPEARANCE_{family.upper()}_SOURCE_MANIFEST/v1",
            "source": "CIPSOFT_CLIENT_ASSETS", "client_version": "15.30", "family": family,
            "top_level_field": field, "identity_namespace": f"cipsoft/appearance-{directory}",
            "record_count": len(rows), "authority": AUTHORITY,
            "inputs": {INPUT_ROLE: {"sha256": sha, "file": f"content/assets/files/{INPUT_ROLE}-{sha}.dat"}},
            "excluded_top_level": {"field_1_objects": EXPECTED_OBJECTS, "field_5": extra},
            "files": files,
        }, indent=True)
    return outputs


def compare(output: Path, expected: dict[str, bytes]) -> list[str]:
    problems = []
    for directory, *_ in TABLES.values():
        base_dir = output / directory
        present = {p.name for p in base_dir.glob("*")} if base_dir.is_dir() else set()
        wanted = {rel.split("/")[1] for rel in expected if rel.startswith(f"{directory}/")}
        problems += [f"unexpected file {directory}/{n}" for n in sorted(present - wanted)]
    for rel, data in expected.items():
        path = output / rel
        if not path.is_file() or path.read_bytes() != data:
            problems.append(f"differs or missing: {rel}")
    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input-dir", type=Path, default=base.DEFAULT_INPUT_DIR)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true", help="verify committed bytes equal regeneration; write nothing")
    args = parser.parse_args(argv)
    expected = build(base.read_input(args.input_dir, INPUT_ROLE))
    if args.check:
        problems = compare(args.output, expected)
        for problem in problems:
            print(problem, file=sys.stderr)
        print("appearance table staging check: " + ("FAIL" if problems else f"ok ({len(expected)} files)"))
        return 1 if problems else 0
    for rel, data in expected.items():
        path = args.output / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
