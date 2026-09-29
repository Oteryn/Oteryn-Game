#!/usr/bin/env python3
import json
import re
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
    "Creature": 1463, "Presentation": 2551, "Behavior": 2551, "Loot": 1026, "Ability": 5841, "Effect": 4447, "Formula": 4761,
    "NPC": 1088, "Dialogue": 701, "Service.Trade": 322, "Service.Travel": 55, "Encounter": 58,
}
assert lock["source_binding_counts"] == {"Item": 165, "Mount": 252, "Creature": 1463, "Encounter": 58, "NPC": 2330}
assert lock["editor_entry_counts"] == {"Item": 165, "Mount": 252}

paths = [row["path"] for row in manifest["managed_files"]]
assert len(paths) == len(set(paths))
shard_name = re.compile(r"-\d{5}-\d{5}\.json$")
for directory in {(ROOT / path).parent for path in paths if shard_name.search(path)}:
    stale = sorted(file.relative_to(ROOT).as_posix() for file in directory.glob("*.json")
                   if shard_name.search(file.name) and file.relative_to(ROOT).as_posix() not in paths)
    assert not stale, stale
assert sum(path.startswith("content/items/definitions/items-") for path in paths) == 77
assert any(path.startswith("content/cosmetics/mounts/mounts-") for path in paths)
assert any(path.startswith("content/creatures/definitions/creatures-") for path in paths)
assert any(path.startswith("content/npcs/definitions/npcs-") for path in paths)
assert any(path.startswith("content/dialogues/definitions/dialogues-") for path in paths)
assert any(path.startswith("content/encounters/definitions/encounters-") for path in paths)
assert any(path.startswith("content/services/trade/trade-") for path in paths)
assert any(path.startswith("content/services/travel/travel-") for path in paths)
assert "imports/canary/bindings/creatures.json" in paths
assert "imports/tibiawiki/bindings/creatures.json" in paths
assert all(not path.startswith("content/world/") for path in paths)

assert manifest["families"]["NPC"] == {"records": 1088, "index": "content/npcs/definitions/index.json"}
assert manifest["families"]["Dialogue"] == {"records": 701, "index": "content/dialogues/definitions/index.json"}
assert manifest["families"]["Service.Trade"] == {"records": 322, "index": "content/services/trade/index.json"}
assert manifest["families"]["Service.Travel"] == {"records": 55, "index": "content/services/travel/index.json"}
assert "NPC" in project["migrated_families"] and "Dialogue" in project["migrated_families"] and "Service" in project["migrated_families"]
assert "NPC" not in project["next_population_families"] and "Dialogue" not in project["next_population_families"] and "Service" not in project["next_population_families"]

print(f"PASS managed_files={len(paths)} item_shards=77")
