#!/usr/bin/env python3
from __future__ import annotations
import importlib.util
from pathlib import Path
import sys

HERE=Path(__file__).resolve().parent
SPEC=importlib.util.spec_from_file_location('animated_fullworld',HERE/'animated.py')
assert SPEC and SPEC.loader
module=importlib.util.module_from_spec(SPEC); sys.modules[SPEC.name]=module; SPEC.loader.exec_module(module)

class FakeAppearance:
    class ProductError(RuntimeError): pass
    @staticmethod
    def resolve_object_animation_ref(product, appearance_source_id, pattern):
        if appearance_source_id==100:
            return {'animation_program_id':'animation-program:fixture','pattern':pattern,'variant_id':'animation-variant:fixture'}
        return None

def main():
    record={'presentation':[
        {'appearance_source_id':100,'resolved_primitives':[{'pattern':{'x':1,'y':0,'z':0}},{'pattern':{'x':1,'y':0,'z':0}}]},
        {'appearance_source_id':101,'resolved_primitives':[{'pattern':{'x':0,'y':0,'z':0}}]},
        {'appearance_source_id':102,'presentation_resolution_state':'UNRESOLVED_APPEARANCE','resolved_primitives':[]},
    ]}
    result,stats=module.enrich_tile_record(record,Path('/unused'),appearance_module=FakeAppearance)
    assert stats=={'animated_presentations':1,'static_presentations':1,'unresolved_animation_presentations':1}
    assert result['presentation'][0]['animation_resolution_state']=='RESOLVED'
    assert result['presentation'][1]['animation_resolution_state']=='STATIC'
    assert result['presentation'][2]['animation_resolution_state']=='UNRESOLVED_PRESENTATION'
    bad={'presentation':[{'appearance_source_id':100,'resolved_primitives':[{'pattern':{'x':0,'y':0,'z':0}},{'pattern':{'x':1,'y':0,'z':0}}]}]}
    try: module.enrich_tile_record(bad,Path('/unused'),appearance_module=FakeAppearance)
    except FakeAppearance.ProductError: pass
    else: raise AssertionError('pattern disagreement accepted')
    print('game-atlas animated fullworld self-test: PASS')
    return 0

if __name__=='__main__': raise SystemExit(main())
