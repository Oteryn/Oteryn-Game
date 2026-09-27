#!/usr/bin/env python3
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
project = json.loads((ROOT / "content/project.json").read_text(encoding="utf-8"))
manifest = json.loads((ROOT / "content/manifest.json").read_text(encoding="utf-8"))
lock = json.loads((ROOT / "content/content.lock.json").read_text(encoding="utf-8"))

assert project["runtime_source"] == "legacy_until_separately_qualified"
assert manifest["compatibility"] == {
    "legacy_mutated": False,
    "legacy_root": "content/world",
    "runtime_switch_authorized": False,
}
assert lock["family_counts"] == {
    "Item": 38157, "Mount": 252,
    "Creature": 1315, "Presentation": 1335, "Behavior": 1335, "Loot": 976, "Ability": 5240, "Effect": 3908, "Formula": 4211,
}
assert lock["source_binding_counts"] == {"Item": 165, "Mount": 252, "Creature": 1315}
assert lock["editor_entry_counts"] == {"Item": 165, "Mount": 252}

paths = [row["path"] for row in manifest["managed_files"]]
assert len(paths) == len(set(paths))
assert sum(path.startswith("content/items/definitions/items-") for path in paths) == 77
assert any(path.startswith("content/cosmetics/mounts/mounts-") for path in paths)
assert any(path.startswith("content/creatures/definitions/creatures-") for path in paths)
assert "imports/canary/bindings/creatures.json" in paths
assert all(not path.startswith("content/world/") for path in paths)

print(f"PASS managed_files={len(paths)} item_shards=77")
