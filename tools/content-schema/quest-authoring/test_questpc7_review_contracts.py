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

    def test_jack_reports_after_a_saved_zelos_kill(self):
        rows = quest_file("npc-callbacks/questpc7-programmes")
        passage = row(
            quest_file("questpc7-world-callbacks"), "questpc7-grave-zelos-passage"
        )
        fresh = row(rows, "questpc7-inquisition-commission")
        questline, outfit = fresh["participants"][0]["storage"]
        track = next(
            t
            for q in quest_file("grave-danger-npc-state", "quests")
            for t in q["tracks"]
            if t["key"] == questline["track"]
        )
        passages = next(
            s for s in passage["storage"] if s["track"].endswith("/passages")
        )
        boss = next(s for s in passage["storage"] if s["track"].endswith("/boss_6"))
        for completed_passages, kills, eligible in (
            (passages["minimum"], boss["minimum"] - 1, False),
            (passages["minimum"], boss["minimum"], True),
            (passages["set"], boss["minimum"] - 1, True),
            (passages["set"] + 1, boss["minimum"] - 1, True),
        ):
            for outfit_value in (outfit["minimum"], outfit["set"]):
                with self.subTest(
                    passages=completed_passages, kills=kills, outfit=outfit_value
                ):
                    state = {
                        fresh["storage"][0]["track"]: -1,
                        questline["track"]: track["initial"],
                        outfit["track"]: outfit_value,
                        passages["track"]: completed_passages,
                        boss["track"]: kills,
                    }
                    matching = [
                        c
                        for c in rows
                        if c["npc"] == fresh["npc"]
                        and c["topic"] == 0
                        and "report" in c["keywords"]
                        and holds(c, state)
                        and all(holds(p, state) for p in c.get("participants", []))
                    ]
                    self.assertEqual(len(matching), int(eligible))
                    if matching:
                        callback = matching[0]
                        effects = {
                            s["track"]: s
                            for s in callback["participants"][0]["storage"]
                        }
                        self.assertEqual(
                            effects[questline["track"]]["set"], questline["set"]
                        )
                        self.assertEqual(
                            callback["outfit"] is not None, outfit_value < outfit["set"]
                        )

    def test_golden(self):
        recipe = next(
            r["definition"]
            for r in quest_file("definitions/quests-00100-00199", "records")
            if r["definition"]["identity"]["key"] == "oteryn:quest.golden_outfits_quest"
        )
        selected_prices = [
            r["count"]
            for r in recipe["oteryn_recipe"]["payload"]["recipe"]["stages"]
            if r["kind"] == "use"
        ]
        source_prices = q.golden_source_prices(q.donor_inputs()[q.GOLDEN_NPC_SOURCE])
        for document, suffixes, prices in [
            (
                "shared-reward-callbacks",
                ["l10-golden-base", "l10-golden-boots", "l10-golden-helmet"],
                source_prices,
            ),
            (
                "npc-callbacks/questpc7-programmes",
                [
                    "questpc7-golden-base",
                    "questpc7-golden-gloves",
                    "questpc7-golden-helmet",
                ],
                selected_prices,
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

    def test_golden_owned_milestones_advance_without_repurchase(self):
        offers = quest_file("shared-reward-offers", "offers")
        for document, prefix, middle in [
            ("shared-reward-callbacks", "l10", "boots"),
            ("npc-callbacks/questpc7-programmes", "questpc7", "gloves"),
        ]:
            rows = quest_file(document)
            for suffix in ("base", middle, "helmet"):
                with self.subTest(document=document, milestone=suffix):
                    paid = row(rows, f"{prefix}-golden-{suffix}")
                    progress = paid["storage"][0]
                    (offer,) = [
                        o
                        for o in offers
                        if o["npc"] == paid["npc"]
                        and o["next_topic"] == paid["topic"]
                        and o["storage"]
                        and o["storage"][0]["track"] == progress["track"]
                    ]
                    candidates = [
                        c
                        for c in rows
                        if c["npc"] == offer["npc"]
                        and c["topic"] == offer["next_topic"]
                        and holds(c, {progress["track"]: progress["minimum"]})
                        and c.get("fee") is None
                        and c["outfit"] is None
                    ]
                    self.assertEqual(len(candidates), 1)
                    owned = candidates[0]
                    self.assertTrue(
                        any(
                            word in " ".join(offer["reply"])
                            for word in owned["keywords"]
                        )
                    )
                    for kind in ("outfits", "addons"):
                        self.assertEqual(
                            owned.get("requires_" + kind), paid.get("forbids_" + kind)
                        )
                    self.assertEqual(owned["storage"], paid["storage"])
                    self.assertEqual(owned.get("completes"), paid.get("completes"))

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

    def test_rafzan_legacy_confirmation_obeys_report_clock(self):
        programme = quest_file("npc-callbacks/questpc7-programmes")
        legacy = quest_file("npc-callbacks/m13-guardcatcher")
        offer = row(legacy, "guards-offer")
        (grant,) = [
            c
            for c in legacy
            if c["topic"] == offer["next_topic"] and "yes" in c["keywords"]
        ]
        start = row(programme, "questpc7-goblin-accept-guards")
        report = row(programme, "questpc7-goblin-report-guards")
        (clock,) = [
            op["track"]
            for op in report["state_ops"]
            if op["effect"]["kind"] == "set_now"
        ]
        (guard,) = [op for op in start["state_ops"] if op["track"] == clock]
        post_report = {
            s["track"]: s["set"] for s in start["storage"] + report["storage"]
        }
        offer_stage = offer["storage"][0]
        post_report[offer_stage["track"]] = offer_stage["minimum"]
        self.assertTrue(holds(offer, post_report))
        post_report[offer_stage["track"]] = offer_stage["set"]
        self.assertTrue(holds(grant, post_report))
        self.assertIn(guard, grant.get("state_ops", []))

    def test_rafzan_repeat_accepts_require_first_confirmation(self):
        programme = quest_file("npc-callbacks/questpc7-programmes")
        legacy = quest_file("npc-callbacks/m13-guardcatcher")
        first_offer = row(legacy, "guards-offer")["storage"][0]
        first_grant = row(legacy, "guardcatcher-grant")
        direct = row(programme, "questpc7-goblin-accept-guards")
        confirmed = row(programme, "questpc7-goblin-accept-guards-confirmed")
        for accept in (direct, confirmed):
            state = {guard["track"]: guard["minimum"] for guard in accept["storage"]}
            for stage, eligible in (
                (first_offer["minimum"], False),
                (first_offer["set"], False),
                (first_grant["storage"][0]["set"], True),
            ):
                with self.subTest(callback=accept["key"], stage=stage):
                    state[first_offer["track"]] = stage
                    self.assertEqual(holds(accept, state), eligible)

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
