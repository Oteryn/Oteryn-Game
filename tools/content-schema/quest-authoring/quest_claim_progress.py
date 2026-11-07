"""Explicit native claim-to-progress bindings through existing quest obligations.

The physical claim must already have a complete pinned native projection. These
bindings neither admit rewards nor replace their source guards or once receipts.
"""
import hashlib
import json
import re

PATH = 'tools/content-schema/quest-authoring/native-claim-progress.json'
TOOL = 'tools/content-schema/quest-authoring/quest_claim_progress.py'
SOURCE = 'tools/content-schema/reward-claim-authoring/container_native_source.json'
FIELDS = {'claim', 'claim_revision', 'unique_id', 'item', 'source_completion',
          'quest', 'track', 'transition', 'initial', 'minimum', 'maximum', 'value',
          'source', 'assumption'}


def require(condition, message):
    if not condition:
        raise ValueError('native claim progress: ' + message)


def load(root):
    path = root / PATH
    if not path.exists():
        return []
    document = json.loads(path.read_text())
    require(isinstance(document, dict) and set(document) == {'schema', 'bindings'}
            and type(document['schema']) is int and document['schema'] == 1,
            'unknown document shape')
    rows = document['bindings']
    require(isinstance(rows, list) and len(rows) <= 512, 'binding bounds')
    seen_claims, seen_transitions = set(), set()
    for row in rows:
        require(isinstance(row, dict) and set(row) == FIELDS, 'unknown binding fields')
        for field, prefix in [('claim', 'oteryn:reward-claim.'),
                              ('item', 'oteryn:item.'), ('quest', 'oteryn:quest'),
                              ('track', 'oteryn:quest-progress/'),
                              ('transition', 'oteryn:quest-transition/')]:
            value = row[field]
            require(isinstance(value, str) and value.startswith(prefix)
                    and len(value) <= 128 and re.fullmatch(r'oteryn:[A-Za-z0-9._:/-]+', value),
                    'invalid ' + field)
        require(row['claim'] not in seen_claims and row['transition'] not in seen_transitions,
                'duplicate claim or transition')
        seen_claims.add(row['claim'])
        seen_transitions.add(row['transition'])
        require(type(row['unique_id']) is int and 1 <= row['unique_id'] <= 65535,
                'invalid source unique id')
        require(all(type(row[field]) is int and -(2**63) <= row[field] < 2**63
                    for field in ('initial', 'minimum', 'maximum', 'value'))
                and row['minimum'] <= row['initial'] < row['value'] == row['maximum'],
                'invalid track bounds or advance')
        require(all(isinstance(row[field], str) and row[field]
                    for field in ('claim_revision', 'source_completion', 'assumption')),
                'missing revision, source completion or explicit rule')
        source = row['source']
        require(isinstance(source, dict) and set(source) == {
            'repository', 'revision', 'path', 'line', 'blob_sha1', 'url'}, 'source witness shape')
        require(type(source['line']) is int and source['line'] > 0
                and all(isinstance(source[field], str) and source[field]
                        for field in ('repository', 'path', 'url'))
                and isinstance(source['revision'], str) and isinstance(source['blob_sha1'], str)
                and re.fullmatch('[0-9a-f]{40}', source['revision'])
                and re.fullmatch('[0-9a-f]{40}', source['blob_sha1']), 'invalid source witness')
    return rows


def apply_claims(root, records):
    rows = load(root)
    if not rows:
        return
    raw = (root / SOURCE).read_bytes()
    source = json.loads(raw)
    definitions = {r['definition']['identity']['key']: r['definition'] for r in records}
    for row in rows:
        definition = definitions.get(row['claim'])
        require(definition is not None and definition.get('definition_profile') == 'authored_variant_v1'
                and definition['identity']['revision'] == row['claim_revision']
                and not definition.get('data_holds'), 'claim is missing, held or has another revision')
        projection = definition.get('native_contents', {})
        require(projection.get('revision') == row['source']['revision']
                and projection.get('source_sha256') == hashlib.sha256(raw).hexdigest(),
                'claim lacks a complete pinned native projection')
        witness = row['source']
        require(all(witness[field] == source[source_field] for field, source_field in [
            ('repository', 'repository'), ('revision', 'revision'),
            ('path', 'source_path'), ('blob_sha1', 'source_blob_sha1')]), 'source witness mismatch')
        placements = definition['placements']
        recipes = projection.get('placements', [])
        require(len(placements) == len(recipes) == 1, 'one exact source placement is required')
        recipe, placement = recipes[0], placements[0]
        require(recipe.get('unique_id') == row['unique_id'] and recipe.get('line') == witness['line']
                and recipe.get('root') == {'family': 'Item', 'key': row['item'], 'revision': 'definition-r1'}
                and recipe.get('completion') == {'kind': 'storage', 'key': row['source_completion']},
                'instruction identity, source placement or completion mismatch')
        require(placement.get('quest_transition') in (None, row['transition']),
                'cannot replace an existing claim obligation')
        placement['quest_transition'] = row['transition']


def extend_state(root, quests, definitions):
    return extend_bindings(quests, definitions, load(root))


def extend_bindings(quests, definitions, rows):
    """Shared monotone flag lowering after a producer validates its source binding."""
    for row in rows:
        definition = next((d for d in definitions if d['identity']['key'] == row['quest']), None)
        require(definition is not None, 'unknown owner quest')
        owner = next((q for q in quests if q['quest'] == row['quest']), None)
        if owner is None:
            owner = {'quest': row['quest'], 'source_quest': definition['source_refs']['quest']['key'],
                     'completion': 'NOT_LOWERED_NO_MISSIONS', 'tracks': [], 'transitions': []}
            quests.append(owner)
        require(not any(t['key'] == row['track'] for q in quests for t in q['tracks']),
                'track already has an owner')
        require(not any(t['key'] == row['transition'] for q in quests for t in q['transitions']),
                'transition already has an owner')
        tail = row['track'].removeprefix('oteryn:quest-progress/')
        source_track = ('crystalserver:quest-progress/' + tail.removeprefix('crystalserver/')
                        if tail.startswith('crystalserver/') else 'canary:quest-progress/' + tail)
        owner['tracks'].append({'key': row['track'], 'quest': row['quest'],
            'initial': row['initial'], 'min': row['minimum'], 'max': row['maximum'],
            'source_key': source_track, 'bounds_basis': 'native_claim_source_binding'})
        owner['transitions'].append({'key': row['transition'], 'quest': row['quest'],
            'completes': False, 'requested_by': None,
            # The claim receipt owns eligibility. Teaching an already learned
            # flag must still consume its obligation, without lowering a value.
            'effects': [{'track': row['track'], 'from': {'op': 'ANY'},
                         'from_exact': True, 'effect': {'kind': 'SET', 'value': row['value']}}],
            'source': {'key': 'chest', 'owner': 'chest', 'script': row['source']['path'],
                       'servers': ['crystalserver']}})
        owner['tracks'].sort(key=lambda t: t['key'])
        owner['transitions'].sort(key=lambda t: t['key'])
    quests.sort(key=lambda q: q['quest'])
    return [row['claim'] for row in rows]
