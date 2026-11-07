import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PREFIX = 'oteryn:quest-transition/questpc3/threatened-barrier/'
BASE = 'oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/'


class ThreatenedBarrierContracts(unittest.TestCase):
    def test_fifteen_distinct_targets_have_all_five_counter_states(self):
        document = json.loads((ROOT/'content/quests/world-callbacks.json').read_text())
        rows = {row['key'][len(PREFIX):]: row for row in document['callbacks']
                if row['key'].startswith(PREFIX)}
        self.assertEqual(len(rows), 75)
        self.assertLessEqual(len(document['callbacks']), 512)
        cases = [
            ('moon', 25730, 25729, 'charged_moon_mirror', 'moon_mirror_pos',
             [(33386,32215,-7),(33511,32320,-6),(33518,32193,-7),(33549,32219,-7),(33597,32182,-7)]),
            ('star', 25732, 25731, 'charged_starlight_vial', 'starlight_pos',
             [(33442,32201,-7),(33458,32299,-7),(33562,32256,-7),(33541,32244,-7),(33529,32187,-7)]),
            ('sun', 25734, 25733, 'charged_sun_catcher', 'sun_catcher_pos',
             [(33419,32242,-7),(33484,32192,-7),(33546,32155,-7),(33547,32206,-7),(33568,32243,-7)]),
        ]
        distinct_flags = set()
        for name, charged, empty, counter, flag, targets in cases:
            for number, (x, y, floor) in enumerate(targets, 1):
                for remaining in range(1, 6):
                    row = rows[f'{name}-pos{number:02d}-charges{remaining}']
                    with self.subTest(tool=name, target=number, charges=remaining):
                        self.assertEqual(row['trigger'], 'use_carried_on_target')
                        self.assertEqual(row['selector'], dict(position=dict(x=x,y=y,floor=floor),
                                                             unique=None, action=None, appearance=None))
                        story, unique, charges = row['storage']
                        self.assertEqual(story, dict(track=BASE+'nightmare_intruders',minimum=7,
                                                     maximum=7,set=7,preserve=True))
                        self.assertEqual(unique, dict(track=BASE+flag+f'{number:02d}', minimum=-1,
                                                      maximum=0,set=1,preserve=False))
                        self.assertEqual(charges, dict(track=BASE+counter,minimum=remaining,
                                                       maximum=remaining,set=remaining-1,preserve=False))
                        distinct_flags.add(unique['track'])
                        self.assertEqual(row['item_on_target'], dict(item=f'oteryn:item.tibia.i{charged}',
                            count=1,replacement=f'oteryn:item.tibia.i{empty if remaining == 1 else charged}',
                            target_replacement=None))
                        self.assertEqual(row['carried_use']['time_window'], dict(
                            from_minute=360 if name=='sun' else 1080,
                            to_minute=1080 if name=='sun' else 360))
                        self.assertEqual(row['carried_use']['actor_region'], dict(
                            minimum=dict(x=x-1,y=y-1,floor=floor), maximum=dict(x=x+1,y=y+1,floor=floor)))
                        self.assertEqual(row['passages'], [])
                        self.assertEqual(row['offerings'], [])
                        self.assertIsNone(row['teleport'])
                        self.assertIsNone(row['lever'])
                        self.assertTrue(any('git-blob=' in source and 'sha256=' in source for source in row['sources']))
        self.assertEqual(len(distinct_flags), 15)


if __name__ == '__main__':
    unittest.main()
