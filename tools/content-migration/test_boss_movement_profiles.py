"""Actual native input must authorize the sourced Ground-only movement Items."""
import json
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[2]


class BossMovementProfileTests(unittest.TestCase):
    def test_source_overlay_admits_only_the_exact_ground_profiles(self):
        import boss_movement_item_profiles as movement
        import boss_corpse_profiles as base
        generated={'profiles':base.encode({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':[]}),
                   'manifest':base.encode({'item_profiles':{'sha256':'0'*64}})}
        movement.admit(ROOT,generated,'profiles','manifest',
            lambda row:row['production_definition']['production_key'],base.encode)
        records=json.loads(generated['profiles'])['records']
        self.assertEqual(len(records),13)
        self.assertEqual({int(row['production_binding']['external_id']) for row in records},movement.IDS)
        self.assertTrue(all(row['admission']['legal_destinations']==['Ground'] for row in records))

    def test_reference_declarations_keep_the_existing_ordinals_and_unknown_semantics(self):
        import boss_movement_item_profiles as movement
        index=json.loads((ROOT/'content/items/index.json').read_bytes())
        records=[]
        for shard in index['shards']:
            records.extend(row['definition'] for row in json.loads((ROOT/shard).read_bytes())['records'])
        before=json.dumps(records,sort_keys=True)
        movement.extend_records(records,ROOT)
        self.assertEqual(json.dumps(records,sort_keys=True),before)
        for offset,expected in enumerate(movement.definitions()):
            self.assertEqual(records[movement.FIRST+offset],expected)
        # A plausible source key at an occupied ordinal is still not this
        # declaration. The source packet cannot turn it into a new alias.
        records[movement.FIRST]['identity']['key']='oteryn:item.tibia.i42851'
        with self.assertRaisesRegex(ValueError,'BOSS_MOVEMENT_(REFERENCE|NEXT_FREE_ORDINAL)_CHANGED'):
            movement.extend_records(records,ROOT)

    def test_actual_native_input_has_literal_rotten_fog_portal_policy(self):
        document=json.loads((ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
        rows=[row for row in document['records'] if row['production_binding']['external_id']=='33004']
        self.assertEqual(len(rows),1,'actual native source fog portal33004 policy is missing')
        row=rows[0]
        self.assertEqual(row['admission']['legal_destinations'],['Ground'])
        self.assertEqual(row['semantics']['classification']['value']['item_type'],{'state':'KNOWN','value':'TELEPORT'})
        self.assertFalse(row['attributes']['blocks_movement'])
        self.assertEqual(row['semantics']['physical']['value']['movable'],{'state':'KNOWN','value':False})
        self.assertEqual(row['semantics']['physical']['value']['pickupable'],{'state':'KNOWN','value':False})

    def test_actual_native_input_has_source_event_vortices_without_invented_teleports(self):
        document=json.loads((ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
        by_id={row['production_binding']['external_id']:row for row in document['records']}
        self.assertEqual([item for item in (20121,23726,23727,23728) if str(item) not in by_id],[],
                         'actual native event source closure is missing')
        for item in (20121,23726,23727,23728):
            row=by_id[str(item)]
            self.assertEqual(row['admission']['legal_destinations'],['Ground'])
            self.assertNotEqual(row['semantics']['classification'].get('value',{}).get('item_type',{}).get('value'),'TELEPORT')
            self.assertEqual(row['semantics']['temporal']['state'],'NOT_APPLICABLE')

    def test_actual_native_input_has_sourced_ground_movement_closure(self):
        document=json.loads((ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
        by_id={row['production_binding']['external_id']:row for row in document['records']}
        for item in (37000,42851,42853,43294,43295,43589,43590,43625):
            self.assertTrue(str(item) in by_id,f'actual native movement Item{item} is unavailable')
            self.assertEqual(by_id[str(item)]['admission']['legal_destinations'],['Ground'])


if __name__=='__main__':unittest.main()
