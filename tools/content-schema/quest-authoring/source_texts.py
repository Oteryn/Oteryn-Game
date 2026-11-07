"""Exact SOURCE prose references and capture witnesses; never evaluates Lua."""
import hashlib
import json
import re
from collections import defaultdict

import lua_tables

FIELDS = {'sha256', 'length', 'placeholders'}
PLACEHOLDER = re.compile(r'%[-0-9.]*[dsif]|\|[A-Z_]+\|')


def supported_literal(kind, literal):
    return kind == 'lstring' or all(m.group(1) in 'ntz\\\"\''
                                    for m in re.finditer(r'\\(.)', literal, re.S))


def reference(text):
    return {'sha256': hashlib.sha256(text.encode('utf-8')).hexdigest(),
            'length': len(text), 'placeholders': sorted(set(PLACEHOLDER.findall(text)))}


def identity(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False)


def walk(value, path=''):
    if isinstance(value, dict):
        yield path, value
        for key, child in value.items():
            yield from walk(child, path + '/' + str(key).replace('~', '~0').replace('/', '~1'))
    elif isinstance(value, list):
        for ordinal, child in enumerate(value):
            yield from walk(child, path + '/' + str(ordinal))


def owner_index(data):
    owners = defaultdict(set)
    quests = {q['identity']['key'] for q in data['quests']}
    for q in data['quests']:
        owners[q['identity']['key']].add(q['identity']['key'])
    for track in data['progress']:
        owners[track['key']].update(track.get('start_of', []) + track.get('auxiliary_of', []))
        owners[track['key']].update(m.split('#', 1)[0] for m in track['missions'])
    for role in ('claims', 'gates'):
        for row in data[role]:
            if row.get('quest'):
                owners[row['identity']['key']].add(row['quest']['key'])
    gates = {g['identity']['key']: g for g in data['gates']}
    for track in data['progress']:
        for gate in track['read_by_gates']:
            if gates.get(gate, {}).get('quest'):
                owners[track['key']].add(gates[gate]['quest']['key'])
    graphs = list(data['interactions'])
    graphs += [a['interaction'] for c in data.get('interaction_source_conflicts', []) for a in c['alternatives']]
    for row in graphs:
        key = row['identity']['key']
        for _, node in walk(row):
            for value in node.values():
                if isinstance(value, str) and value in owners:
                    owners[key].update(owners[value])
            if node.get('family') in ('RewardClaim', 'Quest'):
                owners[key].update(owners[node['key']])
    return {key: sorted(values & quests) for key, values in owners.items()}


def wanted(data):
    owners = owner_index(data)
    entries = {}
    collections = [(role, row, row['key'] if role == 'progress' else row['identity']['key'], '')
                   for role in ('quests', 'progress', 'interactions', 'gates', 'claims') for row in data[role]]
    collections += [('wiki_catalogue', row, row['wiki_title'], '') for row in data['wiki_catalogue']['quests']]
    collections += [('interactions', a['interaction'], c['interaction'], '/conflict_alternatives/' + a['source'])
                    for c in data.get('interaction_source_conflicts', []) for a in c['alternatives']]
    for role, row, key, prefix in collections:
        row_owners = (sorted({q['identity']['key'] for q in row['authored_candidates']})
                      if role == 'wiki_catalogue' else owners.get(row['key'] if role=='progress' else row['identity']['key'], []))
        for path, node in walk(row, prefix):
            if set(node) == FIELDS:
                entry = entries.setdefault(identity(node), {'reference': node, 'references': []})
                entry['references'].append({'role': role, 'record': key, 'path': path, 'owners': row_owners})
    for entry in entries.values():
        entry['references'] = [json.loads(k) for k in sorted({identity(r) for r in entry['references']})]
    return [entries[k] for k in sorted(entries)]


def validate_capture(capture):
    seen = set()
    for entry in capture['texts']:
        key = identity(entry['reference'])
        if key in seen or reference(entry['text']) != entry['reference']:
            raise ValueError('duplicate or mismatched exact source text')
        seen.add(key)
        for witness in entry['sources']:
            literal = witness['raw_literal']
            if not supported_literal(witness['token_kind'],literal):
                raise ValueError('unsupported short literal escape cannot be verified')
            if witness['source_read_mode']=='universal_newlines':
                literal=literal.replace('\r\n','\n').replace('\r','\n')
            token = (witness['token_kind'], literal, witness['line'])
            tokens = lua_tables.tokenize(literal)
            decoded=lua_tables._string(token)
            if witness['text_transform']=='chest_written_text_strip_lf':
                if not witness['path'].endswith('/scripts/actions/system/quest_reward_common.lua'):
                    raise ValueError('written-text transformation outside chest table')
                decoded=decoded.strip('\n')
            if len(tokens) != 1 or tokens[0][0] != witness['token_kind'] or decoded != entry['text']:
                raise ValueError('source literal witness does not decode to exact text')
            if hashlib.sha256(witness['raw_literal'].encode('utf-8')).hexdigest() != witness['literal_sha256']:
                raise ValueError('source literal witness digest mismatch')


def registry(data, capture):
    validate_capture(capture)
    by_ref = {identity(row['reference']): row for row in capture['texts']}
    texts = []
    for row in wanted(data):
        known = by_ref.get(identity(row['reference']))
        texts.append({**row, 'classification': 'KNOWN' if known else 'UNKNOWN',
                      **({'text': known['text'], 'sources': known['sources']} if known else
                         {'reason': 'No exact captured literal matches hash, length and placeholders'})})
    return {'schema': 'OTERYN_SOURCE_TEXT_REGISTRY/v1', 'classification': 'OTS_HYPOTHESIS_ONLY',
            'scope': 'source_reference_prose_only; no native or runtime admission',
            'texts': texts, 'summary': {'wanted': len(texts), 'known': sum(t['classification'] == 'KNOWN' for t in texts),
                                      'unknown': sum(t['classification'] == 'UNKNOWN' for t in texts)}}


def unresolved_gaps(data):
    gaps = []
    for entry in data.get('source_texts', {}).get('texts', []):
        if entry['classification'] == 'UNKNOWN':
            for location in entry['references']:
                gaps.append({'owners': location['owners'], 'record': location['record'] + location['path'],
                             'reason': 'UNKNOWN source text literal: ' + entry['reference']['sha256']})
    return gaps
