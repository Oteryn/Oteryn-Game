"""Build an unactivated completion candidate for the accepted QuestState loader.

The production Source lowering remains untouched. This candidate starts from an
exact current-main catalogue, adds chosen authored stage counters and applies
five finite Source-proved effect refinements. It does not supply event callers.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

TOOL = 'tools/content-schema/quest-authoring/'
MAIN = 'ad7a08f96caa4e7bd0e7fa90c67b39229d637277'
BASE = TOOL + 'samples/state-effect-refinements/main-state.json'
BASE_SHA = 'c82b7e6e32456527e2c90d54c28573ebde89e0db73dc926ca3a937be1a0ad649'
OUTPUT = 'content/quests/missions/quest-state-completion-candidate.json'
RECEIPT = 'content/quests/missions/completion-candidate.json'
STAGES = TOOL + 'samples/server-completion/chosen-progress/builder.py'
REFINE = TOOL + 'samples/state-effect-refinements/effect_refinements.py'
EVENTS = TOOL + 'samples/server-completion/events-rewards/packet.json'
EVENTS_SHA = '4b77071f3e44e90c64260c4db6830c5acab7f0142627d9819d2b4266c6d100a1'
NPC = TOOL + 'samples/server-completion/npc-dialogue/candidates.json'
NPC_SHA = 'e72477218339f8c2527706ec877750823014843566e13bc0a134ccaf6e4b2286'
PLAN = 'content/quests/missions/completion-binding-plan.json'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def encode(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n').encode()


def module(root, path, name):
    spec = importlib.util.spec_from_file_location(name, root / path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


def pinned(root, path, expected_sha):
    raw = (root / path).read_bytes()
    if sha(raw) != expected_sha:
        raise ValueError('Approved binding packet drift: ' + path)
    return json.loads(raw)


def binding_plan(choices, events, npc):
    event_rows = {q['quest_ref']['key']: q for q in events['records']}
    npc_rows = {(r['quest']['key'], r['stage']): r for r in npc['records']}
    if len(event_rows) != 68 or len(npc_rows) != len(npc['records']):
        raise ValueError('Duplicate or missing binding owner')
    records = []
    for quest in choices['quests']:
        owner = quest['quest']
        event = event_rows[owner]
        by_stage = {s['stage_key']: s for s in event['stages']}
        rows = []
        for transition in quest['transitions']:
            stage = transition['source']['chosen_stage']
            intent = by_stage[stage['key']]
            if intent['kind'] != stage['kind'] or intent['count'] != stage['count']:
                raise ValueError('Chosen event/count mismatch')
            dialogue = npc_rows.get((owner, stage['key']))
            if (stage['kind'] == 'talk') != (dialogue is not None):
                raise ValueError('Talk stage binding inventory differs')
            rows.append({
                'stage_key': stage['key'],
                'quest_transition_key': transition['key'],
                'event_identity_associations': intent,
                'NPC_dialogue_candidates': dialogue,
                'selected_NPC_branch': None,
                'native_dispatch_binding': None,
                'runtime_enabled': False,
            })
        records.append({
            'quest': owner, 'stages': rows,
            'reward_identity_associations': event['reward_intents'],
            'native_reward_delivery_binding': None,
            'runtime_enabled': False,
        })
    return {
        'schema': 'OTERYN_QUEST_COMPLETION_BINDING_PLAN/v1',
        'runtime_enabled': False,
        'counts': {'quests': len(records), 'stages': sum(len(q['stages']) for q in records)},
        'input_packets': [{'path': EVENTS, 'sha256': EVENTS_SHA}, {'path': NPC, 'sha256': NPC_SHA}],
        'limits': ['Identity associations and derived branch candidates are not executable bindings',
                   'Transition keys refer to the actual typed progress candidate, not a new family'],
        'records': records,
    }


def expected(root):
    raw = (root / BASE).read_bytes()
    if sha(raw) != BASE_SHA:
        raise ValueError('Accepted current-main catalogue drift')
    source = json.loads(raw)
    stages = module(root, STAGES, 'chosen_completion_stages')
    refine = module(root, REFINE, 'chosen_completion_refinements')
    choices = stages.build(root)
    stages.validate_packet(choices)
    plan = binding_plan(choices, pinned(root, EVENTS, EVENTS_SHA), pinned(root, NPC, NPC_SHA))
    refined, proof = refine.apply(root, source['quests'])
    base = dict(source, quests=refined)
    candidate = stages.merge(base, choices)
    inputs = [BASE, STAGES, REFINE, proof['path'],
              TOOL + 'samples/state-effect-refinements/effect_refinements.schema.json',
              EVENTS, NPC,
              TOOL + 'quest_completion_import.py']
    sources = [{'path': path, 'sha256': sha((root / path).read_bytes())} for path in inputs]
    sources += choices['authoring_sources']
    candidate['classification'] = 'LOCAL_SOURCE_AND_CHOSEN_IMPORT_CANDIDATE_NOT_ACTIVATED'
    # Original current-main provenance remains in the pinned base; include it
    # along with the local projection inputs, without claiming local Source
    # shards are identical to the newer main's four changed Source cores.
    candidate['authoring_sources'] = source['authoring_sources'] + sources
    transitions = [t for q in candidate['quests'] for t in q['transitions']]
    unsupported = sum(any(e['effect']['kind'] == 'COMPUTED' or not e['from_exact']
                          for e in t['effects']) for t in transitions)
    if unsupported != 382 or candidate['counts']['quests'] != 164:
        raise ValueError('Finite completion selection changed')
    data = encode(candidate)
    receipt = {
        'schema': 'OTERYN_QUEST_COMPLETION_IMPORT_CANDIDATE/v1',
        'accepted_source_main': MAIN,
        'accepted_source_state_sha256': BASE_SHA,
        'path': OUTPUT, 'sha256': sha(data),
        'counts': candidate['counts'],
        'unsupported_transitions': unsupported,
        'chosen_progress': choices['summary'],
        'source_effect_refinements': 5,
        'input_provenance': sources,
        'production_source_lowering_unchanged': True,
        'canonical_quest_definitions_unchanged': True,
        'runtime_activated': False,
        'native_event_dispatch_bindings': 0,
        'native_NPC_dialogue_bindings': 0,
        'native_reward_delivery_bindings': 0,
        'binding_plan': {'path': PLAN, 'sha256': sha(encode(plan)), 'counts': plan['counts']},
        'limits': [
            'Stage counts are explicitly chosen occurrence counters, not donor equivalence',
            'NPC/event eligibility, level/premium/prerequisite checks and rewards require owning callers',
            'Nine daily recipes have no cycle-reset binding',
            'XP is retained as intent; the accepted JSON loader does not import experience',
            'This file is not the production embedded quest-state.json',
        ],
    }
    return {OUTPUT: data, RECEIPT: encode(receipt), PLAN: encode(plan)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    root = args.root.resolve()
    for path, raw in expected(root).items():
        target = root / path
        if args.check:
            if not target.is_file() or target.read_bytes() != raw:
                raise ValueError('Completion import drift: ' + path)
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(raw)
    print('Completion import: 164 quests; 1746 tracks; 3679 transitions; 382 held; activation=false')


if __name__ == '__main__':
    main()
