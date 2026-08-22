#!/usr/bin/env python3
from __future__ import annotations
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
        return {
            'outfit_presentation_id':'outfit-presentation:fixture',
            'groups':[
                {
                    'animation_program_id':'animation-program:idle',
                    'directions':{'north':0,'east':1,'south':2,'west':3},
                    'enabled_addon_pattern_y':[0],
                    'frame_group':{'id':0,'semantic':'outfit-idle','type':0},
                    'pattern_z':0,
                    'phase_count':2,
                    'animation':{'loop_type':'infinite','presentation_durations_ms':[100,100]},
                },
                {
                    'animation_program_id':'animation-program:moving',
                    'directions':{'north':0,'east':1,'south':2,'west':3},
                    'enabled_addon_pattern_y':[0],
                    'frame_group':{'id':1,'semantic':'outfit-moving','type':1},
                    'pattern_z':0,
                    'phase_count':8,
                    'animation':{'loop_type':'infinite','presentation_durations_ms':[100]*8},
                },
            ],
        }

STATIC={
    'contract_id':'oteryn-game-atlas-export-v1','capability':'static-creatures-v1','semantic_digest':'sha256:old',
    'statistics':{'npcs':4,'monster_spawns':1,'unresolved':1,'ambiguous':1},
    'npcs':[
        {'name':'Known','resolution_state':'RESOLVED','appearance':{'outfit_key':'128-1-2-3-4-1','look_type':128,'head':1,'body':2,'legs':3,'feet':4,'addons':1}},
        {'name':'KnownAgain','resolution_state':'RESOLVED','appearance':{'outfit_key':'128-1-2-3-4-1','look_type':128,'head':1,'body':2,'legs':3,'feet':4,'addons':1}},
        {'name':'UnknownLook','resolution_state':'RESOLVED','appearance':{'outfit_key':'999-0-0-0-0-0','look_type':999,'head':0,'body':0,'legs':0,'feet':0,'addons':0}},
        {'name':'Ambiguous','resolution_state':'AMBIGUOUS'},
    ],
    'monster_spawns':[{'name':'Monster','resolution_state':'RESOLVED','appearance':{'outfit_key':'21-0-0-0-0-0','look_type':21,'head':0,'body':0,'legs':0,'feet':0,'addons':0}}],
}

def main():
    with tempfile.TemporaryDirectory() as tmp:
        root=Path(tmp)
        (root/'manifest.json').write_text(json.dumps({'capability':FakeAppearance.CAPABILITY,'contract_id':FakeAppearance.CONTRACT_ID,'source':FakeAppearance._source_identity(),'product_root':'sha256:fixture'}))
        FakeAppearance.calls=0
        first=module.enrich_creatures(STATIC,root,appearance_module=FakeAppearance)
        assert FakeAppearance.calls==3, FakeAppearance.calls
        FakeAppearance.calls=0
        second=module.enrich_creatures(STATIC,root,appearance_module=FakeAppearance)
        assert FakeAppearance.calls==3, FakeAppearance.calls
        assert first==second
        assert first['capability']=='animated-creatures-v1'
        projection=first['npcs'][0]['outfit_presentation']['static_projection']
        assert projection['frame_group']['semantic']=='outfit-idle'
        assert projection['animation_program_id']=='animation-program:idle'
        assert projection['direction']=='south' and projection['pattern_x']==2 and projection['pattern_z']==0
        assert projection['uses_moving_group_in_place'] is False
        assert first['npcs'][0]['presentation_resolution_state']=='RESOLVED'
        assert first['npcs'][2]['presentation_resolution_state']=='UNRESOLVED_APPEARANCE'
        assert first['npcs'][2]['presentation_reason']=='UNKNOWN_LOOK_TYPE'
        assert first['npcs'][2]['presentation_fallback']=='factual-marker'
        assert first['npcs'][3]['presentation_resolution_state']=='FALLBACK_MARKER'
        assert first['monster_spawns'][0]['presentation_resolution_state']=='RESOLVED'
        assert first['statistics']['presentation_unresolved']==1
        assert first['statistics']['outfit_resolution_cache_entries']==3
        assert first['statistics']['npc_presentation']['resolved_animated_unique_outfits']==1
        assert first['statistics']['monster_presentation']['resolved_animated_unique_outfits']==1
    print('game-atlas animated creatures self-test: PASS')
    return 0

if __name__=='__main__': raise SystemExit(main())
