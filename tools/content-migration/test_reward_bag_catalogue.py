"""The executable catalogue preserves every captured outcome and source digest."""
import hashlib
import json
import unittest

from reward_bag_catalogue import ROOT, DEST, catalogue


class RewardBagCatalogue(unittest.TestCase):
    def test_generated_catalogue_matches_captured_rules(self):
        generated = catalogue(ROOT)
        self.assertEqual(json.loads((ROOT / DEST).read_bytes()), generated)
        events = json.loads((ROOT / 'tools/content-schema/native-gameplay/event-item-actions.json').read_bytes())
        bags = generated['bags'][3:3+len(events['rules'])]
        self.assertEqual(len(bags), 24)
        for bag, rule in zip(bags, events['rules']):
            self.assertEqual(bag['bag'], f"oteryn:item.tibia.i{rule['item_id']}")
            self.assertEqual(bag['selection'], rule['selection'])
            self.assertEqual(bag['picks'], rule['picks'])
            self.assertEqual(bag['roll_max'], rule.get('roll_max'))
            self.assertEqual([(r['item'], r['minimum'], r['maximum'], r['weight'])
                              for r in bag['rewards']],
                             [(f"oteryn:item.tibia.i{r['item_id']}", r['minimum'],
                               r['maximum'], r['weight']) for r in rule['rewards']])
        quest_bags=json.loads((ROOT/'tools/content-schema/native-gameplay/qengine-reward-bag-profiles.json').read_bytes())['bags']
        self.assertEqual(generated['bags'][3+len(events['rules']):],quest_bags)
        supply=next(b for b in quest_bags if b['bag']=='oteryn:item.tibia.i15698')
        self.assertEqual(supply['roll_max'],10001)
        self.assertEqual(sum(r['weight'] for r in supply['rewards']),10001)
        rare={r['item']:r['weight'] for r in supply['rewards']}
        self.assertEqual((rare['oteryn:item.tibia.i16257'],rare['oteryn:item.tibia.i16254']),(111,89))
        for source in generated['sources']:
            self.assertEqual(source['sha256'], hashlib.sha256((ROOT / source['path']).read_bytes()).hexdigest())
