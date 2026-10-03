"""Appearance-only closed successor; wiki visual mappings are explicitly approximate."""
import copy, hashlib, json, re
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
from npc_bulk_enrich import indexed, values, update
from npc_admission_stage import presentation_profile
from npc_bulk_stage import canonical
FROM='g4-npc-provisional-enrichment-r24'
TO='g4-npc-provisional-enrichment-r25'
PARTS={'look_type','head','body','legs','feet','addons','mount'}

def valid_looks():
    root=ROOT/'imports/cipsoft-appearances/outfits'
    manifest=json.loads((root/'manifest.json').read_text())
    valid=set()
    for meta in manifest['files']:
        body=(root/meta['path']).read_bytes()
        if hashlib.sha256(body).hexdigest()!=meta['sha256']:
            raise ValueError('client outfit shard custody drift')
        for row in json.loads(body)['records']:
            if any(any(sprite>0 for sprite in g.get('sprite_ids',[])) for g in row['frame_groups']):
                valid.add(row['source_id'])
    return valid

def valid_objects():
    root=ROOT/'imports/official/appearance-membership'
    admitted=json.loads((root/'admitted.json').read_text())
    meta=next(m for m in admitted['files'] if m['label']==admitted['newest'])
    body=(root/meta['manifest']).read_bytes()
    if hashlib.sha256(body).hexdigest()!=meta['manifest_sha256']:
        raise ValueError('client object membership custody drift')
    return {row[0] for row in json.loads(body)['entries']}

def build(baseline, selections):
    declarations=indexed(baseline['records'],lambda r:r['identity']['key'])
    profiles=indexed(baseline['authoring_profiles'],lambda r:r['target']['key'])
    parent=json.loads((ROOT/'docs/agents/evidence/OTV2-20261002-npc-enrichment-r24/native-enrichment.json').read_text())
    actors={r['after']['identity']['key'] for r in parent['repairs'] if r['after']['kind']=='NPC'}
    selected=indexed(selections,lambda r:r['key']);looks=valid_looks();objects=valid_objects()
    if len(actors)!=133 or set(selected)-actors:raise ValueError('foreign visual mapping actor')
    repairs=[];profile_repairs=[]
    for key in sorted(actors):
        before=declarations[key];after=copy.deepcopy(before);v=values(before);q=json.loads(v['quality'])
        if key in selected:
            item=selected[key]
            if item['name']!=v['name'] or q['presentation']!='defaulted':
                raise ValueError('foreign actor or preserved donor appearance')
            source=item['source']
            if not source.get('url') or not re.fullmatch('[0-9a-f]{64}',source.get('sha256','')) or not item.get('rationale'):
                raise ValueError('missing wiki visual mapping custody/rationale')
            outfit=item['outfit']
            object_mapping=set(outfit)=={'item_look'}
            if object_mapping:
                if type(outfit['item_look']) is not int or outfit['item_look'] not in objects:
                    raise ValueError('invalid client object appearance')
            else:
                if set(outfit)!=PARTS or type(outfit['look_type']) is not int or outfit['look_type'] not in looks:
                    raise ValueError('invalid client lookType or mixed appearance families')
                for part in {'head','body','legs','feet'}:
                    if type(outfit[part]) is not int or not 0<=outfit[part]<=132:
                        raise ValueError('invalid client palette')
                if type(outfit['addons']) is not int or not 0<=outfit['addons']<=3:
                    raise ValueError('invalid client addon mask')
                if type(outfit['mount']) is bool or (outfit['mount'] not in (None,0) and
                        (type(outfit['mount']) is not int or outfit['mount'] not in looks)):
                    raise ValueError('invalid client mount lookType')
            old=profiles[before['presentation']['key']];new=copy.deepcopy(old)
            new['data']['profile']=presentation_profile(outfit)
            profile_repairs.append({'before':old,'after':new})
            previous=json.loads(v['appearance_selection'])
            classification='APPROXIMATE_WIKI_VISUAL_OBJECT_MAPPING' if object_mapping else 'APPROXIMATE_WIKI_VISUAL_MAPPING'
            document={**item,'classification':classification,
                'actor_exact_match':False,'target_native_appearance_verified':False,
                'canonical_tibia_fidelity_claim':False,'visual_palette_is_source_numeric_fact':False,
                'native_runtime_loaded':False,'image_assets_rehosted':False,
                'movement_changed':False,'services_changed':False,
                'previous_project_choice':previous}
            metadata=json.loads(v['source_metadata']);metadata['r25_visual_mapping']=document
            # Approximate is a documentary classification, not a fake donor numeric fact.
            for part in PARTS:q['presentation.'+part]='defaulted'
            if object_mapping:q['presentation.item_look']='defaulted'
            update(after,{'appearance_selection':document,'source_metadata':metadata,'quality':q})
        repairs.append({'before':before,'after':after})
    return {'schema':'OTERYN_NPC_BULK_ENRICHMENT/v1','from_project_revision':FROM,
        'project_revision':TO,'repairs':repairs,'profile_repairs':profile_repairs}

if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--baseline',type=Path,required=True)
    p.add_argument('--selections',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    args=p.parse_args()
    packet=build(json.loads(args.baseline.read_text()),json.loads(args.selections.read_text()))
    args.output.write_bytes(canonical(packet))
    print(hashlib.sha256(args.output.read_bytes()).hexdigest())
