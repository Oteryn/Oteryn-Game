#!/usr/bin/env python3
from __future__ import annotations
import copy
import hashlib
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
        idle={
            'animation_program_id':'animation-program:idle',
            'directions':{'north':0,'east':1,'south':2,'west':3},
            'enabled_addon_pattern_y':enabled,
            'frame_group':{'id':0,'semantic':'outfit-idle','type':0},
            'pattern_z':0,
            'phase_count':2,
            'animation':{'loop_type':'infinite','presentation_durations_ms':[100,100]},
        }
        moving={
            'animation_program_id':'animation-program:moving',
            'directions':{'north':0,'east':1,'south':2,'west':3},
            'enabled_addon_pattern_y':enabled,
            'frame_group':{'id':1,'semantic':'outfit-moving','type':1},
            'pattern_z':0,
            'phase_count':8,
            'animation':{'loop_type':'infinite','presentation_durations_ms':[80,90,100,110,120,130,140,150]},
        }
        if values['look_type']==555:
            groups=[idle]
        elif values['look_type']==556:
            groups=[{**idle,'directions':{'south':0}},{**moving,'directions':{'south':0}}]
        elif values['look_type']==557:
            groups=[idle,{**moving,'directions':{'north':0,'east':1}}]
        else:
            groups=[idle,moving]
        return {
            'outfit_presentation_id':f"outfit-presentation:{values['look_type']}",
            'colors_rgb':{'head':[1,2,3],'body':[4,5,6],'legs':[7,8,9],'feet':[10,11,12]},
            'mask_policy':'tibia-outfit-mask-layer1-v1',
            'groups':groups,
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
                {128:row(128),21:row(21),555:row(555),556:row(556),557:row(557),777:row(777,True)})

STATIC={
    'contract_id':'oteryn-game-atlas-export-v1','capability':'static-creatures-v1','semantic_digest':'sha256:old',
    'statistics':{'npcs':8,'monster_spawns':1,'unresolved':1,'ambiguous':1},
    'npcs':[
        {'record_id':'npc:known','name':'Known','x':100,'y':200,'floor':7,'resolution_state':'RESOLVED','appearance':{'outfit_key':'128-1-2-3-4-1','look_type':128,'head':1,'body':2,'legs':3,'feet':4,'addons':1}},
        {'name':'KnownAgain','resolution_state':'RESOLVED','appearance':{'outfit_key':'128-1-2-3-4-1','look_type':128,'head':1,'body':2,'legs':3,'feet':4,'addons':1}},
        {'name':'UnknownLook','resolution_state':'RESOLVED','appearance':{'outfit_key':'999-0-0-0-0-0','look_type':999,'head':0,'body':0,'legs':0,'feet':0,'addons':0}},
        {'name':'Reverse','resolution_state':'RESOLVED','appearance':{'outfit_key':'777-0-0-0-0-1','look_type':777,'head':0,'body':0,'legs':0,'feet':0,'addons':1}},
        {'name':'IdleOnly','resolution_state':'RESOLVED','appearance':{'outfit_key':'555-0-0-0-0-0','look_type':555,'head':0,'body':0,'legs':0,'feet':0,'addons':0}},
        {'name':'WidthOne','resolution_state':'RESOLVED','appearance':{'outfit_key':'556-0-0-0-0-0','look_type':556,'head':0,'body':0,'legs':0,'feet':0,'addons':0}},
        {'name':'UnsupportedMoving','resolution_state':'RESOLVED','appearance':{'outfit_key':'557-0-0-0-0-0','look_type':557,'head':0,'body':0,'legs':0,'feet':0,'addons':0}},
        {'name':'Ambiguous','resolution_state':'AMBIGUOUS'},
    ],
    'monster_spawns':[{'record_id':'monster:fixture','name':'Monster','x':300,'y':400,'floor':7,'resolution_state':'RESOLVED','appearance':{'outfit_key':'21-0-0-0-0-0','look_type':21,'head':0,'body':0,'legs':0,'feet':0,'addons':0}}],
}

def main():
    with tempfile.TemporaryDirectory() as tmp:
        root=Path(tmp); spatial=root/'spatial'; spatial.mkdir()
        (root/'manifest.json').write_text(json.dumps({'capability':FakeAppearance.CAPABILITY,'contract_id':FakeAppearance.CONTRACT_ID,'source':FakeAppearance._source_identity(),'product_root':'sha256:fixture'}))
        FakeAppearance.calls=0
        first=module.enrich_creatures(STATIC,root,spatial,appearance_module=FakeAppearance,spatial_module=FakeSpatial)
        assert FakeAppearance.calls==7, FakeAppearance.calls
        FakeAppearance.calls=0
        second=module.enrich_creatures(STATIC,root,spatial,appearance_module=FakeAppearance,spatial_module=FakeSpatial)
        assert FakeAppearance.calls==7, FakeAppearance.calls
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
        assert first['playback_projection_capability']=='creature-moving-in-place-v1'
        playback=first['npcs'][0]['outfit_presentation']['playback_projection']
        assert playback['frame_group']['semantic']=='outfit-moving'
        assert playback['animation_program_id']=='animation-program:moving'
        assert playback['direction']=='south' and playback['pattern_x']==2 and playback['pattern_z']==0
        assert playback['phase_count']==8
        assert playback['animation']['presentation_durations_ms']==[80,90,100,110,120,130,140,150]
        assert playback['enabled_addon_pattern_y']==[0,1]
        assert playback['displacement']=={'x':8,'y':4}
        assert playback['outfit_presentation_id']=='outfit-presentation:128'
        assert playback['selection_policy']=='prefer-outfit-moving-in-place-else-static-v1'
        assert playback['playback_resolution_state']=='RESOLVED_MOVING_IN_PLACE'
        assert playback['presentation_mode']=='moving-in-place'
        assert playback['world_position_policy']=='UNCHANGED'
        assert first['npcs'][0]['record_id']=='npc:known'
        assert (first['npcs'][0]['x'],first['npcs'][0]['y'],first['npcs'][0]['floor'])==(100,200,7)
        idle_only=first['npcs'][4]['outfit_presentation']['playback_projection']
        assert idle_only['playback_resolution_state']=='FALLBACK_STATIC_PROJECTION'
        assert idle_only['playback_reason']=='MOVING_GROUP_UNAVAILABLE'
        assert idle_only['frame_group']['semantic']=='outfit-idle'
        width_one=first['npcs'][5]['outfit_presentation']['playback_projection']
        assert width_one['playback_resolution_state']=='RESOLVED_MOVING_IN_PLACE'
        assert width_one['pattern_x']==0 and width_one['direction']=='south'
        unsupported=first['npcs'][6]['outfit_presentation']['playback_projection']
        assert unsupported['playback_resolution_state']=='FALLBACK_STATIC_PROJECTION'
        assert unsupported['playback_reason']=='MOVING_DIRECTION_UNAVAILABLE'
        spatial_row=FakeSpatial.load_index(spatial)[1][128]
        ambiguous_source=copy.deepcopy(first['npcs'][0]['outfit_presentation'])
        ambiguous_source['groups'].append(copy.deepcopy(ambiguous_source['groups'][1]))
        ambiguous=module._playback_projection(ambiguous_source,spatial_row,1,projection)
        assert ambiguous['playback_resolution_state']=='FALLBACK_STATIC_PROJECTION'
        assert ambiguous['playback_reason']=='AMBIGUOUS_MOVING_GROUP'
        malformed_source=copy.deepcopy(first['npcs'][0]['outfit_presentation'])
        malformed_source['groups'][1]['animation']=None
        malformed=module._playback_projection(malformed_source,spatial_row,1,projection)
        assert malformed['playback_resolution_state']=='FALLBACK_STATIC_PROJECTION'
        assert malformed['playback_reason']=='MOVING_TIMING_UNAVAILABLE'
        assert first['statistics']['npc_presentation']['resolved_moving_playback_records']==3
        assert first['statistics']['npc_presentation']['fallback_static_playback_records']==2
        module.verify_enriched_creatures(first,spatial,spatial_module=FakeSpatial)
        corrupted=copy.deepcopy(first)
        corrupted['npcs'][0]['outfit_presentation']['playback_projection']['pattern_x']=99
        body=copy.deepcopy(corrupted); body.pop('semantic_digest',None)
        corrupted['semantic_digest']='sha256:'+hashlib.sha256(module._canonical(body)).hexdigest()
        try:
            module.verify_enriched_creatures(corrupted,spatial,spatial_module=FakeSpatial)
        except RuntimeError:
            pass
        else:
            raise AssertionError('corrupt playback projection was accepted')
        assert first['npcs'][0]['presentation_resolution_state']=='RESOLVED'
        assert first['npcs'][2]['presentation_resolution_state']=='UNRESOLVED_APPEARANCE'
        assert first['npcs'][2]['presentation_reason']=='UNKNOWN_LOOK_TYPE'
        assert first['npcs'][2]['presentation_fallback']=='factual-marker'
        assert first['npcs'][3]['presentation_resolution_state']=='UNRESOLVED_APPEARANCE'
        assert first['npcs'][3]['presentation_reason']=='UNSUPPORTED_REVERSE_ADDONS_SOUTH'
        assert first['npcs'][7]['presentation_resolution_state']=='FALLBACK_MARKER'
        assert first['monster_spawns'][0]['presentation_resolution_state']=='RESOLVED'
        assert first['statistics']['presentation_unresolved']==2
        assert first['statistics']['outfit_resolution_cache_entries']==7
        assert first['statistics']['npc_presentation']['resolved_animated_unique_outfits']>=1
        assert first['statistics']['monster_presentation']['resolved_animated_unique_outfits']==1
    print('game-atlas animated creatures self-test: PASS')
    return 0

if __name__=='__main__': raise SystemExit(main())
