#!/usr/bin/env python3
"""Validate exact semantic equivalence of the successor Item/Mount tree."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
LEGACY = ROOT / "content" / "world"

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


def validate_item_enrichment(reference: Any, declarations: Any, sources: Any, batches: list[Any],
                             migrated_authoring: dict[tuple[str, str, str], dict[str, Any]]) -> tuple[int, int, int, int]:
    """Round-trip Item authoring/taxonomy/relations and prove per-fact provenance."""
    staged = load(ROOT / "docs/agents/evidence/OTV2-20260925-item-enrichment-wave1-staged.json")
    assignments = load(ROOT / "docs/agents/evidence/OTV2-20260925-tibiawiki-item-master-field-census-v1.json")["family_assignments"]
    legacy_authoring = {target_id(row["item"]): row for row in declarations.get("item_authoring", [])}
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

    staged_items = {target_id(item["target"]): item for item in staged["items"]}
    require(set(staged_items) == set(legacy_authoring), "WAVE1_AUTHORING_TARGETS")
    relation_count = 0
    seen_sources = set()
    for row in relations["records"]:
        key = target_id(row["source"])
        require(key in definitions and key not in seen_sources, "RELATION_SOURCE_UNRESOLVED")
        seen_sources.add(key)
        rulesets = sorted(relation["ruleset"] for relation in row["relations"])
        require(rulesets == staged_items[key]["capability_relations"], "RELATION_DERIVATION_DISAGREES")
        for ruleset in rulesets:
            require((ROOT / ruleset / "index.json").is_file(), f"RELATION_RULESET_UNRESOLVED:{ruleset}")
        relation_count += len(rulesets)
    require(sum(bool(item["capability_relations"]) for item in staged["items"]) == len(seen_sources), "RELATION_COVERAGE")

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
            require(known.get(entry["field_path"]) == entry["value"], f"PROVENANCE_DEFINITION_FACT:{entry['field_path']}")
            fact_count += 1
        for entry in record["authoring_facts"]:
            authoring_value(legacy_authoring[key], entry["field_path"])
            fact_count += 1
        # Blocked contracts stay UNKNOWN even when the source carried a value.
        for blocked in ("physical.weight", "stack.stack_max"):
            require(blocked not in known, f"BLOCKED_FIELD_PROMOTED:{blocked}")
        require(definitions[key].get("semantics", {}).get("equipment", {}).get("state", "UNKNOWN") == "UNKNOWN", "BLOCKED_EQUIPMENT_PROMOTED")
    require(fact_count == staged["counts"]["definition_facts"] + staged["counts"]["authoring_facts"], "PROVENANCE_FACT_COUNT")
    return len(legacy_authoring), len(taxonomy["records"]), relation_count, fact_count


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
    require(lock["family_counts"] == {"Item": 38157, "Mount": 252}, "LOCK_COUNTS")
    require(item_index["record_count"] == 38157 and len(item_index["shards"]) == 77, "ITEM_INDEX")
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

    require(migrated_items == reference["records"] and expected_start == 38157, "ITEM_DEFINITION_ROUNDTRIP")

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

    require(len({target_id(row["identity"]) for row in migrated_items}) == 38157, "ITEM_IDENTITY_UNIQUENESS")
    require(len({
        ("Mount", row["identity"]["key"], row["identity"]["revision"])
        for row in migrated_mounts
    }) == 252, "MOUNT_IDENTITY_UNIQUENESS")

    require(load(ROOT / "imports/tibiawiki/bindings/items.json")["bindings"] == legacy_item_bindings, "IMPORT_ITEM_BINDINGS")
    require(load(ROOT / "imports/tibiawiki/bindings/mounts.json")["bindings"] == legacy_mount_bindings, "IMPORT_MOUNT_BINDINGS")

    authoring_count, taxonomy_count, relation_count, fact_count = validate_item_enrichment(
        reference, declarations, sources, imports_batches(), migrated_authoring)
    print(
        "PASS items=38157 mounts=252 item_editors=165 mount_editors=252 item_bindings=165 mount_bindings=252 "
        f"item_authoring={authoring_count} taxonomy={taxonomy_count} relations={relation_count} provenance_facts={fact_count}"
    )
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
