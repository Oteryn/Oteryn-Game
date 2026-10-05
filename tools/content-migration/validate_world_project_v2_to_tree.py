#!/usr/bin/env python3
"""Validate exact semantic equivalence of the successor Item/Mount/creature tree."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

from item_taxonomy import build_taxonomy, legacy_profile, taxonomy_inputs
from world_project_v2_to_tree import capability_relations

ROOT = Path(__file__).resolve().parents[2]
LEGACY = ROOT / "content" / "world"
# Canary creature admission wave A (OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1 §7).
CREATURE_FAMILY_NODES = {
    "Creature": "content/creatures/definitions/",
    "Presentation": "content/presentations/definitions/",
    "Behavior": "content/behaviors/",
    "Loot": "content/loot/",
    "Ability": "content/abilities/definitions/",
    "Effect": "content/abilities/effects/",
    "Formula": "content/abilities/formulas/",
}
# NPC admission wave A (OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1).
NPC_COUNT = 1282
NPC_BINDING_COUNT = 2747
# Encounter admission (OTERYN_WORLD_PROJECT_V2_ENCOUNTER_ADMISSION_V1 E1-E5).
ENCOUNTER_COUNT = 104
DOCUMENT_COUNT = 1609
DIALOGUE_COUNT = 836
# Charm is a static family with no legacy source (tools/content-schema/charm-authoring).
CHARM_COUNT = 25
# Proficiency likewise (tools/content-schema/proficiency-authoring).
PROFICIENCY_COUNT = 443
PROFICIENCY_BINDING_COUNT = 665  # ITEM-ADD-1 plus the admitted appearance-only Snowball
# RewardClaim likewise (tools/content-schema/reward-claim-authoring).
REWARD_CLAIM_COUNT = len(json.loads(
    (ROOT / "tools/content-schema/quest-authoring/samples/chests/claims.json").read_text())["claims"])
# StarterKit likewise (tools/content-schema/starter-kit-authoring).
STARTER_KIT_COUNT = 1
SERVICE_FAMILY_COUNTS = {"Service.Trade": 324, "Service.Travel": 56}
SERVICE_FAMILY_NODES = {"Service.Trade": ("content/services/trade/", "offers"), "Service.Travel": ("content/services/travel/", "routes")}

class ValidationError(RuntimeError):
    pass

def load(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))

def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")

def canonical_sorted(values: list[Any]) -> list[Any]:
    return sorted(values, key=canonical_bytes)

def require(condition: bool, code: str) -> None:
    if not condition:
        raise ValidationError(code)

def staged_item_target(target: dict[str, Any], aliases: dict[str, str]) -> tuple[str, str, str]:
    """A staged (historical) Item target through the append-only alias table (A12, ITEM-ID-1)."""
    require(target["family"] == "Item" and target["key"] in aliases, f"STAGED_TARGET_WITHOUT_ALIAS:{target['key']}")
    return ("Item", aliases[target["key"]], target["revision"])

def item_alias_targets() -> dict[str, str]:
    current: dict[str, dict[str, Any]] = {}
    for entry in load(ROOT / "content/items/aliases.json")["entries"]:
        current[entry["key"]] = entry
    return {key: entry["target"] for key, entry in current.items() if entry["state"] == "ALIAS"}

def target_id(target: dict[str, Any]) -> tuple[str, str, str]:
    return (target["family"], target["key"], target["revision"])

def imports_batches() -> list[Any]:
    return load(LEGACY / "provenance" / "imports.json")["batches"]


def known_paths(definition: dict[str, Any]) -> dict[str, Any]:
    out: dict[str, Any] = {}
    for group, slot in definition.get("semantics", {}).items():
        if slot.get("state") == "KNOWN" and isinstance(slot.get("value"), dict):
            for field, inner in slot["value"].items():
                if isinstance(inner, dict) and inner.get("state") == "KNOWN":
                    out[f"{group}.{field}"] = inner["value"]
    return out


def authoring_value(entry: dict[str, Any], path: str) -> Any:
    value: Any = entry
    for part in path.split("."):
        require(isinstance(value, dict) and part in value, f"AUTHORING_FACT_ABSENT:{path}")
        value = value[part]
    return value


def closed_forge_owner():
    """One explicit source-qualified Forge pair; no global maximum/default inference."""
    raw = (ROOT / "docs/agents/evidence/OTV2-20261001-item-forge3332-promotion-v1.json").read_bytes()
    require(hashlib.sha256(raw).hexdigest() == "9cb2e9a0aec51870449de0a05c2d3796256ae6db30d114c63db4ace175884df1", "FORGE_PACKET_DIGEST")
    packet = json.loads(raw)
    owner = {"item": {"family": "Item", "key": "oteryn:item.tibia.i3332", "revision": "definition-r1"}, "forge": {"classification": 2, "max_tier": 2}}
    require(packet["schema"] == "OTERYN_ITEM_FORGE3332_PROMOTION/v1"
            and canonical_bytes(packet["promotion"]) == canonical_bytes(owner), "CLOSED_FORGE_OWNER_VALUE")
    return owner


def closed_weapon_metadata():
    """Sealed103 source properties; absolute percentages are not relative hit ratios."""
    raw = (ROOT / "docs/agents/evidence/OTV2-20261002-item-weapon-metadata-promotion-v1.json").read_bytes()
    require(hashlib.sha256(raw).hexdigest() == "9986d17a9c023dfe1d7e0052935c1f2ef517332f819477d4946bf1e8a99febbf", "WEAPON_METADATA_PACKET_DIGEST")
    packet = json.loads(raw)
    proof_raw = (ROOT / "docs/agents/evidence/OTV2-20261002-item-weapon-metadata-source-qualification-v2.json").read_bytes()
    require(hashlib.sha256(proof_raw).hexdigest() == "0a2bc3139df7c18290f5fee081efb4e67b5fa7508b1fde15815b588493b2cf52", "WEAPON_METADATA_PROOF_DIGEST")
    proof = json.loads(proof_raw)
    rows = {target_id(row["target"]): row for row in packet["promotions"]}
    sources = {target_id(row["target"]): row for row in proof["records"]}
    require(packet["schema"] == "OTERYN_ITEM_WEAPON_METADATA_PROMOTION/v1"
            and packet["counts"] == {"fields": 103, "items": 103, "attack_modifier": 78, "absolute_hit_percent": 25}
            and len(rows) == len(packet["promotions"]) == len(sources) == len(proof["records"]) == 103
            and set(rows) == set(sources), "CLOSED_WEAPON_METADATA_SCOPE")
    fields = {"atk_mod": "weapon_attack_modifier_points", "hit_chance": "weapon_absolute_hit_chance_percent"}
    counts = {field: 0 for field in fields}
    for key, row in rows.items():
        source = sources[key]
        require(source["field"] in fields and row["target"] == source["target"]
                and canonical_bytes(row["facts"]) == canonical_bytes({fields[source["field"]]: source["value"]}), "CLOSED_WEAPON_METADATA_VALUE")
        counts[source["field"]] += 1
    require(counts == {"atk_mod": 78, "hit_chance": 25}, "CLOSED_WEAPON_METADATA_FIELDS")
    return rows, proof["current_parent_receipt"]


def extend_weapon_owners(expected):
    rows, receipt = closed_weapon_metadata()
    parent = {target_id(row["item"]): row for row in receipt["parent_authoring"]}
    require(len(parent) == len(receipt["parent_authoring"]) == len(expected) == 316
            and canonical_sorted(list(parent.values())) == canonical_sorted(list(expected.values())), "WEAPON_METADATA_PARENT_OWNERS")
    require(len(set(rows).intersection(parent)) == 8, "WEAPON_METADATA_OWNER_OVERLAP")
    for key, row in rows.items():
        owner = expected.setdefault(key, {"item": row["target"]})
        require(owner["item"] == row["target"], "WEAPON_METADATA_OWNER_IDENTITY")
        for field, value in row["facts"].items():
            require(field not in owner or canonical_bytes(owner[field]) == canonical_bytes(value), "WEAPON_METADATA_OWNER_CONFLICT")
            owner[field] = value
    require(len(expected) == 411, "WEAPON_METADATA_OWNER_COUNT")


def closed_forge289():
    """Exactly289 qualified metadata pairs/bindings; no inferred maximum/default."""
    raw = (ROOT / "docs/agents/evidence/OTV2-20261002-item-forge289-promotion-v1.json").read_bytes()
    require(hashlib.sha256(raw).hexdigest() == "18db9c0067f2e1a9a7091fdf3fd38c1cf56ce81b7807a239f745dd4aea962682", "FORGE289_PACKET_DIGEST")
    packet = json.loads(raw)
    proof_raw = (ROOT / "docs/agents/evidence/OTV2-20261002-item-forge289-source-qualification-v1.json").read_bytes()
    require(hashlib.sha256(proof_raw).hexdigest() == "f03ef849585a067c2b17c94e7c95dac540f0c361758d2ad995cdd2fad517f3ea", "FORGE289_PROOF_DIGEST")
    proof = json.loads(proof_raw)
    rows = {target_id(row["item"]): row for row in packet["promotions"]}
    sources = {target_id(row["qualification"]["target"]): row["qualification"] for row in proof["records"]}
    require(packet["schema"] == "OTERYN_ITEM_FORGE289_SOURCE_IMPORT/v1"
            and len(rows) == len(packet["promotions"]) == len(sources) == 289
            and set(rows) == set(sources), "FORGE289_CLOSED_SCOPE")
    parent = {target_id(row["item"]): row for row in proof["current_parent_authoring"]}
    require(len(parent) == len(proof["current_parent_authoring"]) == 411 and not set(parent).intersection(rows), "FORGE289_PARENT_SCOPE")
    relations = {}
    for key, row in rows.items():
        q = sources[key]
        require(canonical_bytes(row) == canonical_bytes({"item": q["target"], "forge": q["forge"]}), "FORGE289_LITERAL_PAIR")
        slot = q["native_definition"]["semantics"].get("imbuement", {})
        slot = slot.get("value", {}).get("slot_count", {}) if slot.get("state") == "KNOWN" else {}
        refs = [{"relation": "CAPABILITY_GOVERNED_BY", "ruleset": "rulesets/items/exaltation-forge/", "basis": "forge"}]
        if slot.get("state") == "KNOWN" and type(slot.get("value")) is int and slot["value"] >= 1:
            refs.append({"relation": "CAPABILITY_GOVERNED_BY", "ruleset": "rulesets/items/imbuements/", "basis": "imbuement.slot_count>=1"})
        relations[key] = {"source": row["item"], "relations": refs}
    require(sum(len(r["relations"]) == 2 for r in relations.values()) == 138, "FORGE289_EXISTING_NATIVE_SLOT_SCOPE")
    return rows, parent, relations, sources


def extend_forge289_owners(expected):
    rows, parent, _, _ = closed_forge289()
    require(canonical_sorted(list(expected.values())) == canonical_sorted(list(parent.values())), "FORGE289_PARENT_OWNERS")
    expected.update(rows)
    require(len(expected) == 700, "FORGE289_OWNER_COUNT")


def validate_forge289_relation(row, definition, closed, source):
    require(canonical_bytes(row) == canonical_bytes(closed) and row["source"] == definition["identity"], "FORGE289_FULL_RELATION")
    expected = source["native_definition"]["semantics"].get("imbuement", {})
    expected = expected.get("value", {}).get("slot_count", {}) if expected.get("state") == "KNOWN" else {}
    actual = definition["semantics"].get("imbuement", {})
    actual = actual.get("value", {}).get("slot_count", {}) if actual.get("state") == "KNOWN" else {}
    require(canonical_bytes(actual) == canonical_bytes(expected), "FORGE289_CURRENT_NATIVE_SLOT")


def validate_item_authoring_targets(legacy_authoring, staged_items):
    """Retain every admitted cohort plus sealed103 intrinsic weapon properties."""
    raw = (ROOT / "docs/agents/evidence/OTV2-20261002-item-hit-magic-promotion-v1.json").read_bytes()
    require(hashlib.sha256(raw).hexdigest() == "24229adb01ad3abd5f32f74fac8fcecfc0ec1fad5f25778fc253827efc57f574", "HIT_MAGIC_PACKET_DIGEST")
    magic = {target_id(row["target"]): row for row in json.loads(raw)["promotions"]
             if "required_magic_level" in row["facts"]}
    require(len(staged_items) == 164 and len(magic) == 39 and not set(magic).intersection(staged_items), "CLOSED_ML_OWNER_SCOPE")
    expected = {key: {"item": {"family": key[0], "key": key[1], "revision": key[2]}, **row["authoring"]}
                for key, row in staged_items.items()}
    for key, row in magic.items():
        expected[key] = {"item": row["target"], "required_magic_level": row["facts"]["required_magic_level"]}
    raw = (ROOT / "docs/agents/evidence/OTV2-20261002-item-use-observation-promotion-v1.json").read_bytes()
    require(hashlib.sha256(raw).hexdigest() == "adf6082b2aad13c00e6d59fc34437728d150d2b90ff6a61952b5970a4071375b", "USE_OBSERVATION_PACKET_DIGEST")
    packet = json.loads(raw)
    observations = {target_id(row["target"]): row for row in packet["promotions"]}
    require(packet["schema"] == "OTERYN_ITEM_USE_OBSERVATION_PROMOTION/v1"
            and packet["counts"] == {"fields": 345, "items": 139, "by_field": {"damage": 111, "damage_type": 138, "mana_cost": 96}}
            and len(observations) == len(packet["promotions"]) == 139
            and len(set(observations).intersection(magic)) == 27
            and not set(observations).intersection(staged_items), "CLOSED_USE_OWNER_SCOPE")
    for key, row in observations.items():
        owner = expected.setdefault(key, {"item": row["target"]})
        require(owner["item"] == row["target"] and "use_observation" not in owner, "CLOSED_USE_OWNER_IDENTITY")
        owner["use_observation"] = row["facts"]
    forge = closed_forge_owner()
    forge_key = target_id(forge["item"])
    require(len(expected) == 315 and forge_key not in expected, "CLOSED_FORGE_OWNER_SCOPE")
    expected[forge_key] = forge
    extend_weapon_owners(expected)
    extend_forge289_owners(expected)
    require(set(legacy_authoring) == set(expected), "WAVE1_AUTHORING_TARGETS")
    for key, value in expected.items():
        # Canonical bytes distinguish bool from integer and reject extra/partial siblings.
        require(canonical_bytes(legacy_authoring[key]) == canonical_bytes(value), "CLOSED_ITEM_AUTHORING_VALUE")


def validate_forge_relation(row, definition):
    """Existing known three-slot imbuement plus the singleton typed Forge pair."""
    owner = closed_forge_owner()
    require(row["source"] == definition["identity"] == owner["item"], "FORGE_RELATION_TARGET")
    imbuement = definition.get("semantics", {}).get("imbuement", {})
    slot = imbuement.get("value", {}).get("slot_count", {})
    require(imbuement.get("state") == slot.get("state") == "KNOWN"
            and type(slot.get("value")) is int and slot["value"] == 3, "FORGE_RELATION_WITHOUT_EXISTING_SLOT")
    require(row == {"source": owner["item"], "relations": [
        {"relation": "CAPABILITY_GOVERNED_BY", "ruleset": "rulesets/items/exaltation-forge/", "basis": "forge"},
        {"relation": "CAPABILITY_GOVERNED_BY", "ruleset": "rulesets/items/imbuements/", "basis": "imbuement.slot_count>=1"}]}, "FORGE_RELATION_DERIVATION")


def use_relation_targets():
    """The sealed Use owners; only these rows are the PR's Use relations (D322)."""
    raw = (ROOT / "docs/agents/evidence/OTV2-20261002-item-use-observation-promotion-v1.json").read_bytes()
    require(hashlib.sha256(raw).hexdigest() == "adf6082b2aad13c00e6d59fc34437728d150d2b90ff6a61952b5970a4071375b", "USE_OBSERVATION_PACKET_DIGEST")
    return {target_id(p["target"]) for p in json.loads(raw)["promotions"]}


def validate_use_relation(row, definition):
    """Existing known imbuement governance becomes visible with a sealed Use owner."""
    require(target_id(row["source"]) in use_relation_targets() and row["source"] == definition["identity"], "USE_RELATION_TARGET")
    imbuement = definition.get("semantics", {}).get("imbuement", {})
    slot = imbuement.get("value", {}).get("slot_count", {})
    require(imbuement.get("state") == slot.get("state") == "KNOWN"
            and type(slot.get("value")) is int and slot["value"] >= 1, "USE_RELATION_WITHOUT_EXISTING_SLOT")
    require(row == {"source": definition["identity"], "relations": [{"relation": "CAPABILITY_GOVERNED_BY", "ruleset": "rulesets/items/imbuements/", "basis": "imbuement.slot_count>=1"}]}, "USE_RELATION_DERIVATION")


def validate_weapon_relation(row, definition, weapon_rows, admitted_relations):
    """Only the closed47 newly visible references to already-known imbuement slots."""
    key = target_id(row["source"])
    require(key in weapon_rows and key in admitted_relations
            and row["source"] == definition["identity"] == weapon_rows[key]["target"], "WEAPON_RELATION_TARGET")
    imbuement = definition.get("semantics", {}).get("imbuement", {})
    slot = imbuement.get("value", {}).get("slot_count", {})
    require(imbuement.get("state") == slot.get("state") == "KNOWN"
            and type(slot.get("value")) is int and slot["value"] >= 1, "WEAPON_RELATION_WITHOUT_EXISTING_SLOT")
    expected = {"source": definition["identity"], "relations": [{"relation": "CAPABILITY_GOVERNED_BY", "ruleset": "rulesets/items/imbuements/", "basis": "imbuement.slot_count>=1"}]}
    require(canonical_bytes(row) == canonical_bytes(expected) == canonical_bytes(admitted_relations[key]), "WEAPON_RELATION_DERIVATION")


def validate_item_enrichment(reference: Any, declarations: Any, sources: Any, batches: list[Any],
                             migrated_authoring: dict[tuple[str, str, str], dict[str, Any]]) -> tuple[int, int, int, int]:
    """Round-trip Item authoring/taxonomy/relations and prove per-fact provenance."""
    from quest_reward_item_semantics import load_admissions
    reward_admissions = load_admissions(ROOT)
    staged = load(ROOT / "docs/agents/evidence/OTV2-20260925-item-enrichment-wave1-staged.json")
    stats = load(ROOT / "docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json")
    content_path = {"weapon.range_cells": "weapon.range"}
    superseding = {
        (("Item", row["item_key"], "definition-r1"), content_path.get(row["field_path"], row["field_path"])): row["typed_value"]["value"]
        for row in stats["promotions"]
    }
    assignments = load(ROOT / "docs/agents/evidence/OTV2-20260925-tibiawiki-item-master-field-census-v1.json")["family_assignments"]
    authoring_rows = declarations.get("item_authoring", [])
    legacy_authoring = {target_id(row["item"]): row for row in authoring_rows}
    require(len(legacy_authoring) == len(authoring_rows), "DUPLICATE_ITEM_AUTHORING_OWNER")
    taxonomy = load(ROOT / "content/items/taxonomy/items.json")
    relations = load(ROOT / "content/items/relations/items.json")
    facts = load(ROOT / "imports/tibiawiki/facts/items-wave1.json")
    definitions = {target_id(row["identity"]): row for row in reference["records"]}

    rebuilt = {key: dict(value) for key, value in migrated_authoring.items()}
    require(taxonomy["records"] == build_taxonomy(
        definitions, legacy_authoring, assignments, *taxonomy_inputs(ROOT)
    ), "TAXONOMY_SOURCE_COVERAGE")
    for row in taxonomy["records"]:
        key = target_id(row["target"])
        require(key in definitions, "TAXONOMY_TARGET_UNRESOLVED")
        if "source_evidence" in row:
            continue  # Source-qualified navigation supplement; never legacy authoring.
        require(row["family_profile"] == legacy_profile(row["source_taxonomy"]["primary"], assignments), "TAXONOMY_FAMILY_PROFILE")
        require(row["family_profile"] is None or row["family_profile"] in set(assignments.values()), "TAXONOMY_PROFILE_UNKNOWN")
        rebuilt.setdefault(key, {"item": row["target"]})["taxonomy"] = row["source_taxonomy"]
    require(canonical_sorted(list(rebuilt.values())) == canonical_sorted(list(legacy_authoring.values())), "ITEM_AUTHORING_ROUNDTRIP")

    aliases = item_alias_targets()
    staged_items = {staged_item_target(item["target"], aliases): item for item in staged["items"]}
    validate_item_authoring_targets(legacy_authoring, staged_items)
    relation_count = 0
    use_relation_count = 0
    forge_relation_count = 0
    forge_target = target_id(closed_forge_owner()["item"])
    weapon_rows, weapon_receipt = closed_weapon_metadata()
    weapon_relations = {target_id(row["source"]): row for row in weapon_receipt["new_derived_relations"]}
    prior_relations = {target_id(row["source"]): row for row in weapon_receipt["parent_relations"]}
    require(len(weapon_relations) == len(weapon_receipt["new_derived_relations"]) == 47
            and len(prior_relations) == len(weapon_receipt["parent_relations"]) == 203
            and not set(weapon_relations).intersection(prior_relations), "CLOSED_WEAPON_RELATION_SCOPE")
    _, _, forge289_relations, forge289_sources = closed_forge289()
    forge289_relation_count = 0
    weapon_relation_count = 0
    main_only_relation_count = 0
    use_targets = use_relation_targets()
    seen_sources = set()
    expected_relations = {
        key: capability_relations(definition, legacy_authoring.get(key))
        for key, definition in definitions.items() if key[0] == "Item"
    }
    for row in relations["records"]:
        key = target_id(row["source"])
        require(key in definitions and key not in seen_sources, "RELATION_SOURCE_UNRESOLVED")
        seen_sources.add(key)
        rulesets = sorted(relation["ruleset"] for relation in row["relations"])
        require(row["relations"] == expected_relations[key], "RELATION_DERIVATION_DISAGREES")
        if key in forge289_relations:
            validate_forge289_relation(row, definitions[key], forge289_relations[key], forge289_sources[key])
            forge289_relation_count += 1
        elif key in weapon_relations:
            validate_weapon_relation(row, definitions[key], weapon_rows, weapon_relations)
            weapon_relation_count += 1
        elif key in staged_items:
            require(rulesets == staged_items[key]["capability_relations"], "RELATION_DERIVATION_DISAGREES")
            basis = {"rulesets/items/enchanting/": "lifecycle.enchantable=true", "rulesets/items/exaltation-forge/": "forge", "rulesets/items/imbuements/": "imbuement.slot_count>=1"}
            require(row == {"source": definitions[key]["identity"], "relations": [{"relation": "CAPABILITY_GOVERNED_BY", "ruleset": rule, "basis": basis[rule]} for rule in rulesets]}, "WAVE1_RELATION_FULL_VALUE")
        elif key == forge_target:
            validate_forge_relation(row, definitions[key])
            forge_relation_count += 1
        elif key in use_targets:
            validate_use_relation(row, definitions[key])
            use_relation_count += 1
        else:
            # D322: a main-only row keeps main's generic derivation and coverage, checked above.
            main_only_relation_count += 1
        for ruleset in rulesets:
            require((ROOT / ruleset / "index.json").is_file(), f"RELATION_RULESET_UNRESOLVED:{ruleset}")
        relation_count += len(rulesets)
    require(use_relation_count == 55 and forge_relation_count == 1 and weapon_relation_count == 47 and forge289_relation_count == 289
            and sum(bool(item["capability_relations"]) for item in staged["items"]) + use_relation_count + forge_relation_count + weapon_relation_count + forge289_relation_count + main_only_relation_count == len(seen_sources), "RELATION_COVERAGE")
    require(canonical_sorted([row for row in relations["records"] if target_id(row["source"]) in prior_relations])
            == canonical_sorted(list(prior_relations.values())), "WEAPON_PARENT_RELATIONS_CHANGED")
    require({key for key, values in expected_relations.items() if values} == seen_sources, "RELATION_COVERAGE")

    batch = next(row for row in batches if row["batch_id"] == staged["batch_id"])
    require(batch["source_artifact_sha256"] == staged["source"]["snapshot_sha256"] == facts["snapshot_sha256"], "PROVENANCE_BATCH_DIGEST")
    bindings = {(row["target"]["key"], row["external_id"]) for row in sources["source_identity_bindings"]
                if row["source_key"] == staged["source"]["source_key"] and row["disposition"] == "EXACT"}
    canonical_keys = {key for _, key, _ in definitions}
    fact_count = 0
    require(len(facts["records"]) == len(staged["items"]), "PROVENANCE_RECORD_COUNT")
    for record in facts["records"]:
        key = target_id(record["target"])
        require((record["target"]["key"], record["external_id"]) in bindings, "PROVENANCE_WITHOUT_EXACT_BINDING")
        require(record["external_id"] not in canonical_keys and record["batch_id"] == staged["batch_id"], "SOURCE_ID_AS_CANONICAL_ID")
        require(len(record["source_digest"]) == 64 and record["revision_id"] > 0, "PROVENANCE_SOURCE_COORDINATES")
        known = known_paths(definitions[key])
        for entry in record["definition_facts"]:
            # ITEM-SEM-2b: an English TibiaWiki stat row supersedes the Wave 1 value; the
            # definition must then carry the superseding value instead.
            expected = superseding.get((key, entry["field_path"]), entry["value"])
            require(known.get(entry["field_path"]) == expected, f"PROVENANCE_DEFINITION_FACT:{entry['field_path']}")
            fact_count += 1
        for entry in record["authoring_facts"]:
            authoring_value(legacy_authoring[key], entry["field_path"])
            fact_count += 1
        # B3 §4.3 supersedes the old maximum hold only for this digest-verified
        # reward admission's proven stackable items. Other blocked facts stay unknown.
        for blocked in ("stack.stack_max",):
            if blocked in known:
                admitted = reward_admissions.get(key[1])
                require(admitted is not None and admitted[0]["stackable"] and known[blocked] == 100,
                        f"BLOCKED_FIELD_PROMOTED:{blocked}")
        equipment = superseding.get((key, "equipment.patterns"))
        if equipment is None:
            require(definitions[key].get("semantics", {}).get("equipment", {}).get("state", "UNKNOWN") == "UNKNOWN", "BLOCKED_EQUIPMENT_PROMOTED")
        else:
            require(known.get("equipment.patterns") == equipment, "EQUIPMENT_WITHOUT_QUALIFIED_WIKI_PATTERN")
    require(fact_count == staged["counts"]["definition_facts"] + staged["counts"]["authoring_facts"], "PROVENANCE_FACT_COUNT")
    return len(legacy_authoring), len(taxonomy["records"]), relation_count, fact_count


def creature_family_counts(reference: Any) -> dict[str, int]:
    counts = {family: sum(row['identity']['family'] == family for row in reference['records'])
              for family in CREATURE_FAMILY_NODES}
    require(counts['Creature'] == 1763, 'LEGACY_CREATURE_COUNT')
    return counts


def validate_creature_families(reference: Any, declarations: Any, sources: Any) -> tuple[int, int, int]:
    """Round-trip the admitted creature families, their authoring profiles and source bindings."""
    family_counts = creature_family_counts(reference)
    legacy_profiles = {target_id(row["target"]): row["data"] for row in declarations.get("authoring_profiles", [])
                       if row["target"]["family"] != "Encounter"}
    migrated_profiles: dict[tuple[str, str, str], Any] = {}
    migrated_bindings: list[Any] = []
    for family, node in CREATURE_FAMILY_NODES.items():
        legacy = [row for row in reference["records"] if row["identity"]["family"] == family]
        require(len(legacy) == family_counts[family], f"LEGACY_{family.upper()}_COUNT")
        index = load(ROOT / node / "index.json")
        require(index["schema"] == "OTERYN_FAMILY_INDEX/v1" and index["family"] == family, f"{family.upper()}_INDEX")
        require(index["record_count"] == len(legacy), f"{family.upper()}_INDEX_COUNT")
        migrated: list[Any] = []
        expected_start = 0
        for shard_path in index["shards"]:
            require(isinstance(shard_path, str) and shard_path.startswith(node), f"{family.upper()}_SHARD_REF")
            payload = load(ROOT / shard_path)
            require(payload["family"] == family and payload["shard"]["start"] == expected_start, f"{family.upper()}_SHARD_GAP")
            require(payload["shard"]["count"] == len(payload["records"]), f"{family.upper()}_SHARD_COUNT")
            for row in payload["records"]:
                definition = row["definition"]
                migrated.append(definition)
                if "authoring" in row:
                    migrated_profiles[target_id(definition["identity"])] = row["authoring"]
                for binding in row.get("source_bindings", []):
                    require(target_id(binding["target"]) == target_id(definition["identity"]), f"{family.upper()}_BINDING_TARGET")
                    migrated_bindings.append(binding)
            expected_start = payload["shard"]["end"] + 1
        require(migrated == legacy, f"{family.upper()}_DEFINITION_ROUNDTRIP")
    require(migrated_profiles == legacy_profiles, "CREATURE_AUTHORING_ROUNDTRIP")
    legacy_bindings = [row for row in sources["source_identity_bindings"] if row["target"]["family"] in CREATURE_FAMILY_NODES]
    require(canonical_sorted(migrated_bindings) == canonical_sorted(legacy_bindings), "CREATURE_BINDING_ROUNDTRIP")
    require(len(legacy_bindings) == family_counts["Creature"], "CREATURE_BINDING_COUNT")
    canary_creatures = load(ROOT / "imports/canary/bindings/creatures.json")["bindings"]
    wiki_creatures = load(ROOT / "imports/tibiawiki/bindings/creatures.json")["bindings"]
    crystal_creatures = load(ROOT / "imports/crystalserver/bindings/creatures.json")["bindings"]
    require(all(row["source_key"] == "oteryn:source.canary" for row in canary_creatures)
            and all(row["source_key"] == "oteryn:source.tibiawiki" for row in wiki_creatures)
            and all(row["source_key"] == "oteryn:source.crystalserver" for row in crystal_creatures)
            and canonical_sorted(canary_creatures + wiki_creatures + crystal_creatures) == canonical_sorted(legacy_bindings),
            "IMPORT_CREATURE_BINDINGS")
    require(load(ROOT / "imports/canary/index.json")["population_state"] == "POPULATED", "IMPORT_CANARY_MARKER_STATE")
    return sum(family_counts.values()), len(migrated_profiles), len(migrated_bindings)


def validate_npc_services(declarations: Any, sources: Any) -> tuple[int, int, int]:
    """Round-trip the admitted NPC declarations (with source bindings) and Service declarations."""
    legacy_npcs = [row for row in declarations["records"] if row.get("kind") == "NPC"]
    require(len(legacy_npcs) == NPC_COUNT, "LEGACY_NPC_COUNT")
    legacy_npc_bindings = [row for row in sources["source_identity_bindings"] if row["target"]["family"] == "NPC"]
    require(len(legacy_npc_bindings) == NPC_BINDING_COUNT, "LEGACY_NPC_BINDING_COUNT")

    npc_index = load(ROOT / "content/npcs/definitions/index.json")
    require(npc_index["schema"] == "OTERYN_FAMILY_INDEX/v1" and npc_index["family"] == "NPC", "NPC_INDEX")
    require(npc_index["record_count"] == len(legacy_npcs), "NPC_INDEX_COUNT")
    migrated_npcs: list[Any] = []
    migrated_npc_bindings: list[Any] = []
    expected_start = 0
    for shard_path in npc_index["shards"]:
        require(isinstance(shard_path, str) and shard_path.startswith("content/npcs/definitions/"), "NPC_SHARD_REF")
        payload = load(ROOT / shard_path)
        require(payload["family"] == "NPC" and payload["shard"]["start"] == expected_start, "NPC_SHARD_GAP")
        require(payload["shard"]["count"] == len(payload["records"]), "NPC_SHARD_COUNT")
        for row in payload["records"]:
            declaration = row["declaration"]
            migrated_npcs.append(declaration)
            target = {"family": "NPC", "key": declaration["identity"]["key"], "revision": declaration["identity"]["revision"]}
            for binding in row.get("source_bindings", []):
                require(target_id(binding["target"]) == target_id(target), "NPC_BINDING_TARGET")
                migrated_npc_bindings.append(binding)
        expected_start = payload["shard"]["end"] + 1
    require(expected_start == NPC_COUNT, "NPC_SHARD_COVERAGE")
    require(migrated_npcs == legacy_npcs, "NPC_DECLARATION_ROUNDTRIP")
    require(canonical_sorted(migrated_npc_bindings) == canonical_sorted(legacy_npc_bindings), "NPC_BINDING_ROUNDTRIP")
    require(npc_index["attached_source_bindings"] == len(migrated_npc_bindings), "NPC_INDEX_BINDING_COUNT")
    require(len({(row["identity"]["key"], row["identity"]["revision"]) for row in migrated_npcs}) == NPC_COUNT, "NPC_IDENTITY_UNIQUENESS")

    legacy_services = [row for row in declarations["records"] if row.get("kind") == "Service"]
    require(len(legacy_services) == sum(SERVICE_FAMILY_COUNTS.values()), "LEGACY_SERVICE_COUNT")
    migrated_service_count = 0
    for family, count in SERVICE_FAMILY_COUNTS.items():
        node, field = SERVICE_FAMILY_NODES[family]
        stem = "trade" if family == "Service.Trade" else "travel"
        legacy = [row for row in legacy_services
                  if field in row or row["identity"]["key"].startswith(f"oteryn:service.{stem}.")]
        require(len(legacy) == count, f"LEGACY_{family.upper()}_COUNT")
        index = load(ROOT / node / "index.json")
        require(index["schema"] == "OTERYN_FAMILY_INDEX/v1" and index["family"] == family, f"{family.upper()}_INDEX")
        require(index["record_count"] == len(legacy), f"{family.upper()}_INDEX_COUNT")
        migrated: list[Any] = []
        expected_start = 0
        for shard_path in index["shards"]:
            require(isinstance(shard_path, str) and shard_path.startswith(node), f"{family.upper()}_SHARD_REF")
            payload = load(ROOT / shard_path)
            require(payload["family"] == family and payload["shard"]["start"] == expected_start, f"{family.upper()}_SHARD_GAP")
            require(payload["shard"]["count"] == len(payload["records"]), f"{family.upper()}_SHARD_COUNT")
            for row in payload["records"]:
                require(set(row) == {"declaration"}, f"{family.upper()}_ROW_SHAPE")
                migrated.append(row["declaration"])
            expected_start = payload["shard"]["end"] + 1
        require(expected_start == count, f"{family.upper()}_SHARD_COVERAGE")
        require(migrated == legacy, f"{family.upper()}_DECLARATION_ROUNDTRIP")
        migrated_service_count += len(migrated)
    require(migrated_service_count == len(legacy_services), "SERVICE_ROUNDTRIP_TOTAL")
    return len(migrated_npcs), len(migrated_npc_bindings), migrated_service_count


def validate_encounters(declarations: Any, sources: Any) -> int:
    """Round-trip the admitted Encounter declarations with their authoring profiles and source bindings."""
    legacy = [row for row in declarations["records"] if row.get("kind") == "Encounter"]
    require(len(legacy) == ENCOUNTER_COUNT, "LEGACY_ENCOUNTER_COUNT")
    legacy_profiles = {target_id(row["target"]): row["data"] for row in declarations.get("authoring_profiles", [])
                       if row["target"]["family"] == "Encounter"}
    legacy_bindings = [row for row in sources["source_identity_bindings"] if row["target"]["family"] == "Encounter"]
    require(len(legacy_profiles) == len(legacy_bindings) == ENCOUNTER_COUNT, "LEGACY_ENCOUNTER_ATTACHMENTS")
    index = load(ROOT / "content/encounters/definitions/index.json")
    require(index["schema"] == "OTERYN_FAMILY_INDEX/v1" and index["family"] == "Encounter", "ENCOUNTER_INDEX")
    require(index["record_count"] == len(legacy), "ENCOUNTER_INDEX_COUNT")
    migrated: list[Any] = []
    migrated_profiles: dict[tuple[str, str, str], Any] = {}
    migrated_bindings: list[Any] = []
    expected_start = 0
    for shard_path in index["shards"]:
        require(isinstance(shard_path, str) and shard_path.startswith("content/encounters/definitions/"), "ENCOUNTER_SHARD_REF")
        payload = load(ROOT / shard_path)
        require(payload["family"] == "Encounter" and payload["shard"]["start"] == expected_start, "ENCOUNTER_SHARD_GAP")
        require(payload["shard"]["count"] == len(payload["records"]), "ENCOUNTER_SHARD_COUNT")
        for row in payload["records"]:
            declaration = row["declaration"]
            migrated.append(declaration)
            target = ("Encounter", declaration["identity"]["key"], declaration["identity"]["revision"])
            if "authoring" in row:
                migrated_profiles[target] = row["authoring"]
            for binding in row.get("source_bindings", []):
                require(target_id(binding["target"]) == target, "ENCOUNTER_BINDING_TARGET")
                migrated_bindings.append(binding)
        expected_start = payload["shard"]["end"] + 1
    require(expected_start == ENCOUNTER_COUNT, "ENCOUNTER_SHARD_COVERAGE")
    require(migrated == legacy, "ENCOUNTER_DECLARATION_ROUNDTRIP")
    require(migrated_profiles == legacy_profiles, "ENCOUNTER_AUTHORING_ROUNDTRIP")
    require(canonical_sorted(migrated_bindings) == canonical_sorted(legacy_bindings), "ENCOUNTER_BINDING_ROUNDTRIP")
    return len(migrated)


def validate_dialogue(declarations: Any) -> int:
    """Round-trip the admitted Dialogue declarations (no source bindings exist for Dialogue)."""
    legacy_dialogues = [row for row in declarations["records"] if row.get("kind") == "Dialogue"]
    require(len(legacy_dialogues) == DIALOGUE_COUNT, "LEGACY_DIALOGUE_COUNT")

    dialogue_index = load(ROOT / "content/dialogues/definitions/index.json")
    require(dialogue_index["schema"] == "OTERYN_FAMILY_INDEX/v1" and dialogue_index["family"] == "Dialogue", "DIALOGUE_INDEX")
    require(dialogue_index["record_count"] == len(legacy_dialogues), "DIALOGUE_INDEX_COUNT")
    migrated_dialogues: list[Any] = []
    expected_start = 0
    for shard_path in dialogue_index["shards"]:
        require(isinstance(shard_path, str) and shard_path.startswith("content/dialogues/definitions/"), "DIALOGUE_SHARD_REF")
        payload = load(ROOT / shard_path)
        require(payload["family"] == "Dialogue" and payload["shard"]["start"] == expected_start, "DIALOGUE_SHARD_GAP")
        require(payload["shard"]["count"] == len(payload["records"]), "DIALOGUE_SHARD_COUNT")
        for row in payload["records"]:
            require(set(row) == {"declaration"}, "DIALOGUE_ROW_SHAPE")
            migrated_dialogues.append(row["declaration"])
        expected_start = payload["shard"]["end"] + 1
    require(expected_start == DIALOGUE_COUNT, "DIALOGUE_SHARD_COVERAGE")
    require(migrated_dialogues == legacy_dialogues, "DIALOGUE_DECLARATION_ROUNDTRIP")
    require(len({(row["identity"]["key"], row["identity"]["revision"]) for row in migrated_dialogues}) == DIALOGUE_COUNT,
            "DIALOGUE_IDENTITY_UNIQUENESS")
    return len(migrated_dialogues)


def validate_document_rows(rows: list[Any], declarations: Any, sources: Any) -> int:
    legacy = [row for row in declarations['records'] if row.get('kind') == 'Document']
    require(len(legacy) == DOCUMENT_COUNT, 'LEGACY_DOCUMENT_COUNT')
    require(all(set(row) == {'declaration'} for row in rows), 'DOCUMENT_ROW_SHAPE')
    migrated = [row['declaration'] for row in rows]
    require(migrated == legacy, 'DOCUMENT_DECLARATION_ROUNDTRIP')
    identities = {('Document', row['identity']['key'], row['identity']['revision']) for row in migrated}
    require(len(identities) == DOCUMENT_COUNT, 'DOCUMENT_IDENTITY_UNIQUENESS')
    require(not any(row['target']['family'] == 'Document' for row in sources['source_identity_bindings']),
            'DOCUMENT_SOURCE_BINDING_FABRICATED')
    for row in declarations.get('authoring_profiles', []):
        if row['data']['kind'] != 'Creature':
            continue
        ref = row['data']['profile'].get('details', {}).get('encyclopedia_document')
        if ref is not None:
            require(target_id(ref) in identities, 'CREATURE_DOCUMENT_REFERENCE_UNRESOLVED')
    return len(migrated)


def validate_documents(declarations: Any, sources: Any) -> int:
    index = load(ROOT / 'content/documents/index.json')
    require(index['schema'] == 'OTERYN_FAMILY_INDEX/v1' and index['family'] == 'Document', 'DOCUMENT_INDEX')
    require(index['record_count'] == DOCUMENT_COUNT, 'DOCUMENT_INDEX_COUNT')
    rows, expected_start = [], 0
    for path in index['shards']:
        require(isinstance(path, str) and path.startswith('content/documents/') and '..' not in path.split('/'), 'DOCUMENT_SHARD_REF')
        shard = load(ROOT / path)
        require(shard['schema'] == 'OTERYN_DOCUMENT_AUTHORING_SHARD/v1' and shard['family'] == 'Document', 'DOCUMENT_SHARD_SCHEMA')
        require(shard['shard']['start'] == expected_start, 'DOCUMENT_SHARD_GAP')
        require(shard['shard']['count'] == len(shard['records']), 'DOCUMENT_SHARD_COUNT')
        require(shard['shard']['end'] == expected_start + len(shard['records']) - 1, 'DOCUMENT_SHARD_END')
        rows.extend(shard['records']); expected_start = shard['shard']['end'] + 1
    require(expected_start == DOCUMENT_COUNT, 'DOCUMENT_SHARD_COVERAGE')
    return validate_document_rows(rows, declarations, sources)


def main() -> int:
    from register_spell_families import outputs as spell_outputs
    for relative, expected in spell_outputs(ROOT).items():
        require((ROOT / relative).is_file() and (ROOT / relative).read_bytes() == expected,
                f"SPELL_IMPORT_DRIFT:{relative}")
    reference = load(LEGACY / "definitions" / "reference.json")
    # Equivalence is protected legacy plus the accepted tree-first reward Item packet.
    from quest_reward_item_semantics import apply_admissions
    apply_admissions([row for row in reference["records"] if row["identity"]["family"] == "Item"], ROOT)
    declarations = load(LEGACY / "definitions" / "declarations.json")
    legacy_mount_declarations = [row for row in declarations["records"] if row.get("kind") == "Mount"]
    require(len(legacy_mount_declarations) == 252, "LEGACY_MOUNT_COUNT")
    editor = load(LEGACY / "editor" / "author.json")
    sources = load(LEGACY / "provenance" / "sources.json")
    project = load(ROOT / "content" / "project.json")
    manifest = load(ROOT / "content" / "manifest.json")
    lock = load(ROOT / "content" / "content.lock.json")
    item_index = load(ROOT / "content" / "items" / "index.json")
    mount_index = load(ROOT / "content" / "cosmetics" / "mounts" / "index.json")

    require(project["runtime_source"] == "legacy_until_separately_qualified", "RUNTIME_SWITCHED_EARLY")
    require(manifest["compatibility"] == {
        "legacy_root": "content/world",
        "legacy_mutated": False,
        "runtime_switch_authorized": False,
    }, "COMPATIBILITY_BOUNDARY")
    from world_project_v2_to_tree import retained_quest_registration
    quest_families, quest_paths = retained_quest_registration(ROOT)
    if quest_families:
        require("Quest" in project["migrated_families"] and "Quest" not in project["next_population_families"], "QUEST_PROJECT_REGISTRATION")
        require(set(quest_paths).issubset({row["path"] for row in manifest["managed_files"]}), "QUEST_MANAGED_FILES")
    require(lock["family_counts"] == {"Item": 34033, "Mount": 252, **creature_family_counts(reference), "NPC": NPC_COUNT,
                                       "Encounter": ENCOUNTER_COUNT, "Dialogue": DIALOGUE_COUNT, "Document": DOCUMENT_COUNT, **SERVICE_FAMILY_COUNTS,
                                       "Charm": CHARM_COUNT, "Proficiency": PROFICIENCY_COUNT,
                                       "RewardClaim": REWARD_CLAIM_COUNT, "StarterKit": STARTER_KIT_COUNT,
                                       **{family: value["records"] for family, value in quest_families.items()}},
            "LOCK_COUNTS")
    require(lock["source_binding_counts"]["NPC"] == NPC_BINDING_COUNT, "LOCK_NPC_BINDING_COUNT")
    require(item_index["record_count"] == 34033 and len(item_index["shards"]) == 69, "ITEM_INDEX")
    require(mount_index["record_count"] == 252 and len(mount_index["shards"]) == 1, "MOUNT_INDEX")

    migrated_items: list[Any] = []
    migrated_authoring: dict[tuple[str, str, str], dict[str, Any]] = {}
    item_editors: list[Any] = []
    item_bindings: list[Any] = []
    expected_start = 0
    for shard_path in item_index["shards"]:
        require(isinstance(shard_path, str), "ITEM_SHARD_REF")
        payload = load(ROOT / shard_path)
        require(payload["shard"]["start"] == expected_start, "ITEM_SHARD_GAP")
        require(payload["shard"]["count"] == len(payload["records"]), "ITEM_SHARD_COUNT")
        for row in payload["records"]:
            definition = row["definition"]
            migrated_items.append(definition)
            if "editor" in row:
                require(target_id(row["editor"]["target"]) == target_id(definition["identity"]), "ITEM_EDITOR_TARGET")
                item_editors.append(row["editor"])
            for binding in row.get("source_bindings", []):
                require(target_id(binding["target"]) == target_id(definition["identity"]), "ITEM_BINDING_TARGET")
                item_bindings.append(binding)
            if "authoring" in row:
                require("item" not in row["authoring"] and "taxonomy" not in row["authoring"], "ITEM_AUTHORING_ROW_SHAPE")
                migrated_authoring[target_id(definition["identity"])] = {"item": definition["identity"], **row["authoring"]}
        expected_start = payload["shard"]["end"] + 1

    legacy_items = [row for row in reference["records"] if row["identity"]["family"] == "Item"]
    require(migrated_items == legacy_items and expected_start == 34033, "ITEM_DEFINITION_ROUNDTRIP")

    require(isinstance(mount_index["shards"][0], str), "MOUNT_SHARD_REF")
    mount_payload = load(ROOT / mount_index["shards"][0])
    migrated_mounts = [row["declaration"] for row in mount_payload["records"]]
    require(migrated_mounts == legacy_mount_declarations, "MOUNT_DECLARATION_ROUNDTRIP")
    mount_editors: list[Any] = []
    mount_bindings: list[Any] = []
    for row in mount_payload["records"]:
        declaration = row["declaration"]
        expected_target = {
            "family": "Mount",
            "key": declaration["identity"]["key"],
            "revision": declaration["identity"]["revision"],
        }
        if "editor" in row:
            require(target_id(row["editor"]["target"]) == target_id(expected_target), "MOUNT_EDITOR_TARGET")
            mount_editors.append(row["editor"])
        for binding in row.get("source_bindings", []):
            require(target_id(binding["target"]) == target_id(expected_target), "MOUNT_BINDING_TARGET")
            mount_bindings.append(binding)

    legacy_item_editors = [row for row in editor["entries"] if row["target"]["family"] == "Item"]
    legacy_mount_editors = [row for row in editor["entries"] if row["target"]["family"] == "Mount"]
    legacy_item_bindings = [row for row in sources["source_identity_bindings"] if row["target"]["family"] == "Item"]
    legacy_mount_bindings = [row for row in sources["source_identity_bindings"] if row["target"]["family"] == "Mount"]

    require(canonical_sorted(item_editors) == canonical_sorted(legacy_item_editors), "ITEM_EDITOR_ROUNDTRIP")
    require(canonical_sorted(mount_editors) == canonical_sorted(legacy_mount_editors), "MOUNT_EDITOR_ROUNDTRIP")
    require(canonical_sorted(item_bindings) == canonical_sorted(legacy_item_bindings), "ITEM_BINDING_ROUNDTRIP")
    require(canonical_sorted(mount_bindings) == canonical_sorted(legacy_mount_bindings), "MOUNT_BINDING_ROUNDTRIP")

    require(len({target_id(row["identity"]) for row in migrated_items}) == 34033, "ITEM_IDENTITY_UNIQUENESS")
    require(len({
        ("Mount", row["identity"]["key"], row["identity"]["revision"])
        for row in migrated_mounts
    }) == 252, "MOUNT_IDENTITY_UNIQUENESS")

    require(load(ROOT / "imports/tibiawiki/bindings/items.json")["bindings"] == legacy_item_bindings, "IMPORT_ITEM_BINDINGS")
    require(load(ROOT / "imports/tibiawiki/bindings/mounts.json")["bindings"] == legacy_mount_bindings, "IMPORT_MOUNT_BINDINGS")

    authoring_count, taxonomy_count, relation_count, fact_count = validate_item_enrichment(
        reference, declarations, sources, imports_batches(), migrated_authoring)
    creature_records, creature_profiles, creature_bindings = validate_creature_families(reference, declarations, sources)
    npc_records, npc_bindings, service_records = validate_npc_services(declarations, sources)
    document_records = validate_documents(declarations, sources)
    require(manifest["families"]["Document"] == {"records": document_records, "index": "content/documents/index.json"}, "DOCUMENT_MANIFEST")
    require("Document" in project["migrated_families"], "DOCUMENT_PROJECT_FAMILY")
    dialogue_records = validate_dialogue(declarations)
    encounter_records = validate_encounters(declarations, sources)
    charm_index = load(ROOT / "content" / "charms" / "index.json")
    require(charm_index["schema"] == "OTERYN_FAMILY_INDEX/v1" and charm_index["family"] == "Charm", "CHARM_INDEX")
    require(charm_index["record_count"] == CHARM_COUNT and len(charm_index["shards"]) == 1, "CHARM_INDEX_COUNT")
    charm_shard = load(ROOT / charm_index["shards"][0])
    require(charm_shard["family"] == "Charm" and charm_shard["shard"]["count"] == len(charm_shard["records"]) == CHARM_COUNT, "CHARM_SHARD")
    require(len({row["definition"]["identity"]["key"] for row in charm_shard["records"]}) == CHARM_COUNT, "CHARM_IDENTITY_UNIQUENESS")
    proficiency_index = load(ROOT / "content" / "proficiencies" / "index.json")
    require(proficiency_index["schema"] == "OTERYN_FAMILY_INDEX/v1" and proficiency_index["family"] == "Proficiency",
            "PROFICIENCY_INDEX")
    require(proficiency_index["record_count"] == PROFICIENCY_COUNT, "PROFICIENCY_INDEX_COUNT")
    proficiency_keys = set()
    proficiency_refs = set()
    for shard_path in proficiency_index["shards"]:
        shard = load(ROOT / shard_path)
        require(shard["family"] == "Proficiency" and shard["shard"]["count"] == len(shard["records"]), "PROFICIENCY_SHARD")
        proficiency_keys |= {row["definition"]["identity"]["key"] for row in shard["records"]}
        proficiency_refs |= {("Proficiency", row["definition"]["identity"]["key"],
                              row["definition"]["identity"]["revision"]) for row in shard["records"]}
    require(len(proficiency_keys) == PROFICIENCY_COUNT, "PROFICIENCY_IDENTITY_UNIQUENESS")
    proficiency_bindings = load(ROOT / "content" / "proficiencies" / "bindings.json")
    binding_rows = proficiency_bindings["records"]
    require(proficiency_bindings["record_count"] == len(binding_rows) == PROFICIENCY_BINDING_COUNT,
            "PROFICIENCY_BINDING_COUNT")
    item_refs = {("Item", definition["identity"]["key"], definition["identity"]["revision"])
                 for definition in migrated_items}

    def ref(value):
        return (value["family"], value["key"], value["revision"])

    require(len({row["item"]["key"] for row in binding_rows}) == len(binding_rows), "PROFICIENCY_BINDING_UNIQUENESS")
    require(all(ref(row["item"]) in item_refs and ref(row["profile_binding"]) in proficiency_refs
                and row["threshold_class"] in {"standard", "knight", "crossbow"} for row in binding_rows),
            "PROFICIENCY_BINDING_REFERENCES")
    # Completeness against the pinned ITEM-PROF-1 source: every source weapon whose Item is
    # defined and whose class is known is bound, so a newly defined Item cannot stay unbound.
    binding_source_path = ROOT / proficiency_bindings["source"]["path"]
    require(hashlib.sha256(binding_source_path.read_bytes()).hexdigest() == proficiency_bindings["source"]["sha256"],
            "PROFICIENCY_BINDING_SOURCE_DIGEST")
    defined_item_keys = {key for _, key, _ in item_refs}
    expected_bindings = {}
    expected_excluded = {"item_not_defined": 0, "unknown_threshold_class": 0}
    for row in load(binding_source_path)["bindings"]:
        if row["item_key"] not in defined_item_keys:
            expected_excluded["item_not_defined"] += 1
        elif row["threshold_class"] not in {"standard", "knight", "crossbow"}:
            expected_excluded["unknown_threshold_class"] += 1
        else:
            expected_bindings[row["item_key"]] = (
                f"oteryn:proficiency.tibia.p{row['proficiency_id']}", row["threshold_class"])
    require({row["item"]["key"]: (row["profile_binding"]["key"], row["threshold_class"]) for row in binding_rows}
            == expected_bindings and proficiency_bindings["excluded"] == expected_excluded,
            "PROFICIENCY_BINDING_COMPLETENESS")
    reward_claim_index = load(ROOT / "content" / "interactions" / "reward_claims" / "index.json")
    require(reward_claim_index["schema"] == "OTERYN_FAMILY_INDEX/v1" and reward_claim_index["family"] == "RewardClaim",
            "REWARD_CLAIM_INDEX")
    require(reward_claim_index["record_count"] == REWARD_CLAIM_COUNT, "REWARD_CLAIM_INDEX_COUNT")
    reward_claim_keys = set()
    for shard_path in reward_claim_index["shards"]:
        shard = load(ROOT / shard_path)
        require(shard["family"] == "RewardClaim" and shard["shard"]["count"] == len(shard["records"]), "REWARD_CLAIM_SHARD")
        reward_claim_keys |= {row["definition"]["identity"]["key"] for row in shard["records"]}
    require(len(reward_claim_keys) == REWARD_CLAIM_COUNT, "REWARD_CLAIM_IDENTITY_UNIQUENESS")
    starter_kit_index = load(ROOT / "content" / "starter" / "index.json")
    require(starter_kit_index["schema"] == "OTERYN_FAMILY_INDEX/v1" and starter_kit_index["family"] == "StarterKit",
            "STARTER_KIT_INDEX")
    require(starter_kit_index["record_count"] == STARTER_KIT_COUNT, "STARTER_KIT_INDEX_COUNT")
    starter_kit_keys = set()
    for shard_path in starter_kit_index["shards"]:
        shard = load(ROOT / shard_path)
        require(shard["family"] == "StarterKit" and shard["shard"]["count"] == len(shard["records"]), "STARTER_KIT_SHARD")
        starter_kit_keys |= {(row["definition"]["template"], row["definition"]["identity"]["key"]) for row in shard["records"]}
    require(len(starter_kit_keys) == STARTER_KIT_COUNT, "STARTER_KIT_IDENTITY_UNIQUENESS")
    print(
        "PASS items=34033 mounts=252 item_editors=165 mount_editors=252 item_bindings=165 mount_bindings=252 "
        f"item_authoring={authoring_count} taxonomy={taxonomy_count} relations={relation_count} provenance_facts={fact_count} "
        f"creature_records={creature_records} creature_profiles={creature_profiles} creature_bindings={creature_bindings} "
        f"npc_records={npc_records} npc_bindings={npc_bindings} service_records={service_records} dialogue_records={dialogue_records} "
        f"encounter_records={encounter_records} charm_records={CHARM_COUNT} "
        f"proficiency_records={PROFICIENCY_COUNT} proficiency_bindings={PROFICIENCY_BINDING_COUNT} "
        f"reward_claim_records={REWARD_CLAIM_COUNT} starter_kit_records={STARTER_KIT_COUNT}"
    )
    subprocess.run([sys.executable,
                    str(ROOT / "tools/content-schema/imbuement-authoring/imbuement_content.py"),
                    "content", "--check"], cwd=ROOT, check=True)
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
