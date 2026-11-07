import copy
import json
import os
import unittest
from pathlib import Path
import quest_reward_followup as f


class RewardFollowupTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        here = Path(__file__).parent / 'samples/recipe-followup'
        cls.root = Path(os.environ.get('QUEST_PRODUCT_ROOT', Path(__file__).resolve().parents[3]))
        cls.rows = json.loads((here / 'reward-followup-fixtures.json').read_text())
        cls.packet = json.loads((here / 'rewards.json').read_text())
        cls.sources = f.load_sources(cls.root, cls.packet)

    def test_exact_two_rewards_preserve_source_core(self):
        result = f.apply_packet(self.rows, self.packet, self.sources)
        expected = copy.deepcopy(self.rows)
        for row in expected:
            fix = next(v for v in self.packet['changes'] if v['canonical_key'] == row['definition']['identity']['key'])
            recipe = row['definition']['oteryn_recipe']['payload']['recipe']
            recipe['reward_intents'].append(fix['reward_intent'])
            recipe['source_notes'].append(fix['source_note'])
        self.assertEqual(expected, result)
        self.assertEqual([r['definition']['missing_data'] for r in self.rows], [r['definition']['missing_data'] for r in result])
        self.assertEqual([r['definition']['requirements'] for r in self.rows], [r['definition']['requirements'] for r in result])

    def test_excludes_other_spectulus_quest_grants(self):
        sea = [r for r in self.packet['source_files'] if r['canonical_key'].endswith('sea_of_light_quest')]
        self.assertEqual(len(sea), 2)
        for source in sea:
            self.assertEqual(sorted(s['experience'] for s in source['grant_spans']), [100, 400, 500, 1000])
            self.assertNotIn('JackFutureQuest', '\n'.join(s['raw'] for s in source['grant_spans']))

    def test_wrong_sum_rejected(self):
        packet = copy.deepcopy(self.packet)
        packet['changes'][0]['reward_intent']['count'] += 6000
        with self.assertRaisesRegex(ValueError, 'amount differs'):
            f.apply_packet(self.rows, packet, self.sources)

    def test_guarded_source_span_tamper_rejected(self):
        packet = copy.deepcopy(self.packet)
        packet['source_files'][0]['grant_spans'][0]['start_byte'] += 1
        with self.assertRaisesRegex(ValueError, 'span differs'):
            f.apply_packet(self.rows, packet, self.sources)

    def test_source_raw_tamper_rejected(self):
        sources = dict(self.sources)
        k = next(iter(sources))
        sources[k] += b' '
        with self.assertRaisesRegex(ValueError, 'bytes differ'):
            f.apply_packet(self.rows, self.packet, sources)

    def test_incomplete_donor_coverage_rejected(self):
        packet = copy.deepcopy(self.packet)
        packet['source_files'] = packet['source_files'][1:]
        with self.assertRaisesRegex(ValueError, 'donor coverage'):
            f.apply_packet(self.rows, packet, self.sources)

    def test_changed_recipe_rejected(self):
        rows = copy.deepcopy(self.rows)
        rows[0]['definition']['oteryn_recipe']['payload']['recipe']['requirements']['min_level'] += 1
        with self.assertRaisesRegex(ValueError, 'recipe fence'):
            f.apply_packet(rows, self.packet, self.sources)

    def test_duplicate_owner_rejected(self):
        with self.assertRaisesRegex(ValueError, 'owner differs'):
            f.apply_packet(self.rows + [self.rows[0]], self.packet, self.sources)

    def test_runtime_promotion_rejected(self):
        packet = copy.deepcopy(self.packet)
        packet['runtime_enabled'] = True
        with self.assertRaisesRegex(ValueError, 'scope differs'):
            f.apply_packet(self.rows, packet, self.sources)


if __name__ == '__main__':
    unittest.main()
