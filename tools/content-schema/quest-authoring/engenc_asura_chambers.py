"""Emit owner-authored Asura encounter admission, bounded kills and personal doors."""

import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "world-authoring"))
import world_region_codec as codec

INPUT = "tools/content-schema/quest-authoring/samples/engenc/asura-chambers.json"
BASE = "oteryn:quest-progress/engenc/asura"


def attach_chambers(document, root):
    quest = document["quests"][0]
    rows = json.loads((root / INPUT).read_text())["chambers"]
    palette = json.loads((root / "content/world/placements/index.json").read_text())[
        "palette"
    ]
    regions = {}

    def pos(value):
        return {"x": value[0], "y": value[1], "floor": -value[2]}

    def selector(value):
        x, y, z = value
        region = f"content/world/placements/region-z{z:02}-x128-y127.b3"
        if region not in regions:
            regions[region] = {
                (t[0], t[1]): t
                for _, sector in codec.decode_region((root / region).read_bytes())[3]
                for t in sector
            }
            quest["sources"].append(region)
        tile = regions[region][x, y]
        ordinal = len(tile[5]) - 1
        appearance = palette[tile[5][ordinal][0]]["source_item_id"]
        return {
            "position": pos(value),
            "unique": None,
            "action": None,
            "appearance": appearance,
            "native": {
                "position": pos(value),
                "ordinal": ordinal,
                "appearance": appearance,
            },
        }

    def track(key, maximum):
        quest["tracks"].append(
            {"key": key, "initial": 0, "minimum": 0, "maximum": maximum}
        )

    def gate(key, value):
        return {
            "track": key,
            "minimum": value,
            "maximum": value,
            "set": value,
            "preserve": True,
        }

    def callback(key, point, storage):
        cb = copy.deepcopy(quest["stages"][0]["trigger"]["callback"])
        cb.update(
            key=key,
            trigger="use",
            selector=selector(point),
            storage=storage,
            sources=[INPUT, "authored per owner rule"],
        )
        return cb

    quest["sources"].append(INPUT)
    for number, chamber in enumerate(rows, 2):
        stage = chamber["stage"]
        count, entered, opened = (
            f"{BASE}/{stage}/{name}" for name in ["count", "entered", "opened"]
        )
        for key, maximum in [(count, 2), (entered, 1), (opened, 1)]:
            track(key, maximum)
        source = f"oteryn:quest-callback/engenc/asura/{stage}/entry"
        previous = (
            "oteryn:quest-progress/authored/asura_palace_quest/s1"
            if number == 2
            else f"{BASE}/s{number - 1}/opened"
        )
        admission = callback(
            source,
            chamber["entry"],
            [
                gate(previous, 1),
                {"track": entered, "minimum": 0, "maximum": 0, "set": 1},
            ],
        )
        admission["effects"] = {
            "transform": None,
            "facing": None,
            "magic": None,
            "encounter": {
                "minimum_level": 1,
                "players": [
                    {"from": pos(chamber["entry"]), "to": pos(chamber["landing"])}
                ],
                "monsters": [
                    {"position": pos(p), "creature": chamber["creature"]}
                    for p in chamber["defenders"]
                ],
                "room_min": pos(chamber["room_min"]),
                "room_max": pos(chamber["room_max"]),
                "timing": None,
            },
        }
        door = callback(
            f"oteryn:quest-callback/engenc/asura/{stage}/door",
            chamber["door"],
            [
                gate(entered, 1),
                gate(count, 2),
                {"track": opened, "minimum": 0, "maximum": 0, "set": 1},
            ],
        )
        if door["selector"]["appearance"] != chamber["door_appearance"]:
            raise ValueError("Asura authored door changed")
        door["effects"] = {
            "transform": chamber["door_appearance"] + 1,
            "facing": None,
            "magic": None,
        }
        quest["stages"].extend(
            [
                {
                    "key": f"{stage}-entry",
                    "completes": False,
                    "trigger": {"kind": "map_use", "callback": admission},
                },
                {
                    "key": stage,
                    "completes": False,
                    "trigger": {
                        "kind": "kill",
                        "counter": {
                            "key": f"oteryn:quest-transition/native-kills/engenc-asura-{stage}",
                            "creature": chamber["creature"],
                            "track": count,
                            "gate": entered,
                            "gate_value": 1,
                            "maximum": 2,
                            "encounter_source": source,
                        },
                    },
                },
                {
                    "key": f"{stage}-door",
                    "completes": False,
                    "trigger": {"kind": "map_use", "callback": door},
                },
            ]
        )
    quest["sources"] = list(dict.fromkeys(quest["sources"]))
    return document
