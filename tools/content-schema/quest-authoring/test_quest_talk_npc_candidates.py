import pathlib
import unittest

import quest_talk_npc_candidates as tool


ROOT = pathlib.Path(__file__).resolve().parents[3]


class QuestTalkNpcCandidatesTest(unittest.TestCase):
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
                "talk_stages": 250,
                "statuses": {
                    "AMBIGUOUS_MULTIPLE_NPCS": 36,
                    "EXACT_NPC_NO_DIALOGUE": 23,
                    "EXACT_NPC_WITH_DIALOGUE": 174,
                    "NO_EXACT_NPC": 17,
                },
                "dialogue_candidates": 174,
                "native_dispatch_bindings": 0,
                "selected_dialogue_branches": 0,
            },
            self.packet["summary"],
        )
        self.assertFalse(self.packet["runtime_admitted"])
        self.assertTrue(
            all(not row["runtime_admitted"] for row in self.packet["records"])
        )

    def test_exact_candidate_uses_canonical_npc_and_dialogue_refs(self):
        row = self.by_stage[("oteryn:quest.a_piece_of_cake", "s1")]
        self.assertEqual("EXACT_NPC_WITH_DIALOGUE", row["status"])
        self.assertEqual(
            "oteryn:npc.biff_the_baker",
            row["candidate"]["npc_ref"]["key"],
        )
        self.assertEqual(
            "oteryn:dialogue.npc.biff_the_baker",
            row["candidate"]["dialogue_ref"]["key"],
        )
        self.assertIsNone(row["selected_NPC_branch"])
        self.assertIsNone(row["native_dispatch_binding"])

    def test_non_npc_targets_do_not_block_one_exact_npc(self):
        row = self.by_stage[
            ("oteryn:quest.an_interest_in_botany_quest", "s1")
        ]
        self.assertEqual(["Rabaz", "Botany Almanach"], row["raw_targets"])
        self.assertEqual("EXACT_NPC_WITH_DIALOGUE", row["status"])
        self.assertEqual(
            ["oteryn:npc.rabaz"],
            [match["npc_ref"]["key"] for match in row["matches"]],
        )

    def test_newly_lowered_terminal_quest_stages_resolve_exact_npcs_only(self):
        cases = {
            ("oteryn:quest.barbarian_arena_quest", "s1"): "oteryn:npc.halvar",
            ("oteryn:quest.rift_warrior_outfits_quest", "s3"): "oteryn:npc.cledwyn",
            ("oteryn:quest.rift_warrior_outfits_quest", "s5"): "oteryn:npc.cledwyn",
        }
        for stage, npc_key in cases.items():
            row = self.by_stage[stage]
            self.assertEqual("EXACT_NPC_WITH_DIALOGUE", row["status"])
            self.assertEqual([npc_key], [match["npc_ref"]["key"] for match in row["matches"]])
            self.assertIsNone(row["selected_NPC_branch"])
            self.assertIsNone(row["native_dispatch_binding"])

    def test_multiple_exact_npcs_are_held_not_selected(self):
        row = self.by_stage[
            ("oteryn:quest.between_the_lines_quest", "s3")
        ]
        self.assertEqual("AMBIGUOUS_MULTIPLE_NPCS", row["status"])
        self.assertEqual(
            {"oteryn:npc.phillip", "oteryn:npc.wyrdin"},
            {match["npc_ref"]["key"] for match in row["matches"]},
        )
        self.assertIsNone(row["candidate"])
        self.assertIn(
            "MULTIPLE_CANONICAL_NPCS_MATCH_STAGE_TARGETS", row["holds"]
        )

    def test_exact_npc_without_dialogue_is_held(self):
        row = self.by_stage[
            ("oteryn:quest.barbarian_test", "s1")
        ]
        self.assertEqual("EXACT_NPC_NO_DIALOGUE", row["status"])
        self.assertEqual(
            "oteryn:npc.sven", row["matches"][0]["npc_ref"]["key"]
        )
        self.assertIsNone(row["matches"][0]["dialogue_ref"])
        self.assertIsNone(row["candidate"])

    def test_unresolved_label_is_not_fuzzy_matched(self):
        row = self.by_stage[
            ("oteryn:quest.25_years_of_tibia_quest", "s5")
        ]
        self.assertEqual("NO_EXACT_NPC", row["status"])
        self.assertEqual(["Lord Retro"], row["raw_targets"])
        self.assertEqual([], row["matches"])

    def test_committed_packet_matches_generator(self):
        target = ROOT / tool.OUTPUT
        self.assertTrue(target.is_file())
        self.assertEqual(target.read_bytes(), tool.compact(tool.expected(ROOT)))


if __name__ == "__main__":
    unittest.main()
