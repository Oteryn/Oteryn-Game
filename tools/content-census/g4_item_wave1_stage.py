#!/usr/bin/env python3
"""Stage Item Wave 1 typed facts from the frozen exact-revision snapshot.

Input: the committed source snapshot (captured by ``g4_item_wave1_capture.py``),
the protected field census, and the current legacy Reference records, which are
read only for the conflict report. Output: a deterministic staged document that
the canonical WorldProject seed applies. Unknown stays unknown: every source
observation that is absent, unparsed, blocked by an unaccepted typed contract,
or after the protected target cut is reported instead of being promoted.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
SNAPSHOT = ROOT / "docs/agents/evidence/OTV2-20260925-item-enrichment-wave1-source-snapshot.json"
FIELD_CENSUS = ROOT / "docs/agents/evidence/OTV2-20260925-tibiawiki-item-master-field-census-v1.json"
LEGACY_REFERENCE = ROOT / "content/world/definitions/reference.json"
STAGED = ROOT / "docs/agents/evidence/OTV2-20260925-item-enrichment-wave1-staged.json"

SCHEMA = "OTERYN_G4_ITEM_WAVE1_STAGED/v1"
MAPPER = "OTERYN_G4_ITEM_WAVE1_STAGE/v1"
BATCH_ID = "g4-item-wave1-tibiawiki-r1"
SNAPSHOT_SHA256 = "5d8b84eee85e226e99d516beb7b40b8dc201c923e9b63b5ef18313085c3cbdf5"
TARGET_CUT = "2026-07-28T23:59:59Z"
MAX_IMBUEMENT_SLOTS = 3

# Exact source enum spellings (TibiaWiki BR `type`) mapped to the serialized protected
# `ReferenceWeaponType` spelling, so staged values compare equal to legacy values.
WEAPON_TYPES = {"Espada": "SWORD", "Machado": "AXE", "Clava": "CLUB", "Distância": "DISTANCE", "Punho": "FIST"}
# Deterministic capability relations: a promoted typed fact names the ruleset that governs it.
RELATION_RULES = (
    ("imbuement.slot_count", lambda value: value >= 1, "rulesets/items/imbuements/"),
    ("forge", lambda value: True, "rulesets/items/exaltation-forge/"),
    ("lifecycle.enchantable", lambda value: value is True, "rulesets/items/enchanting/"),
)
# Retained observations that stay UNKNOWN in Wave 1, with the blocking contract.
BLOCKED = {
    "weight": "TYPED_WEIGHT_UNIT",
    "hands": "SLOT_SEMANTICS",
    "slottype": "SLOT_SEMANTICS",
    "levelrequired": "REQUIREMENTS_EQUIP_PATTERN",
    "vocrequired": "REQUIREMENTS_EQUIP_PATTERN",
    "itemclass": "NO_TYPED_ITEM_CLASS_SLOT",
    "edible": "CONSUMABLE_PROFILE_OUT_OF_WAVE1",
    "regenseconds": "CONSUMABLE_PROFILE_OUT_OF_WAVE1",
}


class StageError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256_hex(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def bool_pt(value: Any) -> bool | None:
    if isinstance(value, bool):
        return value
    if isinstance(value, str):
        folded = value.strip().casefold()
        if folded == "sim":
            return True
        if folded in {"não", "nao"}:
            return False
    return None


def positive_int(value: Any) -> int | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, int):
        return value
    if isinstance(value, str) and value.strip().isdigit():
        return int(value.strip())
    return None


def text(value: Any) -> str | None:
    if isinstance(value, str) and value.strip() == value and value and not any(ord(ch) < 32 for ch in value):
        return value
    return None


def known_semantics(record: dict[str, Any]) -> dict[str, Any]:
    """Flatten currently KNOWN Reference semantics to `group.field` paths."""
    out: dict[str, Any] = {}
    for group, slot in record.get("semantics", {}).items():
        if slot.get("state") != "KNOWN" or not isinstance(slot.get("value"), dict):
            continue
        for field, inner in slot["value"].items():
            if isinstance(inner, dict) and inner.get("state") == "KNOWN":
                out[f"{group}.{field}"] = inner["value"]
    return out


def stage(snapshot: dict[str, Any], census: dict[str, Any], legacy: dict[str, Any]) -> dict[str, Any]:
    assignments = census["family_assignments"]
    by_key = {record["identity"]["key"]: record for record in legacy["records"]}
    items: list[dict[str, Any]] = []
    unknown: list[dict[str, Any]] = []
    conflicts: list[dict[str, Any]] = []
    rejected: list[dict[str, Any]] = []
    for row in sorted(snapshot["rows"], key=lambda value: value["target"]["key"]):
        fields = row["fields"]
        key = row["target"]["key"]
        record = by_key.get(key)
        if record is None or record["identity"] != row["target"]:
            raise StageError(f"TARGET_ABSENT:{key}")
        provenance = {"external_id": row["external_id"], "revision_id": row["revision_id"], "source_digest": row["source_digest"]}

        def note(parameter: str, reason: str) -> None:
            unknown.append({"external_id": row["external_id"], "target_key": key, "source_parameter": parameter, "reason": reason})

        if row["revision_timestamp"] > TARGET_CUT:
            rejected.append({**provenance, "target_key": key, "reason": "REVISION_AFTER_TARGET_CUT"})
            continue
        name = fields.get("name", {}).get("value")
        if name != row["title"]:
            rejected.append({**provenance, "target_key": key, "reason": "NAME_TITLE_DISAGREE"})
            continue

        def value_of(parameter: str) -> Any:
            observation = fields.get(parameter)
            if observation is None:
                return None
            if observation.get("state") != "VALUE":
                note(parameter, "SOURCE_UNPARSED")
                return None
            return observation.get("value")

        facts: list[dict[str, Any]] = []

        def fact(path: str, value: Any, parameter: str) -> None:
            facts.append({"field_path": path, "value": value, "source_parameter": parameter, "source_value": fields[parameter]["value"]})

        weapon_type = value_of("type")
        if weapon_type is not None:
            if weapon_type in WEAPON_TYPES:
                fact("weapon.weapon_type", WEAPON_TYPES[weapon_type], "type")
            else:
                note("type", "WEAPON_TYPE_NOT_IN_PROTECTED_ENUM")
        slots = value_of("imbuement")
        if slots is not None:
            if positive_int(slots) is not None and positive_int(slots) <= MAX_IMBUEMENT_SLOTS:
                fact("imbuement.slot_count", positive_int(slots), "imbuement")
            else:
                note("imbuement", "IMBUEMENT_SLOTS_INVALID")
        stackable = value_of("stackable")
        if stackable is not None:
            parsed = bool_pt(stackable)
            if parsed is False and record["stack_class"] != "StackCapable":
                fact("stack.stackable", False, "stackable")
            elif parsed is True:
                note("stackable", "STACK_MAX")
            else:
                note("stackable", "STACKABLE_INVALID_OR_STACK_CLASS_CONFLICT")
        marketable = value_of("mercado")
        if marketable is not None:
            parsed = bool_pt(marketable)
            if parsed is None:
                note("mercado", "MARKETABLE_INVALID")
            else:
                fact("trade_restrictions.marketable", parsed, "mercado")

        authoring: dict[str, Any] = {}
        authoring_provenance: list[dict[str, Any]] = []

        def authored(path: str, parameter: str) -> None:
            authoring_provenance.append({"field_path": path, "source_parameter": parameter, "source_value": fields[parameter]["value"]})

        primary = text(value_of("primarytype"))
        if primary is None:
            note("primarytype", "TAXONOMY_PRIMARY_ABSENT")
        else:
            taxonomy: dict[str, Any] = {"primary": primary}
            authored("taxonomy.primary", "primarytype")
            for parameter, slot in (("secondarytype", "secondary"), ("tertiarytype", "tertiary")):
                value = value_of(parameter)
                if text(value) is not None:
                    taxonomy[slot] = value
                    authored(f"taxonomy.{slot}", parameter)
                elif value not in (None, ""):
                    note(parameter, "TAXONOMY_TEXT_INVALID")
            authoring["taxonomy"] = taxonomy
        classification, max_tier = positive_int(value_of("classificacao")), positive_int(value_of("max_tier"))
        if classification and max_tier:
            authoring["forge"] = {"classification": classification, "max_tier": max_tier}
            authored("forge.classification", "classificacao")
            authored("forge.max_tier", "max_tier")
        elif "classificacao" in fields or "max_tier" in fields:
            note("classificacao", "FORGE_PROFILE_INCOMPLETE")
        enchantable = value_of("enchantable")
        if enchantable is not None:
            parsed = bool_pt(enchantable)
            if parsed is None:
                note("enchantable", "ENCHANTABLE_INVALID")
            else:
                authoring["lifecycle"] = {"enchantable": parsed}
                authored("lifecycle.enchantable", "enchantable")
        lifecycle: dict[str, str] = {}
        for parameter, slot in (("implemented", "implemented"), ("removed", "removed")):
            value = text(value_of(parameter))
            if value is not None:
                lifecycle[slot] = value
                authored(f"source_lifecycle.{slot}", parameter)
        if lifecycle:
            authoring["source_lifecycle"] = lifecycle

        for parameter, reason in sorted(BLOCKED.items()):
            if parameter in fields:
                note(parameter, reason)

        existing = known_semantics(record)
        for entry in facts:
            prior = existing.get(entry["field_path"])
            if prior is not None and prior != entry["value"]:
                conflicts.append({**provenance, "target_key": key, "field_path": entry["field_path"], "existing": prior, "candidate": entry["value"]})

        family = assignments.get(primary) if primary is not None else None
        if primary is not None and family is None:
            note("primarytype", "FAMILY_PROFILE_UNASSIGNED")
        relation_values = {entry["field_path"]: entry["value"] for entry in facts}
        if "forge" in authoring:
            relation_values["forge"] = authoring["forge"]
        if "lifecycle" in authoring:
            relation_values["lifecycle.enchantable"] = authoring["lifecycle"]["enchantable"]
        relations = sorted(
            ruleset for path, predicate, ruleset in RELATION_RULES
            if path in relation_values and predicate(relation_values[path])
        )
        items.append({
            **provenance,
            "target": row["target"],
            "facts": sorted(facts, key=lambda entry: entry["field_path"]),
            "authoring": authoring,
            "authoring_provenance": sorted(authoring_provenance, key=lambda entry: entry["field_path"]),
            "family_profile": family,
            "capability_relations": relations,
        })
    if conflicts:
        raise StageError(f"CONFLICTS:{canonical_bytes(conflicts).decode('utf-8')}")
    counts = {
        "items_examined": len(snapshot["rows"]),
        "exact_identity_matches": len(snapshot["rows"]),
        "items_staged": len(items),
        "items_rejected": len(rejected),
        "definition_facts": sum(len(item["facts"]) for item in items),
        "authoring_facts": sum(len(item["authoring_provenance"]) for item in items),
        "taxonomy_assignments": sum("taxonomy" in item["authoring"] for item in items),
        "family_profiles_assigned": sum(item["family_profile"] is not None for item in items),
        "capability_relations": sum(len(item["capability_relations"]) for item in items),
        "unknown_preserved": len(unknown),
        "conflicts": 0,
    }
    per_field: dict[str, int] = {}
    for item in items:
        for entry in item["facts"] + item["authoring_provenance"]:
            per_field[entry["field_path"]] = per_field.get(entry["field_path"], 0) + 1
    return {
        "schema": SCHEMA,
        "mapper": MAPPER,
        "batch_id": BATCH_ID,
        "source": {
            "source_key": snapshot["source"]["source_key"],
            "source_revision": snapshot["source"]["source_revision"],
            "identity_namespace": snapshot["source"]["identity_namespace"],
            "snapshot_sha256": SNAPSHOT_SHA256,
            "field_census_sha256": sha256_hex(FIELD_CENSUS.read_bytes()),
            "evidence": "Derived",
        },
        "target_cut": TARGET_CUT,
        "counts": counts,
        "per_field": dict(sorted(per_field.items())),
        "items": items,
        "rejected": rejected,
        "unknown_report": sorted(unknown, key=lambda entry: (entry["target_key"], entry["source_parameter"], entry["reason"])),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail unless the committed staged document is reproduced byte for byte")
    args = parser.parse_args()
    snapshot_bytes = SNAPSHOT.read_bytes()
    if sha256_hex(snapshot_bytes) != SNAPSHOT_SHA256:
        raise StageError("SNAPSHOT_DIGEST_DRIFT")
    staged = canonical_bytes(stage(
        json.loads(snapshot_bytes),
        json.loads(FIELD_CENSUS.read_bytes()),
        json.loads(LEGACY_REFERENCE.read_bytes()),
    ))
    if args.check:
        if STAGED.read_bytes() != staged:
            raise StageError("STAGED_NOT_REPRODUCED")
    else:
        STAGED.write_bytes(staged)
    summary = json.loads(staged)["counts"]
    print("PASS " + " ".join(f"{key}={value}" for key, value in summary.items()) + f" sha256={sha256_hex(staged)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
