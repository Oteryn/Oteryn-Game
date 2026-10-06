import unittest
from pathlib import Path

import quest_completion_matrix as matrix


class QuestCompletionMatrixTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parents[3]
        cls.result = matrix.expected(cls.root)

    def test_all_pinned_wiki_titles_map_to_canonical_quests(self):
        self.assertEqual(373, self.result["summary"]["wiki_titles"])
        self.assertEqual(373, self.result["summary"]["wiki_titles_mapped"])
        self.assertEqual(373, len(self.result["records"]))
        self.assertTrue(all(row["canonical"] for row in self.result["records"]))

    def test_matrix_never_claims_playability(self):
        self.assertEqual(0, self.result["summary"]["playable_verified"])
        self.assertTrue(all(row["playable_verification"] == "NOT_ASSESSED"
                            for row in self.result["records"]))

    def test_donor_coverage_matches_pinned_inventory(self):
        self.assertEqual(
            {
                "DONOR_IMPLEMENTATION_AVAILABLE": 332,
                "DONOR_PARTIAL_PLUS_REFERENCE": 12,
                "REFERENCE_AUTHORED_REQUIRED": 29,
            },
            self.result["summary"]["donor_mode"],
        )

    def test_native_lowering_is_not_misclassified_as_missing_source_data(self):
        by_title = {row["wiki_title"]: row for row in self.result["records"]}
        self.assertEqual(
            "NATIVE_LOWERING_PENDING",
            by_title["An Interest In Botany Quest"]["work_state"],
        )
        self.assertEqual(
            {
                "BLOCKED_ON_SOURCE_FIDELITY": 14,
                "DEFINITION_READY_RUNTIME_UNKNOWN": 41,
                "MAPPING_REVIEW": 1,
                "NATIVE_BINDINGS_PENDING": 68,
                "NATIVE_LOWERING_PENDING": 249,
            },
            self.result["summary"]["implementation_state"],
        )
        self.assertEqual(
            {
                "SOURCE_HOLDS_CLEAR": 151,
                "SOURCE_HOLDS_PRESENT": 222,
            },
            self.result["summary"]["source_fidelity_state"],
        )

    def test_canonical_inventory_is_not_assumed_one_to_one(self):
        self.assertEqual(352, self.result["summary"]["canonical_definitions"])
        self.assertEqual(
            350,
            self.result["summary"]["unique_canonical_definitions_representing_wiki_titles"],
        )
        self.assertEqual(
            2,
            self.result["summary"]["canonical_definitions_not_in_wiki_title_inventory"],
        )


if __name__ == "__main__":
    unittest.main()
