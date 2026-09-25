#!/usr/bin/env python3
"""Regenerate the successor Item/Mount authoring tree from protected WorldProject/v2."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
LEGACY = ROOT / "content" / "world"
ITEM_SHARD_SIZE = 500
ADMISSION_MAIN = "08a8d5d49e767476df7be10949042e539db414ca"
REVISION = "tree-items-wave1-r1"
FIELD_CENSUS = ROOT / "docs" / "agents" / "evidence" / "OTV2-20260925-tibiawiki-item-master-field-census-v1.json"
WAVE1_STAGED = ROOT / "docs" / "agents" / "evidence" / "OTV2-20260925-item-enrichment-wave1-staged.json"
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

    item_shards: list[str] = []
    for start in range(0, len(reference["records"]), ITEM_SHARD_SIZE):
        rows = []
        for definition in reference["records"][start:start + ITEM_SHARD_SIZE]:
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

    item_bindings = [row for row in sources["source_identity_bindings"] if row["target"]["family"] == "Item"]
    mount_bindings = [row for row in sources["source_identity_bindings"] if row["target"]["family"] == "Mount"]
    outputs = {
        "imports/crystalserver/sources.json": {"schema": "OTERYN_IMPORT_SOURCES/v1", "sources": [row for row in sources["sources"] if row["key"] == "oteryn:source.crystalserver"]},
        "imports/crystalserver/batches.json": {"schema": "OTERYN_IMPORT_BATCHES/v1", "batches": [row for row in imports["batches"] if row["source_repository"] == "zimbadev/crystalserver"]},
        "imports/tibiawiki/sources.json": {"schema": "OTERYN_IMPORT_SOURCES/v1", "sources": [row for row in sources["sources"] if row["key"] == "oteryn:source.tibiawiki"]},
        "imports/tibiawiki/batches.json": {"schema": "OTERYN_IMPORT_BATCHES/v1", "batches": [row for row in imports["batches"] if row["source_repository"] == "tibiawiki.com.br"]},
        "imports/tibiawiki/bindings/items.json": {"schema": "OTERYN_SOURCE_IDENTITY_BINDINGS/v1", "family": "Item", "bindings": item_bindings},
        "imports/tibiawiki/bindings/mounts.json": {"schema": "OTERYN_SOURCE_IDENTITY_BINDINGS/v1", "family": "Mount", "bindings": mount_bindings},
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
        "record_count": len(reference["records"]),
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
            "git_blob_sha": git_blob_sha(LEGACY / "definitions" / "declarations.json"),
            "schema": declarations["schema"],
        },
        "attached_editor_entries": sum(row["target"]["family"] == "Mount" for row in editor["entries"]),
        "attached_source_bindings": len(mount_bindings),
    })

    managed = sorted([*item_shards, mount_relative, "content/items/index.json", "content/cosmetics/mounts/index.json", *outputs.keys()])
    write("content/manifest.json", {
        "schema": "OTERYN_GAME_CONTENT_TREE_MANIFEST/v1",
        "project_revision": REVISION,
        "admission_main": ADMISSION_MAIN,
        "managed_files": [{"path": path} for path in managed],
        "families": {
            "Item": {"records": len(reference["records"]), "index": "content/items/index.json"},
            "Mount": {"records": len(mount_rows), "index": "content/cosmetics/mounts/index.json"},
        },
        "compatibility": {"legacy_root": "content/world", "legacy_mutated": False, "runtime_switch_authorized": False},
    })
    write("content/content.lock.json", {
        "schema": "OTERYN_GAME_CONTENT_TREE_LOCK/v1",
        "project_revision": REVISION,
        "admission_main": ADMISSION_MAIN,
        "legacy_blobs": {
            "reference": git_blob_sha(LEGACY / "definitions" / "reference.json"),
            "declarations": git_blob_sha(LEGACY / "definitions" / "declarations.json"),
            "editor": git_blob_sha(LEGACY / "editor" / "author.json"),
            "sources": git_blob_sha(LEGACY / "provenance" / "sources.json"),
            "imports": git_blob_sha(LEGACY / "provenance" / "imports.json"),
        },
        "family_counts": {"Item": len(reference["records"]), "Mount": len(mount_rows)},
        "item_authoring_counts": {"authoring": len(authoring_by_target), "taxonomy": len(taxonomy_rows), "relation_sources": len(relation_rows)},
        "source_binding_counts": {"Item": len(item_bindings), "Mount": len(mount_bindings)},
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
        "migrated_families": ["Item", "Mount"],
        "legacy_compatibility_root": "content/world",
        "runtime_source": "legacy_until_separately_qualified",
        "next_population_families": [
            "Creature", "Loot", "NPC", "Dialogue", "Service", "Ability", "Effect", "Formula",
            "Quest", "Achievement", "Outfit", "Charm", "Encounter", "Area", "House", "WorldObject",
        ],
    })
    print(f"PASS items={len(reference['records'])} item_shards={len(item_shards)} mounts={len(mount_rows)} authoring={len(authoring_by_target)} taxonomy={len(taxonomy_rows)} relation_sources={len(relation_rows)} relations={sum(len(row['relations']) for row in relation_rows)}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
