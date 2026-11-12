"""Source regressions for the reviewed row command-owner boundaries.

These checks deliberately do not claim to execute a native player journey.
"""
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
PHASES = ROOT / "apps/game-server/src/qualification/client_phases"
USE_COUNTS = {
    "client_row_i09": 2, "client_row_i11": 2, "client_row_i12": 1,
    "client_row_m06": 1, "client_row_m07": 1, "client_row_m08": 1,
    "client_row_m09": 1, "client_row_w19a": 1,
    "full_row_g08": 1, "full_row_g09": 1, "full_row_g36": 1,
    "full_row_i10": 1, "full_row_i11": 2, "full_row_i14": 1,
    "full_row_m06": 1, "full_row_m08": 2, "full_row_m09": 1,
    "full_row_w07": 1, "full_row_w09": 1, "full_row_w19a": 1,
    "full_row_w19b": 1, "full_row_w19d": 1, "full_row_w19e": 1,
}


class RowCommandRoutes(unittest.TestCase):
    def test_all_27_reviewed_use_calls_use_the_core_session_owner(self):
        self.assertEqual(sum(USE_COUNTS.values()), 27)
        for name, count in USE_COUNTS.items():
            with self.subTest(row=name):
                source = (PHASES / f"{name}.rs").read_text()
                self.assertEqual(source.count(".world_context_use("), count)
                self.assertNotRegex(source, r"gated_command\(\s*(?:\w+::)*CAPABILITY_ITEM_USE_V1")

    def test_both_bed_calls_use_capability_11(self):
        for name in ("client_row_s07", "full_row_s07"):
            source = (PHASES / f"{name}.rs").read_text()
            self.assertRegex(source, r"gated_command\(\s*(?:\w+::)*CAPABILITY_WORLD_INTERACTION_V1,\s*COMMAND_TYPE_BED_SLEEP_INTENT")
            self.assertNotIn("CAPABILITY_ITEM_USE_V1", source)

    def test_core_achievements_are_not_sent_as_capability_zero(self):
        for name in ("client_row_p11", "full_row_p11"):
            with self.subTest(row=name):
                source = (PHASES / f"{name}.rs").read_text()
                self.assertNotRegex(source, r"gated_command\(\s*0,")
                self.assertIn(".account_achievements(", source)
                self.assertIn("decode_account_achievements_query(&[0x08])", source)
                self.assertIn("next_command_id()", source)
                self.assertIn("last_server_sequence()", source)

    def test_countdown_negative_probe_is_local_and_cannot_abort_observation(self):
        for name in ("client_row_w05a", "full_row_w05a"):
            with self.subTest(row=name):
                source = (PHASES / f"{name}.rs").read_text()
                self.assertNotRegex(source, r"gated_command\([^)]*CAPABILITY_SERVER_SAVE_COUNTDOWN_V1[^)]*\)\s*\.await\?")
                self.assertIn("SessionError::CapabilityNotSelected", source)
                self.assertIn("next_command_id()", source)
                self.assertIn("last_server_sequence()", source)
                self.assertIn("service_liveness(Duration::from_secs(7))", source)

    def test_p11_capture_does_not_present_local_refusal_as_a_wire_status(self):
        capture = (PHASES / "client_row_p11.rs").read_text()
        replay = (ROOT / "crates/session/src/tests/e2e_p11.rs").read_text()
        self.assertNotIn("refused_status=Rejected", capture)
        self.assertIn("malformed_query=local-codec-refusal", capture)
        self.assertIn("malformed_wire_submission=false", capture)
        self.assertIn('value(e, "malformed_query")', replay)
        self.assertIn('value(e, "malformed_wire_submission")', replay)


if __name__ == "__main__":
    unittest.main()
