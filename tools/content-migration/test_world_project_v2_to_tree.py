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
    "Creature": 1316, "Presentation": 2299, "Behavior": 2299, "Loot": 978, "Ability": 5245, "Effect": 3913, "Formula": 4214,
    "NPC": 983, "Service.Trade": 289, "Service.Travel": 53,
}
assert lock["source_binding_counts"] == {"Item": 165, "Mount": 252, "Creature": 1316, "NPC": 2035}
assert lock["editor_entry_counts"] == {"Item": 165, "Mount": 252}

paths = [row["path"] for row in manifest["managed_files"]]
assert len(paths) == len(set(paths))
assert sum(path.startswith("content/items/definitions/items-") for path in paths) == 77
assert any(path.startswith("content/cosmetics/mounts/mounts-") for path in paths)
assert any(path.startswith("content/creatures/definitions/creatures-") for path in paths)
assert any(path.startswith("content/npcs/definitions/npcs-") for path in paths)
assert any(path.startswith("content/services/trade/trade-") for path in paths)
assert any(path.startswith("content/services/travel/travel-") for path in paths)
assert "imports/canary/bindings/creatures.json" in paths
assert all(not path.startswith("content/world/") for path in paths)

assert manifest["families"]["NPC"] == {"records": 983, "index": "content/npcs/definitions/index.json"}
assert manifest["families"]["Service.Trade"] == {"records": 289, "index": "content/services/trade/index.json"}
assert manifest["families"]["Service.Travel"] == {"records": 53, "index": "content/services/travel/index.json"}
assert "NPC" in project["migrated_families"] and "Service" in project["migrated_families"]
assert "NPC" not in project["next_population_families"] and "Service" not in project["next_population_families"]

print(f"PASS managed_files={len(paths)} item_shards=77")
