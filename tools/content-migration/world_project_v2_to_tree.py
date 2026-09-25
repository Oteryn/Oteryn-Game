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
ADMISSION_MAIN = "2389c6671000b8b0efe341540a62e303e307ad15"
REVISION = "tree-items-mounts-r1"

def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")

def load(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))

def write(relative: str, value: Any) -> int:
    data = canonical_bytes(value)
    path = ROOT / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    return len(data)

def target_id(target: dict[str, Any]) -> tuple[str, str, str]:
    return (target["family"], target["key"], target["revision"])

def git_blob_sha(path: Path) -> str:
    data = path.read_bytes()
    return hashlib.sha1(b"blob " + str(len(data)).encode("ascii") + b"\0" + data).hexdigest()

def main() -> int:
    reference = load(LEGACY / "definitions" / "reference.json")
    declarations = load(LEGACY / "definitions" / "declarations.json")
    editor = load(LEGACY / "editor" / "author.json")
    sources = load(LEGACY / "provenance" / "sources.json")
    imports = load(LEGACY / "provenance" / "imports.json")

    editors = {target_id(row["target"]): row for row in editor["entries"]}
    bindings: dict[tuple[str, str, str], list[dict[str, Any]]] = {}
    for row in sources["source_identity_bindings"]:
        bindings.setdefault(target_id(row["target"]), []).append(row)
    for rows in bindings.values():
        rows.sort(key=canonical_bytes)

    sizes: dict[str, int] = {}
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
            rows.append(row)
        end = start + len(rows) - 1
        relative = f"content/items/definitions/items-{start:05d}-{end:05d}.json"
        item_shards.append(relative)
        sizes[relative] = write(relative, {
            "schema": "OTERYN_ITEM_AUTHORING_SHARD/v1",
            "family": "Item",
            "source_legacy_role": "content/world/definitions/reference.json",
            "shard": {"index": start // ITEM_SHARD_SIZE, "start": start, "end": end, "count": len(rows)},
            "records": rows,
        })

    mount_rows = []
    for declaration in declarations["records"]:
        target = {"family": "Mount", "key": declaration["identity"]["key"], "revision": declaration["identity"]["revision"]}
        key = target_id(target)
        row: dict[str, Any] = {"declaration": declaration}
        if key in editors:
            row["editor"] = editors[key]
        if key in bindings:
            row["source_bindings"] = bindings[key]
        mount_rows.append(row)
    mount_relative = f"content/cosmetics/mounts/mounts-00000-{len(mount_rows)-1:05d}.json"
    sizes[mount_relative] = write(mount_relative, {
        "schema": "OTERYN_MOUNT_AUTHORING_SHARD/v1",
        "family": "Mount",
        "source_legacy_role": "content/world/definitions/declarations.json",
        "shard": {"index": 0, "start": 0, "end": len(mount_rows) - 1, "count": len(mount_rows)},
        "records": mount_rows,
    })

    item_bindings = [row for row in sources["source_identity_bindings"] if row["target"]["family"] == "Item"]
    mount_bindings = [row for row in sources["source_identity_bindings"] if row["target"]["family"] == "Mount"]
    outputs = {
        "imports/crystalserver/sources.json": {"schema": "OTERYN_IMPORT_SOURCES/v1", "sources": [row for row in sources["sources"] if row["key"] == "oteryn:source.crystalserver"]},
        "imports/crystalserver/batches.json": {"schema": "OTERYN_IMPORT_BATCHES/v1", "batches": [row for row in imports["batches"] if row["source_repository"] == "zimbadev/crystalserver"]},
        "imports/tibiawiki/sources.json": {"schema": "OTERYN_IMPORT_SOURCES/v1", "sources": [row for row in sources["sources"] if row["key"] == "oteryn:source.tibiawiki"]},
        "imports/tibiawiki/batches.json": {"schema": "OTERYN_IMPORT_BATCHES/v1", "batches": [row for row in imports["batches"] if row["source_repository"] == "tibiawiki.com.br"]},
        "imports/tibiawiki/bindings/items.json": {"schema": "OTERYN_SOURCE_IDENTITY_BINDINGS/v1", "family": "Item", "bindings": item_bindings},
        "imports/tibiawiki/bindings/mounts.json": {"schema": "OTERYN_SOURCE_IDENTITY_BINDINGS/v1", "family": "Mount", "bindings": mount_bindings},
    }
    for relative, value in outputs.items():
        sizes[relative] = write(relative, value)

    sizes["content/items/index.json"] = write("content/items/index.json", {
        "schema": "OTERYN_FAMILY_INDEX/v1",
        "family": "Item",
        "record_count": len(reference["records"]),
        "shard_size": ITEM_SHARD_SIZE,
        "shards": [{"path": path, "bytes": sizes[path]} for path in item_shards],
        "legacy_source": {
            "path": "content/world/definitions/reference.json",
            "git_blob_sha": git_blob_sha(LEGACY / "definitions" / "reference.json"),
            "schema": reference["schema"],
            "world_id": reference["world_id"],
            "coordinate_frame": reference["coordinate_frame"],
        },
        "attached_editor_entries": sum(row["target"]["family"] == "Item" for row in editor["entries"]),
        "attached_source_bindings": len(item_bindings),
    })
    sizes["content/cosmetics/mounts/index.json"] = write("content/cosmetics/mounts/index.json", {
        "schema": "OTERYN_FAMILY_INDEX/v1",
        "family": "Mount",
        "record_count": len(mount_rows),
        "shards": [{"path": mount_relative, "bytes": sizes[mount_relative]}],
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
        "managed_files": [{"path": path, "bytes": sizes[path]} for path in managed],
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
    print(f"PASS items={len(reference['records'])} item_shards={len(item_shards)} mounts={len(mount_rows)}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
