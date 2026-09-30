#!/usr/bin/env python3
"""Stage 15.30 client staticdata creature, bestiary-class, boss and quest-line observations.

Extends the #1277 staging pattern and reuses its strict wire reader
(stage_staticdata_houses_achievements.py). Input is the pinned, owner-provided
official client staticdata file under content/assets/files/ (read-only). Every
record shape is asserted: unknown fields, wrong wire types, duplicate ids and
count mismatches are rejected. Unlabelled numeric fields keep raw names
(f4..f7); inferred meanings are only noted in the README. The staged data are
source observations, not Oteryn definitions.
"""
from __future__ import annotations

import argparse
import importlib.util
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
_spec = importlib.util.spec_from_file_location("stage_base", HERE / "stage_staticdata_houses_achievements.py")
if _spec is None or _spec.loader is None:
    raise RuntimeError("base staging script import failed")
base = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(base)

ROLE = "staticdata"
SHA = base.INPUTS[ROLE]
DEFAULT_INPUT_DIR = base.DEFAULT_INPUT_DIR
DEFAULT_OUTPUT = base.DEFAULT_OUTPUT
# directory -> (family, file stem, identity namespace, shard size, expected count)
FAMILIES = {
    "creatures": ("Creature", "creatures", "creature_id", 250, 833),
    "bestiary-classes": ("BestiaryClass", "bestiary-classes", "bestiary_class_id", 500, 21),
    "bosses": ("Boss", "bosses", "boss_id", 250, 447),
    "quest-lines": ("QuestLine", "quest-lines", "quest_line_id", 500, 102),
}


def rows(data: bytes, number: int, spec: dict[int, str], name: str) -> list[tuple[int, dict, bytes]]:
    out, seen = [], set()
    for index, raw in enumerate(base.table(data, number)):
        f = base.message(raw, spec, name=name)
        if f[1] in seen:
            raise ValueError(f"duplicate {name} id {f[1]}")
        seen.add(f[1])
        out.append((index, f, raw))
    return out


def look(raw: bytes) -> dict:
    """Keep the look sub-message verbatim (hex); it must parse as strict wire data."""
    base.parse(raw)
    return {"hex": raw.hex()}


def stage_creatures(data: bytes) -> list[dict]:
    spec = {1: "v", 2: "b", 3: "b", 4: "v", 5: "v", 6: "v", 7: "v"}
    return [{
        "source_id": f[1], "source_index": i, "name": base.text(f[2]), "look": look(f[3]),
        "f4": f[4], "f5": f[5], "f6": f[6], "f7": f[7], "staticdata_sha256": base.digest(raw),
    } for i, f, raw in rows(data, 1, spec, "creature")]


def stage_classes(data: bytes) -> list[dict]:
    return [{"source_id": f[1], "source_index": i, "name": base.text(f[2]), "staticdata_sha256": base.digest(raw)}
            for i, f, raw in rows(data, 2, {1: "v", 2: "b"}, "bestiary class")]


def stage_bosses(data: bytes) -> list[dict]:
    spec = {1: "v", 2: "b", 3: "b", 4: "v"}
    return [{
        "source_id": f[1], "source_index": i, "name": base.text(f[2]), "look": look(f[3]),
        "f4": f[4], "staticdata_sha256": base.digest(raw),
    } for i, f, raw in rows(data, 5, spec, "boss")]


def stage_quests(data: bytes) -> list[dict]:
    return [{"source_id": f[1], "source_index": i, "name": base.text(f[2]), "staticdata_sha256": base.digest(raw)}
            for i, f, raw in rows(data, 6, {1: "v", 2: "b"}, "quest line")]


def build(staticdata: bytes) -> dict[str, bytes]:
    sets = {
        "creatures": stage_creatures(staticdata), "bestiary-classes": stage_classes(staticdata),
        "bosses": stage_bosses(staticdata), "quest-lines": stage_quests(staticdata),
    }
    outputs: dict[str, bytes] = {}
    for directory, (family, stem, namespace, shard, expected) in FAMILIES.items():
        records = sets[directory]
        if len(records) != expected:
            raise ValueError(f"{directory}: expected {expected} records, found {len(records)}")
        files = []
        for start in range(0, len(records), shard):
            chunk = records[start:start + shard]
            name = f"{stem}-{start:05d}-{start + len(chunk) - 1:05d}.json"
            payload = base.dumps({"schema": f"OTERYN_CIPSOFT_STATICDATA_{family.upper()}_SOURCE_OBSERVATIONS/v1",
                                  "family": family, "records": chunk})
            outputs[f"{directory}/{name}"] = payload
            files.append({"path": name, "records": len(chunk), "sha256": base.digest(payload)})
        manifest = {
            "schema": f"OTERYN_CIPSOFT_STATICDATA_{family.upper()}_SOURCE_MANIFEST/v1",
            "source": "CIPSOFT_CLIENT_STATICDATA", "client_version": "15.30", "family": family,
            "identity_namespace": f"cipsoft/staticdata/{namespace}",
            "record_count": len(records), "authority": base.AUTHORITY,
            "inputs": {ROLE: {"sha256": SHA, "file": f"content/assets/files/{ROLE}-{SHA}.dat"}},
            "files": files,
        }
        outputs[f"{directory}/manifest.json"] = base.dumps(manifest, indent=True)
    return outputs


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input-dir", type=Path, default=DEFAULT_INPUT_DIR)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true", help="verify committed bytes equal regeneration; write nothing")
    args = parser.parse_args(argv)
    expected = build(base.read_input(args.input_dir, ROLE))
    if args.check:
        problems = base.compare(args.output, expected)
        for problem in problems:
            print(problem, file=sys.stderr)
        print("staticdata creatures/bosses/quests staging check: " + ("FAIL" if problems else f"ok ({len(expected)} files)"))
        return 1 if problems else 0
    for rel, data in expected.items():
        path = args.output / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
