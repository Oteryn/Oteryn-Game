"""Export the frozen NPC audit as portable, unactivated native DTO candidates."""
import argparse
import copy
import hashlib
import json
from collections import Counter
from pathlib import Path

INPUTS = {
    'finish-integration-r4/qualified-native-project/authoring-project/definitions/declarations.json':
        'e83d24bd0b09915c6f0af1a99bfa3c31124e4b343bac7e76fe298ac99df34b45',
    'finish-dialogue-programs/dialogue-source-program-overlay-final-r1.json':
        '528fcee96b4f588660f417048a6ed9e007b8e124d0ecd510e8079404e6b4d819',
    'finish-integration-r3/completion-native-input.json':
        '9ac0ff00997baf406b5da86aaee1616b08763be426122f77fbbaabed3a4863b3',
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def encode(value):
    return (json.dumps(value, ensure_ascii=False, indent=1, sort_keys=True) + '\n').encode('utf-8')


def portable(value):
    """Keep source identities/digests; replace unavailable local capture paths."""
    if isinstance(value, list):
        return [portable(x) for x in value]
    if isinstance(value, dict):
        return {k: portable(v) for k, v in value.items()}
    if isinstance(value, str):
        if value == 'REMOVED_CURRENT_GLOBAL':
            return 'TWO_WIKI_DEPRECATED_OR_INACCESSIBLE'
        if value.startswith(('C:\\', 'C:/')):
            return 'capture-artifact:' + value.replace('\\', '/').split('/')[-1]
        if value.startswith(('{', '[')):
            try:
                return json.dumps(portable(json.loads(value)), ensure_ascii=False,
                                  sort_keys=True, separators=(',', ':'))
            except json.JSONDecodeError:
                pass
    return value


def text_field(path, value):
    return {'field_path': path, 'value': {'type': 'Text', 'value':
            json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))}}


def normalized(value):
    if isinstance(value, list):
        return [normalized(x) for x in value]
    if isinstance(value, dict):
        return {k: sorted(v) if k == 'triggers' else normalized(v)
                for k, v in value.items() if not (k == 'children' and not v)}
    return value


def export(frozen_root, output):
    inputs = {}
    for name, expected in INPUTS.items():
        raw = (frozen_root / name).read_bytes()
        if digest(raw) != expected:
            raise ValueError('Frozen input changed: ' + name)
        inputs[name] = json.loads(raw)
    overlay = inputs[next(name for name in INPUTS if 'overlay-final' in name)]
    native = inputs[next(name for name in INPUTS if '/definitions/' in name)]['records']
    prior = inputs['finish-integration-r3/completion-native-input.json']
    programs = {p['canonical_key']: p for p in overlay['canonical_npcs']}
    by_key = {r['identity']['key']: r for r in native}
    npcs, dialogues, index = [], {}, []
    for key, source in sorted(programs.items()):
        npc = copy.deepcopy(by_key[key])
        if npc.get('dialogue'):
            dkey = npc['dialogue']['key']
            dialogue = copy.deepcopy(by_key[dkey])
            dialogue.setdefault('keywords', [])
            expected = source['declaration']['keywords'] if source['declaration'] else []
            assert normalized(dialogue['keywords']) == normalized(expected), key
            dialogue['fields'] = [text_field('oteryn:source.npc.source_audit', {
                'npc_key': key, 'source_id': source['primary_source_id'],
                'source_status': source['primary_source_status'],
                'runtime_qualified': False, 'global_branch_complete': False,
                'empty_source_hold': not bool(dialogue['keywords'])})]
            dialogues[dkey] = dialogue
        npcs.append(portable(npc))
        index.append(portable({
            'npc_key': key, 'name': source['candidate_name'],
            'dialogue': npc.get('dialogue'), 'source_id': source['primary_source_id'],
            'source_status': source['primary_source_status'],
            'source_metadata': source['primary_source_metadata'],
            'accepted_roots': len(source['declaration']['keywords']) if source['declaration'] else 0,
            'held_root_groups': len(source['held_root_groups']),
            'candidate_keywords_sha256': digest(encode(dialogues[npc['dialogue']['key']]['keywords']))
                if npc.get('dialogue') else None,
            'source_keywords_sha256': digest(encode(source['declaration']['keywords']))
                if source['declaration'] else None,
            'held_groups_sha256': digest(encode(source['held_root_groups'])),
            'game_keyword_matcher_proven': False, 'game_handler_precedence_proven': False,
            'quest_guards_focus_actions_proven': False, 'runtime_qualified': False,
            'global_branch_complete': False}))
    # These are candidate corrections, never an overlay applied to active content.
    services = portable(prior['replacement_services'])
    artifacts, artifact_indices, proof_rows = [], {}, []
    for source in overlay['canonical_npcs']:
        if not source['declaration']:
            continue
        roots = []
        for branch in source['accepted_complete_roots']:
            locators = []
            for proof in branch['proofs']:
                ref = proof['declaration_reference']
                source_sha = ref['sha256']
                if source_sha not in artifact_indices:
                    artifact_indices[source_sha] = len(artifacts)
                    artifacts.append({'sha256': source_sha, 'availability': 'OFFLINE_NOT_COMMITTED'})
                locators.append([artifact_indices[source_sha], ref['pointer'],
                                 proof['source_blocks'], proof['first_prompt_source_order']])
            roots.append([branch['node']['key'], branch['complete_root_sha256'], locators])
        proof_rows.append({'npc_key': source['canonical_key'], 'root_proofs': roots})
    proofs = {'schema': 'NPC_SOURCE_ROOT_PROOFS/v1', 'native_source_artifacts': artifacts,
              'root_columns': ['native_keyword_key', 'semantic_sha256', 'source_locators'],
              'locator_columns': ['artifact_index', 'json_pointer', 'player_blocks', 'first_prompt_order'],
              'semantic_hash': 'UTF8 sorted compact JSON; node keys excluded',
              'npcs': proof_rows, 'runtime_qualified': False}
    payloads = {
        'npc-candidates.json': {'records': npcs},
        'dialogue-candidates.json': {'records': [dialogues[k] for k in sorted(dialogues)]},
        'service-corrections.json': {'records': services},
        'source-index.json': {'records': index},
        'root-proofs.json': proofs,
    }
    output.mkdir(parents=True, exist_ok=True)
    files = []
    for name, value in payloads.items():
        raw = encode(value)
        (output / name).write_bytes(raw)
        files.append({'path': name, 'sha256': digest(raw), 'bytes': len(raw)})
    summary = {
        'scope': 'SOURCE_AUTHORING_CANDIDATES_ONLY', 'runtime_qualified': False,
        'global_complete': False, 'active_content_modified': False,
        'source_context_sha': 'c1b6d3ce43eab12455df3509c860554a60921da0',
        'publication_dependency_sha': '25d79fb52104f1d2ce006f080f4bec665cc346ee',
        'inputs': INPUTS, 'files': files, 'npc_candidates': len(npcs),
        'nonempty_dialogues': sum(bool(d['keywords']) for d in dialogues.values()),
        'empty_dialogue_holds': sum(not d['keywords'] for d in dialogues.values()),
        'service_corrections': len(services),
        'accepted_roots': sum(p['accepted_roots'] for p in index),
        'held_root_groups': sum(p['held_root_groups'] for p in index),
        'nonempty_source_statuses': dict(Counter(p['source_status'] for p in index
                                               if p['accepted_roots'])),
        'unallocated_npc_proposals_included': 0,
        'external_native_qualification': 'Prior R4 construction/parse/roundtrip/link PASS; not candidate CI',
        'reference_playable_qualification': 'FAIL: 56480 definitions exceed Item-only limit 38157',
        'capture_artifacts_available_in_repository': False,
        'disposition_correction': 'REMOVED_CURRENT_GLOBAL narrowed to TWO_WIKI_DEPRECATED_OR_INACCESSIBLE',
    }
    (output / 'manifest.json').write_bytes(encode(summary))
    print(json.dumps({k: summary[k] for k in ('npc_candidates', 'nonempty_dialogues',
                                             'empty_dialogue_holds', 'service_corrections')}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--frozen-root', required=True, type=Path)
    parser.add_argument('--out', required=True, type=Path)
    args = parser.parse_args()
    export(args.frozen_root, args.out)
