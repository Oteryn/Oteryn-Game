#!/usr/bin/env python3
"""Weapon Proficiency authoring: build and validate the static Proficiency catalogue.

`build` promotes the staged 15.30 client definitions (imports/cipsoft-staticdata/proficiencies)
into typed Oteryn definitions using the owner-accepted D199 perk code map, and rejects any
unmapped code or key set. `build --check` diffs an in-memory build against the committed
candidate. `validate` runs the schema and the semantic rules, including a lossless round trip
of every perk back to its staged source record. `content` writes (`--check` verifies) the
populated content/proficiencies/ family and its registration in content/project.json,
content/manifest.json and content/content.lock.json.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import sys
from itertools import pairwise
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SCHEMA = HERE / "proficiency.schema.json"
CATALOGUE = HERE / "samples" / "proficiencies-candidate.json"
STAGING = ROOT / "imports" / "cipsoft-staticdata" / "proficiencies"
THRESHOLD_SOURCE = (
    ROOT
    / "tools"
    / "content-schema"
    / "item-authoring"
    / "samples"
    / "item-weapon-proficiency-15-30-7fea90ec.json"
)
PROFICIENCIES_SHA256 = (
    "7fea90ec1cfd472f4b5978f4456b430d3271598b4e545b7919d692641411e015"
)
REVISION = "definition-r1"
EXPECTED_COUNT = 443
FAMILY = "Proficiency"
CONTENT_DIR = "content/proficiencies/"
INDEX_PATH = CONTENT_DIR + "index.json"
SHARD_SIZE = 150
CATALOGUE_REL = (
    "tools/content-schema/proficiency-authoring/samples/proficiencies-candidate.json"
)
CATALOGUE_SCHEMA = "OTERYN_PROFICIENCY_AUTHORING_CATALOGUE/v1"
INDEX_SCHEMA = "OTERYN_FAMILY_INDEX/v1"
SHARD_SCHEMA = "OTERYN_PROFICIENCY_SHARD/v1"

# D199 (owner-accepted): source enum codes -> Oteryn enum names.
SKILLS = {
    1: "magic_level",
    6: "shielding",
    7: "distance",
    8: "sword",
    9: "club",
    10: "axe",
    11: "fist",
    13: "fishing",
}
ELEMENTS = {
    1: "physical",
    8: "fire",
    16: "earth",
    32: "energy",
    64: "ice",
    128: "holy",
    256: "death",
    1048576: "healing",
}
AUGMENTS = {
    2: "base_damage",
    3: "healing",
    6: "cooldown",
    14: "life_leech",
    15: "mana_leech",
    16: "critical_extra_damage",
    17: "critical_hit_chance",
}

# D199: source Type -> (kind, [(source key, field, enum table or None)]).
# Every kind except homing_missile also carries source Value as `value`.
PERK_TYPES: dict[int, tuple[str, list[tuple[str, str, dict | None]]]] = {
    0: ("attack_damage", []),
    1: ("defence", []),
    2: ("weapon_shield_defence", []),
    3: ("skill_bonus", [("SkillId", "skill", SKILLS)]),
    4: ("specialized_magic_level", [("DamageType", "element", ELEMENTS)]),
    5: (
        "spell_augment",
        [("SpellId", "spell_client_id", None), ("AugmentType", "augment", AUGMENTS)],
    ),
    6: (
        "bestiary_class_damage",
        [
            ("BestiaryId", "bestiary_class_id", None),
            ("BestiaryName", "bestiary_class_name", None),
        ],
    ),
    7: ("boss_damage", []),
    8: ("critical_hit_chance", []),
    9: ("elemental_critical_hit_chance", [("ElementId", "element", ELEMENTS)]),
    10: ("rune_critical_hit_chance", []),
    11: ("auto_attack_critical_hit_chance", []),
    12: ("critical_extra_damage", []),
    13: ("elemental_critical_extra_damage", [("ElementId", "element", ELEMENTS)]),
    14: ("rune_critical_extra_damage", []),
    15: ("auto_attack_critical_extra_damage", []),
    16: ("mana_leech", []),
    17: ("life_leech", []),
    18: ("mana_on_hit", []),
    19: ("life_on_hit", []),
    20: ("mana_on_kill", []),
    21: ("life_on_kill", []),
    22: ("damage_at_range", [("Range", "range", None)]),
    23: ("ranged_hit_chance", []),
    24: ("attack_range", []),
    25: ("skill_scaled_auto_attack_damage", [("SkillId", "skill", SKILLS)]),
    26: ("skill_scaled_spell_damage", [("SkillId", "skill", SKILLS)]),
    27: ("skill_scaled_healing", [("SkillId", "skill", SKILLS)]),
    28: ("alpha_strike_damage", []),
    29: ("omega_strike_damage", []),
    30: ("armor_penetration", []),
    31: ("elemental_pierce", [("ElementId", "element", ELEMENTS)]),
    32: (
        "homing_missile",
        [
            ("ElementId", "element", ELEMENTS),
            ("MissileId", "missile_client_id", None),
            ("Probability", "probability", None),
            ("Multiplier", "multiplier", None),
        ],
    ),
}
NO_VALUE_KINDS = {"homing_missile"}
KIND_TYPES = {kind: code for code, (kind, _) in PERK_TYPES.items()}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def dumps(catalogue: dict) -> str:
    """Header fields indented; one definition per line, to keep the file reviewable."""
    head = {k: v for k, v in catalogue.items() if k != "proficiencies"}
    body = json.dumps(head, indent=2, ensure_ascii=False, sort_keys=True)
    rows = ",\n".join(
        "    " + json.dumps(p, ensure_ascii=False, sort_keys=True)
        for p in catalogue["proficiencies"]
    )
    return body[:-2] + ',\n  "proficiencies": [\n' + rows + "\n  ]\n}\n"


def encode_perk(raw: dict) -> dict:
    code = raw.get("Type")
    if code not in PERK_TYPES:
        raise ValueError(f"unmapped perk Type {code!r}")
    kind, params = PERK_TYPES[code]
    expected = {"Type"} | {src for src, _, _ in params}
    if kind not in NO_VALUE_KINDS:
        expected.add("Value")
    if set(raw) != expected:
        raise ValueError(
            f"perk Type {code} has keys {sorted(raw)}, expected {sorted(expected)}"
        )
    perk: dict = {"kind": kind}
    for src, field, table in params:
        value = raw[src]
        if table is not None:
            if value not in table:
                raise ValueError(f"unmapped {src} {value!r} in perk Type {code}")
            value = table[value]
        perk[field] = value
    if kind not in NO_VALUE_KINDS:
        perk["value"] = raw["Value"]
    return perk


def decode_perk(perk: dict) -> dict:
    """Inverse of encode_perk, used by the round-trip rule."""
    code = KIND_TYPES[perk["kind"]]
    raw: dict = {"Type": code}
    for src, field, table in PERK_TYPES[code][1]:
        value = perk[field]
        if table is not None:
            value = {v: k for k, v in table.items()}[value]
        raw[src] = value
    if perk["kind"] not in NO_VALUE_KINDS:
        raw["Value"] = perk["value"]
    return raw


def staged_records() -> tuple[list[dict], list[dict]]:
    manifest = load_json(STAGING / "manifest.json")
    if manifest["inputs"]["proficiencies"]["sha256"] != PROFICIENCIES_SHA256:
        raise ValueError(
            "staging manifest is not pinned to the 15.30 proficiencies file"
        )
    files, records = [], []
    for entry in manifest["files"]:
        data = (STAGING / entry["path"]).read_bytes()
        if sha256(data) != entry["sha256"]:
            raise ValueError(
                f"staged file {entry['path']} does not match its manifest digest"
            )
        files.append({"path": entry["path"], "sha256": entry["sha256"]})
        records += json.loads(data)["records"]
    if len(records) != EXPECTED_COUNT:
        raise ValueError(
            f"expected {EXPECTED_COUNT} staged definitions, found {len(records)}"
        )
    return files, records


def definition(record: dict) -> dict:
    pid = record["source_id"]
    return {
        "identity": {
            "family": "Proficiency",
            "key": f"oteryn:proficiency.tibia.p{pid}",
            "revision": REVISION,
        },
        "name": record["name"],
        "source": {
            "proficiency_id": pid,
            "version": record["version"],
            "source_record_sha256": record["source_record_sha256"],
        },
        "levels": [
            {"level": i, "perks": [encode_perk(p) for p in level["Perks"]]}
            for i, level in enumerate(record["levels"], start=1)
        ],
    }


def threshold_tables() -> dict:
    staging = load_json(THRESHOLD_SOURCE)
    source = staging["threshold_source"]
    return {
        "source": {
            "url": source["url"],
            "revid": source["revid"],
            "acceptance": source["acceptance"],
        },
        "mastery_offset": staging["mastery_offset"],
        **{
            cls: staging["thresholds"][cls]
            for cls in ("standard", "knight", "crossbow")
        },
    }


def build() -> dict:
    files, records = staged_records()
    return {
        "schema": "OTERYN_PROFICIENCY_AUTHORING_CATALOGUE/v1",
        "client_version": "15.30",
        "source": {
            "staging": "imports/cipsoft-staticdata/proficiencies",
            "proficiencies_sha256": PROFICIENCIES_SHA256,
            "staged_files": files,
        },
        "perk_code_map": {
            "decision": "D199",
            "source": "tools/content-schema/item-authoring/samples/"
            "item-weapon-proficiency-15-30-7fea90ec.json perk_mapping",
        },
        "threshold_tables": threshold_tables(),
        "proficiencies": [
            definition(r) for r in sorted(records, key=lambda r: r["source_id"])
        ],
    }


def semantic_errors(catalogue: dict, source: list[dict] | None = None) -> list[str]:
    errors: list[str] = []
    seen_keys: set[str] = set()
    seen_ids: set[int] = set()
    for definition_ in catalogue["proficiencies"]:
        key = definition_["identity"]["key"]
        pid = definition_["source"]["proficiency_id"]
        if key in seen_keys:
            errors.append(f"duplicate key {key}")
        if pid in seen_ids:
            errors.append(f"duplicate proficiency_id {pid}")
        seen_keys.add(key)
        seen_ids.add(pid)
        if key != f"oteryn:proficiency.tibia.p{pid}":
            errors.append(f"{key}: key does not follow proficiency_id {pid}")
        levels = [lv["level"] for lv in definition_["levels"]]
        if levels != list(range(1, len(levels) + 1)):
            errors.append(f"{key}: levels must be 1..n in order, found {levels}")
        for lv in definition_["levels"]:
            for perk in lv["perks"]:
                if perk["kind"] in NO_VALUE_KINDS:
                    continue
                cooldown = (
                    perk["kind"] == "spell_augment" and perk["augment"] == "cooldown"
                )
                if cooldown and perk["value"] >= 0:
                    errors.append(
                        f"{key} level {lv['level']}: cooldown augment must be negative"
                    )
                if not cooldown and perk["value"] <= 0:
                    errors.append(
                        f"{key} level {lv['level']}: {perk['kind']} value must be positive"
                    )
    tables = catalogue["threshold_tables"]
    for cls in ("standard", "knight", "crossbow"):
        values = tables[cls]
        if any(b <= a for a, b in pairwise(values)):
            errors.append(f"threshold table {cls} must strictly increase")
    if source is not None:
        by_id = {r["source_id"]: r for r in source}
        if set(by_id) != seen_ids:
            errors.append("catalogue ids differ from the staged definitions")
        for definition_ in catalogue["proficiencies"]:
            record = by_id.get(definition_["source"]["proficiency_id"])
            if record is None:
                continue
            key = definition_["identity"]["key"]
            if (
                definition_["source"]["source_record_sha256"]
                != record["source_record_sha256"]
            ):
                errors.append(f"{key}: source_record_sha256 differs from staging")
            raw = [
                [decode_perk(p) for p in lv["perks"]] for lv in definition_["levels"]
            ]
            if raw != [lv["Perks"] for lv in record["levels"]]:
                errors.append(f"{key}: perks do not round-trip to the staged source")
    return errors


def validate(catalogue: dict, source: list[dict] | None = None) -> list[str]:
    validator = Draft202012Validator(load_json(SCHEMA), format_checker=FormatChecker())
    errors = [
        f"{'/'.join(map(str, e.absolute_path)) or '<root>'}: {e.message}"
        for e in sorted(
            validator.iter_errors(catalogue), key=lambda e: list(e.absolute_path)
        )
    ]
    return errors or semantic_errors(catalogue, source)


def compact(payload: object) -> str:
    return (
        json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    )


def content_files(catalogue: dict) -> dict[str, str]:
    """The populated family: shards of definitions and the family index.

    Threshold tables are progression rules (rulesets/progression/weapon-proficiency/), so only
    the definitions enter content/.
    """
    records = [
        {
            "definition": {
                **{k: v for k, v in d.items() if k != "identity"},
                "identity": {
                    "key": d["identity"]["key"],
                    "revision": d["identity"]["revision"],
                },
            }
        }
        for d in catalogue["proficiencies"]
    ]
    files, shards = {}, []
    for index, start in enumerate(range(0, len(records), SHARD_SIZE)):
        chunk = records[start : start + SHARD_SIZE]
        end = start + len(chunk) - 1
        path = f"{CONTENT_DIR}proficiencies-{start:05d}-{end:05d}.json"
        shards.append(path)
        files[path] = compact(
            {
                "family": FAMILY,
                "records": chunk,
                "schema": SHARD_SCHEMA,
                "shard": {
                    "count": len(chunk),
                    "end": end,
                    "index": index,
                    "start": start,
                },
            }
        )
    files[INDEX_PATH] = compact(
        {
            "authoring_source": {
                "path": CATALOGUE_REL,
                "schema": CATALOGUE_SCHEMA,
                "sha256": sha256(dumps(catalogue).encode()),
            },
            "family": FAMILY,
            "record_count": len(records),
            "schema": INDEX_SCHEMA,
            "shards": shards,
        }
    )
    return files


def registered(
    project: dict, manifest: dict, lock: dict, count: int, paths: list[str]
) -> tuple:
    """The three registration documents with the Proficiency family registered."""
    project, manifest, lock = map(copy.deepcopy, (project, manifest, lock))
    project["migrated_families"] = [
        f for f in project["migrated_families"] if f != FAMILY
    ] + [FAMILY]
    project["next_population_families"] = [
        f for f in project["next_population_families"] if f != FAMILY
    ]
    manifest["families"][FAMILY] = {"records": count, "index": INDEX_PATH}
    managed = {row["path"] for row in manifest["managed_files"]}
    managed = {p for p in managed if not p.startswith(CONTENT_DIR)} | set(paths)
    manifest["managed_files"] = [{"path": p} for p in sorted(managed)]
    lock["family_counts"][FAMILY] = count
    return project, manifest, lock


def content_command(check: bool) -> int:
    catalogue = load_json(CATALOGUE)
    errors = validate(catalogue, staged_records()[1])
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        return 1
    names = ("project", "manifest", "content.lock")
    docs = [load_json(ROOT / f"content/{n}.json") for n in names]
    outputs = content_files(catalogue)
    count = len(catalogue["proficiencies"])
    for name, doc in zip(names, registered(*docs, count, sorted(outputs)), strict=True):
        outputs[f"content/{name}.json"] = compact(doc)
    stale = []
    for rel, text in sorted(outputs.items()):
        path = ROOT / rel
        if path.is_file() and path.read_text(encoding="utf-8") == text:
            continue
        stale.append(rel)
        if not check:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8", newline="\n")
    if check:
        print(
            "proficiency content check: " + (f"FAIL, stale {stale}" if stale else "ok")
        )
        return 1 if stale else 0
    print(f"proficiency content: wrote {stale}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    build_cmd = sub.add_parser(
        "build", help="write (or --check) the candidate catalogue"
    )
    build_cmd.add_argument("--check", action="store_true")
    validate_cmd = sub.add_parser("validate", help="validate a catalogue file")
    validate_cmd.add_argument("path", type=Path)
    content_cmd = sub.add_parser(
        "content", help="write the content/proficiencies/ family and its registration"
    )
    content_cmd.add_argument(
        "--check",
        action="store_true",
        help="fail if the committed content tree differs from the candidate catalogue",
    )
    args = parser.parse_args()

    if args.command == "content":
        return content_command(args.check)

    if args.command == "build":
        text = dumps(build())
        if args.check:
            if not CATALOGUE.exists() or CATALOGUE.read_text(encoding="utf-8") != text:
                print(
                    f"{CATALOGUE.relative_to(ROOT)} is stale; run build",
                    file=sys.stderr,
                )
                return 1
            print("build --check: candidate catalogue is up to date")
            return 0
        CATALOGUE.write_text(text, encoding="utf-8")
        print(f"wrote {CATALOGUE.relative_to(ROOT)}")
        return 0

    errors = validate(load_json(args.path), staged_records()[1])
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        return 1
    print(f"{args.path}: valid")
    return 0


if __name__ == "__main__":
    sys.exit(main())
