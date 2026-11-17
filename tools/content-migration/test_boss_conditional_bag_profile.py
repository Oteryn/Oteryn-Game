"""The explicit onDeath corpse bag needs its real native carried Item policy."""
import json
import unittest
import boss_corpse_profiles as corpses
import boss_reward_yield_profiles as yields


class ConditionalBagProfileTests(unittest.TestCase):
    def test_actual_conditional_corpse_bag_admission_has_source_item_policy(self):
        generated={'profiles':corpses.encode({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':[]}),
                   'manifest':corpses.encode({'item_profiles':{'sha256':'0'*64}})}
        yields.admit(yields.ROOT,generated,'profiles','manifest',
            lambda row:row['production_definition']['production_key'],corpses.encode)
        rows=json.loads(generated['profiles'])['records']
        bag=[row for row in rows if row['production_binding']['external_id']=='43860']
        self.assertEqual(len(bag),1,'pinned Bakragore death callback adds real43860 to its physical corpse')
        self.assertEqual(bag[0]['admission']['legal_destinations'],['CharacterInventory','Ground'])
        self.assertEqual(bag[0]['production_definition']['production_key'],'oteryn:item.tibia.i43860')
        self.assertEqual(bag[0]['production_binding']['disposition'],'EXACT')


if __name__=='__main__':unittest.main()
