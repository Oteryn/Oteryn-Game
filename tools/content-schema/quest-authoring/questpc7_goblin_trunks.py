"""Three native junk trunks with donor decay and third-use hammer breakage."""

import hashlib
import json
import sys


def build(root):
    region = "content/world/placements/region-z10-x129-y125.b3"
    raw = (root / region).read_bytes()
    digest = "49f46bde1d2fd6d8e3083ba3aea1e02bd60b2199efe086d3760178f8a7e348a1"
    if hashlib.sha256(raw).hexdigest() != digest:
        raise ValueError("QUESTPC7_GOBLIN_TRUNK_REGION_DRIFT")
    sys.path.insert(0, str(root / "tools/content-schema/world-authoring"))
    import world_region_codec as codec

    palette = json.loads((root / "content/world/placements/index.json").read_bytes())[
        "palette"
    ]
    cells = {
        (x, y): (flags, house, [palette[i]["source_item_id"] for i, _, _ in items])
        for _, tiles in codec.decode_region(raw)[3]
        for x, y, flags, house, _, items in tiles
    }
    sites = [
        (33101, 32047, [9528, 17677]),
        (33143, 32051, [9531, 16298, 17677]),
        (33136, 32076, [9529, 17677]),
    ]
    if any(cells[(x, y)] != (0, 0, items) for x, y, items in sites):
        raise ValueError("QUESTPC7_GOBLIN_TRUNK_NATIVE_CELL_DRIFT")
    path = "content/quests/definitions/quests-00100-00199.json"
    canonical = (root / path).read_bytes()
    index, definition = next(
        (i, r["definition"])
        for i, r in enumerate(json.loads(canonical)["records"])
        if r["definition"]["identity"]["key"] == "oteryn:quest.goblin_merchant_quest"
    )
    stage = definition["oteryn_recipe"]["payload"]["recipe"]["stages"][5]
    if (stage["kind"], stage["count"]) != ("use", 3):
        raise ValueError("QUESTPC7_GOBLIN_TRUNK_RECIPE_DRIFT")
    track = "oteryn:quest-progress/questpc7/goblin/"
    result = []
    for i, (x, y, _) in enumerate(sites, 1):
        for last in (False, True):
            lo, hi = (2, 2) if last else (0, 1)
            result.append(
                {
                    "key": f"oteryn:quest-callback/questpc7-goblin-trunk-{i}"
                    + ("-last" if last else ""),
                    "quest": definition["identity"]["key"],
                    "trigger": "use",
                    "selector": {
                        "position": {"x": x, "y": y, "floor": -10},
                        "appearance": 17677,
                        "unique": None,
                        "action": None,
                    },
                    "lever": None,
                    "reversible": False,
                    "passages": [],
                    "teleport": None,
                    "storage": [
                        {
                            "track": track + "programme",
                            "minimum": 0,
                            "maximum": 6,
                            "set": 0,
                            "preserve": True,
                        },
                        {
                            "track": track + "trunks",
                            "minimum": lo,
                            "maximum": hi,
                            "set": hi + 1,
                            "add": 1,
                        },
                        {
                            "track": track + f"trunk-site-{i}",
                            "minimum": -1,
                            "maximum": -1,
                            "set": 1,
                        },
                    ],
                    "activation": {
                        "track": track + "trunks",
                        "minimum": lo,
                        "maximum": hi,
                    },
                    "minimum_level": None,
                    "experience": None,
                    "monk_reward": None,
                    "offerings": [],
                    "requires_world_admission": True,
                    "item_on_target": {
                        "item": "oteryn:item.tibia.i17676",
                        "count": 1,
                        "replacement": None,
                        "target_replacement": None,
                        "reusable": not last,
                        "selected_instance": True,
                    },
                    "effects": {"transform": 17678, "reset_after_ms": 240000},
                    "return_on_failure": False,
                    "failure_message": None,
                    "success_message": "You smash the last trunk and your hammer breaks."
                    if last
                    else "You smash the junk trunk.",
                    "sources": [
                        f"{region}:1:sha256:{digest}",
                        f"canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(canonical).hexdigest()}",
                        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/items/items.xml:item-17678",
                        "https://www.tibiawiki.com.br/index.php?title=Task:Rafzan&oldid=401277",
                    ],
                }
            )
    return result
