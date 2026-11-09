"""M30: bind the canonical palace entry only after checking its actual base tile.

The three unbound sealed doors are not three admitted defender rosters. This
producer deliberately supplies no guessed chamber, spawn or door association.
"""

import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
import world_region_codec as codec

OUTPUT = "content/quests/authored-bindings/engenc-asura-palace.json"
QUEST = "oteryn:quest.authored.asura_palace_quest"
TRACK = "oteryn:quest-progress/authored/asura_palace_quest/s1"
ENTRY = "oteryn:quest-callback/m01b/asura-palace/s1/entry"
REGION = "content/world/placements/region-z07-x128-y127.b3"
AREA = "content/world/areas/hunting-places/hunting-places-00000-00444.json"
DEFINITIONS = "content/quests/definitions/quests-00200-00299.json"


def read(root, path):
    return json.loads((root / path).read_bytes())


def entry_binding(root=ROOT):
    definition = next(
        r["definition"]
        for r in read(root, DEFINITIONS)["records"]
        if r["definition"]["identity"]["key"] == QUEST
    )
    recipe = definition["recipe"]
    area = next(
        r["declaration"]
        for r in read(root, AREA)["records"]
        if r["declaration"]["identity"]["key"]
        == "oteryn:area.hunting_place.asura_palace"
    )
    source = area["position"]
    if (source["x"], source["y"], source["floor"]) != (32953, 32685, 7):
        raise ValueError("Asura canonical entry changed")
    tiles = [
        tile
        for _, sector in codec.decode_region((root / REGION).read_bytes())[3]
        for tile in sector
        if tile[:2] == (32953, 32685)
    ]
    palette = read(root, "content/world/placements/index.json")["palette"]
    if len(tiles) != 1 or len(tiles[0][5]) != 1:
        raise ValueError("Asura entry is not one plain native ground tile")
    index, depth, attributes = tiles[0][5][0]
    if (palette[index]["source_item_id"], depth, attributes) != (923, 0, None):
        raise ValueError("Asura entry appearance/ordinal changed")
    if (
        recipe["stages"][0]["kind"] != "explore"
        or not recipe["requirements"]["premium"]
        or recipe["requirements"]["min_level"] != 0
        or recipe["requirements"]["prerequisites"]
    ):
        raise ValueError("Asura entry recipe changed")
    position = {"x": 32953, "y": 32685, "floor": -7}
    callback = {
        "key": ENTRY,
        "quest": QUEST,
        "trigger": "step_in",
        "admission": {"premium": True},
        "selector": {
            "position": position,
            "unique": None,
            "action": None,
            "appearance": 923,
            "native": {"position": position, "ordinal": 0, "appearance": 923},
        },
        "lever": None,
        "reversible": False,
        "requires_world_admission": True,
        "passages": [],
        "teleport": None,
        "storage": [
            {"track": TRACK + "/visited/entry", "minimum": 0, "maximum": 0, "set": 1},
            {"track": TRACK, "minimum": 0, "maximum": 0, "set": 1, "increment": True},
        ],
        "offerings": [],
        "return_on_failure": False,
        "failure_message": None,
        "sources": [
            AREA,
            REGION,
            "OTERYN_CHOSEN_EXPLORE_ENTRY:canonical-area-point;first-cycle-only",
        ],
    }
    digest = hashlib.sha256(
        json.dumps(
            recipe, sort_keys=True, ensure_ascii=False, separators=(",", ":")
        ).encode()
    ).hexdigest()
    return {
        "schema": 1,
        "quests": [
            {
                "quest": QUEST,
                "recipe_sha256": digest,
                "sources": [DEFINITIONS, AREA, REGION],
                "tracks": [
                    {"key": key, "initial": 0, "minimum": 0, "maximum": 1}
                    for key in [TRACK, TRACK + "/visited/entry"]
                ],
                "stages": [
                    {
                        "key": "s1",
                        "completes": False,
                        "trigger": {"kind": "step_in", "callback": callback},
                    }
                ],
            }
        ],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = json.dumps(entry_binding(), indent=2, ensure_ascii=False) + "\n"
    target = ROOT / OUTPUT
    if args.check:
        if target.read_text() != data:
            raise SystemExit("Asura entry binding needs regeneration")
    else:
        target.write_text(data)


if __name__ == "__main__":
    main()
