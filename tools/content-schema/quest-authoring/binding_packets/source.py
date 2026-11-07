"""Read committed Quest records once and produce evidence-only source crosswalk."""
import argparse
import hashlib
import json
from pathlib import Path

PREFIX = 'tools/content-schema/quest-authoring/samples/'

def build(repo):
    repo = Path(repo)
    files = ['content/quests/definitions/index.json', PREFIX+'source_migration/bundle.json', PREFIX+'interactions/manifest.json']
    inputs = {}
    def read(path):
        raw = (repo/path).read_bytes()
        inputs[path] = hashlib.sha256(raw).hexdigest()
        return json.loads(raw)
    index, bundle, manifest = (read(p) for p in files)
    donors = {q['identity']['key']: q for q in bundle['quests']}
    graphs = {g['identity']['key']: g for g in bundle['interactions']}
    tracks = {p['key']: p for p in bundle['progress']}
    manifest_by_graph = {e['destination']: e for e in manifest['entries']}
    holds = {}
    for hold in bundle['gap_evidence']['coverage_holds']:
        holds.setdefault(hold['quest'], []).append(hold)
    records = []
    seen = set()
    for shard in index['shards']:
        for offset, wrapped in enumerate(read(shard)['records']):
            q = wrapped['definition']
            key = q['identity']['key']
            donor_key = q.get('source_refs', {}).get('quest', {}).get('key')
            if donor_key not in donors:
                continue
            if donor_key in seen:
                raise ValueError('duplicate donor quest identity: '+donor_key)
            seen.add(donor_key)
            donor = donors[donor_key]
            if q['display_name'] != donor['display_name']:
                raise ValueError('wrong quest association: '+key)
            sd = q.get('source_data', {})
            if sd and sd['quest']['identity']['key'] != donor_key:
                raise ValueError('source_data belongs to wrong quest: '+key)
            source_claims = q['source_refs']['claims']
            native_claims = q['claims']
            mapping = []
            # Equal-length zipped refs alone are not an authoritative alias.
            # Match explicit source-ref data in canonical RewardClaim via key only
            # when canonical claim mapping was already admitted on definition.
            for ref in source_claims:
                matched = [c for c in bundle['claims'] if c['identity']['key'] == ref['key']]
                if len(matched) != 1:
                    raise ValueError('missing/ambiguous source claim: '+ref['key'])
                claim = matched[0]
                if (claim.get('quest') or {}).get('key') != donor_key:
                    raise ValueError('claim belongs to wrong quest: '+ref['key'])
                mapping.append({'source_ref':ref,'source_record':claim})
            linked_tracks = sd.get('progress', [])
            if not sd:
                used = {m['progress'] for m in donor.get('missions', [])}
                if donor.get('start'): used.add(donor['start']['progress'])
                linked_tracks = [tracks[k] for k in sorted(used) if k in tracks]
            progress_bindings = []
            for track in linked_tracks:
                if track['key'] not in tracks:
                    raise ValueError('missing progress identity: '+track['key'])
                writes=[]
                for transition in track['transitions']:
                    writes.append({'transition':transition['key'],'script':transition.get('script'),
                        'write':transition.get('write'),'source_occurrences':transition.get('source_occurrences', [])})
                progress_bindings.append({'source_track':track['key'],'missions':track['missions'],
                    'start_of':track['start_of'],'read_by_gates':track['read_by_gates'],
                    'writes':writes,'source_write_occurrences':sum(len(t['source_occurrences']) for t in writes),
                    'native_track_admission':'NOT_ASSESSED'})
            trigger_bindings=[]
            for graph in sd.get('interactions', []):
                gkey=graph['identity']['key']
                if gkey not in graphs or gkey not in manifest_by_graph:
                    raise ValueError('missing interaction identity/provenance: '+gkey)
                entry=manifest_by_graph[gkey]
                trigger_bindings.append({'source_graph':gkey,'edge':graph.get('source',{}).get('edge'),
                    'callback':graph.get('source',{}).get('callback'),
                    'target_registrations':graph.get('source',{}).get('target_registrations',[]),
                    'sources':entry['sources'],'native_trigger_admission':'NOT_ASSESSED'})
            gates=[g for g in bundle['gates'] if (g.get('quest') or {}).get('key')==donor_key]
            concrete= [g for g in q.get('missing_data',[]) if g['code'] not in ('quest_native_lowering_missing','claim_native_lowering_missing','script_reward_runtime_missing')]
            records.append({'quest_key':key,'donor_quest_key':donor_key,'display_name':q['display_name'],
                'canonical_location':{'path':shard,'pointer':'/records/'+str(offset)+'/definition'},
                'mission_bindings':[{'mission_key':m['key'],'source_progress':m['progress'],
                    'start_value':m['start_value'],'end_value':m['end_value'],'journal':m.get('journal')} for m in donor.get('missions',[])],
                'progress_bindings':progress_bindings,'trigger_bindings':trigger_bindings,
                'gate_bindings':gates,'source_reward_bindings':mapping,'native_reward_refs':native_claims,
                'source_coverage_holds':holds.get(donor_key,[]),'remaining_data_flags':concrete,
                'stage_full_coverage':'NOT_ASSESSED','runtime_readiness':'NOT_ASSESSED',
                'counts':{'missions':len(donor.get('missions',[])),'progress':len(progress_bindings),
                    'source_write_occurrences':sum(p['source_write_occurrences'] for p in progress_bindings),
                    'triggers':len(trigger_bindings),'gates':len(gates),'rewards':len(mapping),
                    'native_reward_refs':len(native_claims)}})
    if len(records)!=len(donors):
        raise ValueError('donor coverage mismatch')
    return {'schema':'oteryn-quest-source-crosswalk-v1','scope':'Evidence-only DATA joins; no executable, complete-stage or native-track admission inferred',
        'input_sha256':dict(sorted(inputs.items())),'quest_count':len(records),'quests':sorted(records,key=lambda q:q['quest_key'])}

def main():
    ap=argparse.ArgumentParser();ap.add_argument('repository');ap.add_argument('output');args=ap.parse_args()
    out=build(args.repository)
    Path(args.output).write_text(json.dumps(out,ensure_ascii=False,sort_keys=True,indent=2)+'\n')

if __name__=='__main__':main()
