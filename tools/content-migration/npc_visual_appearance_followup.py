"""R26 visual follow-up restricted to the twelve explicitly held R25 actors."""
import copy, hashlib, json, re
from pathlib import Path
from npc_visual_appearance_stage import ROOT, build as previous_build, values, update
from npc_bulk_stage import canonical
FROM='g4-npc-provisional-enrichment-r25'
TO='g4-npc-provisional-enrichment-r26'

def invisible(selection):
    if selection['key']!='oteryn:npc.opticorder_forge_npc' or selection['name']!='Opticorder Forge (NPC)':
        raise ValueError('documented invisibility is restricted to Opticorder Forge NPC')
    source=selection['source'];proof=selection.get('visibility_evidence',{})
    if source.get('url')!='https://tibia.fandom.com/wiki/Opticorder_Forge_(NPC)' or not re.fullmatch('[0-9a-f]{64}',source.get('sha256','')):
        raise ValueError('missing exact invisible NPC article/body custody')
    quote=proof.get('original_exact_text','')
    if 'Cipsoft had to make an invisible NPC just above it.' not in quote:
        raise ValueError('missing literal source-documented invisible NPC proof')
    start,end=proof.get('start_byte'),proof.get('end_byte_exclusive')
    if type(start) is not int or type(end) is not int or not 0<=start<end or not re.fullmatch('[0-9a-f]{64}',proof.get('raw_fragment_sha256','')):
        raise ValueError('invisible NPC proof lacks exact source byte custody')
    if selection.get('outfit') is not None or not selection.get('rationale'):
        raise ValueError('invisible mapping cannot invent a numeric outfit')

def build(baseline,selections):
    facts=json.loads((ROOT/'docs/agents/evidence/OTV2-20261002-npc-appearance-r25/source-facts.json').read_text())
    held={r['key'] for r in facts['held']}
    if len(held)!=12 or any(s['key'] not in held for s in selections):
        raise ValueError('visual follow-up may only select the twelve previously held actors')
    actors={r['identity']['key']:r for r in baseline['records'] if r['kind']=='NPC'}
    for selection in selections:
        metadata=json.loads(values(actors[selection['key']])['source_metadata'])
        if 'r25_visual_mapping' in metadata:
            raise ValueError('follow-up cannot replace a prior R25 visual selection')
    hidden=[s for s in selections if s.get('visibility')=='invisible']
    defaults=[s for s in selections if s.get('choice_kind')=='project_default_no_source_sprite']
    for selection in defaults:
        if selection['key'] not in {'oteryn:npc.mud','oteryn:npc.planestrider_npc'}:
            raise ValueError('source-unknown neutral choice is restricted to MUD and Planestrider')
        previous=json.loads(values(actors[selection['key']])['appearance_selection'])
        if selection.get('outfit')!=previous['outfit'] or selection.get('visibility')=='invisible':
            raise ValueError('source-unknown neutral choice must preserve the existing outfit')
    if len(hidden)>1:raise ValueError('duplicate invisible NPC selection')
    for selection in hidden:invisible(selection)
    if any(s.get('visibility') not in (None,'visible','invisible') for s in selections):
        raise ValueError('unknown visual visibility mapping')
    packet=previous_build(baseline,[s for s in selections if s not in hidden])
    packet['from_project_revision']=FROM;packet['project_revision']=TO
    selected={s['key'] for s in selections}
    for repair in packet['repairs']:
        if repair['after']['identity']['key'] in selected:
            metadata=json.loads(values(repair['after'])['source_metadata'])
            hidden_selection=next((s for s in hidden if s['key']==repair['after']['identity']['key']),None)
            if hidden_selection:
                v=values(repair['before']);q=json.loads(v['quality'])
                if q['presentation']!='defaulted':raise ValueError('invisible selection cannot hide preserved donor')
                document={**hidden_selection,'classification':'APPROXIMATE_WIKI_DOCUMENTED_INVISIBLE_MAPPING',
                    'previous_project_choice':json.loads(v['appearance_selection']),
                    'actor_exact_match':False,'target_native_appearance_verified':False,
                    'canonical_tibia_fidelity_claim':False,'native_runtime_visibility_qualified':False}
                metadata['r26_visual_mapping']=document;q['presentation.visibility']='defaulted'
                update(repair['after'],{'appearance_selection':document,'quality':q})
                profiles={r['target']['key']:r for r in baseline['authoring_profiles']}
                old=profiles[repair['before']['presentation']['key']];new=copy.deepcopy(old)
                new['data']['profile']={'selection':'Invisible','light_level':0}
                packet['profile_repairs'].append({'before':old,'after':new})
            else:
                document=metadata.pop('r25_visual_mapping')
                if repair['after']['identity']['key'] in {s['key'] for s in defaults}:
                    document.update(classification='PROJECT_DEFAULT_NO_SOURCE_SPRITE',
                        source_completeness='unknown',visual_correspondence='unknown',
                        actor_exact_match=False,target_native_appearance_verified=False,
                        canonical_tibia_fidelity_claim=False,native_runtime_visibility_qualified=False)
                    update(repair['after'],{'appearance_selection':document})
                metadata['r26_visual_mapping']=document
            update(repair['after'],{'source_metadata':metadata})
    default_targets={s['key'].replace('oteryn:npc.','oteryn:presentation.npc.') for s in defaults}
    packet['profile_repairs']=[r for r in packet['profile_repairs'] if r['before']['target']['key'] not in default_targets]
    return packet

if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--baseline',type=Path,required=True)
    p.add_argument('--selections',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    args=p.parse_args()
    packet=build(json.loads(args.baseline.read_text()),json.loads(args.selections.read_text()))
    args.output.write_bytes(canonical(packet));print(hashlib.sha256(args.output.read_bytes()).hexdigest())
