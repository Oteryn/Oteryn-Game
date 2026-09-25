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
    item_editors: list[Any] = []
    item_bindings: list[Any] = []
    expected_start = 0
    for shard_ref in item_index["shards"]:
        payload = load(ROOT / shard_ref["path"])
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
        expected_start = payload["shard"]["end"] + 1

    require(migrated_items == reference["records"] and expected_start == 38157, "ITEM_DEFINITION_ROUNDTRIP")

    mount_payload = load(ROOT / mount_index["shards"][0]["path"])
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

    print("PASS items=38157 mounts=252 item_editors=165 mount_editors=252 item_bindings=165 mount_bindings=252")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
