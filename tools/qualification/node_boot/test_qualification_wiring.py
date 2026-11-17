"""Check the live harness uses its evidence and recovery paths in the required order."""

from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[3]
GAME = ROOT / "apps/game-server/src"
PHASES = GAME / "qualification/client_phases"


class QualificationWiringTests(unittest.TestCase):
    def test_private_channel_fixture_precedes_both_create_commands(self):
        journey = (PHASES / "full_e2ef19.rs").read_text().split("async fn journey", 1)[1]
        first, second, following = journey.split("prepare_channel_owner(&mut a).await?")
        self.assertNotIn("PrivateChannelIntent::Create", first)
        self.assertEqual(second.count("PrivateChannelIntent::Create"), 1)
        self.assertEqual(following.count("PrivateChannelIntent::Create"), 1)

    def test_stored_teardown_is_sampled_between_logout_and_readmission(self):
        journey = (PHASES / "full_e2ef19.rs").read_text()
        before, after = journey.split("logout(&mut a).await?;", 1)
        self.assertIn("durable_channel_rows(&database", before)
        read, admission = after.split("a = connect_character", 1)
        self.assertIn("durable_channel_rows(&database", read)
        self.assertNotIn("channel_list(", read)
        self.assertIn("relog-transient-list", admission)

    def test_c14_consumes_refreshed_maxima_and_independently_checks_award(self):
        source = (PHASES / "full_e2ef18.rs").read_text()
        self.assertIn("max_mana == expected_mana", source)
        self.assertIn("after.vitals = await_level_publication", source)
        self.assertIn("catalog_deadline", source)
        self.assertNotIn("hunting_experience::hunting_experience", source)
        self.assertIn("let expected = expected_rat_award", source)

    def test_spawn_selection_advances_beyond_an_empty_lane(self):
        source = (PHASES / "world.rs").read_text().split("async fn approach_spawn_target", 1)[1]
        self.assertIn("remaining_lanes.remove", source)
        self.assertIn("point.respawn_ms", source)
        self.assertNotIn("unreachable.insert", (PHASES / "full_gold.rs").read_text())

    def test_gold_cancellation_invalidates_the_session(self):
        source = (PHASES / "full_gold.rs").read_text()
        self.assertEqual(source.count("invalidate_after_cancellation()"), 4)

    def test_action_failure_is_preserved_before_cleanup(self):
        source = (PHASES / "full_e2ef1.rs").read_text()
        rows = source.split("async fn rows", 1)[1]
        compact = "".join(rows.split())
        self.assertLess(
            compact.index("verdicts.action_failure=Some"),
            compact.index("inventory_signature(&client)?"),
        )
        self.assertIn("action.or(self.aborted.take())", "".join(source.split()))

    def test_combat_setup_reconciles_both_retained_writes(self):
        source = (GAME / "qualification_spell_character.rs").read_text()
        self.assertIn("reconcile_character_experience(authority, xp_occurrence)", source)
        self.assertIn("reconcile_character_build(authority, build_occurrence)", source)
        self.assertIn("!resumed && experience.get() > 0", source)


if __name__ == "__main__":
    unittest.main()
