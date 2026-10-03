"""Build a closed, source-flagged successor for 133 provisional NPCs.

Wiki and donor facts remain documentary; selected simple dialogue responses are
explicit Oteryn approximations. No services, placements or runtime are enabled.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path

from npc_admission_stage import behavior_profile, presentation_profile
from npc_bulk_stage import canonical, field

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / 'docs/agents/evidence/OTV2-20261002-npc-enrichment-r21'
PREFIX = 'oteryn:source.npc.bulk.'


def indexed(rows, key):
    result = {key(r): r for r in rows}
    if len(result) != len(rows):
        raise ValueError('duplicate enrichment identity')
    return result


def values(row):
    return {f['field_path'].removeprefix(PREFIX): f['value']['value'] for f in row['fields']}


def update(row, additions):
    fields = {f['field_path']: f for f in row['fields']}
    for name, value in additions.items():
        f = field(name, value)
        fields[f['field_path']] = f
    row['fields'] = [fields[k] for k in sorted(fields)]


def build(baseline, facts):
    declarations = indexed(baseline['records'], lambda r: r['identity']['key'])
    profiles = indexed(baseline['authoring_profiles'], lambda r: r['target']['key'])
    parents = [json.loads((ROOT / ('docs/agents/evidence/' + p + '/native-additions.json')).read_text())
               for p in ['OTV2-20261002-npc-bulk-first45', 'OTV2-20261002-npc-bulk-remaining88']]
    actors = {r['identity']['key'] for p in parents for r in p['native_additions']['declarations'] if r['kind'] == 'NPC'}
    wiki = indexed(facts['wiki']['records'], lambda r: r['key'])
    speech = indexed(facts['dialogue']['records'], lambda r: r['key'])
    voices = indexed(facts['voices']['records'], lambda r: r['key'])
    appearances = facts['appearance']['source_profiles']
    if len(actors) != 133 or set(wiki) != actors or set(speech) != actors or (set(voices) | set(appearances)) - actors:
        raise ValueError('enrichment actor inventory drifted')
    repairs, profile_repairs, progress = [], [], []
    for key in sorted(actors):
        before = declarations[key]
        if before['kind'] != 'NPC' or before['services'] or values(before)['status'] != 'provisional':
            raise ValueError('enrichment predecessor is not disabled provisional NPC')
        row = copy.deepcopy(before)
        q = json.loads(values(row)['quality'])
        w, d = wiki[key], speech[key]
        if w['name'] != values(before)['name'] or d['name'] != w['name'] or w['runtime_enabled'] or d['runtime_qualified']:
            raise ValueError('enrichment actor attribution/runtime drifted')
        wf = w['wiki_facts']
        q['profession'] = 'verified' if w['field_quality']['profession'] == 'wiki_documented' else 'todo'
        q['locations.reference'] = 'verified'
        q['quests.reference'] = 'verified' if wf['quest_references'] else 'todo'
        q['services.reference'] = 'verified' if any(wf['br_trade_reference'].values()) or any(wf['tp_trade_reference'].values()) else 'todo'
        q['dialogue.reference'] = 'verified' if wf['npc_lines'] else 'todo'
        q['wiki.tibiopedia.cached_reference'] = 'verified'
        document = {'sources': w['sources'], 'wiki_facts': wf,
                    'identity_evidence': w.get('identity_evidence'),
                    'scope': 'Documentary public reference facts; coordinates are not native placements; quest/trade refs do not activate actions.'}
        update(row, {'source_metadata': document, 'wiki_profession': wf['profession'], 'wiki_quests': wf['quest_references']})
        if key in voices:
            q['voices.reference'] = 'donor'
            update(row, {'voices': voices[key]})
        elif wf['voices_text']:
            q['voices.reference'] = 'verified'
            update(row, {'wiki_voices': wf['voices_text']})
        if key in appearances:
            a = appearances[key]
            if a['source']['literal_name'] != w['name']:
                raise ValueError('appearance actor identity drifted')
            q['presentation'] = 'donor'
            for part in a['outfit']:
                q['presentation.' + part] = 'defaulted' if part in a['missing_fields'] else 'donor'
            q['movement.floor_change'] = 'donor'
            target = before['presentation']['key']
            old = profiles[target]
            new = copy.deepcopy(old)
            new['data']['profile'] = presentation_profile(a['outfit'])
            profile_repairs.append({'before': old, 'after': new})
            update(row, {'source_reference': a['source']})
        if key == 'oteryn:npc.dragon_ancestor_spirit':
            # Crystal's 2000/2 assignments are commented TODOs, not source facts.
            q['movement.walk_interval_ms'] = q['movement.walk_radius'] = 'defaulted'
            q['movement.floor_change'] = 'donor'
            old = profiles[before['behavior']['key']]
            new = copy.deepcopy(old)
            new['data']['profile'] = behavior_profile({'walk_interval_ms': 0, 'walk_radius': 2, 'floor_change': False})
            profile_repairs.append({'before': old, 'after': new})
        old = declarations[before['dialogue']['key']]
        new = copy.deepcopy(old)
        dq = {'dialogue': 'placeholder', 'dialogue.greet': 'placeholder', 'dialogue.farewell': 'placeholder',
              'dialogue.name': 'placeholder', 'dialogue.job': 'placeholder'}
        selected = []
        for message, proposal in d['static_messages'].items():
            if message not in {'greet', 'farewell'} or not proposal['text'].strip() or not 0 <= proposal['source_index'] < len(d['sources']):
                raise ValueError('invalid static dialogue proposal')
            new[message] = [proposal['text']]
            dq['dialogue.' + message] = 'donor'
            selected.append({'message': message, **proposal})
        for proposal in d.get('keyword_candidates', []):
            keyword = proposal['keyword']
            if keyword not in {'name', 'job'} or not proposal['no_stateful_actions'] or not 0 <= proposal['source_index'] < len(d['sources']):
                raise ValueError('stateful/foreign keyword proposal')
            node = next(n for n in new['keywords'] if n['key'] == keyword)
            node['reply'] = proposal['reply']
            dq['dialogue.' + keyword] = 'donor'
            selected.append(proposal)
        if selected:
            dq['dialogue'] = q['dialogue'] = 'donor'
            q.update({k: v for k, v in dq.items() if k != 'dialogue'})
            update(new, {'quality': dq, 'dialogue_source': {
                'sources': d['sources'], 'selected': selected,
                'policy': 'Original source text with owner-approved Oteryn basic greet/farewell/name/job mapping; matcher and state fidelity unproven; no actions.'}})
            repairs.append({'before': old, 'after': new})
        # Complex exchanges remain documentary, never flattened into native replies.
        update(row, {'dialogue_reference': {'sources': d['sources'], 'observations': d['keyword_observations'],
                                         'unassigned_wiki_speech': d['wiki_speech_without_triggers']}, 'quality': q})
        repairs.append({'before': before, 'after': row})
        progress.append({'key': key, 'name': w['name'], 'state': 'DATA_READY_PARTIAL', 'field_quality': q,
                         'native_runtime_loaded': False, 'remaining_tasks': [f'{k}: {v}' for k, v in q.items() if v in {'todo', 'defaulted', 'placeholder'}]})
    packet = {'schema': 'OTERYN_NPC_BULK_ENRICHMENT/v1', 'from_project_revision': parents[-1]['project_revision'],
              'project_revision': 'g4-npc-provisional-enrichment-r21', 'repairs': repairs, 'profile_repairs': profile_repairs}
    return packet, progress


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--predecessor', type=Path, required=True)
    args = parser.parse_args()
    raw = (EVIDENCE / 'source-facts.json').read_bytes()
    custody = json.loads((EVIDENCE / 'source-custody.json').read_text())
    if hashlib.sha256(raw).hexdigest() != custody['portable_facts_sha256']:
        raise ValueError('source facts custody drifted')
    packet, progress = build(json.loads((args.predecessor / 'definitions/declarations.json').read_text()), json.loads(raw))
    (EVIDENCE / 'native-enrichment.json').write_bytes(canonical(packet))
    (EVIDENCE / 'progress.json').write_text(json.dumps({'records': progress, 'count': 133, 'native_runtime_loaded': 0}, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'actors': 133, 'declaration_repairs': len(packet['repairs']), 'profile_repairs': len(packet['profile_repairs']),
                      'packet_sha256': hashlib.sha256(canonical(packet)).hexdigest()}))
