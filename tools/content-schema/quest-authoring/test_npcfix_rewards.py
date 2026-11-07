"""Source closure must fill missing lifetime facts without rewriting other owners."""
import copy
import json
import sys
import unittest

import qengine_rewards as q

sys.path.insert(0, str(q.ROOT / 'tools/content-migration'))
import qengine_reward_item_profiles as items
import register_spell_families as native


class NpcfixRewardProfiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.outputs = q.build()

    def profiles(self):
        outputs = copy.deepcopy(self.outputs)
        profiles = json.loads(outputs[items.PROFILE])
        pick = next(row for row in profiles['records']
                    if row['production_definition']['production_key'] == 'oteryn:item.tibia.i3456')
        return outputs, profiles, pick

    def test_lifetime_enrichment_retains_every_other_existing_pick_fact(self):
        outputs, profiles, pick = self.profiles()
        pick['semantics']['temporal'] = {'state': 'UNKNOWN'}
        before = copy.deepcopy(pick)
        outputs[items.PROFILE] = q.encoded(profiles)
        items.admit(q.ROOT, outputs, items.PROFILE, 'content/spells.manifest.json',
                    native.provider_order, q.encoded)
        after = next(row for row in json.loads(outputs[items.PROFILE])['records']
                     if row['production_definition']['production_key'] == 'oteryn:item.tibia.i3456')
        self.assertEqual(after['semantics']['temporal'], {'state': 'NOT_APPLICABLE'})
        after['semantics']['temporal'] = before['semantics']['temporal']
        self.assertEqual(after, before)

    def test_existing_known_weight_conflict_is_never_overridden(self):
        outputs, profiles, pick = self.profiles()
        pick['semantics']['physical']['value']['weight']['value'] += 1
        outputs[items.PROFILE] = q.encoded(profiles)
        with self.assertRaisesRegex(ValueError, 'QENGINE_REWARD_ITEM_NATIVE_CONFLICT'):
            items.admit(q.ROOT, outputs, items.PROFILE, 'content/spells.manifest.json',
                        native.provider_order, q.encoded)


if __name__ == '__main__':
    unittest.main()
