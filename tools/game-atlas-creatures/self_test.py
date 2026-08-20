#!/usr/bin/env python3
from __future__ import annotations
import importlib.util
import json
from pathlib import Path
import tempfile

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("creature_export", HERE / "export.py")
assert SPEC and SPEC.loader
module = importlib.util.module_from_spec(SPEC); SPEC.loader.exec_module(module)

NPC = '''local internalNpcName = "Alice"\nnpcConfig.outfit = { lookType = 128, lookHead = 1, lookBody = 2, lookLegs = 3, lookFeet = 4, lookAddons = 1 }\n'''
MONSTER = '''local mType = Game.createMonsterType("Rat")\nmonster.outfit = { lookType = 21 }\n'''
NPC_XML = '''<npcs><npc centerx="100" centery="200" centerz="7" radius="2"><npc name="alice" x="1" y="-1" z="7" spawntime="60"/></npc></npcs>'''
MONSTER_XML = '''<monsters><monster centerx="300" centery="400" centerz="8" radius="3"><monster name="RAT" x="-2" y="2" z="8" spawntime="30"/></monster></monsters>'''


def build(root: Path):
    world, npcs, monsters = root / "world", root / "npc", root / "monster"
    world.mkdir(); npcs.mkdir(); monsters.mkdir()
    (world / "base-npc.xml").write_text(NPC_XML, encoding="utf-8")
    (world / "base-monster.xml").write_text(MONSTER_XML, encoding="utf-8")
    (npcs / "alice.lua").write_text(NPC, encoding="utf-8")
    (monsters / "rat.lua").write_text(MONSTER, encoding="utf-8")
    return world, npcs, monsters


def main() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        paths = build(Path(tmp)); first = module.export_creatures(*paths); second = module.export_creatures(*paths)
        assert first == second
        assert first["statistics"] == {"npcs": 1, "monster_spawns": 1, "unresolved": 0, "ambiguous": 0}
        npc = first["npcs"][0]; monster = first["monster_spawns"][0]
        assert npc["position"] == {"x": 101, "y": 199, "floor": -7}
        assert npc["appearance"]["outfit_key"] == "128-1-2-3-4-1"
        assert monster["position"] == {"x": 298, "y": 402, "floor": -8}
        assert monster["appearance"]["look_type"] == 21
        assert "source" not in npc and "source" not in monster
        (paths[1] / "conflict.lua").write_text(NPC.replace("lookType = 128", "lookType = 129"), encoding="utf-8")
        conflict = module.export_creatures(*paths)
        assert conflict["npcs"][0]["resolution_state"] == "AMBIGUOUS"
        assert "appearance" not in conflict["npcs"][0]
        assert conflict["statistics"]["ambiguous"] == 1
    print("game-atlas-creatures self-test: PASS")
    return 0


if __name__ == "__main__": raise SystemExit(main())
