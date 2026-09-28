#!/usr/bin/env python3
"""Regenerate the successor Item/Mount/creature authoring tree from protected WorldProject/v2."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from collections import Counter
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
LEGACY = ROOT / "content" / "world"
ITEM_SHARD_SIZE = 500
ADMISSION_MAIN = "ec0e12a7927dcd4d98f7d1151f6b8ee100c1b65c"
REVISION = "tree-npc-dialogue-wave-a-r1"
FIELD_CENSUS = ROOT / "docs" / "agents" / "evidence" / "OTV2-20260925-tibiawiki-item-master-field-census-v1.json"
WAVE1_STAGED = ROOT / "docs" / "agents" / "evidence" / "OTV2-20260925-item-enrichment-wave1-staged.json"
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
    authoring_by_target = {target_id(row["item"]): row for row in declarations.get("item_authoring", [])}
    editors = {target_id(row["target"]): row for row in editor["entries"]}
    bindings: dict[tuple[str, str, str], list[dict[str, Any]]] = {}
    for row in sources["source_identity_bindings"]:
        bindings.setdefault(target_id(row["target"]), []).append(row)
    for rows in bindings.values():
        rows.sort(key=canonical_bytes)

    item_records = [row for row in reference["records"] if row["identity"]["family"] == "Item"]
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
    taxonomy_rows = []
    relation_rows = []
    for key, authoring in sorted(authoring_by_target.items()):
        if "taxonomy" in authoring:
            taxonomy_rows.append({
                "target": authoring["item"],
                "source_taxonomy": authoring["taxonomy"],
                "family_profile": family_assignments.get(authoring["taxonomy"]["primary"]),
            })
        relations = capability_relations(definitions_by_target[key], authoring)
        if relations:
            relation_rows.append({"source": authoring["item"], "relations": relations})
    wave1_facts = [
        {
            "target": item["target"],
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
            "family_profile_contract": "docs/agents/evidence/OTV2-20260925-tibiawiki-item-master-field-census-v1.json#family_assignments",
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
    if len(npc_declarations) != 1093:
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

    dialogue_declarations = [row for row in declarations["records"] if row.get("kind") == "Dialogue"]
    if len(dialogue_declarations) != 701:
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

    service_records = [row for row in declarations["records"] if row.get("kind") == "Service"]
    if len(service_records) != 363:
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

    creature_managed = [path for shards in creature_shards.values() for path in shards]
    creature_managed += [f"{node}index.json" for node, _ in CREATURE_FAMILIES.values()]
    service_managed = [path for shards in service_shards.values() for path in shards]
    service_managed += [f"{node}index.json" for node, _, _ in SERVICE_FAMILIES.values()]
    managed = sorted([*item_shards, mount_relative, "content/items/index.json", "content/cosmetics/mounts/index.json",
                      *creature_managed, *npc_shards, "content/npcs/definitions/index.json",
                      *dialogue_shards, "content/dialogues/definitions/index.json",
                      *service_managed, *outputs.keys()])
    write("content/manifest.json", {
        "schema": "OTERYN_GAME_CONTENT_TREE_MANIFEST/v1",
        "project_revision": REVISION,
        "admission_main": ADMISSION_MAIN,
        "managed_files": [{"path": path} for path in managed],
        "families": {
            "Item": {"records": len(item_records), "index": "content/items/index.json"},
            "Mount": {"records": len(mount_rows), "index": "content/cosmetics/mounts/index.json"},
            **{family: {"records": creature_counts[family], "index": f"{node}index.json"}
               for family, (node, _) in CREATURE_FAMILIES.items()},
            "NPC": {"records": len(npc_rows), "index": "content/npcs/definitions/index.json"},
            "Dialogue": {"records": len(dialogue_rows), "index": "content/dialogues/definitions/index.json"},
            **{family: {"records": service_counts[family], "index": f"{node}index.json"}
               for family, (node, _, _) in SERVICE_FAMILIES.items()},
        },
        "compatibility": {"legacy_root": "content/world", "legacy_mutated": False, "runtime_switch_authorized": False},
    })
    write("content/content.lock.json", {
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
                           "NPC": len(npc_rows), "Dialogue": len(dialogue_rows), **service_counts},
        "item_authoring_counts": {"authoring": len(authoring_by_target), "taxonomy": len(taxonomy_rows), "relation_sources": len(relation_rows)},
        "source_binding_counts": {"Item": len(item_bindings), "Mount": len(mount_bindings), "Creature": len(creature_bindings),
                                   "NPC": len(npc_bindings)},
        "authoring_profile_counts": dict(sorted(Counter(family for family, _, _ in profiles).items())),
        "editor_entry_counts": {
            "Item": sum(row["target"]["family"] == "Item" for row in editor["entries"]),
            "Mount": sum(row["target"]["family"] == "Mount" for row in editor["entries"]),
        },
    })
    write("content/project.json", {
        "schema": "OTERYN_GAME_CONTENT_TREE_PROJECT/v1",
        "project_revision": REVISION,
        "manifest": "content/manifest.json",
        "content_lock": "content/content.lock.json",
        "migrated_families": ["Item", "Mount", *CREATURE_FAMILIES, "NPC", "Dialogue", "Service"],
        "legacy_compatibility_root": "content/world",
        "runtime_source": "legacy_until_separately_qualified",
        "next_population_families": [
            "Quest", "Achievement", "Outfit", "Charm", "Encounter", "Area", "House", "WorldObject",
        ],
    })
    print(f"PASS items={len(item_records)} creatures={creature_counts['Creature']} creature_records={sum(creature_counts.values())} creature_profiles={len(profiles)} item_shards={len(item_shards)} mounts={len(mount_rows)} authoring={len(authoring_by_target)} taxonomy={len(taxonomy_rows)} relation_sources={len(relation_rows)} relations={sum(len(row['relations']) for row in relation_rows)} npcs={len(npc_rows)} npc_bindings={len(npc_bindings)} dialogues={len(dialogue_rows)} service_trade={service_counts['Service.Trade']} service_travel={service_counts['Service.Travel']}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
