import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PREFIX = 'oteryn:quest-transition/questpc3/threatened-light/'


class ThreatenedLightContracts(unittest.TestCase):
    def test_quest_gather_and_cosmetic_branches_have_separate_charge_effects(self):
        rows = {row['key'][len(PREFIX):]: row for row in json.loads(
            (ROOT / 'content/quests/world-callbacks.json').read_text())['callbacks']
            if row['key'].startswith(PREFIX)}
        self.assertEqual(set(rows), {name + '-' + branch
                         for name in ('moon', 'star', 'sun')
                         for branch in ('gather', 'cosmetic')})
        for name, source, charged, cosmetic, floor in [
                ('moon', 25729, 25730, 25975, -7),
                ('star', 25731, 25732, 25976, -4),
                ('sun', 25733, 25734, 25977, -7)]:
            for branch, stage, output in [('gather', 6, charged), ('cosmetic', 8, cosmetic)]:
                row = rows[name + '-' + branch]
                with self.subTest(name=name, branch=branch):
                    self.assertEqual(row['trigger'], 'use_carried')
                    self.assertTrue(all(value is None for value in row['selector'].values()))
                    self.assertEqual(row['item_on_target'], dict(
                        item=f'oteryn:item.tibia.i{source}', count=1,
                        replacement=f'oteryn:item.tibia.i{output}', target_replacement=None))
                    self.assertEqual(row['carried_use']['actor_region']['minimum']['floor'], floor)
                    self.assertEqual(row['carried_use']['time_window'], dict(
                        from_minute=360 if name == 'sun' else 1080,
                        to_minute=1080 if name == 'sun' else 360))
                    mission, *charges = row['storage']
                    self.assertEqual((mission['minimum'], mission['maximum'], mission['set']),
                                     (stage, stage, stage))
                    self.assertEqual(len(charges), 1 if branch == 'gather' else 0)
                    if charges:
                        self.assertEqual((charges[0]['minimum'], charges[0]['maximum'],
                                          charges[0]['set'], charges[0]['preserve']),
                                         (-1, 5, 5, False))
                    self.assertTrue(any('git-blob=' in source and 'sha256=' in source
                                        for source in row['sources']))


if __name__ == '__main__':
    unittest.main()
