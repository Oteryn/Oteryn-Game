"""Focused guards for reviewed G16/M10/G33 evidence ownership and coverage."""
import unittest
from pathlib import Path
from report import results

ROOT = Path(__file__).resolve().parents[3]
PHASES = ROOT / 'apps/game-server/src/qualification/client_phases'

class JourneyEvidenceTests(unittest.TestCase):
    def test_g16_claim_decodes_shared_result_and_binds_inbox_mint(self):
        source = (PHASES / 'full_row_g16.rs').read_text()
        self.assertNotIn('reply.first()', source)
        self.assertIn('decode_reply(DailyRewardCommand::Claim', source)
        self.assertIn('game_daily_reward_picks', source)
        self.assertIn('game_item_character_inbox_locations', source)
        self.assertIn('minted_transaction_id', source)
        self.assertIn('game_character_daily_reward_claims', source)
        self.assertNotIn('game_item_container_entries', source)
        self.assertIn('claim_count(clients, offer.epoch)', source)
        self.assertIn('delivery(clients, &data)', source)

    def test_m10_bounded_drowning_does_not_qualify_whole_matrix_row(self):
        source = (PHASES / 'full_row_m10.rs').read_text()
        self.assertNotIn('if pass { "PASS" } else { "FAIL" }', source)
        self.assertIn('unproven-swimming-traps-special-movement', source)
        self.assertIn('observed-five-native-drowning-steps', source)
        self.assertRegex(source, r'row\.pass\(\s*5,')

    def test_g33_reads_active_epoch_decision_without_process_calendar(self):
        source = (PHASES / 'full_row_g33.rs').read_text()
        self.assertNotIn('SystemTime::now', source)
        self.assertNotIn('fn month_day', source)
        for owner in ['game_world_reset_active', 'game_world_reset_epochs',
                      'game_world_event_epochs', 'seasonal_calendar_revision',
                      'seasonal_event_keys', "AT TIME ZONE 'Europe/Berlin'", 'seasonal_at']:
            self.assertIn(owner, source)
        self.assertIn('server-active-seasonal-window-closed', source)
        self.assertIn('missing-active-seasonal-decision', source)

    def test_g33_closed_decision_token_is_accepted_by_strict_report_parser(self):
        found = results(
            'SEAM_EVIDENCE e2e_all scenario=G33.full status=SKIPPED seconds=1 '
            'evidence=content:server-active-seasonal-window-closed-epoch-42-activated-date-11-12'
        )
        self.assertEqual(found['G33.full'][0], 'SKIPPED')
        self.assertNotIn(';epoch=', (PHASES / 'full_row_g33.rs').read_text())

if __name__ == '__main__':
    unittest.main()
