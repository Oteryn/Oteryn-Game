#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import sys
import tempfile

MODULE_PATH = Path(__file__).with_name("export.py")
spec = importlib.util.spec_from_file_location("game_atlas_semantic_search_export", MODULE_PATH)
assert spec is not None and spec.loader is not None
module = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = module
spec.loader.exec_module(module)


def fixture_creatures() -> dict[str, object]:
    return {
        "contract_id": "oteryn-game-atlas-export-v1",
        "capability": "static-creatures-v1",
        "semantic_digest": "sha256:" + "1" * 64,
        "npcs": [{
            "kind": "npc",
            "name": "Sam",
            "record_id": "npc:fixture-sam",
            "position": {"x": 32361, "y": 32198, "floor": -7},
            "origin": "base-map",
            "resolution_state": "RESOLVED",
        }],
        "monster_spawns": [{
            "kind": "monster",
            "name": "Rat",
            "record_id": "monster:fixture-rat",
            "position": {"x": 32370, "y": 32220, "floor": -7},
            "origin": "base-map",
            "resolution_state": "RESOLVED",
        }],
    }


def run() -> None:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        npc_root = root / "npc"
        npc_root.mkdir()
        (npc_root / "sam.lua").write_text(
            'local internalNpcName = "Sam"\n'
            'npcConfig.shop = { { itemName = "axe", clientId = 1, buy = 1 } }\n',
            encoding="utf-8",
        )
        navigation = [{
            "kind": "town",
            "label": "Thais",
            "position": {"x": 32369, "y": 32241, "floor": -7},
            "source_family": "town",
        }]
        first = module.build_source(fixture_creatures(), npc_root, navigation)
        second = module.build_source(fixture_creatures(), npc_root, navigation)
        assert first == second
        assert first["semantic_digest"] == second["semantic_digest"]
        assert first["input_floor_aliases"]["7"] == -7
        assert first["counts"] == {"records": 3, "kinds": {"monster": 1, "npc": 1, "town": 1}}
        sam = next(record for record in first["records"] if record["label"] == "Sam")
        assert sam["kind"] == "npc"
        assert sam["position"] == {"x": 32361, "y": 32198, "floor": -7}
        assert "shop" in sam["capabilities"]
        assert sam["provenance"]["service_resolution_state"] == "RESOLVED"
        thais = next(record for record in first["records"] if record["label"] == "Thais")
        assert thais["kind"] == "town"
        assert thais["position"] == {"x": 32369, "y": 32241, "floor": -7}
        assert thais["bounds"] is None
        assert thais["id"].startswith("semantic-record:")
        serialized = json.dumps(first, sort_keys=True)
        assert "action_id" not in serialized and "unique_id" not in serialized
        try:
            bad = fixture_creatures()
            bad["contract_id"] = "wrong"
            module.build_source(bad, npc_root, navigation)
        except module.ExportError:
            pass
        else:
            raise AssertionError("unsupported producer contract did not fail closed")


if __name__ == "__main__":
    run()
    print("game-atlas-semantic-search self-test: PASS")
