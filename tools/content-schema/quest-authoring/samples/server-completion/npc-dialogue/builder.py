"""Finite authored-talk stage candidates using existing NPC membership and Source witnesses."""
import argparse
import base64
import collections
import gzip
import hashlib
import json
from pathlib import Path
import re

GENERIC = {'a', 'an', 'the', 'and', 'of', 'to', 'in', 'on', 'for', 'with', 'from', 'at', 'as', 'is', 'be', 'by', 'it', 'or', 'you', 'your', 'quest', 'mission', 'task', 'report', 'talk', 'ask', 'accept', 'return', 'reward', 'receive', 'finish', 'complete', 'first', 'second', 'third', 'offer', 'bring', 'about', 'i', 'me', 'my', 'we', 'our', 'they', 'their', 'have', 'has', 'had', 'can', 'could', 'will', 'would', 'do', 'does', 'done', 'not', 'need', 'this', 'that', 'these', 'those', 'there', 'here', 'then', 'now'}
QUEST_TRIGGERS = {'quest', 'mission', 'task', 'report', 'outfit', 'addon'}


def stable(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode()


def sha(value):
    return hashlib.sha256(value).hexdigest()


def walk(value, pointer=''):
    if isinstance(value, dict):
        if 'node_type' in value:
            yield pointer, value
        for key, child in value.items():
            yield from walk(child, pointer + '/' + key)
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from walk(child, pointer + '/' + str(index))


def tokens(value):
    return set(re.findall(r'[a-z][a-z0-9]*', value.lower())) - GENERIC


def slug(value):
    return re.sub('[^a-z0-9]+', '_', value.lower()).strip('_')


def branches(keywords, pointer, indices=(), keys=(), ancestry_unique=True):
    seen = collections.Counter(k['key'] for k in keywords)
    for index, keyword in enumerate(keywords):
        path = indices + (index,)
        key_path = keys + (keyword['key'],)
        node_pointer = pointer + '/' + str(index)
        unique = ancestry_unique and seen[keyword['key']] == 1
        yield node_pointer, keyword, path, key_path, unique
        yield from branches(keyword.get('children', []), node_pointer + '/children', path, key_path, unique)


class SourceCache:
    def __init__(self, manifest, ast_root):
        self.path = Path(manifest)
        self.ast_root = Path(ast_root)
        rows = json.loads(self.path.read_bytes())['files']
        self.rows = {(r['source'], r['revision'], r['path'], r['sha256']): r for r in rows}
        self.cached = {}
        index = json.loads((self.ast_root / 'index.json').read_bytes())
        self.containers = {row['sha256']: row['container_sha256'] for row in index['captures']}

    def strings(self, identity):
        key = identity['source'], identity['revision'], identity['path'], identity['source_sha256']
        if key in self.cached:
            return self.cached[key]
        row = self.rows.get(key)
        if row is None:
            return []
        source = Path(row['cache_path'])
        if not source.is_absolute():
            source = self.path.parent / source
        raw = source.read_bytes()
        if sha(raw) != key[3] or identity['git_blob_sha1'] != row['git_blob_sha1']:
            raise ValueError('Source identity witness mismatch')
        capture = self.ast_root / 'captures' / (key[3] + '.json.gz')
        if not capture.exists():
            return []
        container = capture.read_bytes()
        if sha(container) != self.containers.get(key[3]):
            raise ValueError('Source AST container mismatch')
        document = json.loads(gzip.decompress(container))
        if base64.b64decode(document['raw_bytes_base64']) != raw:
            raise ValueError('Source AST raw mismatch')
        nodes = list(walk(document['ast']))
        evidence = []
        for pointer, node in nodes:
            if node['node_type'] != 'String':
                continue
            fields = node['fields']
            if fields['delimiter']['name'] not in ['SINGLE_QUOTE', 'DOUBLE_QUOTE'] or '\\' in fields['raw']:
                continue
            text = base64.b64decode(fields['s']['bytes_base64']).decode()
            if text != fields['raw']:
                raise ValueError('Unexpected simple Lua string decoding')
            span = node.get('span')
            if span is None:
                raise ValueError('Source literal span missing')
            source_text = raw.decode('utf-8-sig')[span['start_char']:span['end_char_exclusive']]
            quote = chr(34) if fields['delimiter']['name'] == 'DOUBLE_QUOTE' else chr(39)
            if source_text != quote + text + quote:
                raise ValueError('Source literal token mismatch')
            parents = [(p, n) for p, n in nodes if pointer.startswith(p + '/fields/') and n['node_type'] in ['Invoke', 'Call', 'If', 'ElseIf']]
            invocations = [(p, n) for p, n in parents if n['node_type'] in ['Invoke', 'Call']]
            speech = next(((p, n) for p, n in reversed(invocations) if n['node_type'] == 'Invoke' and n['fields']['func'].get('fields', {}).get('id') in ['say', 'addKeyword', 'addChildKeyword']), None)
            if speech is None:
                continue
            invocation, call = speech
            method = call['fields']['func']['fields']['id']
            if method == 'say':
                if call['fields']['source'].get('node_type') != 'Name' or call['fields']['source']['fields']['id'] != 'npcHandler':
                    continue
                if not pointer.startswith(invocation + '/fields/args/0'):
                    continue
            else:
                callback = call['fields']['args'][1] if len(call['fields']['args']) > 1 else {}
                callback_fields = callback.get('fields', {})
                if callback.get('node_type') != 'Index' or callback_fields.get('value', {}).get('fields', {}).get('id') != 'StdModule' or callback_fields.get('idx', {}).get('fields', {}).get('id') != 'say':
                    continue
                text_fields = [(p, n) for p, n in nodes if n['node_type'] == 'Field' and (n['fields'].get('key') or {}).get('fields', {}).get('id') == 'text' and pointer.startswith(p + '/fields/value')]
                if not pointer.startswith(invocation + '/fields/args/2') or not text_fields:
                    continue
            evidence.append({
                'text': text, 'provenance': identity, 'literal_ast_pointer': pointer,
                'literal_node_sha256': sha(stable(node)), 'literal_span': node['span'],
                'literal_source_text': source_text, 'literal_source_sha256': sha(source_text.encode()),
                'speech_invocation_pointer': speech[0], 'speech_invocation_ast': speech[1],
                'conditional_ancestor_pointers': [p for p, n in parents if n['node_type'] in ['If', 'ElseIf']],
                'dispatch_proven': False, 'method_and_helper_shadowing': 'UNPROVEN_SOURCE_SYNTAX_ONLY',
                'source_spec_complete': False,
            })
        self.cached[key] = evidence
        return evidence


def choose_branches(stage, quest, npc_target, dialogue, pointer):
    specific = tokens(stage['objective'] + ' ' + quest['display_name'])
    specific -= tokens(npc_target)
    result = []
    for branch_pointer, branch, indices, keys, unique in branches(dialogue.get('keywords', []), pointer):
        if not unique or len(keys) > 8:
            continue
        hits = sorted(specific & tokens(' '.join(branch.get('triggers', []) + branch.get('reply', []))))
        generic = sorted(QUEST_TRIGGERS & set(branch.get('triggers', [])))
        if not hits and not generic:
            continue
        result.append({
            'branch_pointer': branch_pointer, 'branch_indices': list(indices), 'branch_keys': list(keys),
            'branch_sha256': sha(stable(branch)), 'triggers': branch.get('triggers', []),
            'reply': branch.get('reply', []), 'semantic_token_hits': hits,
            'scope': 'DERIVED_STAGE_TEXT_HINT' if hits else 'DERIVED_GENERIC_QUEST_TRIGGER',
            'native_bound': False,
        })
    return result


def build(repo_root, declarations, corpus_manifest, ast_root):
    root = Path(repo_root)
    index = json.loads((root / 'content/quests/definitions/index.json').read_bytes())
    world_path = Path(declarations)
    world_raw = world_path.read_bytes()
    world = json.loads(world_raw)
    npcs = {d['identity']['key']: (i, d) for i, d in enumerate(world['records']) if d['kind'] == 'NPC'}
    dialogues = {(d['identity']['key'], d['identity']['revision']): (i, d) for i, d in enumerate(world['records']) if d['kind'] == 'Dialogue'}
    source = SourceCache(corpus_manifest, ast_root)
    records = []
    inputs = []
    authored_count = 0
    for shard in index['shards']:
        raw = (root / shard).read_bytes()
        inputs.append({'path': shard, 'sha256': sha(raw)})
        for n, wrapper in enumerate(json.loads(raw)['records']):
            quest = wrapper['definition']
            if quest.get('definition_profile') != 'oteryn_authored_v1':
                continue
            authored_count += 1
            for stage_index, stage in enumerate(quest['recipe']['stages']):
                if stage['kind'] != 'talk':
                    continue
                row = {
                    'quest': quest['identity'], 'stage': stage['key'], 'objective': stage['objective'],
                    'stage_ref': {'path': shard, 'packet_sha256': sha(raw), 'json_pointer': f'/records/{n}/definition/recipe/stages/{stage_index}', 'record_sha256': sha(stable(stage))},
                    'npc_candidates': [], 'held_targets': [], 'native_bound': False,
                }
                for target in stage['targets']:
                    candidate = npcs.get('oteryn:npc.' + slug(target))
                    if candidate is None:
                        row['held_targets'].append({'target': target, 'reason': 'NO_EXACT_EXISTING_SLUG_KEY_NOT_ASSUMED_TO_BE_NPC'})
                        continue
                    npc_index, npc = candidate
                    ref = npc.get('dialogue')
                    exact = dialogues.get((ref['key'], ref['revision'])) if ref and ref['family'] == 'Dialogue' else None
                    npc_row = {'target': target, 'npc_ref': {'family': 'NPC', **npc['identity']}, 'npc_pointer': f'/records/{npc_index}', 'npc_record_sha256': sha(stable(npc)), 'match': 'DERIVED_TARGET_NAME_TO_EXISTING_SLUG_KEY', 'dialogue_membership_proven': exact is not None, 'dialogue_ref': ref, 'branches': [], 'native_bound': False, 'source_association_scopes': []}
                    witnesses = []
                    for field in npc.get('fields', []):
                        if field['field_path'].endswith('upstream_full_payload_ref'):
                            metadata = json.loads(field['value']['value'])
                            npc_row['source_association_scopes'].append(metadata.get('association'))
                            for entry in metadata['entries']:
                                witnesses.extend(source.strings(entry['identity']))
                    if exact:
                        dialogue_index, dialogue = exact
                        npc_row['dialogue_record_sha256'] = sha(stable(dialogue))
                        npc_row['branches'] = choose_branches(stage, quest, target, dialogue, f'/records/{dialogue_index}/keywords')
                        for branch in npc_row['branches']:
                            branch['source_reply_literal_witnesses'] = [w for w in witnesses if w['text'] in branch['reply']]
                            branch['source_reply_all_literals_witnessed'] = bool(branch['reply']) and all(any(w['text'] == text for w in witnesses) for text in branch['reply'])
                    row['npc_candidates'].append(npc_row)
                records.append(row)
    summary = {
        'authored_quests': authored_count, 'talk_quests': len({r['quest']['key'] for r in records}), 'talk_stages': len(records),
        'quests_with_existing_npc_candidates': len({r['quest']['key'] for r in records if r['npc_candidates']}),
        'quests_with_exact_dialogue_membership': len({r['quest']['key'] for r in records if any(n['dialogue_membership_proven'] for n in r['npc_candidates'])}),
        'derived_branch_candidates': sum(len(n['branches']) for r in records for n in r['npc_candidates']),
        'branches_with_full_source_reply_literals': sum(b['source_reply_all_literals_witnessed'] for r in records for n in r['npc_candidates'] for b in n['branches']),
        'quests_with_derived_branch_candidates': len({r['quest']['key'] for r in records if any(n['branches'] for n in r['npc_candidates'])}),
        'quests_with_full_source_reply_witnesses': len({r['quest']['key'] for r in records if any(any(b['source_reply_all_literals_witnessed'] for b in n['branches']) for n in r['npc_candidates'])}),
        'native_bound': 0,
    }
    return {'schema': 'OTERYN_AUTHORED_QUEST_NPC_STAGE_CANDIDATES/v1', 'scope': 'SOURCE_EVIDENCE_AND_DERIVED_STAGE_HINTS_NOT_NATIVE_BINDINGS', 'inputs': inputs, 'world_declarations_sha256': sha(world_raw), 'source_corpus_manifest_sha256': sha(Path(corpus_manifest).read_bytes()), 'source_ast_index_sha256': sha((Path(ast_root) / 'index.json').read_bytes()), 'reference_scope': 'VERIFIED_NPC_PACKET_WORLD_SNAPSHOT_NOT_LIVE_CATALOGUE', 'records': records, 'summary': summary, 'native_admission': False, 'runtime_activation': False}


def main():
    parser = argparse.ArgumentParser()
    for name in ['repo-root', 'declarations', 'corpus-manifest', 'ast-root', 'out']:
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    result = build(args.repo_root, args.declarations, args.corpus_manifest, args.ast_root)
    args.out.write_bytes(stable(result) + b'\n')
    print(result['summary'])


if __name__ == '__main__':
    main()
