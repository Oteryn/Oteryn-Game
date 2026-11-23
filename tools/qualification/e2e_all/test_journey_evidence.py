"""Focused guards for reviewed G16/M10/G33 evidence ownership and coverage."""
import re
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
        self.assertIn('scenario=M10.full status=FAIL', source)
        self.assertEqual(source.count('scenario=M10.full status='), 1)
        self.assertNotIn('if pass { "PASS" } else { "FAIL" }', source)
        self.assertIn('M10_GAP {gap}', source)
        self.assertIn('step=swimming-look-type missing=served-look-type-267', source)
        self.assertIn('gap_detail(self.step, self.error.as_deref())', source)
        journey = source.split('async fn journey(', 1)[1].split('#[cfg(test)]', 1)[0]
        self.assertEqual(re.findall(r'row\.pass\(\s*(\d+),', journey), list('12345678'))
        self.assertEqual(journey.count('if damage != 20'), 2)
        self.assertIn('refused.world_spatial_delta.is_some()', journey)
        self.assertIn('drown ActorVitals damage continued after dry exit', journey)
        self.assertIn('v.health != saved_vitals.health', journey)
        self.assertIn('surface_swimming(&mut client, world).await?', journey)
        self.assertIn('&SPIKES, "spike").await?', journey)
        self.assertIn('&PRESSURE, "pressure-tile").await?', journey)

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
