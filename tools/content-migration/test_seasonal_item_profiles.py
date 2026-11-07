"""SRV7 source Item policies needed by the real Cake USE/projection callers."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
IDS = {12085,12086,12113}


class SeasonalItemProfilesTest(unittest.TestCase):
    def test_current_native_provider_admits_all_source_cake_interaction_states(self):
        data = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        by_key = {row['production_definition']['production_key']: row for row in data['records']}
        for source in IDS:
            key = f'oteryn:item.tibia.i{source}'
            self.assertTrue(key in by_key, key)
            self.assertEqual(by_key[key]['production_binding']['external_id'], str(source))

    def test_current_native_christmas_present_has_five_slots_and_all_donor_gifts(self):
        ids = {2991,2992,2993,2995,3001,3036,3039,3051,3057,3097,3098,3553,3570,3599,5080,5804,5919,6506,6507,6508,6510,6566,6567,7439,7440,7443,9040,9058,9604,10310,10338,10339}
        data=json.loads((ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
        by_key={r['production_definition']['production_key']:r for r in data['records']}
        for source in ids:
            key=f'oteryn:item.tibia.i{source}'
            self.assertTrue(key in by_key,key)
            self.assertTrue(by_key[key]['admission']['materializable'],key)
        bag=by_key['oteryn:item.tibia.i6510']
        self.assertEqual(bag['semantics']['container']['value']['capacity'],{'state':'KNOWN','value':5})
        self.assertEqual(bag['admission']['stack_class'],'NonStackable')

    def test_native_santa_candy_cane_has_source_food_duration(self):
        data=json.loads((ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
        record=next(r for r in data['records'] if r['production_definition']['production_key']=='oteryn:item.tibia.i3599')
        self.assertEqual(record['semantics'].get('consumption'), {'state':'KNOWN','value':{'kind':'FOOD','value':{'regeneration_seconds':24}}})

    def test_native_santa_broken_piggy_bank_is_source_qualified(self):
        data=json.loads((ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
        by_key={r['production_definition']['production_key']:r for r in data['records']}
        self.assertTrue('oteryn:item.tibia.i2996' in by_key, 'missing source-qualified broken piggy bank 2996')
        self.assertTrue(by_key['oteryn:item.tibia.i2996']['admission']['materializable'])


class SeasonalExistingPhysicsTest(unittest.TestCase):
    def fixture(self):
        import copy
        import seasonal_item_profiles as producer
        rows=json.loads((ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())['records']
        selected=[copy.deepcopy(r) for r in rows if r['production_definition']['production_key'] in {'oteryn:item.tibia.i2875','oteryn:item.tibia.i3599'}]
        self.assertEqual(len(selected),2)
        bottle=next(r for r in selected if r['production_definition']['production_key']=='oteryn:item.tibia.i2875')
        bottle['attributes']={'speed_bonus':0}
        document={'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':selected}
        generated={'profile':json.dumps(document).encode(),'manifest':json.dumps({'item_profiles':{'sha256':'0'*64}}).encode()}
        def apply():
            generated['profile']=json.dumps(document).encode()
            producer.admit(ROOT,generated,'profile','manifest',lambda r:r['production_definition']['production_key'],lambda v:json.dumps(v,sort_keys=True).encode())
            return next(r for r in json.loads(generated['profile'])['records'] if r['production_definition']['production_key']=='oteryn:item.tibia.i2875')
        return bottle,apply

    def test_existing_bottle_unknown_physics_uses_exact_source_false_facts(self):
        bottle,apply=self.fixture()
        before=json.loads(json.dumps(bottle['semantics']))
        after=apply()
        for name in ('blocks_movement','blocks_projectile','immovable_block_solid','has_height'):
            self.assertIs(after['attributes'].get(name),False,name)
        self.assertEqual(after['semantics'],before)

    def test_existing_known_physics_conflict_is_not_silently_overwritten(self):
        bottle,apply=self.fixture();bottle['attributes']['blocks_movement']=True
        with self.assertRaisesRegex(ValueError,'SEASONAL_EXISTING_PHYSICS'):
            apply()

    def test_existing_binding_mismatch_cannot_gain_source_physics(self):
        bottle,apply=self.fixture();bottle['production_binding']['external_id']='2876'
        with self.assertRaisesRegex(ValueError,'SEASONAL_EXISTING_PHYSICS'):
            apply()

    def test_existing_physical_semantics_conflict_preserves_authority(self):
        bottle,apply=self.fixture();bottle['semantics']['physical']['value']['weight']={'state':'KNOWN','value':999}
        with self.assertRaisesRegex(ValueError,'SEASONAL_EXISTING_PHYSICS'):
            apply()


if __name__ == '__main__':
    unittest.main()
