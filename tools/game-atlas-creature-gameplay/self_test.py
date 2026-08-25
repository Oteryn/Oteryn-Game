#!/usr/bin/env python3
from __future__ import annotations
import importlib.util
from pathlib import Path
import tempfile

HERE = Path(__file__).resolve().parent
EXPORT_PATH = HERE / "export.py"
assert EXPORT_PATH.exists(), "creature gameplay producer missing"
SPEC = importlib.util.spec_from_file_location("gameplay_export", EXPORT_PATH)
assert SPEC and SPEC.loader
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


def _write(path: Path, text: str) -> None:
    path.write_text(text, encoding="utf-8")


def main() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        npc_root = root / "npc"; monster_root = root / "monster"
        npc_root.mkdir(); monster_root.mkdir()
        _write(npc_root / "alice.lua", (HERE / "fixtures/complete-npc.lua").read_text(encoding="utf-8"))
        _write(monster_root / "dragon.lua", (HERE / "fixtures/complete-monster.lua").read_text(encoding="utf-8"))
        product = module.export_gameplay_profiles(npc_root, monster_root)
        npc = product["npcs"][0]; monster = product["monsters"][0]
        assert npc["entity_id"] == "npc-entity:9e81bffc58a22ac102bfc5135d7a1c15"
        assert npc["shop"]["state"] == "COMPLETE"
        assert [(x["item_name"], x["unit_price"]) for x in npc["shop"]["sells"]] == [("health potion", 50)]
        assert [(x["item_name"], x["unit_price"]) for x in npc["shop"]["buys"]] == [("health potion", 25), ("rope", 15)]
        assert npc["services"] == {"state": "COMPLETE", "values": ["bank", "shop", "travel"]}
        assert npc["travel"]["state"] == "COMPLETE"
        assert npc["travel"]["destinations"] == [{"label":"Thais","position":{"x":32369,"y":32241,"floor":-7},"price":120,"currency":"gold"}]
        assert monster["loot"]["state"] == "COMPLETE"
        assert monster["loot"]["entries"][0]["chance_ppm"] == 800000
        assert monster["loot"]["entries"][0]["min_count"] == 1 and monster["loot"]["entries"][0]["max_count"] == 100
        assert monster["stats"] == {"state":"COMPLETE","health":1000,"experience":700,"armor":25,"defense":20,"speed":200}
        assert monster["resistances"] == {"state":"COMPLETE","elements":[{"type":"fire","percent":-10},{"type":"ice","percent":20}],"immunities":["paralyze"]}
        refs = {row["item_ref"]: row for row in product["referenced_items"]}
        assert monster["loot"]["entries"][0]["item_ref"] in refs
        assert refs[monster["loot"]["entries"][0]["item_ref"]]["appearance_ref"] == {"kind":"client-id","id":3031}

        _write(npc_root / "alice.lua", 'local internalNpcName = "Alice"\nlocal npcConfig = {}\nnpcConfig.shop = {}\n')
        _write(monster_root / "dragon.lua", 'local mType = Game.createMonsterType("Test Dragon")\nlocal monster = {}\nmonster.loot = {}\n')
        empty = module.export_gameplay_profiles(npc_root, monster_root)
        assert empty["npcs"][0]["shop"] == {"state":"COMPLETE","sells":[],"buys":[],"reason_codes":[]}
        assert empty["monsters"][0]["loot"] == {"state":"COMPLETE","entries":[],"reason_codes":[]}
        assert empty["npcs"][0]["travel"]["state"] == "UNKNOWN"
        assert empty["monsters"][0]["stats"]["state"] == "UNKNOWN"

        _write(npc_root / "alice.lua", (HERE / "fixtures/partial-npc.lua").read_text(encoding="utf-8"))
        _write(monster_root / "dragon.lua", (HERE / "fixtures/partial-monster.lua").read_text(encoding="utf-8"))
        partial = module.export_gameplay_profiles(npc_root, monster_root)
        pn = partial["npcs"][0]; pm = partial["monsters"][0]
        assert pn["shop"]["state"] == "PARTIAL" and len(pn["shop"]["sells"]) == 1
        assert "DYNAMIC_SHOP_MUTATION_UNSUPPORTED" in pn["shop"]["reason_codes"]
        assert pn["travel"]["state"] == "PARTIAL" and pn["travel"]["destinations"] == []
        assert pm["loot"]["state"] == "PARTIAL" and len(pm["loot"]["entries"]) == 1
        assert "DYNAMIC_LOOT_MUTATION_UNSUPPORTED" in pm["loot"]["reason_codes"]
        assert pm["resistances"]["state"] == "PARTIAL"

        _write(npc_root / "alice.lua", 'local internalNpcName = "Alice"\nlocal npcConfig = {}\n')
        _write(monster_root / "dragon.lua", 'local mType = Game.createMonsterType("Test Dragon")\nlocal monster = {}\n')
        unknown = module.export_gameplay_profiles(npc_root, monster_root)
        assert unknown["npcs"][0]["shop"]["state"] == "UNKNOWN"
        assert unknown["monsters"][0]["loot"]["state"] == "UNKNOWN"

    print("game-atlas-creature-gameplay self-test: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())