"""Secondary observations stay separate from primary identity, cut and admission authority."""
import unittest
import json
import tempfile
from pathlib import Path
from unittest.mock import patch

import prepare_missing_visible_evidence as evidence


# Fact-only reduced fixture from the normal-HTTP Monk capture on 2026-10-01.
MONK = '''<title>Potwory: Monk - Tibia ~ Tibiopedia.pl</title>
<table><tr><td>Monk</td><td><table class="monster_base_stats">
<tr><td>Doświadczenie:</td><td></td><td><strong>200</strong></td></tr>
<tr><td>Życie:</td><td></td><td><strong>240</strong></td></tr>
<tr><td>Przywołanie:</td><td></td><td>600</td></tr>
<tr><td>Zauroczenie:</td><td></td><td>-</td></tr>
<tr><td>Punkty uroku:</td><td></td><td>15</td></tr>
<tr><td>Trudność:</td><td colspan="2"><img src="difficulty_2.png" title="Łatwe"/></td></tr>
<tr><td>Rzadkość:</td><td colspan="2"><img src="rarity_1.png" title="Pospolity"/></td></tr>
<tr><td>Pancerz:</td><td></td><td>25</td></tr>
<tr><td>Prędkość:</td><td></td><td>120</td></tr>
<tr><td>Mitygacja:</td><td></td><td>2.18%</td></tr>
</table></td></tr></table>'''


class MissingVisibleEvidence(unittest.TestCase):
    def test_actual_monk_fact_fixture_handles_nested_rows_and_exact_numbers(self):
        facts = evidence.extract_facts(MONK)
        self.assertEqual('CURRENT_PAGE_OBSERVATIONS', facts['status'])
        for field, expected in {'experience': 200, 'max_health': 240, 'armor': 25,
                                'speed': 120, 'charm_points': 15,
                                'mitigation_percent': {'numerator': 109, 'denominator': 50},
                                'difficulty': 'Łatwe', 'occurrence': 'Pospolity'}.items():
            self.assertEqual(expected, facts['fields'][field]['value'])
            self.assertGreater(facts['fields'][field]['source_line'], 0)
        self.assertEqual('UNKNOWN', facts['fields']['convince_mana_cost']['status'])

    def test_observed_candidates_cannot_authorize_native_values_or_identity(self):
        packet = evidence.prepare_packet(
            {'identity': 'oteryn:creature.monk', 'name': 'Monk', 'missing_fields': ['mitigation_percent']},
            {'status': 200, 'html': MONK, 'url': 'https://tibiopedia.pl/monsters/Monk',
             'body_sha256': 'captured-digest', 'transport': 'normal_http'})
        self.assertIn('mitigation_percent', packet['typed_observed_candidates'])
        self.assertFalse(packet['admission_authorized'])
        self.assertEqual(0, packet['native_values_changed'])
        self.assertEqual(evidence.PENDING, packet['pending_issues'])
        self.assertEqual('UNKNOWN_PENDING_PRIMARY_EVIDENCE', packet['missing_field_status']['mitigation_percent'])
        self.assertNotIn('html', packet['source'])

    def test_unavailable_or_setup_pages_produce_no_fake_zero_or_bestiary(self):
        case = {'identity': 'oteryn:creature.helper', 'name': 'Helper',
                'missing_fields': ['mitigation_percent', 'bestiary']}
        for response in ({'status': 403}, {'status': 'NOT_REQUESTED'},
                         {'status': 200, 'html': '<title>Setup</title>'}):
            packet = evidence.prepare_packet(case, response)
            self.assertEqual({}, packet['typed_observed_candidates'])
            self.assertEqual({}, packet['observation']['fields'])

    def test_uncertain_and_missing_values_are_never_parsed_as_zero(self):
        for raw in ('-', '?', 'Unknown', '~2.18%', '2.18?%', ''):
            self.assertIsNone(evidence.numeric(raw))
        self.assertEqual(0, evidence.numeric('0'))
        self.assertEqual(1000, evidence.numeric('1 000'))

    def test_inventory_covers_union_once_and_excludes_complete_cases(self):
        rows = []
        for identity, mitigation, charm in [('a', 'SOURCE_UNSPECIFIED', 'PRESENT'),
                                          ('b', 'PRESENT', 'NOT_APPLICABLE_IN_SOURCE'),
                                          ('c', 'SOURCE_UNSPECIFIED', 'NOT_APPLICABLE_IN_SOURCE'),
                                          ('d', 'PRESENT', 'PRESENT')]:
            rows.append({'identity': identity, 'name': identity, 'fields': {
                'mitigation_percent': {'status': mitigation}, 'charm_points': {'status': charm}}})
        cases = evidence.missing_inventory({'monsters': rows})
        self.assertEqual(['a', 'b', 'c'], [case['identity'] for case in cases])
        self.assertEqual(['mitigation_percent', 'bestiary'], cases[2]['missing_fields'])

    def test_host_block_stops_remaining_requests_and_is_not_retried_on_resume(self):
        visible = {'monsters': [{'identity': 'oteryn:creature.' + name, 'name': name, 'fields': {
            'mitigation_percent': {'status': 'SOURCE_UNSPECIFIED'},
            'charm_points': {'status': 'PRESENT'}}} for name in ('first', 'second')]}
        with tempfile.TemporaryDirectory() as directory, patch('builtins.print'):
            path = Path(directory)
            (path / 'visible.json').write_text(json.dumps(visible))
            argv = ['prepare', '--visible', str(path / 'visible.json'), '--out', str(path / 'packets')]
            with patch('sys.argv', argv), patch.object(evidence, 'fetch', return_value={'status': 403}) as fetch:
                evidence.main()
                self.assertEqual(1, fetch.call_count)
            with patch('sys.argv', argv), patch.object(evidence, 'fetch', side_effect=AssertionError('blocked host retry')):
                evidence.main()
            summary = json.loads((path / 'packets' / 'summary.json').read_text())
            self.assertEqual(2, summary['cases'])
            self.assertEqual(0, summary['requests_this_run'])
            self.assertEqual(0, summary['native_values_changed'])


if __name__ == '__main__':
    unittest.main()
