import copy,json,struct
from pathlib import Path
import unittest
import build_source_world as producer

ARTIFACT=Path(__file__).with_name('canary-thalom-world.json')

class SourceWorldTests(unittest.TestCase):
    def test_actual_source_rectangle_has_genuine_temple_rope_and_multifloor_ground(self):
        doc=json.loads(ARTIFACT.read_bytes())
        assert len(doc['tiles'])==2833
        assert doc['source_town']=={'town_id':6,'name':'Thalom','temple':[5854,5298,5]}
        tiles={tuple(t['source_position']):t for t in doc['tiles']}
        assert set(p[2] for p in tiles)=={5,6,7,8}
        origin=tiles[(5861,5312,8)]; dest=tiles[(5861,5311,7)]
        assert origin['native_position']==[7,14,-8]
        assert origin['semantics']['ground']==386
        assert origin['semantics']['ground_speed']==120
        assert dest['native_position']==[7,13,-7]
        assert dest['semantics']['ground']==353
        assert dest['semantics']['ground_speed']==140
        assert not origin['semantics']['unmaterialized_dynamic_items']
        assert not dest['semantics']['unmaterialized_dynamic_items']
        assert tiles[(5854,5298,5)]['semantics']['flags']['protection_zone'] is True
        assert len({tuple(t['native_position']) for t in doc['tiles']})==len(doc['tiles'])
        by_id={p['server_item_id']:p for p in doc['item_policies']}
        for t in doc['tiles']: assert producer.tile_semantics(t,by_id)==t['semantics']

    def test_mutable_source_stacks_are_unavailable_never_empty_current_world(self):
        doc=json.loads(ARTIFACT.read_bytes())
        dynamic=[t for t in doc['tiles'] if t['semantics']['unmaterialized_dynamic_items']]
        assert len(dynamic)==49
        assert all(t['items'] and t['semantics']['top_ids'] for t in dynamic)

    def test_mutable_native_bindings_are_explicit_and_preserve_both_source_pins(self):
        doc=json.loads(ARTIFACT.read_bytes())
        authored=json.loads(Path(__file__).with_name('source-map-native-item-bindings.json').read_bytes())
        native={b['external_id']:b for b in doc['native_item_bindings']}
        assert len(native)==37
        assert doc['native_item_bindings']==authored['bindings']
        source={b['external_id']:b for b in doc['source_identity_bindings']}
        mutable={str(i) for tile in doc['tiles'] for i in tile['semantics']['unmaterialized_dynamic_items']}
        assert set(native)==mutable
        for id,binding in native.items():
            assert source[id]['target']==binding['target']
            assert source[id]['source_revision']==producer.PIN
            assert binding['source_revision']=='ff7ede593c69d4c658b382c97443e8155926924a'
        assert 'src/io/io_definitions.hpp' in {proof['path'] for proof in doc['source_closure']}

    def test_item_movement_attributes_override_type_exactly(self):
        doc=json.loads(ARTIFACT.read_bytes()); policies={p['server_item_id']:p for p in doc['item_policies']}
        mutable=next(p for p in policies.values() if p['movable'] and not p['pickupable'] and not p['container'] and p['dynamic_kind'] is None and not p['ground'])
        row={'items':[{'server_item_id':mutable['server_item_id'],'depth':0,'attributes':{}}],'otbm_flags':0}
        assert producer.tile_semantics(row,policies)['unmaterialized_dynamic_items']
        row['items'][0]['attributes']={'4':100}
        assert not producer.tile_semantics(row,policies)['unmaterialized_dynamic_items']
        row['items'][0]['attributes']={'4':101}
        assert producer.tile_semantics(row,policies)['unmaterialized_dynamic_items']
        row['items'][0]['attributes']={'5':1}
        assert not producer.tile_semantics(row,policies)['unmaterialized_dynamic_items']

    def test_protobuf_truncation_and_source_unknowns_are_rejected(self):
        with self.assertRaises(ValueError):producer.fields(b'\x0a\x04x')
        with self.assertRaises(ValueError):producer.fields(b'\x00\x01')
        with self.assertRaises(ValueError):producer.policies(b'',b'<items/>',{386})

if __name__=="__main__": unittest.main()
