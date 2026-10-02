#!/usr/bin/env python3
"""Validate exact semantic equivalence of the successor Item/Mount/creature tree."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
LEGACY = ROOT / "content" / "world"
# Canary creature admission wave A (OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1 §7).
CREATURE_FAMILY_COUNTS = {
    "Creature": 1503, "Presentation": 2613, "Behavior": 2613, "Loot": 1056, "Ability": 6000, "Effect": 4599, "Formula": 4905,
}
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
NPC_COUNT = 1110
NPC_BINDING_COUNT = 2376
# Encounter admission (OTERYN_WORLD_PROJECT_V2_ENCOUNTER_ADMISSION_V1 E1-E5).
ENCOUNTER_COUNT = 61
DIALOGUE_COUNT = 694
# Charm is a static family with no legacy source (tools/content-schema/charm-authoring).
CHARM_COUNT = 25
# Proficiency likewise (tools/content-schema/proficiency-authoring).
PROFICIENCY_COUNT = 443
PROFICIENCY_BINDING_COUNT = 664  # 642 + 22 bound by the ITEM-ADD-1 donor epoch-2 Items
# RewardClaim likewise (tools/content-schema/reward-claim-authoring).
REWARD_CLAIM_COUNT = 231
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


def validate_item_authoring_targets(legacy_authoring, staged_items):
    """Accept exactly Wave1, sealed ML39 and sealed source-observation139 owners."""
    raw = (ROOT / "docs/agents/evidence/OTV2-20261002-item-hit-magic-promotion-v1.json").read_bytes()
    require(hashlib.sha256(raw).hexdigest() == "8ec1f103c874e4424e7636743e78a3c57821e1cbebef6a69dd1c9a448cd47660", "HIT_MAGIC_PACKET_DIGEST")
    magic = {target_id(row["target"]): row for row in json.loads(raw)["promotions"]
             if "required_magic_level" in row["facts"]}
    require(len(staged_items) == 164 and len(magic) == 39 and not set(magic).intersection(staged_items), "CLOSED_ML_OWNER_SCOPE")
    expected = {key: {"item": {"family": key[0], "key": key[1], "revision": key[2]}, **row["authoring"]}
                for key, row in staged_items.items()}
    for key, row in magic.items():
        expected[key] = {"item": row["target"], "required_magic_level": row["facts"]["required_magic_level"]}
    raw = (ROOT / "docs/agents/evidence/OTV2-20261002-item-use-observation-promotion-v1.json").read_bytes()
    require(hashlib.sha256(raw).hexdigest() == "990eca66d4d34156aaa7a2fd5c42251e26bdf503402f3d46d6beb6f7b2de99eb", "USE_OBSERVATION_PACKET_DIGEST")
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
    require(len(expected) == 315 and set(legacy_authoring) == set(expected), "WAVE1_AUTHORING_TARGETS")
    for key, value in expected.items():
        # Canonical bytes distinguish bool from integer and reject extra/partial siblings.
        require(canonical_bytes(legacy_authoring[key]) == canonical_bytes(value), "CLOSED_ITEM_AUTHORING_VALUE")


def validate_use_relation(row, definition):
    """Existing known imbuement governance becomes visible with a sealed Use owner."""
    raw = (ROOT / "docs/agents/evidence/OTV2-20261002-item-use-observation-promotion-v1.json").read_bytes()
    require(hashlib.sha256(raw).hexdigest() == "990eca66d4d34156aaa7a2fd5c42251e26bdf503402f3d46d6beb6f7b2de99eb", "USE_OBSERVATION_PACKET_DIGEST")
    targets = {target_id(p["target"]) for p in json.loads(raw)["promotions"]}
    require(target_id(row["source"]) in targets and row["source"] == definition["identity"], "USE_RELATION_TARGET")
    imbuement = definition.get("semantics", {}).get("imbuement", {})
    slot = imbuement.get("value", {}).get("slot_count", {})
    require(imbuement.get("state") == slot.get("state") == "KNOWN"
            and type(slot.get("value")) is int and slot["value"] >= 1, "USE_RELATION_WITHOUT_EXISTING_SLOT")
    require(row == {"source": definition["identity"], "relations": [{"relation": "CAPABILITY_GOVERNED_BY", "ruleset": "rulesets/items/imbuements/", "basis": "imbuement.slot_count>=1"}]}, "USE_RELATION_DERIVATION")


def validate_item_enrichment(reference: Any, declarations: Any, sources: Any, batches: list[Any],
                             migrated_authoring: dict[tuple[str, str, str], dict[str, Any]]) -> tuple[int, int, int, int]:
    """Round-trip Item authoring/taxonomy/relations and prove per-fact provenance."""
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
    for row in taxonomy["records"]:
        key = target_id(row["target"])
        require(key in definitions, "TAXONOMY_TARGET_UNRESOLVED")
        require(row["family_profile"] == assignments.get(row["source_taxonomy"]["primary"]), "TAXONOMY_FAMILY_PROFILE")
        require(row["family_profile"] is None or row["family_profile"] in set(assignments.values()), "TAXONOMY_PROFILE_UNKNOWN")
        rebuilt.setdefault(key, {"item": row["target"]})["taxonomy"] = row["source_taxonomy"]
    require(canonical_sorted(list(rebuilt.values())) == canonical_sorted(list(legacy_authoring.values())), "ITEM_AUTHORING_ROUNDTRIP")

    aliases = item_alias_targets()
    staged_items = {staged_item_target(item["target"], aliases): item for item in staged["items"]}
    validate_item_authoring_targets(legacy_authoring, staged_items)
    relation_count = 0
    use_relation_count = 0
    seen_sources = set()
    for row in relations["records"]:
        key = target_id(row["source"])
        require(key in definitions and key not in seen_sources, "RELATION_SOURCE_UNRESOLVED")
        seen_sources.add(key)
        rulesets = sorted(relation["ruleset"] for relation in row["relations"])
        if key in staged_items:
            require(rulesets == staged_items[key]["capability_relations"], "RELATION_DERIVATION_DISAGREES")
            basis = {"rulesets/items/enchanting/": "lifecycle.enchantable=true", "rulesets/items/exaltation-forge/": "forge", "rulesets/items/imbuements/": "imbuement.slot_count>=1"}
            require(row == {"source": definitions[key]["identity"], "relations": [{"relation": "CAPABILITY_GOVERNED_BY", "ruleset": rule, "basis": basis[rule]} for rule in rulesets]}, "WAVE1_RELATION_FULL_VALUE")
        else:
            validate_use_relation(row, definitions[key])
            use_relation_count += 1
        for ruleset in rulesets:
            require((ROOT / ruleset / "index.json").is_file(), f"RELATION_RULESET_UNRESOLVED:{ruleset}")
        relation_count += len(rulesets)
    require(use_relation_count == 55 and sum(bool(item["capability_relations"]) for item in staged["items"]) + use_relation_count == len(seen_sources), "RELATION_COVERAGE")

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
        # Blocked contracts stay UNKNOWN even when the source carried a value. Weight is no
        # longer blocked: the owner fixed its unit (hundredths of an ounce, 2026-09-30) and
        # ITEM-SEM-2b promotes it from TibiaWiki.
        for blocked in ("stack.stack_max",):
            require(blocked not in known, f"BLOCKED_FIELD_PROMOTED:{blocked}")
        equipment = superseding.get((key, "equipment.patterns"))
        if equipment is None:
            require(definitions[key].get("semantics", {}).get("equipment", {}).get("state", "UNKNOWN") == "UNKNOWN", "BLOCKED_EQUIPMENT_PROMOTED")
        else:
            require(known.get("equipment.patterns") == equipment, "EQUIPMENT_WITHOUT_QUALIFIED_WIKI_PATTERN")
    require(fact_count == staged["counts"]["definition_facts"] + staged["counts"]["authoring_facts"], "PROVENANCE_FACT_COUNT")
    return len(legacy_authoring), len(taxonomy["records"]), relation_count, fact_count


def validate_creature_families(reference: Any, declarations: Any, sources: Any) -> tuple[int, int, int]:
    """Round-trip the admitted creature families, their authoring profiles and source bindings."""
    legacy_profiles = {target_id(row["target"]): row["data"] for row in declarations.get("authoring_profiles", [])
                       if row["target"]["family"] != "Encounter"}
    migrated_profiles: dict[tuple[str, str, str], Any] = {}
    migrated_bindings: list[Any] = []
    for family, node in CREATURE_FAMILY_NODES.items():
        legacy = [row for row in reference["records"] if row["identity"]["family"] == family]
        require(len(legacy) == CREATURE_FAMILY_COUNTS[family], f"LEGACY_{family.upper()}_COUNT")
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
    require(len(legacy_bindings) == CREATURE_FAMILY_COUNTS["Creature"], "CREATURE_BINDING_COUNT")
    canary_creatures = load(ROOT / "imports/canary/bindings/creatures.json")["bindings"]
    wiki_creatures = load(ROOT / "imports/tibiawiki/bindings/creatures.json")["bindings"]
    crystal_creatures = load(ROOT / "imports/crystalserver/bindings/creatures.json")["bindings"]
    require(all(row["source_key"] == "oteryn:source.canary" for row in canary_creatures)
            and all(row["source_key"] == "oteryn:source.tibiawiki" for row in wiki_creatures)
            and all(row["source_key"] == "oteryn:source.crystalserver" for row in crystal_creatures)
            and canonical_sorted(canary_creatures + wiki_creatures + crystal_creatures) == canonical_sorted(legacy_bindings),
            "IMPORT_CREATURE_BINDINGS")
    require(load(ROOT / "imports/canary/index.json")["population_state"] == "POPULATED", "IMPORT_CANARY_MARKER_STATE")
    return sum(CREATURE_FAMILY_COUNTS.values()), len(migrated_profiles), len(migrated_bindings)


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
        legacy = [row for row in legacy_services if field in row]
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


def main() -> int:
    reference = load(LEGACY / "definitions" / "reference.json")
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
    require(lock["family_counts"] == {"Item": 34031, "Mount": 252, **CREATURE_FAMILY_COUNTS, "NPC": NPC_COUNT,
                                       "Encounter": ENCOUNTER_COUNT, "Dialogue": DIALOGUE_COUNT, **SERVICE_FAMILY_COUNTS,
                                       "Charm": CHARM_COUNT, "Proficiency": PROFICIENCY_COUNT,
                                       "RewardClaim": REWARD_CLAIM_COUNT, "StarterKit": STARTER_KIT_COUNT},
            "LOCK_COUNTS")
    require(lock["source_binding_counts"]["NPC"] == NPC_BINDING_COUNT, "LOCK_NPC_BINDING_COUNT")
    require(item_index["record_count"] == 34031 and len(item_index["shards"]) == 69, "ITEM_INDEX")
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
    require(migrated_items == legacy_items and expected_start == 34031, "ITEM_DEFINITION_ROUNDTRIP")

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

    require(len({target_id(row["identity"]) for row in migrated_items}) == 34031, "ITEM_IDENTITY_UNIQUENESS")
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
        "PASS items=34031 mounts=252 item_editors=165 mount_editors=252 item_bindings=165 mount_bindings=252 "
        f"item_authoring={authoring_count} taxonomy={taxonomy_count} relations={relation_count} provenance_facts={fact_count} "
        f"creature_records={creature_records} creature_profiles={creature_profiles} creature_bindings={creature_bindings} "
        f"npc_records={npc_records} npc_bindings={npc_bindings} service_records={service_records} dialogue_records={dialogue_records} "
        f"encounter_records={encounter_records} charm_records={CHARM_COUNT} "
        f"proficiency_records={PROFICIENCY_COUNT} proficiency_bindings={PROFICIENCY_BINDING_COUNT} "
        f"reward_claim_records={REWARD_CLAIM_COUNT} starter_kit_records={STARTER_KIT_COUNT}"
    )
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
