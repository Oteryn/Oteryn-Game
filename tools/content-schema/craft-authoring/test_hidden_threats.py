"""Source semantics that must survive authoring/native owner integration."""
import json
from pathlib import Path
import tempfile
import unittest

import hidden_threats as forge


class HiddenThreatsForge(unittest.TestCase):
    def rows(self):
        rows = []
        forge.append_rows(rows)
        return rows

    def test_source_retains_file_and_polishes_both_distinct_fragments(self):
        rows = [r for r in self.rows() if '-file-' in r['identity']['key']]
        self.assertEqual(len(rows), 2)
        pairs = set()
        for row in rows:
            used, target = row['recipe']['operands']
            self.assertEqual((used['item_id']['key'], used['effect']), ('oteryn:item.tibia.i27270', 'KEEP'))
            self.assertNotIn('output_item_id', used)
            pairs.add((target['item_id']['key'], target['output_item_id']['key']))
        self.assertEqual(pairs, {('oteryn:item.tibia.i27261', 'oteryn:item.tibia.i27264'),
                                 ('oteryn:item.tibia.i27262', 'oteryn:item.tibia.i27263')})

    def test_source_accepts_all_four_hot_pairs_and_four_lava_ids(self):
        rows = self.rows()
        joins = [r for r in rows if '-join-' in r['identity']['key']]
        pairs = {tuple(o['item_id']['key'] for o in r['recipe']['operands']) for r in joins}
        self.assertEqual(pairs, {(f'oteryn:item.tibia.i{used}', f'oteryn:item.tibia.i{target}')
                                 for used in [27265, 27266] for target in [27265, 27266]})
        heat = [r for r in rows if '-heat-' in r['identity']['key']]
        self.assertEqual(len(heat), 8)
        self.assertEqual({r['recipe']['target']['definition']['key'] for r in heat},
                         {f'oteryn:item.tibia.i{i}' for i in [7821, 7822, 7823, 7824]})
        self.assertEqual(len(rows), 16)
        self.assertEqual(forge.native_item_ids(), set(range(27261, 27271)) | {7821, 7822, 7823, 7824, 3458, 10113})

    def test_source_fact_substitution_is_rejected_before_emission(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / forge.CAPSULE
            path.parent.mkdir(parents=True)
            data = forge.facts()
            data['joining']['target_output'] = 3031
            path.write_text(json.dumps(data))
            rows = []
            with self.assertRaisesRegex(ValueError, 'HIDDEN_THREATS_CRAFT_SOURCE_PIN'):
                forge.append_rows(rows, root)
            self.assertEqual(rows, [])


if __name__ == '__main__':
    unittest.main()
