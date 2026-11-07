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
OPTIONAL_FIELDS = {'source_claim', 'from'}


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
        require(isinstance(row, dict) and FIELDS <= set(row) <= FIELDS | OPTIONAL_FIELDS, 'unknown binding fields')
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
        if 'from' in row:
            require(type(row['from']) is int and row['minimum'] <= row['from'] <= row['maximum'],
                    'invalid exact source prerequisite')
        if 'source_claim' in row:
            require(isinstance(row['source_claim'], str)
                    and row['source_claim'].startswith('crystalserver:reward-claim/')
                    and 'from' in row, 'plain source claim requires an exact prerequisite')
        source = row['source']
        require(isinstance(source, dict) and set(source) == {
            'repository', 'revision', 'path', 'line',
            ('sha256' if 'source_claim' in row else 'blob_sha1'), 'url'}, 'source witness shape')
        require(type(source['line']) is int and source['line'] > 0
                and all(isinstance(source[field], str) and source[field]
                        for field in ('repository', 'path', 'url'))
                and isinstance(source['revision'], str)
                and re.fullmatch('[0-9a-f]{40}', source['revision'])
                and isinstance(source.get('sha256', source.get('blob_sha1')), str)
                and re.fullmatch('[0-9a-f]{64}' if 'source_claim' in row else '[0-9a-f]{40}',
                                 source.get('sha256', source.get('blob_sha1'))), 'invalid source witness')
    return rows


def apply_claims(root, records, *, plain=False):
    rows = [r for r in load(root) if bool(r.get("source_claim")) == plain]
    if not rows:
        return
    if plain:
        return apply_plain_claims(root, records, rows)
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


def apply_plain_claims(root, records, rows):
    source = json.loads((root / 'tools/content-schema/quest-authoring/native-source-claims.json').read_text())
    definitions = {r['definition']['identity']['key']: r['definition'] for r in records}
    for row in rows:
        matches = [r for r in source['claims'] if r['key'] == row['source_claim']]
        require(len(matches) == 1, 'one normalized source claim is required')
        witness = matches[0]
        definition = definitions.get(row['claim'])
        require(definition is not None and definition.get('readiness') == 'ready'
                and definition['identity']['revision'] == row['claim_revision']
                and definition['provenance']['pilot_key'] == row['source_claim']
                and row['source'] == witness['source']
                and row['unique_id'] == witness['unique_id']
                and row['item'] == f"oteryn:item.tibia.i{witness['item']}"
                and len(definition['placements']) == 1, 'plain source witness mismatch')
        placement = definition['placements'][0]
        require(placement['source_binding']['project_position'] == witness['position']
                and placement['source_binding']['legacy_unique_ids'] == [
                    {'server': 'crystalserver', 'unique_id': row['unique_id']}]
                and placement['appearance_tibia_id'] == witness['appearance']
                and placement['reward']['items'] == [{'item': {'family': 'Item',
                    'key': row['item'], 'revision': 'definition-r1'}, 'count': witness['count']}]
                and placement.get('quest_transition') in (None, row['transition']),
                'plain source projection mismatch')
        placement['quest_transition'] = row['transition']
        placement['quest_requirements'] = [{'track': row['track'],
            'minimum': row['from'], 'maximum': row['from']}]


def extend_state(root, quests, definitions):
    rows = load(root)
    for row in rows:
        definition = next((d for d in definitions if d['identity']['key'] == row['quest']), None)
        require(definition is not None, 'unknown owner quest')
        owner = next((q for q in quests if q['quest'] == row['quest']), None)
        if owner is None:
            owner = {'quest': row['quest'], 'source_quest': definition['source_refs']['quest']['key'],
                     'completion': 'NOT_LOWERED_NO_MISSIONS', 'tracks': [], 'transitions': []}
            quests.append(owner)
        existing = [t for q in quests for t in q['tracks'] if t['key'] == row['track']]
        if existing:
            require('from' in row and len(existing) == 1
                    and existing[0]['quest'] == row['quest']
                    and existing[0]['initial'] == row['initial']
                    and existing[0]['min'] == row['minimum']
                    and existing[0]['max'] <= row['maximum'],
                    'track already has an owner or incompatible source bounds')
            existing[0]['max'] = row['maximum']
        require(not any(t['key'] == row['transition'] for q in quests for t in q['transitions']),
                'transition already has an owner')
        tail = row['track'].removeprefix('oteryn:quest-progress/')
        source_track = ('crystalserver:quest-progress/' + tail.removeprefix('crystalserver/')
                        if tail.startswith('crystalserver/') else 'canary:quest-progress/' + tail)
        if not existing:
            owner['tracks'].append({'key': row['track'], 'quest': row['quest'],
                'initial': row['initial'], 'min': row['minimum'], 'max': row['maximum'],
                'source_key': source_track, 'bounds_basis': 'native_claim_source_binding'})
        owner['transitions'].append({'key': row['transition'], 'quest': row['quest'],
            'completes': False, 'requested_by': None,
            # The claim receipt owns eligibility. Teaching an already learned
            # flag must still consume its obligation, without lowering a value.
            'effects': [{'track': row['track'], 'from': ({'op': 'EQ', 'value': row['from']} if 'from' in row else {'op': 'ANY'}),
                         'from_exact': True, 'effect': {'kind': 'SET', 'value': row['value']}}],
            'source': {'key': 'chest', 'owner': 'chest', 'script': row['source']['path'],
                       'servers': ['crystalserver']}})
        owner['tracks'].sort(key=lambda t: t['key'])
        owner['transitions'].sort(key=lambda t: t['key'])
    quests.sort(key=lambda q: q['quest'])
    return [row['claim'] for row in rows]
