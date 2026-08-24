#!/usr/bin/env python3
from __future__ import annotations
import copy
import importlib.util
import json
from pathlib import Path
import sys
import tempfile

HERE=Path(__file__).resolve().parent
SPEC=importlib.util.spec_from_file_location('animated_creatures',HERE/'animated.py')
assert SPEC and SPEC.loader
module=importlib.util.module_from_spec(SPEC); sys.modules[SPEC.name]=module; SPEC.loader.exec_module(module)

class FakeAppearance:
    CAPABILITY='animated-appearances-v1'
    CONTRACT_ID='oteryn-game-atlas-animated-appearances-v1'
    class ProductError(RuntimeError): pass
    calls=0
    @staticmethod
    def _source_identity(): return {'source':'fixture'}
    @staticmethod
    def resolve_outfit_presentation(product, **values):
        FakeAppearance.calls += 1
        if values['look_type']==999:
            raise FakeAppearance.ProductError('unknown lookType 999')
        enabled=[0,1] if values['addons'] else [0]
        moving_directions={'north':0,'east':1,'south':2,'west':3}
        if values['look_type']==666:
            moving_directions={'north':0,'east':1,'west':3}
        return {
            'outfit_presentation_id':f"outfit-presentation:fixture:{values['look_type']}",
            'groups':[
                {
                    'animation_program_id':'animation-program:idle',
                    'directions':{'north':0,'east':1,'south':2,'west':3},
                    'enabled_addon_pattern_y':enabled,
                    'frame_group':{'id':0,'semantic':'outfit-idle','type':0},
                    'pattern_z':0,
                    'phase_count':2,
                    'animation':{'loop_type':'infinite','presentation_durations_ms':[100,100]},
                },
                {
                    'animation_program_id':'animation-program:moving',
                    'directions':moving_directions,
                    'enabled_addon_pattern_y':enabled,
                    'frame_group':{'id':1,'semantic':'outfit-moving','type':1},
                    'pattern_z':0,
                    'phase_count':8,
                    'animation':{'loop_type':'infinite','presentation_durations_ms':[100]*8},
                },
            ],
        }

class FakeSpatial:
    CAPABILITY='outfit-spatial-v1'
    class SpatialError(RuntimeError): pass
    @staticmethod
    def load_index(product):
        def row(look_type, reverse=False):
            return {
                'look_type':look_type,'spatial_record_id':f'outfit-spatial:{look_type}',
                'anchor_policy':'tile-bottom-right-minus-sprite-overhang-and-displacement-v1',
                'animate_always':False,'displacement':{'x':8,'y':4},
                'reverse_addons':{'north':False,'east':False,'south':reverse,'west':False},
            }
        return ({'capability':FakeSpatial.CAPABILITY,'source':FakeAppearance._source_identity(),'product_root':'sha256:spatial'},
                {128:row(128),21:row(21),666:row(666),777:row(777,True)})

STATIC={
    'contract_id':'oteryn-game-atlas-export-v1','capability':'static-creatures-v1','semantic_digest':'sha256:old',
    'statistics':{'npcs':6,'monster_spawns':1,'unresolved':1,'ambiguous':1},
    'npcs':[
        {'name':'Known','resolution_state':'RESOLVED','appearance':{'outfit_key':'128-1-2-3-4-1','look_type':128,'head':1,'body':2,'legs':3,'feet':4,'addons':1}},
        {'name':'KnownAgain','resolution_state':'RESOLVED','appearance':{'outfit_key':'128-1-2-3-4-1','look_type':128,'head':1,'body':2,'legs':3,'feet':4,'addons':1}},
        {'name':'UnknownLook','resolution_state':'RESOLVED','appearance':{'outfit_key':'999-0-0-0-0-0','look_type':999,'head':0,'body':0,'legs':0,'feet':0,'addons':0}},
        {'name':'Reverse','resolution_state':'RESOLVED','appearance':{'outfit_key':'777-0-0-0-0-1','look_type':777,'head':0,'body':0,'legs':0,'feet':0,'addons':1}},
        {'name':'MovingUnsupported','resolution_state':'RESOLVED','appearance':{'outfit_key':'666-0-0-0-0-0','look_type':666,'head':0,'body':0,'legs':0,'feet':0,'addons':0}},
        {'name':'Ambiguous','resolution_state':'AMBIGUOUS'},
    ],
    'monster_spawns':[{'name':'Monster','resolution_state':'RESOLVED','appearance':{'outfit_key':'21-0-0-0-0-0','look_type':21,'head':0,'body':0,'legs':0,'feet':0,'addons':0}}],
}

def main():
    with tempfile.TemporaryDirectory() as tmp:
        root=Path(tmp);spatial=root/'spatial';spatial.mkdir()
        (root/'manifest.json').write_text(json.dumps({'capability':FakeAppearance.CAPABILITY,'contract_id':FakeAppearance.CONTRACT_ID,'source':FakeAppearance._source_identity(),'product_root':'sha256:fixture'}))
        FakeAppearance.calls=0
        first=module.enrich_creatures(STATIC,root,spatial,appearance_module=FakeAppearance,spatial_module=FakeSpatial)
        assert FakeAppearance.calls==5, FakeAppearance.calls
        FakeAppearance.calls=0
        second=module.enrich_creatures(STATIC,root,spatial,appearance_module=FakeAppearance,spatial_module=FakeSpatial)
        assert FakeAppearance.calls==5, FakeAppearance.calls
        assert first==second
        assert first['capability']=='animated-creatures-v1'
        assert first['outfit_spatial_product_root']=='sha256:spatial'
        projection=first['npcs'][0]['outfit_presentation']['static_projection']
        assert projection['frame_group']['semantic']=='outfit-idle'
        assert projection['animation_program_id']=='animation-program:idle'
        assert projection['direction']=='south' and projection['pattern_x']==2 and projection['pattern_z']==0
        assert projection['displacement']=={'x':8,'y':4}
        assert projection['anchor_policy']=='tile-bottom-right-minus-sprite-overhang-and-displacement-v1'
        assert projection['uses_moving_group_in_place'] is False
        moving=first['npcs'][0]['outfit_presentation']['moving_in_place_projection']
        assert moving['frame_group']['semantic']=='outfit-moving'
        assert moving['animation_program_id']=='animation-program:moving'
        assert moving['direction']=='south' and moving['pattern_x']==2 and moving['pattern_z']==0
        assert moving['displacement']==projection['displacement']
        assert moving['phase_count']==8 and moving['animation']['presentation_durations_ms']==[100]*8
        assert first['npcs'][0]['outfit_presentation']['moving_in_place_resolution_state']=='RESOLVED'
        unsupported=first['npcs'][4]['outfit_presentation']
        assert unsupported['static_projection']['frame_group']['semantic']=='outfit-idle'
        assert unsupported['moving_in_place_resolution_state']=='FALLBACK_STATIC'
        assert unsupported['moving_in_place_reason']=='UNSUPPORTED_MOVING_DIRECTION'
        assert 'moving_in_place_projection' not in unsupported
        assert first['npcs'][0]['presentation_resolution_state']=='RESOLVED'
        assert first['npcs'][2]['presentation_resolution_state']=='UNRESOLVED_APPEARANCE'
        assert first['npcs'][2]['presentation_reason']=='UNKNOWN_LOOK_TYPE'
        assert first['npcs'][2]['presentation_fallback']=='factual-marker'
        assert first['npcs'][3]['presentation_resolution_state']=='UNRESOLVED_APPEARANCE'
        assert first['npcs'][3]['presentation_reason']=='UNSUPPORTED_REVERSE_ADDONS_SOUTH'
        assert first['npcs'][5]['presentation_resolution_state']=='FALLBACK_MARKER'
        assert first['monster_spawns'][0]['presentation_resolution_state']=='RESOLVED'
        assert first['statistics']['presentation_unresolved']==2
        assert first['statistics']['outfit_resolution_cache_entries']==5
        assert first['statistics']['npc_presentation']['resolved_animated_unique_outfits']==2
        assert first['statistics']['monster_presentation']['resolved_animated_unique_outfits']==1
        assert first['statistics']['npc_presentation']['resolved_moving_in_place_records']==2
        assert first['statistics']['npc_presentation']['resolved_dynamic_moving_in_place_records']==2
        assert first['statistics']['npc_presentation']['fallback_static_moving_in_place_records']==1
        assert first['statistics']['monster_presentation']['resolved_moving_in_place_records']==1
        corrupt=copy.deepcopy(first)
        corrupt['npcs'][0]['outfit_presentation']['moving_in_place_projection']['pattern_x']=1
        try:
            module.validate_animated_creatures(corrupt)
        except RuntimeError as exc:
            assert 'direction mismatch' in str(exc)
        else:
            raise AssertionError('corrupt moving-in-place projection was accepted')
    print('game-atlas animated creatures self-test: PASS')
    return 0

if __name__=='__main__': raise SystemExit(main())
