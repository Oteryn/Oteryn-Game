import copy
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

import complete_familiar_population as f


class FamiliarPopulationTests(unittest.TestCase):
    def test_retained_stats_keep_correct_source_indexes(self):
        monster = {'creature': {'stats': {'max_health': 20000}}}
        original = {'creature': {'stats': {'max_health': 15000, 'mitigation_percent': {'numerator': 1, 'denominator': 1}}}}
        manifest = {'sources': [{'repository': 'crystal'}], 'entries': [
            {'destination': '/monster/creature/stats/max_health', 'source_index': 0},
            {'destination': '/monster/behavior/attacks/0', 'source_index': 0}]}
        old = {'sources': [{'repository': 'canary'}, {'kind': 'oteryn_balance_estimate'}], 'entries': [
            {'destination': '/monster/creature/stats/max_health', 'source_index': 0},
            {'destination': '/monster/creature/stats/mitigation_percent', 'source_index': 1}]}
        before = copy.deepcopy((monster, manifest, original, old))
        result, receipt = f.retain_stats(monster, manifest, original, old)
        self.assertEqual(result['creature']['stats'], original['creature']['stats'])
        self.assertEqual(receipt['sources'][receipt['entries'][-1]['source_index']], old['sources'][1])
        self.assertEqual(receipt['entries'][0]['destination'], '/monster/behavior/attacks/0')
        self.assertEqual((monster, manifest, original, old), before)

    def test_challenge_is_honestly_omitted_and_unexpected_rows_fail(self):
        entry = {'source_field': 'attacks[4]', 'status': 'unresolved_semantics', 'resolution': 'challenge'}
        source = {'entries': [entry]}
        result, original = f.omit_challenge(source)
        self.assertEqual(original, entry)
        self.assertEqual(source['entries'][0]['status'], 'unresolved_semantics')
        self.assertEqual(result['entries'][0]['status'], 'approved_omission')
        self.assertIn('FAMILIAR_CHALLENGE_NOT_IMPLEMENTED', result['entries'][0]['resolution'])
        for entries in ([], [entry, entry], [{**entry, 'source_field': 'attacks[5]'}]):
            with self.assertRaises(ValueError):
                f.omit_challenge({'entries': entries})

    def test_refuses_mutating_baseline_before_loading_sources(self):
        with TemporaryDirectory() as directory:
            baseline = Path(directory)
            with self.assertRaisesRegex(ValueError, 'modify source'):
                f.prepare(baseline, baseline, baseline, baseline / 'output')

    def test_refuses_existing_output_before_loading_sources(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / 'output'
            output.mkdir()
            (output / 'keep').write_text('keep')
            with self.assertRaisesRegex(ValueError, 'new or empty'):
                f.prepare(root / 'baseline', root / 'canary', root / 'crystal', output)
            self.assertEqual((output / 'keep').read_text(), 'keep')


if __name__ == '__main__':
    unittest.main()
