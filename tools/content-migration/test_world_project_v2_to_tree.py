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
    "Creature": 1319, "Presentation": 2407, "Behavior": 2407, "Loot": 978, "Ability": 5257, "Effect": 3929, "Formula": 4227,
    "NPC": 1088, "Dialogue": 701, "Service.Trade": 307, "Service.Travel": 55,
}
assert lock["source_binding_counts"] == {"Item": 165, "Mount": 252, "Creature": 1319, "NPC": 2296}
assert lock["editor_entry_counts"] == {"Item": 165, "Mount": 252}

paths = [row["path"] for row in manifest["managed_files"]]
assert len(paths) == len(set(paths))
assert sum(path.startswith("content/items/definitions/items-") for path in paths) == 77
assert any(path.startswith("content/cosmetics/mounts/mounts-") for path in paths)
assert any(path.startswith("content/creatures/definitions/creatures-") for path in paths)
assert any(path.startswith("content/npcs/definitions/npcs-") for path in paths)
assert any(path.startswith("content/dialogues/definitions/dialogues-") for path in paths)
assert any(path.startswith("content/services/trade/trade-") for path in paths)
assert any(path.startswith("content/services/travel/travel-") for path in paths)
assert "imports/canary/bindings/creatures.json" in paths
assert "imports/tibiawiki/bindings/creatures.json" in paths
assert all(not path.startswith("content/world/") for path in paths)

assert manifest["families"]["NPC"] == {"records": 1088, "index": "content/npcs/definitions/index.json"}
assert manifest["families"]["Dialogue"] == {"records": 701, "index": "content/dialogues/definitions/index.json"}
assert manifest["families"]["Service.Trade"] == {"records": 307, "index": "content/services/trade/index.json"}
assert manifest["families"]["Service.Travel"] == {"records": 55, "index": "content/services/travel/index.json"}
assert "NPC" in project["migrated_families"] and "Dialogue" in project["migrated_families"] and "Service" in project["migrated_families"]
assert "NPC" not in project["next_population_families"] and "Dialogue" not in project["next_population_families"] and "Service" not in project["next_population_families"]

print(f"PASS managed_files={len(paths)} item_shards=77")
