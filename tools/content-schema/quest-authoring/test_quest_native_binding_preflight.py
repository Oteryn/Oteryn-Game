import json
from pathlib import Path
import unittest

import quest_native_binding_preflight as preflight


ROOT = Path(__file__).resolve().parents[3]


class QuestNativeBindingPreflightTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.report = preflight.collect(ROOT)

    def test_all_canonical_owners_partition_without_duplicates(self):
        summary = self.report["summary"]
        self.assertEqual(352, summary["canonical_quests"])
        self.assertEqual(373, summary["wiki_titles"])
        self.assertEqual(352, len(self.report["records"]))
        self.assertEqual(
            352,
            len({row["quest"] for row in self.report["records"]}),
        )
        self.assertEqual(
            {
                "CHOSEN_STAGE_CONSUMERS": 304,
                "REWARD_ONLY_NO_TYPED_PROGRESS_CANDIDATE": 42,
                "SOURCE_PROGRESS_NO_CHOSEN_STAGE_PLAN": 6,
            },
            summary["lanes"],
        )

    def test_every_stage_has_real_owner_and_is_unbound(self):
        summary = self.report["summary"]
        self.assertEqual(1891, summary["plan_stages"])
        self.assertEqual(1891, summary["unbound_dispatch_stages"])
        self.assertEqual(0, summary["native_dispatch_bindings"])
        self.assertEqual(0, summary["native_reward_delivery_bindings"])
        self.assertEqual(
            {
                "collect": 277,
                "complete": 304,
                "explore": 248,
                "kill": 275,
                "talk": 329,
                "use": 458,
            },
            summary["event_kinds"],
        )

    def test_association_is_not_native_binding(self):
        summary = self.report["summary"]
        self.assertEqual(
            {
                "AMBIGUOUS_CANONICAL_NAME": 108,
                "CANONICAL_EXACT_NAME_NOT_FOUND": 1569,
                "EXACT_CANONICAL_IDENTITY_ASSOCIATION": 750,
            },
            summary["target_association_statuses"],
        )
        self.assertEqual(25, summary["consumer_seam_statuses"]["EXISTING_DECLARED_ENCOUNTER_OUTCOME"])
        self.assertEqual(656, summary["reward_intents"])

    def test_playability_never_inferred_from_source_or_identity(self):
        self.assertEqual(0, self.report["summary"]["playable_verified"])
        self.assertEqual(
            "NOT_ASSESSED_BY_THIS_REPORT",
            self.report["summary"]["playability_claim"],
        )
        self.assertTrue(all(
            row["playable_verification"] == "NOT_ASSESSED"
            for row in self.report["records"]
        ))

    def test_retained_json_is_deterministic(self):
        path = ROOT / preflight.OUTPUT
        self.assertEqual(preflight.serialize(self.report), path.read_bytes())


if __name__ == "__main__":
    unittest.main()
