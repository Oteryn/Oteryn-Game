#!/usr/bin/env python3
import json
from pathlib import Path
R=Path(__file__).resolve().parents[2]
p=json.loads((R/"content/project.json").read_text());m=json.loads((R/"content/manifest.json").read_text());l=json.loads((R/"content/content.lock.json").read_text())
assert p["runtime_source"]=="legacy_until_separately_qualified";assert m["compatibility"]=={"legacy_mutated":False,"legacy_root":"content/world","runtime_switch_authorized":False};assert l["family_counts"]=={"Item":38157,"Mount":252};assert l["source_binding_counts"]=={"Item":165,"Mount":252};paths=[x["path"] for x in m["managed_files"]];assert len(paths)==len(set(paths));assert sum(x.startswith("content/items/definitions/items-") for x in paths)==77;assert all(not x.startswith("content/world/") for x in paths);print("PASS managed_files=%d item_shards=77"%len(paths))
