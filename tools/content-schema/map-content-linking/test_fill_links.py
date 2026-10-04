import unittest

from fill_links import intervals, sheet_for, spawn_core


class SourceBoundaries(unittest.TestCase):
    def test_sprite_sheet_gaps_and_shared_boundaries_are_not_guessed(self):
        rows, starts = intervals(
            [
                {"type": "sprite", "firstspriteid": 0, "lastspriteid": 143},
                {"type": "sprite", "firstspriteid": 200, "lastspriteid": 300},
            ]
        )
        self.assertEqual(sheet_for(143, rows, starts)["firstspriteid"], 0)
        self.assertEqual(sheet_for(200, rows, starts)["firstspriteid"], 200)
        with self.assertRaisesRegex(ValueError, "without a sheet"):
            sheet_for(144, rows, starts)
        with self.assertRaisesRegex(ValueError, "overlapping"):
            intervals(
                [
                    {"type": "sprite", "firstspriteid": 0, "lastspriteid": 10},
                    {"type": "sprite", "firstspriteid": 10, "lastspriteid": 20},
                ]
            )

    def test_unknown_creature_holds_complete_group_without_erasing_other_point(self):
        xml = b'<spawns><spawn centerx="10" centery="10" centerz="7"><monster name="Rat" x="0" y="0" spawntime="60"/><monster name="Unknown" x="1" y="0" spawntime="60"/></spawn></spawns>'
        records, held, counts = spawn_core(xml, {"oteryn:creature.rat"})
        self.assertEqual(records, [])
        self.assertEqual(len(held[0]["raw_points"]), 2)
        self.assertEqual(counts["raw_points"], 2)
        self.assertIn("UNBOUND_CREATURE:Unknown", held[0]["reasons"])

    def test_weighted_source_and_same_cell_do_not_become_two_ordinary_spawns(self):
        xml = b'<spawns><spawn centerx="10" centery="10" centerz="7"><monster name="Rat" x="0" y="0" spawntime="60" chance="10"/><monster name="Rat" x="0" y="0" spawntime="60" chance="90"/></spawn></spawns>'
        records, held, _ = spawn_core(xml, {"oteryn:creature.rat"})
        self.assertEqual(records, [])
        self.assertIn(
            "SAME_CELL_SELECTION_REQUIRES_OWNING_CONTRACT", held[0]["reasons"]
        )
        self.assertEqual(held[0]["raw_points"][1]["chance"], "90")

    def test_different_floor_and_invalid_interval_stay_held(self):
        xml = b'<spawns><spawn centerx="10" centery="10" centerz="7"><monster name="Rat" x="0" y="0" z="8" spawntime="0"/></spawn></spawns>'
        records, held, _ = spawn_core(xml, {"oteryn:creature.rat"})
        self.assertEqual(records, [])
        self.assertIn("POINT_OFF_SOURCE_FLOOR_OR_PLANE", held[0]["reasons"])
        self.assertIn("RESPAWN_DELAY_OUTSIDE_ACCEPTED_RANGE", held[0]["reasons"])

    def test_duplicate_centres_have_distinct_keys_and_native_delay_and_direction(self):
        xml = b'<spawns><spawn centerx="10" centery="10" centerz="7"><monster name="Rat" x="1" y="-1" spawntime="60" direction="3"/></spawn><spawn centerx="10" centery="10" centerz="7"><monster name="Rat" x="0" y="0" spawntime="1"/></spawn></spawns>'
        records, held, counts = spawn_core(xml, {"oteryn:creature.rat"})
        self.assertEqual(held, [])
        self.assertEqual(counts["raw_points"], 2)
        first = records[0]["declaration"]
        self.assertEqual(
            first["points"][0],
            {
                "cell": {"floor": 7, "x": 11, "y": 9},
                "creature": "oteryn:creature.rat",
                "direction": "west",
                "respawn_ms": 60000,
            },
        )
        self.assertEqual(
            records[1]["declaration"]["identity"]["key"], "oteryn:spawn.x10_y10_z7_2"
        )

    def test_untrusted_xml_entities_refuse(self):
        with self.assertRaisesRegex(ValueError, "DTD/entity"):
            spawn_core(b'<!DOCTYPE spawns [<!ENTITY x "Rat">]><spawns/>', set())


if __name__ == "__main__":
    unittest.main()
