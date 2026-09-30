#!/usr/bin/env python3
"""Condition authoring: build and validate the static ConditionDefinition catalogue (COND-CONTENT-1).

`capture --canary <checkout>` reads the pinned Canary checkout: it parses the `field` items of
data/items/items.xml and checks every cited line range of authored-conditions.json against its needles, and
writes the source facts. `build` derives the catalogue from those facts, the authored rows and the
committed monster condition Effects in content/abilities/; `build --check` diffs a rebuild against
the committed candidate. `validate` runs the schema and the semantic rules. `content` writes
(`--check` verifies) content/conditions/ and its registration; only the control plane's content
train runs it without `--check`.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import subprocess
import sys
import xml.etree.ElementTree as ET
from fractions import Fraction
from itertools import pairwise
from math import gcd
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SCHEMA = HERE / "condition.schema.json"
CATALOGUE = HERE / "samples" / "conditions-candidate.json"
SOURCES = HERE / "samples" / "canary-condition-sources-04b83b51.json"
AUTHORED_ROWS = HERE / "authored-conditions.json"
ABILITIES = ROOT / "content" / "abilities"
CANARY_REVISION = "04b83b512114bfd888000d6e1433ed8ecaec7c5b"
DECISION = "docs/architecture/reviews/OTERYN_GAME_CONDITIONS0_ACTOR_CONDITIONS_DECISION_2026-09-30.md"
ITEM_USE0 = (
    "docs/architecture/reviews/OTERYN_GAME_ITEM_USE0_USING_ITEMS_DECISION_2026-09-30.md"
)
REVISION = "definition-r1"
FAMILY = "Condition"
CONTENT_DIR = "content/conditions/"
INDEX_PATH = CONTENT_DIR + "index.json"
SHARD_SIZE = 500
CATALOGUE_REL = (
    "tools/content-schema/condition-authoring/samples/conditions-candidate.json"
)
CATALOGUE_SCHEMA = "OTERYN_CONDITION_AUTHORING_CATALOGUE/v1"
INDEX_SCHEMA = "OTERYN_FAMILY_INDEX/v1"
SHARD_SCHEMA = "OTERYN_CONDITION_SHARD/v1"
MIN_TICK_MS = 1000  # COND0-RL-02
MAX_CONFLICT_KEYS = 16  # COND0-RL-01
FOOD_MAX_MS = 1_200_000  # ITEM-USE-0 §6.1, CONDITIONS-0 §4.2

ELEMENTS = (
    "poison",
    "fire",
    "energy",
    "bleeding",
    "drown",
    "freezing",
    "dazzled",
    "cursed",
)
FAMILY_KEYS = {
    "SPEED": ("speed",),
    "DAMAGE_OVER_TIME": ELEMENTS,
    "FOOD_REGENERATION": ("food_regeneration",),
    "RECOVERY": ("recovery",),
    "MANA_SHIELD": ("mana_shield",),
    "LIGHT": ("light",),
}
VALUE_BLOCK = {family: family.lower() for family in FAMILY_KEYS}  # the value block
SPEED_KINDS = {"haste": "haste", "paralyze": "paralysis"}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def git_blob(data: bytes) -> str:
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def dumps(catalogue: dict) -> str:
    """Header fields indented; one definition per line, to keep the file reviewable."""
    head = {k: v for k, v in catalogue.items() if k != "conditions"}
    body = json.dumps(head, indent=2, ensure_ascii=False, sort_keys=True)
    rows = ",\n".join(
        "    " + json.dumps(c, ensure_ascii=False, sort_keys=True)
        for c in catalogue["conditions"]
    )
    return body[:-2] + ',\n  "conditions": [\n' + rows + "\n  ]\n}\n"


def capture(canary: Path) -> dict:
    """Source facts from a Canary checkout at CANARY_REVISION: field items and cited Lua lines."""
    head = subprocess.run(
        ["git", "-C", str(canary), "rev-parse", "HEAD"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    if head != CANARY_REVISION:
        raise ValueError(
            f"Canary checkout is at {head}, not the pinned {CANARY_REVISION}"
        )
    if subprocess.run(
        ["git", "-C", str(canary), "status", "--porcelain"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip():
        raise ValueError("Canary checkout has local changes")
    files = {}
    for row in load_json(AUTHORED_ROWS)["rows"]:
        for item in row["evidence"]:
            data = (canary / item["path"]).read_bytes()
            start, end = map(int, item["lines"].split("-"))
            text = "\n".join(data.decode("utf-8").splitlines()[start - 1 : end])
            missing = [n for n in item["needles"] if n not in text]
            if missing:
                raise ValueError(f"{item['path']}:{item['lines']} lacks {missing}")
            files[item["path"]] = git_blob(data)
    xml = canary / "data" / "items" / "items.xml"
    files["data/items/items.xml"] = git_blob(xml.read_bytes())
    fields = []
    for node in ET.parse(xml).getroot():
        for attr in node.findall("attribute"):
            if attr.get("key") != "field":
                continue
            ids = node.get("id") or f"{node.get('fromid')}-{node.get('toid')}"
            first, _, last = ids.partition("-")
            fields.append(
                {
                    "item_ids": list(range(int(first), int(last or first) + 1)),
                    "name": node.get("name"),
                    "element": attr.get("value"),
                    "attributes": [
                        [a.get("key").lower(), int(a.get("value"))]
                        for a in attr.findall("attribute")
                    ],
                }
            )
    return {
        "schema": "OTERYN_CONDITION_SOURCES/v1",
        "canary": {
            "repository": "https://github.com/opentibiabr/canary",
            "revision": CANARY_REVISION,
        },
        "git_blobs": dict(sorted(files.items())),
        "fields": fields,
    }


def field_schedule(attributes: list[list]) -> dict | str:
    """Canary ItemParse::parseFieldCombatDamage in XML order; a string says why there is none.

    `damage` with `start` 0 adds `count` ticks of `damage` every `ticks` ms; with `start` > 0 it adds
    the decreasing list of total `damage` starting at `start`. A `start` after `damage` is ignored.
    A field condition is not delayed, so its first tick is dealt at once.
    """
    interval, count, start, schedules = 0, 1, 0, []
    for key, value in attributes:
        if key == "ticks":
            interval = value
        elif key == "count":
            count = max(1, value)
        elif key == "start":
            start = max(0, value)
        elif key == "damage":
            if value <= 0:
                return "no damage: the field applies no condition"
            if start == 0:
                ticks = [{"count": count, "interval_ms": interval, "amount": value}]
                schedules.append(
                    {"tick_profile": "Fixed", "first_tick": "Immediate", "ticks": ticks}
                )
            else:
                schedules.append(
                    {
                        "tick_profile": "Decreasing",
                        "first_tick": "Immediate",
                        "total_minimum": value,
                        "total_maximum": value,
                        "tick_interval_ms": interval,
                        "initial_tick_amount": start,
                    }
                )
            start = 0
        else:
            return f"unknown field attribute {key!r}"
    if len(schedules) != 1:
        return f"{len(schedules)} damage entries: the field applies no single condition"
    return schedules[0]


def intervals(schedule: dict) -> list[int]:
    if schedule["tick_profile"] == "Fixed":
        return [t["interval_ms"] for t in schedule["ticks"]]
    return [schedule["tick_interval_ms"]]


def schedule_errors(schedule: dict) -> list[str]:
    """The ProjectV2DamageOverTime invariants (apps/game-server/src/content/project/v2/creature.rs)."""
    profile = schedule["tick_profile"]
    if profile == "Decreasing" and not (
        0 < schedule["total_maximum"]
        and schedule["total_minimum"] <= schedule["total_maximum"]
    ):
        return ["a decreasing total range must be non-empty and positive"]
    if profile == "Geometric":
        counts, factor = schedule["tick_counts"], schedule["factor"]
        if schedule["base_minimum"] > schedule["base_maximum"]:
            return ["base_minimum exceeds base_maximum"]
        if any(a >= b for a, b in pairwise(counts)):
            return ["geometric tick_counts must strictly increase"]
        if gcd(factor["numerator"], factor["denominator"]) != 1:
            return ["geometric factor is not in lowest terms"]
    return []


def record(
    key: str,
    name: str,
    family: str,
    conflict_key: str,
    used_by: dict,
    evidence: list,
    values: dict | None,
    blocked: str | None = None,
) -> dict:
    negative = family == "DAMAGE_OVER_TIME" or (values or {}).get("kind") == "paralysis"
    out = {
        "identity": {
            "family": FAMILY,
            "key": f"oteryn:condition.{key}",
            "revision": REVISION,
        },
        "name": name,
        "family": family,
        "conflict_key": conflict_key,
        "negative": negative,
        "used_by": used_by,
        "evidence": evidence,
        "status": "blocked" if blocked else "admitted",
    }
    if blocked:
        out["blocked_reason"] = blocked
    else:
        out[VALUE_BLOCK[family]] = values
    return out


def authored(sources: dict) -> list[dict]:
    canary = sources["canary"]["revision"]
    out = []
    for row in load_json(AUTHORED_ROWS)["rows"]:
        family = row["family"]
        evidence = [
            {
                "reference": "canary",
                "revision": canary,
                "path": e["path"],
                "lines": e["lines"],
                "git_blob": sources["git_blobs"][e["path"]],
            }
            for e in row["evidence"]
        ]
        if "decision" in row:
            evidence.insert(0, {"reference": "decision", "path": row["decision"]})
        values = row.get(VALUE_BLOCK[family])
        if family == "DAMAGE_OVER_TIME" and "blocked" not in row:
            values = {"field": False, "schedule": row["schedule"]}
        kind, ref = row["used_by"]
        conflict = row.get("element") or FAMILY_KEYS[family][0]
        out.append(
            record(
                row["key"],
                row["name"],
                family,
                conflict,
                {"kind": kind, "ref": ref},
                evidence,
                values,
                row.get("blocked"),
            )
        )
    return out


def fields(sources: dict) -> list[dict]:
    out = []
    blob = sources["git_blobs"]["data/items/items.xml"]
    for field in sources["fields"]:
        schedule = field_schedule(field["attributes"])
        if isinstance(schedule, str):
            continue  # no condition: recorded in the source facts, not a definition
        ids = field["item_ids"]
        slow = [i for i in intervals(schedule) if i < MIN_TICK_MS]
        blocked = (
            f"tick interval {slow[0]} ms is below COND0-RL-02 ({MIN_TICK_MS} ms)"
            if slow
            else None
        )
        evidence = [
            {
                "reference": "canary",
                "revision": sources["canary"]["revision"],
                "path": "data/items/items.xml",
                "item_ids": ids,
                "git_blob": blob,
            }
        ]
        out.append(
            record(
                f"field.i{ids[0]}",
                f"{field['name']} ({ids[0]})",
                "DAMAGE_OVER_TIME",
                field["element"],
                {"kind": "field", "ref": ",".join(map(str, ids))},
                evidence,
                {"field": True, "schedule": schedule},
                blocked,
            )
        )
    return out


def content_effects() -> tuple[list[tuple[str, str, dict]], dict]:
    effects, formulas = [], {}
    for path in sorted((ABILITIES / "formulas").glob("formulas-*.json")):
        for r in load_json(path)["records"]:
            formulas[r["definition"]["identity"]["key"]] = r["authoring"]["profile"]

    def walk(node: object, rel: str) -> None:
        if isinstance(node, dict):
            effect = node.get("effect")
            operation = effect.get("operation") if isinstance(effect, dict) else None
            if (
                isinstance(operation, dict)
                and operation.get("operation") == "Condition"
            ):
                effects.append((rel, effect["key"], operation))
            for value in node.values():
                walk(value, rel)
        elif isinstance(node, list):
            for value in node:
                walk(value, rel)

    for path in sorted((ABILITIES / "definitions").glob("abilities-*.json")):
        walk(load_json(path), path.relative_to(ROOT).as_posix())
    return effects, formulas


def thousandths(ratio: dict) -> int | None:
    value = Fraction(ratio["numerator"], ratio["denominator"]) * 1000
    return value.numerator if value.denominator == 1 else None


def monster_conditions() -> list[dict]:
    effects, formulas = content_effects()
    out = []
    for rel, effect_key, op in effects:
        condition = op["condition"]
        ctype = condition["condition_type"]
        key = "monster." + effect_key.removeprefix("oteryn:effect.")
        evidence = [{"reference": "content", "path": rel, "effect_key": effect_key}]
        used_by = {"kind": "monster_attack", "ref": effect_key}
        if ctype in ELEMENTS:
            dot = condition["damage_over_time"]
            out.append(
                record(
                    key,
                    key,
                    "DAMAGE_OVER_TIME",
                    ctype,
                    used_by,
                    evidence,
                    {"field": False, "schedule": dot},
                )
            )
        elif ctype in SPEED_KINDS:
            formula_key = condition["speed_formula"]["key"]
            profile = formulas[formula_key]
            evidence.append(
                {
                    "reference": "content",
                    "path": "content/abilities/formulas/",
                    "formula_key": formula_key,
                }
            )
            a_min = thousandths(profile["minimum_multiplier"])
            a_max = thousandths(profile["maximum_multiplier"])
            blocked = None
            if profile["formula"] != "SpeedModifier" or "duration_ms" not in op:
                blocked = "not a SpeedModifier formula with a fixed duration"
            elif a_min is None or a_max is None:
                ratios = [
                    f"{profile[k]['numerator']}/{profile[k]['denominator']}"
                    for k in ("minimum_multiplier", "maximum_multiplier")
                ]
                blocked = (
                    f"coefficients {ratios} are not whole thousandths (CONDITIONS-0 §3)"
                )
            values = {
                "kind": SPEED_KINDS[ctype],
                "duration_ms": op.get("duration_ms"),
                "a_min": a_min,
                "b_min": profile["minimum_offset"],
                "a_max": a_max,
                "b_max": profile["maximum_offset"],
            }
            out.append(
                record(key, key, "SPEED", "speed", used_by, evidence, values, blocked)
            )
    return out


def build() -> dict:
    sources = load_json(SOURCES)
    conditions = authored(sources) + fields(sources) + monster_conditions()
    return {
        "schema": CATALOGUE_SCHEMA,
        "decision": DECISION,
        "source": {
            "canary": sources["canary"],
            "canary_facts": "tools/content-schema/condition-authoring/samples/"
            + SOURCES.name,
            "canary_facts_sha256": sha256(SOURCES.read_bytes()),
            "monster_effects": "content/abilities/definitions/ (Canary 47dfd51f monster conversion)",
        },
        "conditions": sorted(conditions, key=lambda c: c["identity"]["key"]),
    }


def semantic_errors(catalogue: dict) -> list[str]:
    errors: list[str] = []
    seen: set[str] = set()
    keys_used: set[str] = set()
    for c in catalogue["conditions"]:
        key, family = c["identity"]["key"], c["family"]
        if key in seen:
            errors.append(f"duplicate key {key}")
        seen.add(key)
        if c["conflict_key"] not in FAMILY_KEYS[family]:
            errors.append(
                f"{key}: conflict key {c['conflict_key']} does not belong to {family}"
            )
        admitted = c["status"] == "admitted"
        present = [b for b in VALUE_BLOCK.values() if b in c]
        if admitted != (present == [VALUE_BLOCK[family]]):
            errors.append(
                f"{key}: an admitted record carries exactly its family block {VALUE_BLOCK[family]}"
            )
            continue
        values = c.get(VALUE_BLOCK[family], {})
        paralysis = values.get("kind") == "paralysis"
        if c["negative"] != (family == "DAMAGE_OVER_TIME" or paralysis):
            errors.append(
                f"{key}: negative must mark damage over time and paralysis only"
            )
        if not admitted:
            continue
        keys_used.add(c["conflict_key"])
        if family == "DAMAGE_OVER_TIME":
            schedule = values["schedule"]
            errors += [f"{key}: {e}" for e in schedule_errors(schedule)]
            if any(i < MIN_TICK_MS for i in intervals(schedule)):
                errors.append(
                    f"{key}: a tick interval is below COND0-RL-02 ({MIN_TICK_MS} ms)"
                )
            is_field = c["used_by"]["kind"] == "field"
            if (
                values["field"] != is_field
                or (schedule["first_tick"] == "Immediate") != is_field
            ):
                errors.append(
                    f"{key}: only a field is `field` and ticks at once; others are delayed"
                )
        if family == "RECOVERY":
            gaps = [
                values[k]
                for k in ("health_interval_ms", "mana_interval_ms")
                if k in values
            ]
            if any(g < MIN_TICK_MS for g in gaps):
                errors.append(f"{key}: a regeneration interval is below COND0-RL-02")
        if family == "SPEED" and values["a_min"] > values["a_max"]:
            errors.append(f"{key}: a_min exceeds a_max")
        if family == "FOOD_REGENERATION" and values["max_duration_ms"] != FOOD_MAX_MS:
            errors.append(
                f"{key}: food time is capped at {FOOD_MAX_MS} ms (ITEM-USE-0 §6.1)"
            )
    if len(keys_used) > MAX_CONFLICT_KEYS:
        errors.append(
            f"{len(keys_used)} conflict keys exceed COND0-RL-01 ({MAX_CONFLICT_KEYS})"
        )
    return errors


def validate(catalogue: dict) -> list[str]:
    validator = Draft202012Validator(load_json(SCHEMA), format_checker=FormatChecker())
    errors = [
        f"{'/'.join(map(str, e.absolute_path)) or '<root>'}: {e.message}"
        for e in sorted(
            validator.iter_errors(catalogue), key=lambda e: list(e.absolute_path)
        )
    ]
    return errors or semantic_errors(catalogue)


def compact(payload: object) -> str:
    return (
        json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    )


def content_files(catalogue: dict) -> dict[str, str]:
    """Admitted definitions only: a blocked key is absent, so a reference to it fails closed."""
    records = [
        {
            "definition": {
                **{k: v for k, v in c.items() if k not in ("identity", "status")},
                "identity": {
                    "key": c["identity"]["key"],
                    "revision": c["identity"]["revision"],
                },
            }
        }
        for c in catalogue["conditions"]
        if c["status"] == "admitted"
    ]
    files, shards = {}, []
    for index, start in enumerate(range(0, len(records), SHARD_SIZE)):
        chunk = records[start : start + SHARD_SIZE]
        end = start + len(chunk) - 1
        path = f"{CONTENT_DIR}conditions-{start:05d}-{end:05d}.json"
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
    """The three registration documents with the Condition family registered."""
    project, manifest, lock = map(copy.deepcopy, (project, manifest, lock))
    if FAMILY not in project["migrated_families"]:
        project["migrated_families"] = project["migrated_families"] + [FAMILY]
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
    errors = validate(catalogue)
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        return 1
    names = ("project", "manifest", "content.lock")
    docs = [load_json(ROOT / f"content/{n}.json") for n in names]
    outputs = content_files(catalogue)
    count = json.loads(outputs[INDEX_PATH])["record_count"]
    for name, doc in zip(names, registered(*docs, count, sorted(outputs)), strict=True):
        outputs[f"content/{name}.json"] = compact(doc)
    extra = (
        sorted(
            p.relative_to(ROOT).as_posix()
            for p in (ROOT / CONTENT_DIR).glob("*")
            if p.relative_to(ROOT).as_posix() not in outputs
        )
        if (ROOT / CONTENT_DIR).is_dir()
        else []
    )
    stale = list(extra)
    if not check:
        for rel in extra:
            (ROOT / rel).unlink()
    for rel, text in sorted(outputs.items()):
        path = ROOT / rel
        if path.is_file() and path.read_text(encoding="utf-8") == text:
            continue
        stale.append(rel)
        if not check:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8", newline="\n")
    if check:
        print("condition content check: " + (f"FAIL, stale {stale}" if stale else "ok"))
        return 1 if stale else 0
    print(f"condition content: wrote or removed {stale}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    capture_cmd = sub.add_parser(
        "capture", help="write the Canary source facts (local checkout)"
    )
    capture_cmd.add_argument("--canary", type=Path, required=True)
    build_cmd = sub.add_parser(
        "build", help="write (or --check) the candidate catalogue"
    )
    build_cmd.add_argument("--check", action="store_true")
    validate_cmd = sub.add_parser("validate", help="validate a catalogue file")
    validate_cmd.add_argument("path", type=Path)
    content_cmd = sub.add_parser(
        "content", help="write content/conditions/ and its registration"
    )
    content_cmd.add_argument(
        "--check",
        action="store_true",
        help="fail if the committed content tree differs from the catalogue",
    )
    args = parser.parse_args()

    if args.command == "capture":
        SOURCES.write_text(
            json.dumps(capture(args.canary), indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        print(f"wrote {SOURCES.relative_to(ROOT)}")
        return 0
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
    errors = validate(load_json(args.path))
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        return 1
    print(f"{args.path}: valid")
    return 0


if __name__ == "__main__":
    sys.exit(main())
