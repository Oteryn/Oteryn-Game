"""Generate the source-backed, shared Steamship and Awash contribution bindings."""

import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
BINDINGS_PATH = "content/quests/authored-bindings/world-contributions.json"
OFFERS_PATH = "content/quests/npc-offers/world-contributions.json"
CRYSTAL_REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
CRYSTAL = f"https://github.com/zimbadev/crystalserver/blob/{CRYSTAL_REVISION}/"
ITEMS = CRYSTAL + "data/items/items.xml"
AWASH_SOURCE = "https://www.tibiawiki.com.br/wiki/Awash_World_Change"
STEAMSHIP_SOURCE = "https://tibia.fandom.com/wiki/Steamship_World_Change/Spoiler"
COAL = "oteryn:item.tibia.i12600"
JUNKAR = "oteryn:npc.junkar_thais"
AWASH = "oteryn:quest.awash_world_change"
STEAMSHIP = "oteryn:quest.steamship_world_change"

# Outer-stack ordinals in Crystal's data-global/world/world.otbm.
PUMPS = (
    ("north", 32558, 32007, 12666),
    ("middle", 32560, 32014, 12667),
    ("south", 32567, 32024, 12666),
)


def build(root):
    index = json.loads((root / "content/quests/definitions/index.json").read_text())
    recipes = {}
    for path in index["shards"]:
        for row in json.loads((root / path).read_text())["records"]:
            definition = row["definition"]
            key = definition["identity"]["key"]
            if key in (AWASH, STEAMSHIP):
                chosen = definition["oteryn_recipe"]
                if (
                    not chosen["chosen_data_complete"]
                    or chosen["payload"]["identity"] != definition["identity"]
                ):
                    raise ValueError(f"{key} requires its canonical chosen recipe")
                recipes[key] = chosen["payload"]["recipe"]

    def quest(key, sources, stages):
        recipe = json.dumps(
            recipes[key], ensure_ascii=False, sort_keys=True, separators=(",", ":")
        )
        return {
            "quest": key,
            "recipe_sha256": hashlib.sha256(recipe.encode()).hexdigest(),
            "tracks": [],
            "stages": stages,
            "sources": sources,
        }

    awash_sources = [
        f"crystal:{CRYSTAL_REVISION}:data-global/world/world.otbm",
        CRYSTAL + "data-global/world/world.otbm",
        ITEMS,
        AWASH_SOURCE,
    ]
    pumps = []
    for direction, x, y, appearance in PUMPS:
        key = f"awash-pump-{direction}"
        callback = {
            "key": f"oteryn:quest-callback/authored-{key}",
            "quest": AWASH,
            "trigger": "use",
            "selector": {
                "native": {
                    "position": {"x": x, "y": y, "floor": -9},
                    "ordinal": 1,
                    "appearance": appearance,
                }
            },
            "requires_world_admission": True,
            "reversible": False,
            "passages": [],
            "storage": [],
            "offerings": [],
            "return_on_failure": False,
            "item_on_target": {"item": COAL, "count": 1, "selected_instance": True},
            "sources": awash_sources,
        }
        pumps.append(
            {
                "key": key,
                "completes": False,
                "trigger": {"kind": "map_use", "callback": callback},
            }
        )

    steamship_sources = [
        STEAMSHIP_SOURCE,
        ITEMS,
        CRYSTAL + "data-global/npc/junkar_thais.lua",
    ]
    donation = {
        "key": "oteryn:quest-reward/authored/steamship/donation",
        "quest": STEAMSHIP,
        "npc": JUNKAR,
        "keywords": ["yes"],
        "topic": 1,
        "next_topic": 0,
        "storage": [],
        "take": [{"item": COAL, "count": 1}],
        "repeatable": True,
        "reply": ["Thank you."],
        "failure_reply": ["Sorry, not possible."],
        "sources": steamship_sources,
    }
    offer = {
        "npc": JUNKAR,
        "keywords": ["coal"],
        "topic": 0,
        "next_topic": 1,
        "storage": [],
        "reply": ["Would you like to donate coal for the steamship?"],
    }
    return {
        BINDINGS_PATH: {
            "schema": 1,
            "quests": [
                quest(AWASH, awash_sources, pumps),
                quest(
                    STEAMSHIP,
                    steamship_sources,
                    [
                        {
                            "key": "coal",
                            "completes": False,
                            "trigger": {"kind": "npc_dialogue", "callback": donation},
                        },
                    ],
                ),
            ],
        },
        OFFERS_PATH: {"schema": 1, "offers": [offer]},
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for name, document in build(ROOT).items():
        path = ROOT / name
        expected = (
            json.dumps(document, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
        )
        if args.check:
            if not path.is_file() or path.read_text() != expected:
                raise SystemExit(f"{name} is stale")
        else:
            path.write_text(expected)


if __name__ == "__main__":
    main()
