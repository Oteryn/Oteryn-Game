import pathlib
import unittest

import quest_explore_area_candidates as tool


ROOT = pathlib.Path(__file__).resolve().parents[3]


class QuestExploreAreaCandidatesTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = tool.expected(ROOT)
        cls.by_stage = {
            (row["quest"], row["stage_key"]): row
            for row in cls.packet["records"]
        }

    def test_population_is_exact_and_fail_closed(self):
        self.assertEqual(
            {
                "explore_stages": 239,
                "canonical_areas": 970,
                "statuses": {
                    "AMBIGUOUS_MULTIPLE_AREAS": 34,
                    "EXACT_SINGLE_AREA_ONLY_TARGET": 10,
                    "EXACT_SINGLE_AREA_WITH_OTHER_TARGETS": 11,
                    "NO_EXACT_AREA": 184,
                },
                "exact_area_candidates": 21,
                "clean_single_target_candidates": 10,
                "native_spatial_bindings": 0,
            },
            self.packet["summary"],
        )
        self.assertFalse(self.packet["runtime_admitted"])
        self.assertTrue(
            all(not row["runtime_admitted"] for row in self.packet["records"])
        )

    def test_clean_single_target_area_candidate(self):
        row = self.by_stage[("oteryn:quest.hidden_threats_quest", "s1")]
        self.assertEqual("EXACT_SINGLE_AREA_ONLY_TARGET", row["status"])
        self.assertEqual(["Corym Mines"], row["raw_targets"])
        self.assertEqual(
            "oteryn:area.hunting_place.corym_mines",
            row["candidate"]["area_ref"]["key"],
        )
        self.assertEqual([], row["unmatched_targets"])
        self.assertIsNone(row["native_spatial_binding"])
        self.assertIn("AREA_ENTRY_SEMANTICS_NOT_PROVEN", row["holds"])

    def test_one_area_does_not_erase_other_unresolved_targets(self):
        row = self.by_stage[("oteryn:quest.cults_of_tibia_quest", "s2")]
        self.assertEqual(
            "EXACT_SINGLE_AREA_WITH_OTHER_TARGETS", row["status"]
        )
        self.assertEqual(
            ["Barkless hideout", "Misguided hideout"], row["raw_targets"]
        )
        self.assertEqual(
            "oteryn:content.area.subregion.misguided_hideout",
            row["candidate"]["area_ref"]["key"],
        )
        self.assertEqual(["Barkless hideout"], row["unmatched_targets"])
        self.assertIn("OTHER_STAGE_TARGETS_UNRESOLVED", row["holds"])

    def test_duplicate_area_names_are_held_as_ambiguous(self):
        row = self.by_stage[
            ("oteryn:quest.authored.asura_palace_quest", "s1")
        ]
        self.assertEqual("AMBIGUOUS_MULTIPLE_AREAS", row["status"])
        self.assertEqual(
            {
                "oteryn:area.hunting_place.asura_palace",
                "oteryn:content.area.subregion.asura_palace",
            },
            {match["area_ref"]["key"] for match in row["matches"]},
        )
        self.assertIsNone(row["candidate"])

    def test_fine_grained_location_is_not_fuzzy_mapped(self):
        row = self.by_stage[
            ("oteryn:quest.authored.a_pirate_s_death_to_me", "s3")
        ]
        self.assertEqual("NO_EXACT_AREA", row["status"])
        self.assertEqual(["Pirate Ship upper deck"], row["raw_targets"])
        self.assertEqual([], row["matches"])
        self.assertIsNone(row["candidate"])

    def test_canonical_area_identity_is_not_runtime_binding(self):
        for row in self.packet["records"]:
            self.assertIsNone(row["native_spatial_binding"])
            if row["candidate"] is not None:
                self.assertIn(
                    "SPATIAL_OCCURRENCE_BINDING_PENDING", row["holds"]
                )
                self.assertIn("AREA_ENTRY_SEMANTICS_NOT_PROVEN", row["holds"])

    def test_committed_packet_matches_generator(self):
        target = ROOT / tool.OUTPUT
        self.assertTrue(target.is_file())
        self.assertEqual(target.read_bytes(), tool.compact(tool.expected(ROOT)))


if __name__ == "__main__":
    unittest.main()
