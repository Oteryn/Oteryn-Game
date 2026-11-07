#!/usr/bin/env python3
"""Isolate the source-bound Deaththrower presentation variant; never rewrite input."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/monster-authoring'))
import validate_monster
import creature_admission_stage as native

ACTOR = 'deaththrower'
ABILITY = 'canary:ability/spell/dark_torturer_skill_reducer'
EFFECT = ABILITY + '/effect'
LOCAL_ABILITY = 'canary:ability/deaththrower/dark_torturer_skill_reducer'
LOCAL_EFFECT = 'canary:effect/deaththrower/dark_torturer_skill_reducer/effect'
BASELINE_SHA = '86469f7f1262cd9d92b415fbc818915cf22458169743ed37b2c19c698871c22c'
PRESENTATION_SHA = '03be25cd2a54d4f4721a4069dc8e94891eaa9dc2f6bf40639d0603134129d09e'
FLAG = 'ACTOR_LOCAL_SHARED_ABILITY_PRESENTATION_VARIANT_SOURCE_BOUND'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def localize(documents):
    """Change only typed identity/ref key values, leaving text and all mechanics exact."""
    deps = documents['dependencies.json']
    abilities = [a for a in deps['abilities'] if a['identity']['key'] == ABILITY]
    effects = [e for e in deps['effects'] if e['identity']['key'] == EFFECT]
    if len(abilities) != 1 or len(effects) != 1 or abilities[0].get('effects') != [dict(effects[0]['identity'], family='Effect')]:
        raise ValueError('Expected exactly one known Ability/Effect chain')
    if effects[0].get('presentation', {}).get('projectile_asset_binding') != 'canary.appearance:missile/suddendeath':
        raise ValueError('Expected source-bound Deaththrower projectile variant')
    replacements = {ABILITY: LOCAL_ABILITY, EFFECT: LOCAL_EFFECT}
    for collection in ('abilities', 'effects'):
        if any(r['identity']['key'] in replacements.values() for r in deps[collection]):
            raise ValueError('Actor-local identity already exists')
    result = copy.deepcopy(documents)
    changes = []
    def visit(value, file, pointer=''):
        if isinstance(value, dict):
            for key, child in value.items():
                address = pointer + '/' + key.replace('~', '~0').replace('/', '~1')
                if key == 'key' and isinstance(child, str) and child in replacements:
                    value[key] = replacements[child]
                    changes.append((file, address, child, value[key]))
                else:
                    visit(child, file, address)
        elif isinstance(value, list):
            for i, child in enumerate(value): visit(child, file, pointer + '/' + str(i))
    for file, document in result.items(): visit(document, file)
    if not any(file == 'monster.json' for file, *_ in changes):
        raise ValueError('Expected Deaththrower scheduled Ability reference')
    # Exact inverse ensures this is solely an identity isolation, including formulas/loot.
    inverse = {v: k for k, v in replacements.items()}
    restored = copy.deepcopy(result)
    def undo(value):
        if isinstance(value, dict):
            for key, child in value.items():
                if key == 'key' and isinstance(child, str) and child in inverse: value[key] = inverse[child]
                else: undo(child)
        elif isinstance(value, list):
            for child in value: undo(child)
    undo(restored)
    if restored != documents: raise ValueError('Nonidentity content changed')
    return result, changes


def native_cohesion(baseline, repaired):
    """Exercise unchanged Stage.add collision fence on all four exact donor chains."""
    stage = native.Stage(native.Mapper({}))
    for actor in ('dark_torturer', 'eclipse_knight', 'morgaroth', ACTOR):
        dependencies = repaired['dependencies.json'] if actor == ACTOR else json.loads((baseline/'bundles'/actor/'dependencies.json').read_text())
        key = LOCAL_ABILITY if actor == ACTOR else ABILITY
        ability = next(a for a in dependencies['abilities'] if a['identity']['key'] == key)
        effect_keys = {ref['key'] for ref in ability['effects']}
        effects = [e for e in dependencies['effects'] if e['identity']['key'] in effect_keys]
        formula_keys = {r['key'] for e in effects for r in [e.get('formula',{})] if 'key' in r}
        subset = {'abilities':[ability], 'effects':effects,
                  'formulas':[f for f in dependencies['formulas'] if f['identity']['key'] in formula_keys],
                  'items':[], 'documents':[], 'loot_tables':[]}
        stage.stage_dependencies(subset, actor)
    shared = stage.profiles[('Ability', 'oteryn:ability.spell.dark_torturer_skill_reducer')]
    local_key = 'oteryn:ability.creature.deaththrower.dark_torturer_skill_reducer'
    local = stage.profiles[('Ability', local_key)]
    global_presentation = shared['data']['profile']['details']['effects'][0]['effect']['presentation']
    local_presentation = local['data']['profile']['details']['effects'][0]['effect']['presentation']
    assert 'projectile_asset_binding' not in global_presentation
    assert local_presentation['projectile_asset_binding'] == 'canary.appearance:missile/suddendeath'
    return {'actors_checked':4, 'unique_native_abilities':len(stage.profiles),
            'shared_identity_reuse_passed':True, 'local_native_key':local_key,
            'stage_collision_guard_unchanged':True, 'runtime_execution_qualified':False}


def build(baseline, presentation, collision_report, output):
    baseline, output = baseline.resolve(), output.resolve()
    if output == baseline or baseline in output.parents or output == ROOT or ROOT in output.parents:
        raise ValueError('Output must be outside immutable baseline and repository')
    if sha(baseline/'population-index.json') != BASELINE_SHA or sha(presentation) != PRESENTATION_SHA:
        raise ValueError('Exact prepared population or source presentation packet drifted')
    collisions = json.loads(collision_report.read_text())
    rows = collisions['conflicts']
    expected = 'oteryn:ability.spell.dark_torturer_skill_reducer'
    if collisions.get('unparsed') or len(rows)!=1 or rows[0]['family']!='Ability' or rows[0]['native_key']!=expected:
        raise ValueError('Expected exactly one understood shared Ability conflict')
    source_packet = json.loads(presentation.read_text())
    source_rows = [r for r in source_packet['patches'] if r['monster']==ACTOR and r['file']=='dependencies.json'
                   and r['value'].get('projectile_asset_binding')=='canary.appearance:missile/suddendeath']
    if len(source_rows)!=1: raise ValueError('Missing exact source presentation patch')
    source = dict(source_rows[0]['source'], presentation_packet_sha256=PRESENTATION_SHA,
                  presentation_patch_pointer=source_rows[0]['pointer'],
                  identity_isolation=FLAG, global_parity=False)
    documents = {file:json.loads((baseline/'bundles'/ACTOR/file).read_text()) for file in
                 ('monster.json','dependencies.json','catalog.json')}
    repaired, changes = localize(documents)
    errors = validate_monster.validate(repaired['monster.json'], repaired['dependencies.json'], repaired['catalog.json'])
    if errors: raise ValueError('Repaired actor failed authoring validation: '+repr(errors[:10]))
    cohesion = native_cohesion(baseline, repaired)
    packet = {'schema':'OTERYN_MONSTER_FIELD_PATCH/v1','lane':'shared-presentation-identity-repair',
              'baseline_index_sha256':BASELINE_SHA,'patches':[{'monster':ACTOR,'file':file,'pointer':pointer,
                'expected_present':True,'expected_value':old,'value':new,'source':source,
                'reason':FLAG} for file,pointer,old,new in changes],
              'actor_flags':{ACTOR:[FLAG]},'counts':{'actors':1,'field_patches':len(changes)},
              'qualification':cohesion,'runtime_qualified':False}
    output.mkdir(parents=True,exist_ok=True)
    path=output/'field-patches.json';path.write_text(json.dumps(packet,sort_keys=True,indent=2)+'\n')
    receipt=dict(cohesion,field_packet_sha256=sha(path),authoring_schema_passed=True,
                 inverse_identity_transform_is_exact_original=True,
                 source_collision_report_sha256=sha(collision_report),producer_sha256=sha(Path(__file__)))
    (output/'qualification.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print(json.dumps(receipt));return packet


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('baseline','presentation','collision-report','output'):parser.add_argument('--'+name,type=Path,required=True)
    args=parser.parse_args();build(args.baseline,args.presentation,args.collision_report,args.output)
