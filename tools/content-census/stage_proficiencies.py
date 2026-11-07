#!/usr/bin/env python3
"""Stage 15.30 client weapon proficiency source observations.

Two pinned, owner-provided official client files under content/assets/files/:
the proficiencies JSON (definitions) and the appearances protobuf (objects).
Definitions are validated against a closed set of key shapes. Bindings come from
object flags field 61 (a message holding exactly one varint, the proficiency id)
read with a small strict wire reader. Unknown shapes, duplicate ids and dangling
proficiency ids are rejected. The staged data are source observations, not
Oteryn definitions (see the READMEs next to the output).
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sys

INPUTS = {
    "proficiencies": ("json", "7fea90ec1cfd472f4b5978f4456b430d3271598b4e545b7919d692641411e015"),
    "appearances": ("dat", "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"),
}
DEFAULT_INPUT_DIR = Path("content/assets/files")
DEFAULT_OUTPUT = Path("imports/cipsoft-staticdata")
EXPECTED_COUNTS = {"proficiencies": 443, "weapon-proficiency-bindings": 666}
SHARD = 150
AUTHORITY = "SOURCE_OBSERVATIONS_ONLY; no Oteryn ProductionKeys or gameplay promotion"
OBJECT_FIELD, OBJECT_ID, OBJECT_FLAGS, OBJECT_NAME, FLAG_PROFICIENCY = 1, 1, 3, 4, 61
DEFINITION_KEYS = {"Levels", "Name", "ProficiencyId", "Version"}
PERK_KEY_SETS = {frozenset(k.split(",")) for k in (
    "Type,Value", "SkillId,Type,Value", "AugmentType,SpellId,Type,Value", "BestiaryId,BestiaryName,Type,Value",
    "ElementId,Type,Value", "DamageType,Type,Value", "Range,Type,Value",
    "ElementId,MissileId,Multiplier,Probability,Type",
)}


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
    """Return (field, wire_type, value); varint and length-delimited only, as in the sibling staging tool."""
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


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def dumps(payload: object, *, indent: bool = False) -> bytes:
    if indent:
        return (json.dumps(payload, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode()
    return (json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()


def read_input(directory: Path, role: str) -> bytes:
    ext, sha = INPUTS[role]
    data = (directory / f"{role}-{sha}.{ext}").read_bytes()
    if digest(data) != sha:
        raise ValueError(f"{role}: input digest mismatch")
    return data


def is_int(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def is_number(value: object) -> bool:
    return is_int(value) or isinstance(value, float)


def stage_definitions(raw: bytes) -> list[dict]:
    payload = json.loads(raw.decode("utf-8"))
    if not isinstance(payload, list):
        raise ValueError("proficiencies: top level is not a list")
    rows, seen = [], set()
    for index, item in enumerate(payload):
        if not isinstance(item, dict) or set(item) != DEFINITION_KEYS:
            raise ValueError(f"proficiency #{index}: unexpected keys")
        pid = item["ProficiencyId"]
        if not is_int(pid) or not is_int(item["Version"]) or not isinstance(item["Name"], str):
            raise ValueError(f"proficiency #{index}: wrong field type")
        if pid in seen:
            raise ValueError(f"duplicate proficiency id {pid}")
        seen.add(pid)
        if not isinstance(item["Levels"], list):
            raise ValueError(f"proficiency {pid}: Levels is not a list")
        for level in item["Levels"]:
            if not isinstance(level, dict) or set(level) != {"Perks"} or not isinstance(level["Perks"], list):
                raise ValueError(f"proficiency {pid}: unexpected level shape")
            for perk in level["Perks"]:
                if not isinstance(perk, dict) or frozenset(perk) not in PERK_KEY_SETS:
                    raise ValueError(f"proficiency {pid}: unexpected perk keys")
                if not all(is_number(v) or (k == "BestiaryName" and isinstance(v, str)) for k, v in perk.items()):
                    raise ValueError(f"proficiency {pid}: unexpected perk value type")
        rows.append({
            "source_id": pid, "source_index": index, "name": item["Name"], "version": item["Version"],
            "levels": item["Levels"], "source_record_sha256": digest(dumps(item)),
        })
    return rows


def stage_bindings(appearances: bytes, proficiency_ids: set[int]) -> list[dict]:
    """One record per appearance object whose flags carry field 61 (proficiency)."""
    rows, seen_objects = [], set()
    for index, (field, wire, raw) in enumerate(parse(appearances)):
        if field != OBJECT_FIELD or wire != 2:
            continue
        assert isinstance(raw, bytes)
        object_id, name, flags = None, None, []
        for f, w, v in parse(raw):
            if f == OBJECT_ID and w == 0 and object_id is None:
                object_id = v
            elif f == OBJECT_FLAGS and w == 2:
                flags.append(v)
            elif f == OBJECT_NAME and w == 2 and name is None:
                assert isinstance(v, bytes)
                name = v.decode("utf-8")  # strict
        if object_id is None:
            raise ValueError("appearance object without id")
        if object_id in seen_objects:
            raise ValueError(f"duplicate object id {object_id}")
        seen_objects.add(object_id)
        if len(flags) > 1:
            raise ValueError(f"object {object_id}: multiple flags messages")
        marked = [v for f, _, v in parse(flags[0]) if f == FLAG_PROFICIENCY] if flags else []
        if not marked:
            continue
        if len(marked) != 1 or not isinstance(marked[0], bytes):
            raise ValueError(f"object {object_id}: malformed proficiency flag")
        inner = parse(marked[0])
        if len(inner) != 1 or inner[0][:2] != (1, 0):
            raise ValueError(f"object {object_id}: proficiency flag shape")
        pid = inner[0][2]
        if pid not in proficiency_ids:
            raise ValueError(f"object {object_id}: dangling proficiency id {pid}")
        rows.append({
            "source_id": object_id, "source_index": index, "name": name, "proficiency_id": pid,
            "appearance_object_sha256": digest(raw),
        })
    return rows


def build(proficiencies: bytes, appearances: bytes) -> dict[str, bytes]:
    """Return {relative output path: bytes} for every generated file."""
    definitions = stage_definitions(proficiencies)
    sets = {
        "proficiencies": (definitions, "Proficiency", "proficiencies", "proficiency_id"),
        "weapon-proficiency-bindings": (
            stage_bindings(appearances, {row["source_id"] for row in definitions}),
            "WeaponProficiencyBinding", "weapon-proficiency-bindings", "appearance_object_id"),
    }
    outputs: dict[str, bytes] = {}
    for directory, (rows, family, stem, namespace) in sets.items():
        if len(rows) != EXPECTED_COUNTS[directory]:
            raise ValueError(f"{directory}: expected {EXPECTED_COUNTS[directory]} records, found {len(rows)}")
        upper = family.upper()
        files = []
        for start in range(0, len(rows), SHARD):
            chunk = rows[start:start + SHARD]
            name = f"{stem}-{start:05d}-{start + len(chunk) - 1:05d}.json"
            data = dumps({"schema": f"OTERYN_CIPSOFT_{upper}_SOURCE_OBSERVATIONS/v1", "family": family, "records": chunk})
            outputs[f"{directory}/{name}"] = data
            files.append({"path": name, "records": len(chunk), "sha256": digest(data)})
        used = ("proficiencies",) if directory == "proficiencies" else tuple(INPUTS)
        manifest = {
            "schema": f"OTERYN_CIPSOFT_{upper}_SOURCE_MANIFEST/v1",
            "source": "CIPSOFT_CLIENT_ASSETS", "client_version": "15.30", "family": family,
            "identity_namespace": f"cipsoft/{namespace}",
            "record_count": len(rows), "authority": AUTHORITY,
            "inputs": {role: {"sha256": INPUTS[role][1], "file": f"content/assets/files/{role}-{INPUTS[role][1]}.{INPUTS[role][0]}"}
                       for role in used},
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
        print("proficiency staging check: " + ("FAIL" if problems else f"ok ({len(expected)} files)"))
        return 1 if problems else 0
    for rel, data in expected.items():
        path = args.output / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
