"""Owner-approved derived visual-invisible choice for exactly MUD and Planestrider."""
import copy,hashlib,json,re
from npc_visual_appearance_stage import ROOT,values,update
from npc_bulk_enrich import indexed
from npc_bulk_stage import canonical
TARGETS={'oteryn:npc.mud':'MUD','oteryn:npc.planestrider_npc':'Planestrider (NPC)'}
FROM='g4-npc-provisional-enrichment-r26';TO='g4-npc-provisional-enrichment-r27'
def build(baseline,selections):
    selected=indexed(selections,lambda r:r['key'])
    if set(selected)!=set(TARGETS):raise ValueError('derived invisible choice requires exactly MUD and Planestrider')
    declarations=indexed(baseline['records'],lambda r:r['identity']['key'])
    profiles=indexed(baseline['authoring_profiles'],lambda r:r['target']['key'])
    prior=json.loads((ROOT/'docs/agents/evidence/OTV2-20261002-npc-appearance-r26/native-enrichment.json').read_text())
    actors={r['after']['identity']['key'] for r in prior['repairs'] if r['after']['kind']=='NPC'}
    if len(actors)!=133:raise ValueError('closed predecessor actor inventory drifted')
    repairs=[];profile_repairs=[]
    for key in sorted(actors):
        before=declarations[key];after=copy.deepcopy(before)
        if key in selected:
            s=selected[key];source=s['source']
            if s['name']!=TARGETS[key] or s.get('visibility')!='invisible' or s.get('outfit') is not None:
                raise ValueError('derived invisible actor or selection shape mismatch')
            if s.get('source_visibility_classification')!='derived_visual_reference' or not s.get('rationale'):
                raise ValueError('derived visual inference must be explicit')
            if not source.get('url') or not re.fullmatch('[0-9a-f]{64}',source.get('sha256','')):
                raise ValueError('derived visual reference requires body custody')
            v=values(before);q=json.loads(v['quality']);previous=json.loads(v['appearance_selection'])
            if previous.get('classification')!='PROJECT_DEFAULT_NO_SOURCE_SPRITE':
                raise ValueError('derived invisible refinement requires the prior honest neutral choice')
            document={**s,'classification':'APPROXIMATE_WIKI_VISUAL_INVISIBLE_MAPPING',
                'previous_project_choice':previous,'actor_exact_match':False,
                'target_native_appearance_verified':False,'canonical_tibia_fidelity_claim':False,
                'native_runtime_visibility_qualified':False,'literal_source_invisibility_claim':False}
            metadata=json.loads(v['source_metadata']);metadata['r27_visual_mapping']=document
            q['presentation.visibility']='defaulted'
            update(after,{'appearance_selection':document,'source_metadata':metadata,'quality':q})
            old=profiles[before['presentation']['key']];new=copy.deepcopy(old)
            new['data']['profile']={'selection':'Invisible','light_level':0}
            profile_repairs.append({'before':old,'after':new})
        repairs.append({'before':before,'after':after})
    return {'schema':'OTERYN_NPC_BULK_ENRICHMENT/v1','from_project_revision':FROM,
        'project_revision':TO,'repairs':repairs,'profile_repairs':profile_repairs}
if __name__=='__main__':
    import argparse
    from pathlib import Path
    p=argparse.ArgumentParser(description=__doc__)
    for key in ['baseline','selections','output']:p.add_argument('--'+key,type=Path,required=True)
    a=p.parse_args();packet=build(json.loads(a.baseline.read_text()),json.loads(a.selections.read_text()))
    a.output.write_bytes(canonical(packet));print(hashlib.sha256(a.output.read_bytes()).hexdigest())
