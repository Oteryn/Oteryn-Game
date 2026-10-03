#!/usr/bin/env python3
"""Regenerate the successor Item/Mount/creature authoring tree from protected WorldProject/v2."""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from collections import Counter
from typing import Any

from item_taxonomy import build_taxonomy, taxonomy_inputs

ROOT = Path(__file__).resolve().parents[2]
LEGACY = ROOT / "content" / "world"
ITEM_SHARD_SIZE = 500
ADMISSION_MAIN = "ec0e12a7927dcd4d98f7d1151f6b8ee100c1b65c"
REVISION = "tree-npc-reviewed-definitions-r1"
FIELD_CENSUS = ROOT / "docs" / "agents" / "evidence" / "OTV2-20260925-tibiawiki-item-master-field-census-v1.json"
WAVE1_STAGED = ROOT / "docs" / "agents" / "evidence" / "OTV2-20260925-item-enrichment-wave1-staged.json"
# Charm is a static family authored under tools/content-schema/charm-authoring (`charm_authoring.py content`);
# it has no legacy WorldProject source, so this generator only registers the committed family.
CHARM_INDEX = "content/charms/index.json"
# Proficiency likewise (tools/content-schema/proficiency-authoring, `proficiency_authoring.py content`).
PROFICIENCY_INDEX = "content/proficiencies/index.json"
PROFICIENCY_BINDINGS = "content/proficiencies/bindings.json"
# RewardClaim likewise (tools/content-schema/reward-claim-authoring, `reward_claim_authoring.py content`).
REWARD_CLAIM_INDEX = "content/interactions/reward_claims/index.json"
# StarterKit likewise (tools/content-schema/starter-kit-authoring, `starter_kit_authoring.py content`).
STARTER_KIT_INDEX = "content/starter/index.json"
QUEST_INDEX = "content/quests/definitions/index.json"
# A12 (ITEM-ID-1): the staged packet is history naming retired Item keys; its targets are emitted
# through the append-only alias table (content/items/aliases.json).
ITEM_ALIASES = ROOT / "content" / "items" / "aliases.json"
# Creature admission families (OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1): family -> (tree node, shard stem).
CREATURE_FAMILIES = {
    "Creature": ("content/creatures/definitions/", "creatures"),
    "Presentation": ("content/presentations/definitions/", "presentations"),
    "Behavior": ("content/behaviors/", "behaviors"),
    "Loot": ("content/loot/", "loot"),
    "Ability": ("content/abilities/definitions/", "abilities"),
    "Effect": ("content/abilities/effects/", "effects"),
    "Formula": ("content/abilities/formulas/", "formulas"),
}
# Service declaration shard families (OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1): tree family -> (node, shard stem, discriminating field).
SERVICE_FAMILIES = {
    "Service.Trade": ("content/services/trade/", "trade", "offers"),
    "Service.Travel": ("content/services/travel/", "travel", "routes"),
}
# Canonical capability relations: a known typed fact names the ruleset that governs it.
CAPABILITY_RULES = (
    ("rulesets/items/enchanting/", "lifecycle.enchantable=true"),
    ("rulesets/items/exaltation-forge/", "forge"),
    ("rulesets/items/imbuements/", "imbuement.slot_count>=1"),
)

def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")

def load(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))

def write(relative: str, value: Any) -> None:
    path = ROOT / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical_bytes(value))

def write_registry(relative: str, value: Any) -> None:
    # One key per line, so PRs that register different families merge without a textual conflict.
    text = json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n"
    (ROOT / relative).write_text(text, encoding="utf-8", newline="\n")

def retained_quest_registration(root: Path) -> tuple[dict[str, Any], list[str]]:
    """Retain the separately authored Quest family; never synthesize its records."""
    manifest = load(root / "content/manifest.json")
    if "Quest" not in manifest["families"]:
        return {}, []
    registration = manifest["families"]["Quest"]
    if registration.get("index") != QUEST_INDEX:
        raise RuntimeError("QUEST_INDEX_PATH")
    index = load(root / QUEST_INDEX)
    if index.get("schema") != "OTERYN_FAMILY_INDEX/v1" or index.get("family") != "Quest":
        raise RuntimeError("QUEST_INDEX_MISSING")
    count = index.get("record_count")
    if type(count) is not int or count < 0 or registration.get("records") != count:
        raise RuntimeError("QUEST_REGISTRATION_COUNT")
    shards = index.get("shards")
    if not isinstance(shards, list) or len(shards) != len(set(shards)):
        raise RuntimeError("QUEST_SHARDS")
    actual = 0
    for relative in shards:
        if not isinstance(relative, str) or not re.fullmatch(r"content/quests/definitions/quests-[0-9]{5}-[0-9]{5}\.json", relative):
            raise RuntimeError("QUEST_SHARD_PATH")
        shard = load(root / relative)
        if shard.get("family") != "Quest" or shard.get("shard", {}).get("count") != len(shard.get("records", [])):
            raise RuntimeError("QUEST_SHARD_COUNT")
        actual += len(shard["records"])
    if actual != count:
        raise RuntimeError("QUEST_RECORD_COUNT")
    return {"Quest": {"records": count, "index": QUEST_INDEX}}, [QUEST_INDEX, *shards]


def target_id(target: dict[str, Any]) -> tuple[str, str, str]:
    return (target["family"], target["key"], target["revision"])

def capability_relations(definition: dict[str, Any], authoring: dict[str, Any] | None) -> list[dict[str, str]]:
    semantics = definition.get("semantics", {})
    imbuement = semantics.get("imbuement", {})
    slots = imbuement.get("value", {}).get("slot_count", {}) if imbuement.get("state") == "KNOWN" else {}
    facts = {
        "lifecycle.enchantable=true": bool(authoring and (authoring.get("lifecycle") or {}).get("enchantable") is True),
        "forge": bool(authoring and authoring.get("forge")),
        "imbuement.slot_count>=1": slots.get("state") == "KNOWN" and slots["value"] >= 1,
    }
    return [{"relation": "CAPABILITY_GOVERNED_BY", "ruleset": ruleset, "basis": basis} for ruleset, basis in CAPABILITY_RULES if facts[basis]]

def item_alias_targets() -> dict[str, str]:
    """The current alias target of every retired Item key (latest entry version wins)."""
    current: dict[str, dict[str, Any]] = {}
    for entry in load(ITEM_ALIASES)["entries"]:
        current[entry["key"]] = entry
    return {key: entry["target"] for key, entry in current.items() if entry["state"] == "ALIAS"}

def canonical_item_target(target: dict[str, Any], aliases: dict[str, str]) -> dict[str, Any]:
    if target["family"] != "Item" or target["key"] not in aliases:
        raise RuntimeError(f"STAGED_ITEM_TARGET_WITHOUT_ALIAS:{target['key']}")
    return {**target, "key": aliases[target["key"]]}

def git_blob_sha(path: Path) -> str:
    data = path.read_bytes()
    return hashlib.sha1(b"blob " + str(len(data)).encode("ascii") + b"\0" + data).hexdigest()

def main() -> int:
    reference = load(LEGACY / "definitions" / "reference.json")
    declarations = load(LEGACY / "definitions" / "declarations.json")
    editor = load(LEGACY / "editor" / "author.json")
    sources = load(LEGACY / "provenance" / "sources.json")
    imports = load(LEGACY / "provenance" / "imports.json")

    family_assignments = load(FIELD_CENSUS)["family_assignments"]
    wave1 = load(WAVE1_STAGED)
    item_aliases = item_alias_targets()
    authoring_by_target = {target_id(row["item"]): row for row in declarations.get("item_authoring", [])}
    editors = {target_id(row["target"]): row for row in editor["entries"]}
    bindings: dict[tuple[str, str, str], list[dict[str, Any]]] = {}
    for row in sources["source_identity_bindings"]:
        bindings.setdefault(target_id(row["target"]), []).append(row)
    for rows in bindings.values():
        rows.sort(key=canonical_bytes)

    item_records = [row for row in reference["records"] if row["identity"]["family"] == "Item"]
    # Accepted reward Item facts are tree-first; retain them across legacy regeneration.
    from quest_reward_item_semantics import apply_admissions
    apply_admissions(item_records, ROOT)
    item_shards: list[str] = []
    for start in range(0, len(item_records), ITEM_SHARD_SIZE):
        rows = []
        for definition in item_records[start:start + ITEM_SHARD_SIZE]:
            key = target_id(definition["identity"])
            row: dict[str, Any] = {"definition": definition}
            if key in editors:
                row["editor"] = editors[key]
            if key in bindings:
                row["source_bindings"] = bindings[key]
            authoring = {field: value for field, value in authoring_by_target.get(key, {}).items() if field not in {"item", "taxonomy"}}
            if authoring:
                row["authoring"] = authoring
            rows.append(row)
        end = start + len(rows) - 1
        relative = f"content/items/definitions/items-{start:05d}-{end:05d}.json"
        item_shards.append(relative)
        write(relative, {
            "schema": "OTERYN_ITEM_AUTHORING_SHARD/v1",
            "family": "Item",
            "source_legacy_role": "content/world/definitions/reference.json",
            "shard": {"index": start // ITEM_SHARD_SIZE, "start": start, "end": end, "count": len(rows)},
            "records": rows,
        })

    declarations_blob_sha = git_blob_sha(LEGACY / "definitions" / "declarations.json")

    mount_declarations = [row for row in declarations["records"] if row.get("kind") == "Mount"]
    if len(mount_declarations) != 252:
        raise RuntimeError(f"MOUNT_SOURCE_COUNT_MISMATCH:{len(mount_declarations)}")

    mount_rows = []
    for declaration in mount_declarations:
        target = {"family": "Mount", "key": declaration["identity"]["key"], "revision": declaration["identity"]["revision"]}
        key = target_id(target)
        row: dict[str, Any] = {"declaration": declaration}
        if key in editors:
            row["editor"] = editors[key]
        if key in bindings:
            row["source_bindings"] = bindings[key]
        mount_rows.append(row)
    mount_relative = f"content/cosmetics/mounts/mounts-00000-{len(mount_rows)-1:05d}.json"
    write(mount_relative, {
        "schema": "OTERYN_MOUNT_AUTHORING_SHARD/v1",
        "family": "Mount",
        "source_legacy_role": "content/world/definitions/declarations.json",
        "shard": {"index": 0, "start": 0, "end": len(mount_rows) - 1, "count": len(mount_rows)},
        "records": mount_rows,
    })

    definitions_by_target = {target_id(row["identity"]): row for row in reference["records"]}
    taxonomy_rows = build_taxonomy(
        definitions_by_target, authoring_by_target, family_assignments, *taxonomy_inputs(ROOT)
    )
    relation_rows = []
    for key, definition in sorted(definitions_by_target.items()):
        if key[0] != "Item":
            continue
        relations = capability_relations(definition, authoring_by_target.get(key))
        if relations:
            relation_rows.append({"source": definition["identity"], "relations": relations})
    wave1_facts = [
        {
            "target": canonical_item_target(item["target"], item_aliases),
            "batch_id": wave1["batch_id"],
            "source_key": wave1["source"]["source_key"],
            "source_revision": wave1["source"]["source_revision"],
            "identity_namespace": wave1["source"]["identity_namespace"],
            "external_id": item["external_id"],
            "revision_id": item["revision_id"],
            "source_digest": item["source_digest"],
            "mapper": wave1["mapper"],
            "evidence": wave1["source"]["evidence"],
            "definition_facts": item["facts"],
            "authoring_facts": item["authoring_provenance"],
        }
        for item in wave1["items"]
    ]

    profiles = {target_id(row["target"]): row["data"] for row in declarations.get("authoring_profiles", [])}
    creature_shards: dict[str, list[str]] = {}
    creature_counts: dict[str, int] = {}
    for family, (node, stem) in CREATURE_FAMILIES.items():
        records = [row for row in reference["records"] if row["identity"]["family"] == family]
        creature_counts[family] = len(records)
        creature_shards[family] = []
        for start in range(0, len(records), ITEM_SHARD_SIZE):
            rows = []
            for definition in records[start:start + ITEM_SHARD_SIZE]:
                key = target_id(definition["identity"])
                row = {"definition": definition}
                if key in profiles:
                    row["authoring"] = profiles[key]
                if key in bindings:
                    row["source_bindings"] = bindings[key]
                rows.append(row)
            end = start + len(rows) - 1
            relative = f"{node}{stem}-{start:05d}-{end:05d}.json"
            creature_shards[family].append(relative)
            write(relative, {
                "schema": "OTERYN_CREATURE_ADMISSION_SHARD/v1",
                "family": family,
                "source_legacy_role": "content/world/definitions/reference.json",
                "shard": {"index": start // ITEM_SHARD_SIZE, "start": start, "end": end, "count": len(rows)},
                "records": rows,
            })
        write(f"{node}index.json", {
            "schema": "OTERYN_FAMILY_INDEX/v1",
            "family": family,
            "record_count": len(records),
            "shard_size": ITEM_SHARD_SIZE,
            "shards": creature_shards[family],
            "legacy_source": {
                "path": "content/world/definitions/reference.json",
                "git_blob_sha": git_blob_sha(LEGACY / "definitions" / "reference.json"),
                "schema": reference["schema"],
            },
            "attached_authoring_profiles": sum(target_id(row["identity"]) in profiles for row in records),
            "attached_source_bindings": sum(len(bindings.get(target_id(row["identity"]), [])) for row in records),
        })

    creature_bindings = [row for row in sources["source_identity_bindings"] if row["target"]["family"] == "Creature"]
    item_bindings = [row for row in sources["source_identity_bindings"] if row["target"]["family"] == "Item"]
    mount_bindings = [row for row in sources["source_identity_bindings"] if row["target"]["family"] == "Mount"]
    # Import-only TibiaWiki evidence (a facts file naming its own batch, e.g. the Item family fallback) is not
    # part of WorldProject/v2; keep its source and batch rows when regenerating the shared import indexes.
    wiki_facts = sorted((ROOT / "imports/tibiawiki/facts").glob("*.json"))
    import_only = {load(path).get("batch_id") for path in wiki_facts} - {row["batch_id"] for row in imports["batches"]} - {None}
    kept_sources = [row for row in load(ROOT / "imports/tibiawiki/sources.json")["sources"] if row["import_batch_id"] in import_only]
    kept_batches = [row for row in load(ROOT / "imports/tibiawiki/batches.json")["batches"] if row["batch_id"] in import_only]
    outputs = {
        "imports/crystalserver/sources.json": {"schema": "OTERYN_IMPORT_SOURCES/v1", "sources": [row for row in sources["sources"] if row["key"] == "oteryn:source.crystalserver"]},
        "imports/crystalserver/batches.json": {"schema": "OTERYN_IMPORT_BATCHES/v1", "batches": [row for row in imports["batches"] if row["source_repository"] == "zimbadev/crystalserver"]},
        "imports/canary/sources.json": {"schema": "OTERYN_IMPORT_SOURCES/v1", "sources": [row for row in sources["sources"] if row["key"] == "oteryn:source.canary"]},
        "imports/canary/batches.json": {"schema": "OTERYN_IMPORT_BATCHES/v1", "batches": [row for row in imports["batches"] if row["source_repository"] == "opentibiabr/canary"]},
        "imports/canary/bindings/creatures.json": {"schema": "OTERYN_SOURCE_IDENTITY_BINDINGS/v1", "family": "Creature",
                                                   "bindings": [row for row in creature_bindings if row["source_key"] == "oteryn:source.canary"]},
        # Game version 15.30: creatures CrystalServer has at its pinned 15.30 commit and Canary lacks bind to the Crystal file.
        "imports/crystalserver/bindings/creatures.json": {"schema": "OTERYN_SOURCE_IDENTITY_BINDINGS/v1", "family": "Creature",
                                                          "bindings": [row for row in creature_bindings if row["source_key"] == "oteryn:source.crystalserver"]},
        "imports/tibiawiki/sources.json": {"schema": "OTERYN_IMPORT_SOURCES/v1", "sources": [row for row in sources["sources"] if row["key"] == "oteryn:source.tibiawiki"] + kept_sources},
        "imports/tibiawiki/batches.json": {"schema": "OTERYN_IMPORT_BATCHES/v1", "batches": [row for row in imports["batches"] if row["batch_id"] in {source["import_batch_id"] for source in sources["sources"] if source["key"] == "oteryn:source.tibiawiki"}] + kept_batches},
        "imports/tibiawiki/bindings/items.json": {"schema": "OTERYN_SOURCE_IDENTITY_BINDINGS/v1", "family": "Item", "bindings": item_bindings},
        "imports/tibiawiki/bindings/mounts.json": {"schema": "OTERYN_SOURCE_IDENTITY_BINDINGS/v1", "family": "Mount", "bindings": mount_bindings},
        # D44 wiki-authored creatures bind to their TibiaWiki page id.
        "imports/tibiawiki/bindings/creatures.json": {"schema": "OTERYN_SOURCE_IDENTITY_BINDINGS/v1", "family": "Creature",
                                                      "bindings": [row for row in creature_bindings if row["source_key"] == "oteryn:source.tibiawiki"]},
        "imports/tibiawiki/facts/items-wave1.json": {
            "schema": "OTERYN_IMPORTED_FACT_PROVENANCE/v1",
            "family": "Item",
            "batch_id": wave1["batch_id"],
            "snapshot_sha256": wave1["source"]["snapshot_sha256"],
            "records": wave1_facts,
        },
        "content/items/taxonomy/items.json": {
            "schema": "OTERYN_ITEM_TAXONOMY/v1",
            "family_profile_contract": "tools/content-schema/item-authoring/profile-catalog.json",
            "legacy_family_profile_contract": "docs/agents/evidence/OTV2-20260925-tibiawiki-item-master-field-census-v1.json#family_assignments",
            "records": taxonomy_rows,
        },
        "content/items/relations/items.json": {
            "schema": "OTERYN_ITEM_RELATIONS/v1",
            "records": relation_rows,
        },
    }
    for relative, value in outputs.items():
        write(relative, value)

    write("content/items/index.json", {
        "schema": "OTERYN_FAMILY_INDEX/v1",
        "family": "Item",
        "record_count": len(item_records),
        "shard_size": ITEM_SHARD_SIZE,
        "shards": item_shards,
        "legacy_source": {
            "path": "content/world/definitions/reference.json",
            "git_blob_sha": git_blob_sha(LEGACY / "definitions" / "reference.json"),
            "schema": reference["schema"],
            "world_id": reference["world_id"],
            "coordinate_frame": reference["coordinate_frame"],
        },
        "attached_editor_entries": sum(row["target"]["family"] == "Item" for row in editor["entries"]),
        "attached_source_bindings": len(item_bindings),
        "attached_authoring_entries": len(authoring_by_target),
    })
    write("content/cosmetics/mounts/index.json", {
        "schema": "OTERYN_FAMILY_INDEX/v1",
        "family": "Mount",
        "record_count": len(mount_rows),
        "shards": [mount_relative],
        "legacy_source": {
            "path": "content/world/definitions/declarations.json",
            "git_blob_sha": declarations_blob_sha,
            "schema": declarations["schema"],
        },
        "attached_editor_entries": sum(row["target"]["family"] == "Mount" for row in editor["entries"]),
        "attached_source_bindings": len(mount_bindings),
    })

    npc_declarations = [row for row in declarations["records"] if row.get("kind") == "NPC"]
    if len(npc_declarations) != 1110:
        raise RuntimeError(f"NPC_SOURCE_COUNT_MISMATCH:{len(npc_declarations)}")

    npc_rows = []
    for declaration in npc_declarations:
        target = {"family": "NPC", "key": declaration["identity"]["key"], "revision": declaration["identity"]["revision"]}
        key = target_id(target)
        row = {"declaration": declaration}
        if key in bindings:
            row["source_bindings"] = bindings[key]
        npc_rows.append(row)

    npc_shards: list[str] = []
    for start in range(0, len(npc_rows), ITEM_SHARD_SIZE):
        rows = npc_rows[start:start + ITEM_SHARD_SIZE]
        end = start + len(rows) - 1
        relative = f"content/npcs/definitions/npcs-{start:05d}-{end:05d}.json"
        npc_shards.append(relative)
        write(relative, {
            "schema": "OTERYN_NPC_AUTHORING_SHARD/v1",
            "family": "NPC",
            "source_legacy_role": "content/world/definitions/declarations.json",
            "shard": {"index": start // ITEM_SHARD_SIZE, "start": start, "end": end, "count": len(rows)},
            "records": rows,
        })
    npc_bindings = [row for row in sources["source_identity_bindings"] if row["target"]["family"] == "NPC"]
    write("content/npcs/definitions/index.json", {
        "schema": "OTERYN_FAMILY_INDEX/v1",
        "family": "NPC",
        "record_count": len(npc_rows),
        "shard_size": ITEM_SHARD_SIZE,
        "shards": npc_shards,
        "legacy_source": {
            "path": "content/world/definitions/declarations.json",
            "git_blob_sha": declarations_blob_sha,
            "schema": declarations["schema"],
        },
        "attached_source_bindings": sum(len(row.get("source_bindings", [])) for row in npc_rows),
    })

    # Encounter admission (OTERYN_WORLD_PROJECT_V2_ENCOUNTER_ADMISSION_V1): declaration, profile and source binding per row.
    encounter_rows = []
    for declaration in (row for row in declarations["records"] if row.get("kind") == "Encounter"):
        key = ("Encounter", declaration["identity"]["key"], declaration["identity"]["revision"])
        row = {"declaration": declaration}
        if key in profiles:
            row["authoring"] = profiles[key]
        if key in bindings:
            row["source_bindings"] = bindings[key]
        encounter_rows.append(row)
    encounter_shards: list[str] = []
    for start in range(0, len(encounter_rows), ITEM_SHARD_SIZE):
        rows = encounter_rows[start:start + ITEM_SHARD_SIZE]
        end = start + len(rows) - 1
        relative = f"content/encounters/definitions/encounters-{start:05d}-{end:05d}.json"
        encounter_shards.append(relative)
        write(relative, {
            "schema": "OTERYN_ENCOUNTER_AUTHORING_SHARD/v1",
            "family": "Encounter",
            "source_legacy_role": "content/world/definitions/declarations.json",
            "shard": {"index": start // ITEM_SHARD_SIZE, "start": start, "end": end, "count": len(rows)},
            "records": rows,
        })
    encounter_bindings = [row for row in sources["source_identity_bindings"] if row["target"]["family"] == "Encounter"]
    write("content/encounters/definitions/index.json", {
        "schema": "OTERYN_FAMILY_INDEX/v1",
        "family": "Encounter",
        "record_count": len(encounter_rows),
        "shard_size": ITEM_SHARD_SIZE,
        "shards": encounter_shards,
        "legacy_source": {
            "path": "content/world/definitions/declarations.json",
            "git_blob_sha": declarations_blob_sha,
            "schema": declarations["schema"],
        },
        "attached_authoring_profiles": sum("authoring" in row for row in encounter_rows),
        "attached_source_bindings": sum(len(row.get("source_bindings", [])) for row in encounter_rows),
    })

    dialogue_declarations = [row for row in declarations["records"] if row.get("kind") == "Dialogue"]
    if len(dialogue_declarations) != 694:
        raise RuntimeError(f"DIALOGUE_SOURCE_COUNT_MISMATCH:{len(dialogue_declarations)}")

    # No source bindings exist for Dialogue declarations (WorldProject/v2 NPC admission wave A).
    dialogue_rows = [{"declaration": declaration} for declaration in dialogue_declarations]
    dialogue_shards: list[str] = []
    for start in range(0, len(dialogue_rows), ITEM_SHARD_SIZE):
        rows = dialogue_rows[start:start + ITEM_SHARD_SIZE]
        end = start + len(rows) - 1
        relative = f"content/dialogues/definitions/dialogues-{start:05d}-{end:05d}.json"
        dialogue_shards.append(relative)
        write(relative, {
            "schema": "OTERYN_DIALOGUE_AUTHORING_SHARD/v1",
            "family": "Dialogue",
            "source_legacy_role": "content/world/definitions/declarations.json",
            "shard": {"index": start // ITEM_SHARD_SIZE, "start": start, "end": end, "count": len(rows)},
            "records": rows,
        })
    write("content/dialogues/definitions/index.json", {
        "schema": "OTERYN_FAMILY_INDEX/v1",
        "family": "Dialogue",
        "record_count": len(dialogue_rows),
        "shard_size": ITEM_SHARD_SIZE,
        "shards": dialogue_shards,
        "legacy_source": {
            "path": "content/world/definitions/declarations.json",
            "git_blob_sha": declarations_blob_sha,
            "schema": declarations["schema"],
        },
    })

    document_rows = [{"declaration": row} for row in declarations["records"] if row.get("kind") == "Document"]
    if len(document_rows) != 1609 or any(row["target"]["family"] == "Document" for row in sources["source_identity_bindings"]):
        raise RuntimeError("DOCUMENT_SOURCE_COUNT_OR_BINDING_MISMATCH")
    document_shards = []
    for start in range(0, len(document_rows), ITEM_SHARD_SIZE):
        rows = document_rows[start:start + ITEM_SHARD_SIZE]
        end = start + len(rows) - 1
        relative = f"content/documents/documents-{start:05d}-{end:05d}.json"
        document_shards.append(relative)
        write(relative, {"schema": "OTERYN_DOCUMENT_AUTHORING_SHARD/v1", "family": "Document",
                         "source_legacy_role": "content/world/definitions/declarations.json",
                         "shard": {"index": start // ITEM_SHARD_SIZE, "start": start, "end": end, "count": len(rows)},
                         "records": rows})
    write("content/documents/index.json", {"schema": "OTERYN_FAMILY_INDEX/v1", "family": "Document",
          "record_count": len(document_rows), "shard_size": ITEM_SHARD_SIZE, "shards": document_shards,
          "legacy_source": {"path": "content/world/definitions/declarations.json", "git_blob_sha": declarations_blob_sha,
                            "schema": declarations["schema"]}})

    service_records = [row for row in declarations["records"] if row.get("kind") == "Service"]
    if len(service_records) != 380:
        raise RuntimeError(f"SERVICE_SOURCE_COUNT_MISMATCH:{len(service_records)}")

    service_shards: dict[str, list[str]] = {}
    service_counts: dict[str, int] = {}
    for family, (node, stem, field) in SERVICE_FAMILIES.items():
        records = [row for row in service_records if field in row]
        service_counts[family] = len(records)
        service_shards[family] = []
        for start in range(0, len(records), ITEM_SHARD_SIZE):
            rows = [{"declaration": declaration} for declaration in records[start:start + ITEM_SHARD_SIZE]]
            end = start + len(rows) - 1
            relative = f"{node}{stem}-{start:05d}-{end:05d}.json"
            service_shards[family].append(relative)
            write(relative, {
                "schema": "OTERYN_SERVICE_AUTHORING_SHARD/v1",
                "family": family,
                "source_legacy_role": "content/world/definitions/declarations.json",
                "shard": {"index": start // ITEM_SHARD_SIZE, "start": start, "end": end, "count": len(rows)},
                "records": rows,
            })
        write(f"{node}index.json", {
            "schema": "OTERYN_FAMILY_INDEX/v1",
            "family": family,
            "record_count": len(records),
            "shard_size": ITEM_SHARD_SIZE,
            "shards": service_shards[family],
            "legacy_source": {
                "path": "content/world/definitions/declarations.json",
                "git_blob_sha": declarations_blob_sha,
                "schema": declarations["schema"],
            },
        })
    if sum(service_counts.values()) != len(service_records):
        raise RuntimeError(f"SERVICE_FIELD_SPLIT_MISMATCH:{service_counts}")

    charm_index = load(ROOT / CHARM_INDEX)
    if charm_index.get("schema") != "OTERYN_FAMILY_INDEX/v1" or charm_index.get("family") != "Charm":
        raise RuntimeError("CHARM_INDEX_MISSING")
    charm_count = charm_index["record_count"]
    proficiency_index = load(ROOT / PROFICIENCY_INDEX)
    if proficiency_index.get("schema") != "OTERYN_FAMILY_INDEX/v1" or proficiency_index.get("family") != "Proficiency":
        raise RuntimeError("PROFICIENCY_INDEX_MISSING")
    proficiency_count = proficiency_index["record_count"]
    reward_claim_index = load(ROOT / REWARD_CLAIM_INDEX)
    if reward_claim_index.get("schema") != "OTERYN_FAMILY_INDEX/v1" or reward_claim_index.get("family") != "RewardClaim":
        raise RuntimeError("REWARD_CLAIM_INDEX_MISSING")
    reward_claim_count = reward_claim_index["record_count"]
    starter_kit_index = load(ROOT / STARTER_KIT_INDEX)
    if starter_kit_index.get("schema") != "OTERYN_FAMILY_INDEX/v1" or starter_kit_index.get("family") != "StarterKit":
        raise RuntimeError("STARTER_KIT_INDEX_MISSING")
    starter_kit_count = starter_kit_index["record_count"]
    quest_families, quest_managed = retained_quest_registration(ROOT)
    creature_managed = [path for shards in creature_shards.values() for path in shards]
    creature_managed += [f"{node}index.json" for node, _ in CREATURE_FAMILIES.values()]
    service_managed = [path for shards in service_shards.values() for path in shards]
    service_managed += [f"{node}index.json" for node, _, _ in SERVICE_FAMILIES.values()]
    managed = sorted([*item_shards, mount_relative, "content/items/index.json", "content/cosmetics/mounts/index.json",
                      *creature_managed, *npc_shards, "content/npcs/definitions/index.json",
                      *encounter_shards, "content/encounters/definitions/index.json",
                      *dialogue_shards, "content/dialogues/definitions/index.json",
                      *document_shards, "content/documents/index.json",
                      *service_managed, CHARM_INDEX, *charm_index["shards"],
                      PROFICIENCY_INDEX, *proficiency_index["shards"], PROFICIENCY_BINDINGS,
                      REWARD_CLAIM_INDEX, *reward_claim_index["shards"],
                      STARTER_KIT_INDEX, *starter_kit_index["shards"], *quest_managed, *outputs.keys()])
    # A family that grows renames its last shard; drop the superseded shard files so every shard is managed.
    shard_name = re.compile(r"-\d{5}-\d{5}\.json$")
    managed_set = set(managed)
    for directory in sorted({(ROOT / path).parent for path in managed if shard_name.search(path)}):
        for stale in sorted(directory.glob("*.json")):
            relative = stale.relative_to(ROOT).as_posix()
            if shard_name.search(stale.name) and relative not in managed_set:
                stale.unlink()
    write_registry("content/manifest.json", {
        "schema": "OTERYN_GAME_CONTENT_TREE_MANIFEST/v1",
        "project_revision": REVISION,
        "admission_main": ADMISSION_MAIN,
        "managed_files": [{"path": path} for path in managed],
        "families": {
            "Item": {"records": len(item_records), "index": "content/items/index.json"},
            "Mount": {"records": len(mount_rows), "index": "content/cosmetics/mounts/index.json"},
            "Charm": {"records": charm_count, "index": CHARM_INDEX},
            "Proficiency": {"records": proficiency_count, "index": PROFICIENCY_INDEX},
            "RewardClaim": {"records": reward_claim_count, "index": REWARD_CLAIM_INDEX},
            "StarterKit": {"records": starter_kit_count, "index": STARTER_KIT_INDEX},
            **quest_families,
            **{family: {"records": creature_counts[family], "index": f"{node}index.json"}
               for family, (node, _) in CREATURE_FAMILIES.items()},
            "NPC": {"records": len(npc_rows), "index": "content/npcs/definitions/index.json"},
            "Encounter": {"records": len(encounter_rows), "index": "content/encounters/definitions/index.json"},
            "Dialogue": {"records": len(dialogue_rows), "index": "content/dialogues/definitions/index.json"},
            "Document": {"records": len(document_rows), "index": "content/documents/index.json"},
            **{family: {"records": service_counts[family], "index": f"{node}index.json"}
               for family, (node, _, _) in SERVICE_FAMILIES.items()},
        },
        "compatibility": {"legacy_root": "content/world", "legacy_mutated": False, "runtime_switch_authorized": False},
    })
    write_registry("content/content.lock.json", {
        "schema": "OTERYN_GAME_CONTENT_TREE_LOCK/v1",
        "project_revision": REVISION,
        "admission_main": ADMISSION_MAIN,
        "legacy_blobs": {
            "reference": git_blob_sha(LEGACY / "definitions" / "reference.json"),
            "declarations": declarations_blob_sha,
            "editor": git_blob_sha(LEGACY / "editor" / "author.json"),
            "sources": git_blob_sha(LEGACY / "provenance" / "sources.json"),
            "imports": git_blob_sha(LEGACY / "provenance" / "imports.json"),
        },
        "family_counts": {"Item": len(item_records), "Mount": len(mount_rows), **creature_counts,
                           "NPC": len(npc_rows), "Encounter": len(encounter_rows), "Dialogue": len(dialogue_rows),
                           "Document": len(document_rows),
                           **service_counts, "Charm": charm_count, "Proficiency": proficiency_count,
                           "RewardClaim": reward_claim_count, "StarterKit": starter_kit_count,
                           **{family: value["records"] for family, value in quest_families.items()}},
        "item_authoring_counts": {"authoring": len(authoring_by_target), "taxonomy": len(taxonomy_rows), "relation_sources": len(relation_rows)},
        "source_binding_counts": {"Item": len(item_bindings), "Mount": len(mount_bindings), "Creature": len(creature_bindings),
                                   "NPC": len(npc_bindings), "Encounter": len(encounter_bindings)},
        "authoring_profile_counts": dict(sorted(Counter(family for family, _, _ in profiles).items())),
        "editor_entry_counts": {
            "Item": sum(row["target"]["family"] == "Item" for row in editor["entries"]),
            "Mount": sum(row["target"]["family"] == "Mount" for row in editor["entries"]),
        },
    })
    write_registry("content/project.json", {
        "schema": "OTERYN_GAME_CONTENT_TREE_PROJECT/v1",
        "project_revision": REVISION,
        "manifest": "content/manifest.json",
        "content_lock": "content/content.lock.json",
        "migrated_families": ["Item", "Mount", *CREATURE_FAMILIES, "NPC", "Encounter", "Dialogue", "Document", "Service", "Charm",
                             "Proficiency", "RewardClaim", "StarterKit", *quest_families],
        "legacy_compatibility_root": "content/world",
        "runtime_source": "legacy_until_separately_qualified",
        "next_population_families": [family for family in
            ["Quest", "Achievement", "Outfit", "Area", "House", "WorldObject"]
            if family not in quest_families],
    })
    # Preserve independently authored spell collections on every regeneration.
    from register_spell_families import outputs as spell_outputs
    for relative, data in spell_outputs(ROOT).items():
        destination = ROOT / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(data)
    print(f"PASS items={len(item_records)} creatures={creature_counts['Creature']} creature_records={sum(creature_counts.values())} creature_profiles={len(profiles)} item_shards={len(item_shards)} mounts={len(mount_rows)} authoring={len(authoring_by_target)} taxonomy={len(taxonomy_rows)} relation_sources={len(relation_rows)} relations={sum(len(row['relations']) for row in relation_rows)} npcs={len(npc_rows)} encounters={len(encounter_rows)} npc_bindings={len(npc_bindings)} dialogues={len(dialogue_rows)} service_trade={service_counts['Service.Trade']} service_travel={service_counts['Service.Travel']}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
