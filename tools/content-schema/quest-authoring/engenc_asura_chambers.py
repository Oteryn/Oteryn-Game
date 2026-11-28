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

    def cross_door(door, destination):
        # The authorized actor crosses by USE while the native door stays solid.
        door["teleport"] = pos(destination)

    quest["sources"].append(INPUT)
    for number, chamber in enumerate(rows, 2):
        stage = chamber["stage"]
        count, entered, opened = (
            f"{BASE}/{stage}/{name}" for name in ["count", "entered", "opened"]
        )
        for key, maximum in [
            (count, 2),
            (entered, 1),
            (opened, 2 if stage == "s3" else 1),
        ]:
            track(key, maximum)
        source = f"oteryn:quest-callback/engenc/asura/{stage}/entry"
        previous = (
            "oteryn:quest-progress/authored/asura_palace_quest/s1"
            if number == 2
            else f"{BASE}/s{number - 1}/opened"
        )
        prior = gate(previous, 1)
        if stage == "s4":
            prior["maximum"] = 2
        admission = callback(
            source,
            chamber["entry"],
            [
                prior,
                {"track": entered, "minimum": 0, "maximum": 1, "set": 1},
            ],
        )
        admission["effects"] = {
            "transform": None,
            "facing": None,
            "magic": None,
            "encounter": {
                "minimum_level": 1,
                "players": [
                    {
                        "from": pos(chamber.get("staging", chamber["entry"])),
                        "to": pos(chamber["landing"]),
                    }
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
                {
                    "track": opened,
                    "minimum": 0,
                    "maximum": 2 if stage == "s3" else 1,
                    "set": 1,
                    "at_least": True,
                },
            ],
        )
        if door["selector"]["appearance"] != chamber["door_appearance"]:
            raise ValueError("Asura authored door changed")
        x, y, z = chamber["door"]
        cross_door(door, chamber.get("exit", [x + 1, y, z]))
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
    # The occupied fire chamber and Orchid's retained source room share geometry.
    # The same native door admits Orchid after the three defender chambers.
    # Dispatch prefers Orchid when both callbacks at this door are eligible.
    orchid = "oteryn:creature.the_flaming_orchid"
    source = "oteryn:quest-callback/engenc/asura/orchid/entry"
    fire_door = f"{BASE}/s3/opened"
    count = f"{BASE}/orchid/count"
    track(count, 1)
    approach = [32960, 32679, 2]
    ingress = callback(
        source,
        [32959, 32679, 2],
        [
            gate(f"{BASE}/s4/opened", 1),
            {"track": fire_door, "minimum": 1, "maximum": 2, "set": 2},
            gate(count, 0),
        ],
    )
    ingress["selector"] = {
        "position": None,
        "unique": None,
        "action": None,
        "appearance": None,
        "native": ingress["selector"]["native"],
    }
    ingress["activation"] = {"track": fire_door, "minimum": 1, "maximum": 2}
    ingress["sources"].append(
        "https://github.com/zimbadev/crystalserver/blob/"
        "201189f871d23597b82caf88fd611454a2ddefb8/data-global/world/world-monster.xml"
    )
    ingress["effects"] = {
        "encounter": {
            "minimum_level": 1,
            "players": [{"from": pos(approach), "to": pos([32958, 32678, 2])}],
            "monsters": [{"position": pos([32955, 32679, 2]), "creature": orchid}],
            "room_min": pos([32953, 32677, 2]),
            "room_max": pos([32958, 32681, 2]),
            "timing": None,
        }
    }
    quest["stages"].extend(
        [
            {
                "key": "orchid-entry",
                "completes": False,
                "trigger": {"kind": "map_use", "callback": ingress},
            },
            {
                "key": "orchid-kill",
                "completes": False,
                "trigger": {
                    "kind": "kill",
                    "counter": {
                        "key": "oteryn:quest-transition/native-kills/engenc-asura-orchid",
                        "creature": orchid,
                        "track": count,
                        "gate": fire_door,
                        "gate_value": 2,
                        "maximum": 1,
                        "encounter_source": source,
                    },
                },
            },
        ]
    )
    for gem, item, position, appearance, completes in [
        ("sapphire", 3029, [32944, 32678, 1], 2472, False),
        ("ruby", 3030, [32957, 32681, 1], 1993, True),
    ]:
        claimed = f"{BASE}/orchid/{gem}"
        track(claimed, 1)
        reward = callback(
            f"oteryn:quest-callback/engenc/asura/orchid/{gem}",
            position,
            [
                {"track": claimed, "minimum": 0, "maximum": 0, "set": 1},
            ],
        )
        if reward["selector"]["native"]["appearance"] != appearance:
            raise ValueError("Asura treasure source changed")
        reward["sources"].append(
            "https://tibia.fandom.com/index.php?curid=75336&oldid=1138350"
        )
        reward["effects"] = {
            "item_reward": {"item": f"oteryn:item.tibia.i{item}", "count": 3}
        }
        if completes:
            reward["completes"] = True
        quest["stages"].append(
            {
                "key": f"orchid-{gem}",
                "completes": completes,
                "trigger": {"kind": "map_use", "callback": reward},
            }
        )

    library_open = f"{BASE}/orchid/library-open"
    track(library_open, 1)
    library_door = callback(
        "oteryn:quest-callback/engenc/asura/orchid/library-door",
        [32962, 32674, 2],
        [
            gate(count, 1),
            {"track": library_open, "minimum": 0, "maximum": 1, "set": 1},
        ],
    )
    if library_door["selector"]["native"]["appearance"] != 1676:
        raise ValueError("Asura library door source changed")
    library_door["minimum_level"] = 250
    cross_door(library_door, [32962, 32673, 2])
    library_door["sources"].append(
        "https://github.com/zimbadev/crystalserver/blob/"
        "201189f871d23597b82caf88fd611454a2ddefb8/"
        "data-global/scripts/quests/the_secret_library_quest/the_lament_asuras/actions_doors.lua"
    )
    quest["stages"].append(
        {
            "key": "orchid-library-door",
            "completes": False,
            "trigger": {"kind": "map_use", "callback": library_door},
        }
    )

    quest["sources"] = list(dict.fromkeys(quest["sources"]))
    return document
