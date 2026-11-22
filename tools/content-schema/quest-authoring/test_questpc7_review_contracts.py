import json
import unittest

import qengine_rewards as q

ROOT = q.ROOT


def read(path):
    return json.loads((ROOT / path).read_bytes())


def quest_file(path, field="callbacks"):
    return read("content/quests/" + path + ".json")[field]


def row(rows, suffix):
    return next(row for row in rows if row["key"].endswith("/" + suffix))


def holds(row, state):
    return all(
        guard["minimum"] <= state[guard["track"]] <= guard["maximum"]
        for guard in row["storage"] + row.get("requires", [])
    )


class Review(unittest.TestCase):
    def test_well(self):
        well = row(quest_file("shared-access-callbacks"), "ghoul-room-well")
        selector = well["selector"]
        palette = read("content/world/placements/index.json")["palette"]
        codec = q.load_module(
            "questpc7_review_region_codec",
            "tools/content-schema/world-authoring/world_region_codec.py",
        )
        region = codec.decode_region(
            (ROOT / "content/world/placements/region-z13-x126-y125.b3").read_bytes()
        )
        (attrs,) = [
            attrs
            for _, tiles in region[3]
            for x, y, _, _, _, items in tiles
            if (x, y) == (selector["position"]["x"], selector["position"]["y"])
            for i, depth, attrs in items
            if depth == 0 and palette[i]["source_item_id"] == selector["appearance"]
        ]
        self.assertEqual(selector["action"], 1000)
        self.assertEqual(attrs["action"], selector["action"])

    def test_jack(self):
        callback = row(
            quest_file("npc-callbacks/questpc7-programmes"),
            "questpc7-inquisition-commission",
        )
        source = quest_file("grave-danger-final-source", "transitions")[0]
        updates = {
            s["track"]: s
            for p in callback.get("participants", [])
            for s in p["storage"]
        }
        for effect in source["effects"]:
            self.assertIn(effect["track"], updates)
            storage = updates[effect["track"]]
            self.assertEqual(storage["set"], effect["effect"]["value"])
            if effect["from"]["op"] == "EQ":
                self.assertEqual(
                    (storage["minimum"], storage["maximum"]),
                    (effect["from"]["value"],) * 2,
                )

    def test_golden(self):
        recipe = next(
            r["definition"]
            for r in quest_file("definitions/quests-00100-00199", "records")
            if r["definition"]["identity"]["key"] == "oteryn:quest.golden_outfits_quest"
        )
        prices = [
            r["count"]
            for r in recipe["oteryn_recipe"]["payload"]["recipe"]["stages"]
            if r["kind"] == "use"
        ]
        for document, suffixes in [
            (
                "shared-reward-callbacks",
                ["l10-golden-base", "l10-golden-boots", "l10-golden-helmet"],
            ),
            (
                "npc-callbacks/questpc7-programmes",
                [
                    "questpc7-golden-base",
                    "questpc7-golden-gloves",
                    "questpc7-golden-helmet",
                ],
            ),
        ]:
            with self.subTest(document=document):
                paid = [row(quest_file(document), suffix) for suffix in suffixes]
                self.assertEqual([c["fee"]["gold"] for c in paid], prices)
                self.assertEqual(
                    [c["storage"][0]["minimum"] for c in paid], list(range(3))
                )
                self.assertEqual([c["outfit"]["addons"] for c in paid], list(range(3)))

    def test_legacy_golden_completion(self):
        base = row(quest_file("shared-reward-callbacks"), "l10-golden-base")
        track = base["storage"][0]["track"]
        offers = quest_file("shared-reward-offers", "offers")
        (offer,) = [
            o
            for o in offers
            if o["npc"] == base["npc"] and "outfit" in o["keywords"] and o["topic"] == 0
        ]
        self.assertTrue(
            any(
                c.get("completes")
                and c["npc"] == base["npc"]
                and c["topic"] == offer["next_topic"]
                and "yes" in c["keywords"]
                and c.get("fee") is None
                and holds(c, {track: 3})
                for c in quest_file("shared-reward-callbacks")
            )
        )

    def test_legacy_glooth_completion(self):
        rows = quest_file("shared-reward-callbacks")
        first = row(rows, "l3-glooth-engineer-first-low")
        second = row(rows, "l3-glooth-engineer-second")
        state = {
            s["track"]: 6 if s["minimum"] == 6 else 1
            for s in second["requires"] + first["storage"] + second["storage"]
        }
        self.assertTrue(
            any(
                c.get("completes")
                and c["achievement"]
                and "addon" in c["keywords"]
                and c["npc"] == first["npc"]
                and c["outfit"] is None
                and holds(
                    c,
                    state
                    | {s["track"]: -1 for s in c["storage"] if s["track"] not in state},
                )
                for c in rows
            )
        )

    def test_rafzan_starts(self):
        programme = quest_file("npc-callbacks/questpc7-programmes")
        guards = quest_file("npc-callbacks/m13-guardcatcher")
        offers = quest_file("shared-reward-offers", "offers")
        for keyword, progress in zip(
            ("advertising", "perfume", "guards", "busy", "destroy", "kill"),
            ("signs", "odours", "captures", "urine", "trunks", "stalkers"),
        ):
            with self.subTest(keyword=keyword):
                track = "oteryn:quest-progress/questpc7/goblin/" + progress
                topics = {
                    o["next_topic"]
                    for o in offers + guards
                    if o["npc"] == "oteryn:npc.rafzan"
                    and keyword in o["keywords"]
                    and o["topic"] == 0
                }
                self.assertTrue(
                    any(
                        c["topic"] in topics
                        and "yes" in c["keywords"]
                        and any(
                            s["track"] == track and s["minimum"] <= -1 and s["set"] == 0
                            for s in c["storage"]
                        )
                        for c in programme + guards
                    )
                )

    def test_rafzan_gas(self):
        rows = [
            r
            for r in quest_file("shared-access-callbacks")
            if "/rafzan-swamp-gas-" in r["key"]
        ]
        slug = row(
            quest_file("npc-callbacks/m13-goblin-capture"),
            "questpc7-goblin-slug-sample",
        )["storage"][1]
        self.assertEqual(len(rows), 3)
        for r in rows:
            self.assertTrue(
                any(
                    g["track"] == slug["track"] and g["minimum"] == slug["set"]
                    for g in r["storage"]
                ),
                r["key"],
            )

    def test_rafzan_tools(self):
        rows = [
            r
            for r in quest_file("questpc7-world-callbacks")
            if "/questpc7-goblin-" in r["key"]
            and r["selector"]["appearance"] in (17677, 17679)
        ]
        self.assertEqual(len(rows), 9)
        for r in rows:
            self.assertTrue(r["item_on_target"].get("selected_instance"), r["key"])


if __name__ == "__main__":
    unittest.main()
