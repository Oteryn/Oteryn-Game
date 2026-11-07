"""Select source-linked approximate NPC replies/roles, preserving original facts.

Only basic static dialogue and documentary fields change. Image labels are not
numeric outfits; inferred roles and generated job replies are Oteryn choices.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path

from npc_bulk_enrich import ROOT, indexed, update, values
from npc_bulk_stage import canonical

EVIDENCE = ROOT / 'docs/agents/evidence/OTV2-20261002-npc-enrichment-r22'


def build(baseline, facts):
    declarations = indexed(baseline['records'], lambda r: r['identity']['key'])
    old = json.loads((ROOT / 'docs/agents/evidence/OTV2-20261002-npc-enrichment-r21/native-enrichment.json').read_text())
    actors = {r['after']['identity']['key'] for r in old['repairs'] if r['after']['kind'] == 'NPC'}
    speech = indexed(facts['dialogue']['records'], lambda r: r['key'])
    roles = indexed(facts['profession']['records'], lambda r: r['key'])
    portraits = indexed(facts['portraits'], lambda r: r['key'])
    if len(actors) != 133 or set(speech) != actors or (set(roles) | set(portraits)) - actors:
        raise ValueError('followup actor scope drifted')
    repairs, progress = [], []
    counts = {'actors': 133, 'dialogue_changed': 0, 'source_messages': 0, 'source_keywords': 0,
              'role_selections': 0, 'original_role_job_replies': 0, 'portrait_references': len(portraits)}
    for key in sorted(actors):
        before = declarations[key]
        if before['kind'] != 'NPC' or before['services'] or values(before)['status'] != 'provisional':
            raise ValueError('followup predecessor is not disabled provisional actor')
        row = copy.deepcopy(before)
        q = json.loads(values(row)['quality'])
        d = speech[key]
        if d['name'] != values(row)['name'] or d['runtime_qualified'] or d['stateful_actions_enabled']:
            raise ValueError('followup attribution/runtime drifted')
        role = roles.get(key)
        selected_role = (role.get('profession_selection') or {}).get('label') if role else None
        if selected_role:
            if role['name'] != d['name'] or role['profession_selection']['classification'] != 'INFERRED_APPROXIMATE' or q['profession'] != 'todo' or role['runtime_enabled'] or role['services_enabled'] or not role['evidence']:
                raise ValueError('role overrides known profession or lacks evidence')
            q['profession'] = 'donor'
            q['profession.wiki'] = 'todo'
            update(row, {'profession_selection': role})
            counts['role_selections'] += 1
        if key in portraits:
            if q['presentation'] != 'placeholder' or portraits[key]['name'] != d['name']:
                raise ValueError('portrait label cannot override native appearance')
            q['presentation.reference'] = 'verified'
            update(row, {'appearance_reference': portraits[key]})
        dialogue = declarations[before['dialogue']['key']]
        next_dialogue = copy.deepcopy(dialogue)
        dq = json.loads(values(dialogue)['quality'])
        selections = []
        sources = d['sources']
        for message, proposal in d['static_messages'].items():
            if message not in {'greet', 'farewell'} or not proposal['no_stateful_actions'] or not 0 <= proposal['source_index'] < len(sources):
                raise ValueError('unsupported static source message')
            if proposal['text'] != proposal['original_exact_text']:
                raise ValueError('source message was rewritten')
            next_dialogue[message] = [proposal['text']]
            dq['dialogue.' + message] = 'donor'
            selections.append({'message': message, **proposal})
            counts['source_messages'] += 1
        for proposal in d['keyword_candidates']:
            keyword = proposal['keyword']
            if keyword not in {'name', 'job', 'story'} or not proposal['no_stateful_actions'] or not 0 <= proposal['source_index'] < len(sources):
                raise ValueError('unsupported/stateful keyword')
            replies = proposal['reply']
            if not 1 <= len(replies) <= 2 or not all(isinstance(x, str) and x.strip() for x in replies):
                raise ValueError('invalid static replies')
            node = next((n for n in next_dialogue['keywords'] if n['key'] == keyword), None)
            if node is None:
                if keyword != 'story':
                    raise ValueError('missing predecessor name/job keyword')
                node = {'key': 'story', 'triggers': ['story'], 'reply': []}
                next_dialogue['keywords'].append(node)
            node['reply'] = replies
            dq['dialogue.' + keyword] = 'donor'
            selections.append(proposal)
            counts['source_keywords'] += 1
        # Role prose is an original project reply, never attributed to a transcript.
        if selected_role and dq.get('dialogue.job', 'placeholder') == 'placeholder':
            node = next(n for n in next_dialogue['keywords'] if n['key'] == 'job')
            node['reply'] = ['My role here is ' + selected_role + '.']
            dq['dialogue.job'] = 'defaulted'
            selections.append({'message': 'job', 'quality': 'OTERYN_ORIGINAL_APPROXIMATE_ROLE_REPLY',
                               'text': node['reply'][0], 'source_quote': False, 'role_evidence': role})
            counts['original_role_job_replies'] += 1
        if selections:
            if d['static_messages'] or d['keyword_candidates']:
                dq['dialogue'] = 'donor'
            elif dq['dialogue'] == 'placeholder':
                dq['dialogue'] = 'defaulted'
            update(next_dialogue, {'quality': dq, 'dialogue_followup': {
                'sources': sources, 'selected': selections,
                'policy': 'Selected Oteryn simple replies and optional story leaf; source state/matcher fidelity unproven; original role prose identified separately; no actions.'}})
            repairs.append({'before': dialogue, 'after': next_dialogue})
            counts['dialogue_changed'] += 1
            q.update(dq)
        update(row, {'quality': q})
        repairs.append({'before': before, 'after': row})
        progress.append({'key': key, 'name': d['name'], 'state': 'DATA_READY_PARTIAL', 'field_quality': q,
                         'native_runtime_loaded': False, 'remaining_tasks': [f'{k}: {v}' for k, v in q.items() if v in {'todo', 'defaulted', 'placeholder'}]})
    packet = {'schema': 'OTERYN_NPC_BULK_ENRICHMENT/v1', 'from_project_revision': old['project_revision'],
              'project_revision': 'g4-npc-provisional-enrichment-r22', 'repairs': repairs, 'profile_repairs': []}
    counts['source_selected_dialogues'] = sum(r['field_quality']['dialogue'] == 'donor' for r in progress)
    counts['role_only_dialogues'] = sum(r['field_quality']['dialogue'] == 'defaulted' for r in progress)
    counts['generic_dialogues'] = sum(r['field_quality']['dialogue'] == 'placeholder' for r in progress)
    return packet, progress, counts


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--predecessor', type=Path, required=True)
    args = parser.parse_args()
    raw = (EVIDENCE / 'source-facts.json').read_bytes()
    custody = json.loads((EVIDENCE / 'source-custody.json').read_text())
    if hashlib.sha256(raw).hexdigest() != custody['portable_facts_sha256']:
        raise ValueError('followup source custody drifted')
    packet, rows, counts = build(json.loads((args.predecessor / 'definitions/declarations.json').read_text()), json.loads(raw))
    (EVIDENCE / 'native-enrichment.json').write_bytes(canonical(packet))
    (EVIDENCE / 'progress.json').write_text(json.dumps({'records': rows, 'counts': counts, 'runtime_loaded': 0}, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({**counts, 'packet_sha256': hashlib.sha256(canonical(packet)).hexdigest()}))
