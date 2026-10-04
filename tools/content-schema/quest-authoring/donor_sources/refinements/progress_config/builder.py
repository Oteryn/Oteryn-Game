"""Finite Cults Orc mission Source config-to-callback write specialization (not execution)."""
import argparse
import base64
import hashlib
import json
import sys
from pathlib import Path

SOURCE = 'canary'
REVISION = '04b83b512114bfd888000d6e1433ed8ecaec7c5b'
SCRIPT = 'data-otservbr-global/scripts/quests/cults_of_tibia/creaturescripts_bosses_mission_cults.lua'
HELPER = 'data/libs/functions/functions.lua'
TRACK = 'canary:quest-progress/quest/u11_40/cults_of_tibia/orcs/mission'
STORAGE_EXPRESSION = 'Storage.Quest.U11_40.CultsOfTibia.Orcs.Mission'
BOSS = 'the unarmored voidborn'
SCRIPT_SHA = '05272eb00ce068612ec38128abef9eba68e4d0946494f0771554b2ea35c4647e'
HELPER_BODY_SHA = '9c6ae3d8a1f61a9caed740cf2d913b959264855c5f0119ba45dd047455b5846c'


def digest(v):
    return hashlib.sha256(json.dumps(v, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def dotted(node):
    if node['node_type'] == 'Name':
        return node['fields']['id']
    if node['node_type'] == 'Index' and node['fields']['notation']['name'] == 'DOT':
        a, b = dotted(node['fields']['value']), dotted(node['fields']['idx'])
        return a+'.'+b if a and b else None
    return None


def project(engine, helper_engine, mission_refs):
    """Closed exact Source shape; callback selection and all original guards are retained."""
    if hashlib.sha256(engine.text.encode()).hexdigest() != SCRIPT_SHA:
        raise ValueError('Cults finite Source script body drift')
    ast=engine.ast
    nodes=dict(engine.nodes)
    declarations=[(p,n) for p,n in engine.nodes if n['node_type']=='LocalAssign' and len(n['fields']['targets'])==1 and dotted(n['fields']['targets'][0])=='bosses']
    if len(declarations)!=1 or declarations[0][1]['fields']['values'][0]['node_type']!='Table':
        raise ValueError('Cults bosses table declaration differs')
    dp,decl=declarations[0];table=decl['fields']['values'][0];found=[]
    for i,field in enumerate(table['fields']['fields']):
        key=field['fields']['key']
        if key['node_type']=='String' and base64.b64decode(key['fields']['s']['bytes_base64']).decode()==BOSS:found.append((dp+'/fields/values/0/fields/fields/'+str(i),field))
    if len(found)!=1:raise ValueError('Cults Orc config key differs')
    config_pointer,config=found[0];values=config['fields']['value']
    if values['node_type']!='Table':raise ValueError('Cults Orc config shape differs')
    fields={dotted(f['fields']['key']):(i,f['fields']['value']) for i,f in enumerate(values['fields']['fields'])}
    if len(fields)!=len(values['fields']['fields']) or set(fields)!= {'storage','value'}:
        raise ValueError('Cults Orc config field set differs')
    if dotted(fields['storage'][1])!=STORAGE_EXPRESSION or fields['value'][1]['node_type']!='Number' or fields['value'][1]['fields']['n']!=2:
        raise ValueError('Cults Orc config storage/value differs')
    lookups=[(p,n) for p,n in engine.nodes if n['node_type']=='LocalAssign' and len(n['fields']['targets'])==1 and dotted(n['fields']['targets'][0])=='boss' and len(n['fields']['values'])==1]
    if len(lookups)!=1:raise ValueError('Cults boss lookup count differs')
    lp,lookup=lookups[0];value=lookup['fields']['values'][0]
    if value['node_type']!='Index' or value['fields']['notation']['name']!='SQUARE' or dotted(value['fields']['value'])!='bosses' or dotted(value['fields']['idx'])!='monsterName':
        raise ValueError('Cults boss lookup expression differs')
    calls=[(p,n) for p,n in engine.nodes if n['node_type']=='Call' and dotted(n['fields']['func'])=='onDeathForDamagingPlayers']
    if len(calls)!=1 or len(calls[0][1]['fields']['args'])!=2 or calls[0][1]['fields']['args'][1]['node_type']!='AnonymousFunction':raise ValueError('Cults damaging players callback differs')
    cp,call=calls[0];callback_prefix=cp+'/fields/args/1'
    writes=[(p,n) for p,n in engine.nodes if p.startswith(callback_prefix+'/') and n['node_type']=='Invoke' and dotted(n['fields']['func'])=='setStorageValue']
    if len(writes)!=1:raise ValueError('Cults generic write count differs')
    wp,write=writes[0]
    if dotted(write['fields']['source'])!='player' or [dotted(a) for a in write['fields']['args']]!=['boss.storage','boss.value']:
        raise ValueError('Cults generic write operands differ')
    # Exact lexical boss lookup must be the one captured inside the callback.
    binding=engine.resolve('boss',wp)
    if not binding or binding['declaration_pointer']!=lp+'/fields/targets/0':raise ValueError('Cults boss lexical binding differs')
    helper_defs=[(p,n) for p,n in helper_engine.nodes if n['node_type']=='Function' and dotted(n['fields']['name'])=='onDeathForDamagingPlayers']
    if len(helper_defs)!=1:raise ValueError('Cults helper declaration differs')
    hp,helper=helper_defs[0];hw=helper_engine.witness(helper,hp)
    if hw.get('slice_sha256')!=HELPER_BODY_SHA:raise ValueError('Cults damaging players helper body drift')
    from donor_sources.refinements.progress import builder as progress
    return {'source_component_id': ':'.join(engine.provenance[k] for k in ('source','revision','path')),
            'quest_keys': sorted({r['quest_key'] for r in mission_refs}), 'source_target': TRACK,
            'canonical_mission_refs': mission_refs, 'provenance': engine.provenance,
            'status': 'SOURCE_CONFIG_MISSION_WRITE_SPEC_EXECUTION_UNPROVEN', 'runtime_enabled': False, 'native_admission': False, 'canonical_gap_replaced': False,
            'case': {'kind':'source_dictionary_case','key':BOSS,'config_witness':engine.witness(config,config_pointer),'table_declaration_witness':engine.witness(decl,dp),'lookup_witness':engine.witness(lookup,lp),'boss_binding':binding,'no_monster_entity_id_inferred':True,'finite_script_sha256':SCRIPT_SHA},
            'source_storage_expression':STORAGE_EXPRESSION,
            'config_fields': {name:{'ast_pointer':config_pointer+'/fields/value/fields/fields/'+str(index)+'/fields/value','witness':engine.witness(n,config_pointer+'/fields/value/fields/fields/'+str(index)+'/fields/value')} for name,(index,n) in fields.items()},
            'write_ast_pointer':wp,'write_witness':engine.witness(write,wp),
            'operation':{'kind':'source_config_selected_storage_write','evaluation_order':['READ_RECEIVER_ONCE','LOOKUP_METHOD','EVALUATE_STORAGE_KEY','EVALUATE_VALUE','CALL_WITH_RECEIVER_SELF'],'source_storage_key':'boss.storage','source_value_expression':'boss.value','declared_case_value':2,'native_storage_slot':None,'branch_source_context':progress.enclosing_context(engine,wp),'helper_call_witness':engine.witness(call,cp),
                         'player_iteration':{'kind':'source_damage_map_player_iteration','helper_witness':hw,'steps':['GET_CREATURE_DAMAGE_MAP','PAIRS_ITERATION_ORDER_UNSPECIFIED','PLAYER_CONSTRUCTOR_FOR_EACH_DAMAGE_MAP_KEY','CALL_CALLBACK_ONLY_FOR_TRUTHY_PLAYER'],'callback_arguments':['original_creature','resolved_player'],'runtime_activation':False},
                         'execution_uncertainties':['GLOBAL_HELPER_AND_PLAYER_CONSTRUCTOR_BINDINGS','ACTUAL_MONSTER_NAME_AND_LOCAL_CONFIG_LOOKUP_AT_RUNTIME','METHOD_INDEX_ERRORS_AND_SIDE_EFFECTS','ACTUAL_STORAGE_SLOT_AND_NATIVE_ACTOR_BINDING']}}


def build(root, manifest_path, ast_root):
    from donor_sources.refinements.fields import builder as fields
    root,manifest_path=Path(root),Path(manifest_path);files=json.loads(manifest_path.read_text())['files']
    selected=[]
    for path in [SCRIPT,HELPER]:
        matches=[r for r in files if r['source']==SOURCE and r['revision']==REVISION and r['path']==path]
        if len(matches)!=1:raise ValueError('Cults exact pinned Source file absent')
        selected.append(matches[0])
    loader=fields.ASTLoader(ast_root,{r['sha256'] for r in selected});engines=[]
    for row in selected:
        raw=fields.read_source(row,manifest_path);engines.append(fields.Engine(loader.capture(row['sha256'],raw),raw,fields.provenance(row)))
    refs=[]
    for file in sorted((root/'content/quests/definitions').glob('quests-*.json')):
        for ri,row in enumerate(json.loads(file.read_text())['records']):
            q=row['definition']
            for i,m in enumerate(q.get('source_data',{}).get('quest',{}).get('missions',[])):
                if m['progress']==TRACK:refs.append({'quest_key':q['identity']['key'],'mission_key':m['key'],'mission':m,'mission_sha256':digest(m),'path':file.relative_to(root).as_posix(),'json_pointer':'/records/'+str(ri)+'/definition/source_data/quest/missions/'+str(i)})
    if len(refs)!=1 or refs[0]['quest_key']!='oteryn:quest.cults_of_tibia_quest':raise ValueError('Cults Orc canonical mission differs')
    record=project(*engines,refs)
    return {'schema':'OTERYN_SOURCE_CONFIG_MISSION_PROGRESS_SUPPLEMENT/v1','records':[record],'original_core_unchanged':True,'runtime_enabled':False,'native_admission':False,'summary':{'records':1,'source_tracks':1,'canonical_mission_joins':1,'source_files':2}}


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for arg in ['repo-root','corpus-manifest','ast-root','out']:p.add_argument('--'+arg,required=True)
    a=p.parse_args();sys.path.insert(0,str(Path(a.repo_root)/'tools/content-schema/quest-authoring'));packet=build(a.repo_root,a.corpus_manifest,a.ast_root);Path(a.out).write_text(json.dumps(packet,ensure_ascii=False,indent=2)+'\n');print(json.dumps(packet['summary']))


if __name__=='__main__':main()
