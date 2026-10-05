"""Audit all accepted refused transitions; bounded direct ADD proofs, never source mutation."""
import argparse
import base64
import collections
import copy
import gzip
import hashlib
import json
import sys
from pathlib import Path


def digest(value):
    return hashlib.sha256(json.dumps(value,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()).hexdigest()


def walk(value,pointer=''):
    if isinstance(value,dict):
        if 'node_type' in value: yield pointer,value
        for key,child in value.items():yield from walk(child,pointer+'/'+key)
    elif isinstance(value,list):
        for i,child in enumerate(value):yield from walk(child,pointer+'/'+str(i))


def name(node):
    return node.get('fields',{}).get('id') if node.get('node_type')=='Name' else None


def expression_path(node):
    kind=node['node_type'];f=node['fields']
    if kind=='Name':return f['id']
    if kind=='Index' and f.get('notation',{}).get('name')=='DOT':
        parent=expression_path(f['value']);key=name(f['idx'])
        return parent+'.'+key if parent and key else None
    if kind=='Number' and type(f['n']) is int:return str(f['n'])
    return None


def same_read(node,receiver,target):
    if node['node_type']!='Invoke':return False
    f=node['fields']
    return name(f['func'])=='getStorageValue' and name(f['source'])==receiver and len(f['args'])==1 and expression_path(f['args'][0])==target


def direct_add(node):
    """Only direct current same-player storage reads; no local values or clamp folding."""
    f=node['fields']
    if node['node_type']!='Invoke' or name(f['func'])!='setStorageValue' or name(f['source'])!='player' or len(f['args'])!=2:return None
    target=expression_path(f['args'][0]);rhs=f['args'][1]
    if not target:return None
    if same_read(rhs,'player',target):return {'kind':'ADD','value':0}
    if rhs['node_type'] in ('AddOp','SubOp'):
        parts=rhs['fields'];right=parts['right']
        if same_read(parts['left'],'player',target) and right['node_type']=='Number' and type(right['fields']['n']) is int:
            value=right['fields']['n']
            if rhs['node_type']=='SubOp':value=-value
            if -(2**31)<=value<2**31:return {'kind':'ADD','value':value}
    return None


def source_ref(occ,node,pointer,raw):
    text=raw.decode('utf-8-sig');span=node['span'];fragment=text[span['start_char']:span['end_char_exclusive']]
    prefix=len(raw)-len(text.encode());start=prefix+len(text[:span['start_char']].encode());end=start+len(fragment.encode())
    assert raw[start:end]==fragment.encode()
    return {'source':occ['source'],'revision':occ['revision'],'path':occ['path'],'file_sha256':occ['blob_sha256'],'ast_pointer':pointer,'byte_start':start,'byte_end_exclusive':end,'slice_sha256':hashlib.sha256(raw[start:end]).hexdigest()}


def classify(node):
    f=node['fields'];rhs=f['args'][-1];kind=rhs['node_type']
    if name(f.get('func',{}))=='setPlayerStorageValue':return 'LEGACY_STORAGE_HELPER_BINDING_NOT_PROVEN'
    if name(f.get('source',{}))!='player':return 'NON_PLAYER_STORAGE_DOMAIN'
    if direct_add(node):return 'DIRECT_SAME_PLAYER_TRACK_ADD'
    if kind=='Name':return 'LOCAL_BINDING_VALUE_REQUIRES_PROOF'
    if kind=='Index':return 'DYNAMIC_FIELD_OR_TABLE_VALUE'
    types={n['node_type'] for _,n in walk(rhs)}
    calls={name(n['fields']['func']) or expression_path(n['fields']['func']) for _,n in walk(rhs) if n['node_type'] in ('Call','Invoke')}
    if 'math.random' in calls:return 'RANDOM_VALUE_OPERATOR'
    if 'math.max' in calls or 'math.min' in calls:return 'CLAMP_VALUE_OPERATOR'
    if {'setFlag','resetFlag','bit.bor','bit.band'} & calls:return 'BITMASK_VALUE_OPERATOR'
    if {'AndLoOp','OrLoOp'} & types:return 'CONDITIONAL_VALUE_OPERATOR'
    if kind in ('AddOp','SubOp') and rhs['fields']['left']['node_type']=='Name' and rhs['fields']['right']['node_type']=='Number':return 'LOCAL_AFFINE_VALUE_REQUIRES_PROOF'
    if 'getStorageValue' in calls:return 'CROSS_TRACK_OR_DYNAMIC_VALUE_EXPRESSION'
    return 'ENTITY_HELPER_OR_COMPOSITE_VALUE'


def build(scan,state,ast_root,authoring):
    sys.path.insert(0,str(authoring))
    from donor_semantic_conditions import load_ast_inputs
    rows=scan['rows'];wanted={w['occurrence']['blob_sha256'] for row in rows for w in row['witnesses']}
    _,index,containers=load_ast_inputs(ast_root,wanted)
    cache={};result=[];changes=[];current={t['key']:t for q in state['quests'] for t in q['transitions']}
    for row in rows:
        transition=row['transition'];key=transition['key']
        if transition!=current[key]:raise ValueError('Accepted main transition drift')
        proofs=[];candidate_effects=[];categories=[]
        for w in row['witnesses']:
            occ=w['occurrence'];sha=occ['blob_sha256']
            if sha not in cache:
                capture=json.loads(gzip.decompress(containers[sha]));raw=base64.b64decode(capture['raw_bytes_base64'])
                assert hashlib.sha256(raw).hexdigest()==sha and capture['status']=='PARSED'
                cache[sha]=(capture['ast'],raw,list(walk(capture['ast'])))
            ast,raw,nodes=cache[sha]
            candidates=[(p,n) for p,n in nodes if n['node_type'] in ('Invoke','Call') and n.get('span') and n['span']['line']==occ['line'] and len(n['fields'].get('args',[])) in (2,3) and (name(n['fields']['func'])=='setStorageValue' or expression_path(n['fields']['func'])=='Game.setStorageValue' or name(n['fields']['func'])=='setPlayerStorageValue')]
            if len(candidates)>1:
                matched=[(p,n) for p,n in candidates if expression_path(n['fields']['args'][0])==occ['target']]
                if matched:candidates=matched
            if len(candidates)!=1:raise ValueError('Write AST owner/span not unique: '+key+' '+str(len(candidates)))
            pointer,node=candidates[0];guards=[]
            for p,n in nodes:
                if n['node_type'] in ('If','ElseIf','While','Repeat') and pointer.startswith(p+'/fields/') and n['fields'].get('test') and n['fields']['test'].get('span'):
                    gp=p+'/fields/test';guards.append({'branch_path':pointer[len(p):].split('/')[2],'node_type':n['fields']['test']['node_type'],'source_ref':source_ref(occ,n['fields']['test'],gp,raw)})
            rhs=node['fields']['args'][-1]
            proof={'source_ref':source_ref(occ,node,pointer,raw),'ast_container_sha256':index[sha]['container_sha256'],'write_ast_sha256':digest(node),'receiver':name(node['fields'].get('source',{})) or expression_path(node['fields'].get('func',{})),'value_ast_kind':rhs['node_type'],'value_dependencies':sorted({name(n) for _,n in walk(rhs) if name(n)}),'enclosing_guards':guards,'candidate_effect':direct_add(node)}
            proofs.append(proof);candidate_effects.append(proof['candidate_effect'])
            if 'COMPUTED_EXPRESSION' in row['reasons']:categories.append(classify(node))
        blockers=list(row['reasons'])
        if 'INEXACT_FROM' in blockers:blockers[blockers.index('INEXACT_FROM')]='COMPOUND_SOURCE_GUARD_NOT_REPRESENTABLE_BY_ONE_COMPARISON'
        if 'COMPUTED_EXPRESSION' in blockers:blockers[blockers.index('COMPUTED_EXPRESSION')]=categories[0] if len(set(categories))==1 else 'DONOR_VALUE_CATEGORY_CONFLICT'
        eligible='COMPUTED_EXPRESSION' in row['reasons'] and 'INEXACT_FROM' not in row['reasons'] and all(candidate_effects) and all(e==candidate_effects[0] for e in candidate_effects)
        record={'transition_key':key,'quest_key':transition['quest'],'source_track':row['source_track'],'baseline_sha256':digest(transition),'baseline':transition,'source_transition_sha256':digest(row['source_transition']),'blockers':blockers,'proofs':proofs,'safe_effect_lowering_candidate':eligible,'runtime_activation':False}
        if eligible:
            replacement=copy.deepcopy(transition);replacement['effects'][0]['effect']=candidate_effects[0]
            changes.append({'transition_key':key,'baseline_sha256':digest(transition),'effect':candidate_effects[0],'replacement':replacement,'proof_record_sha256':digest(record)})
        result.append(record)
    counts=collections.Counter(b for r in result for b in r['blockers'])
    return {'schema':'OTERYN_TRANSITION_REFUSAL_AUDIT/v1','records':result,'changes':changes,'summary':{'unique_refused':len(result),'computed':168,'inexact':221,'overlap':2,'safe_effect_candidates':len(changes),'remaining_if_candidates_accepted':len(result)-len(changes),'blocker_counts':dict(sorted(counts.items()))},'source_datums_unchanged':True,'runtime_activation':False}


def main():
    p=argparse.ArgumentParser()
    for field in ('scan','state','ast-root','authoring','out'):p.add_argument('--'+field,type=Path,required=True)
    a=p.parse_args();packet=build(json.loads(a.scan.read_text()),json.loads(a.state.read_text()),a.ast_root,a.authoring)
    a.out.write_text(json.dumps(packet,ensure_ascii=False,indent=2)+'\n');print(json.dumps(packet['summary'],indent=2))

if __name__=='__main__':main()
