"""Attach qualified donor projections to chosen Quest recipes, leaving Source cores intact."""
import copy
import gzip
import hashlib
import json
from pathlib import Path

import jsonschema

DIRECTORY = 'tools/content-schema/quest-authoring/'
DATA = DIRECTORY + 'samples/donor-source/'
EXTENDED_ID = 'oteryn:schema/quest-completion-donor-extension/v1'
SCHEMA_ID = 'oteryn:schema/quest-donor-attachment/v1'
INPUTS = ('semantic-conditions.json', 'semantic-conditions-qualification.json',
          'semantic-rewards.json', 'semantic-rewards-qualification.json')


def cached_read(root, path, cache):
    cache.setdefault('documents', {})
    cache.setdefault('digests', {})
    if path not in cache['documents']:
        cache['documents'][path] = read(root / path)
        cache['digests'][path] = hashlib.sha256((root / path).read_bytes()).hexdigest()
    return cache['documents'][path]


def verified_output(root, path, summary_path, prefix, cache):
    value = cached_read(root, path, cache)
    summary = cached_read(root, summary_path, cache)
    descriptor = [d for d in summary['outputs'] if prefix + d['path'] == path]
    if len(descriptor) != 1 or descriptor[0]['sha256'] != cache['digests'][path]:
        raise ValueError('Source output qualification digest differs: ' + path)
    return value


def read(path):
    raw = path.read_bytes()
    return json.loads(gzip.decompress(raw) if path.suffix == '.gz' else raw)


def digest(value):
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True,
                                   separators=(',', ':')).encode()).hexdigest()


def derived_completion_schema(base):
    schema = copy.deepcopy(base)
    schema['$id'] = EXTENDED_ID
    schema['$defs']['supplement']['properties']['donor_source_data'] = {'$ref': SCHEMA_ID}
    schema['$defs']['supplement']['properties']['chosen_journal_corrections'] = json.loads(
        Path(__file__).with_name('quest_chosen_journal_attachment.schema.json').read_text())
    return schema


def pointer(value, location):
    if not location.startswith('/'):
        raise ValueError('expected absolute JSON pointer')
    for part in location.split('/')[1:]:
        part = part.replace('~1', '/').replace('~0', '~')
        value = value[int(part)] if isinstance(value, list) else value[part]
    return value


def reference(root, path, location, value, category, scope, target, cache=None):
    return {'category': category, 'projection_scope': scope, 'source_target': target,
            'path': path, 'packet_sha256': cache['digests'][path] if cache else hashlib.sha256((root / path).read_bytes()).hexdigest(),
            'json_pointer': location, 'record_sha256': digest(value)}


def semantic_refs(root, source_data, cache=None):
    """Join only graphs already owned by this exact Source Quest, never directory names."""
    graphs = source_data.get('interactions', [])
    owners = {g['identity']['key']: g for g in graphs}
    result = []
    cache = {} if cache is None else cache
    conditions_path = DATA + 'semantic-conditions.json'
    packet = cached_read(root, conditions_path, cache)
    qualification = cached_read(root, DATA + 'semantic-conditions-qualification.json', cache)
    if qualification['state'] != 'PASS' or qualification['native_semantic_admission'] is not False:
        raise ValueError('condition qualification is not Source-only PASS')
    if qualification['packet_sha256'] != cache['digests'][conditions_path]:
        raise ValueError('condition qualification digest differs')
    baseline_path = DIRECTORY + 'samples/interactions/interactions.json'
    baseline = cached_read(root, baseline_path, cache)
    baseline_sha = cache['digests'][baseline_path]
    for n, row in enumerate(packet['conditions']):
        key = row['interaction']
        if key not in owners:
            continue
        witness = row['baseline_gap']
        if witness['file_sha256'] != baseline_sha:
            raise ValueError('stale condition baseline')
        graph_location = '/'.join(witness['json_pointer'].split('/')[:3])
        if pointer(baseline, graph_location) != owners[key]:
            raise ValueError('condition belongs to another Source graph variant')
        pointer(baseline, witness['json_pointer'])
        result.append(reference(root, conditions_path, f'/conditions/{n}', row,
                                'condition', row['normalization_status'], key, cache))
    rewards_path = DATA + 'semantic-rewards.json'
    rewards = cached_read(root, rewards_path, cache)
    proof = cached_read(root, DATA + 'semantic-rewards-qualification.json', cache)
    if proof['valid'] is not True or proof['native_semantic_admission'] is not False:
        raise ValueError('reward qualification is not Source-only PASS')
    if proof['packet_sha256'] != cache['digests'][rewards_path]:
        raise ValueError('reward qualification digest differs')
    if rewards['baseline_interactions_sha256'] != baseline_sha:
        raise ValueError('stale reward baseline')
    for n, row in enumerate(rewards['records']):
        key = row['interaction']['key']
        if key in owners:
            if row['baseline_graph'] != owners[key] or digest(owners[key]) != row['baseline_graph_sha256']:
                raise ValueError('reward belongs to another Source graph variant')
            result.append(reference(root, rewards_path, f'/records/{n}', row,
                                    'reward', 'GUARDED_SOURCE_CHOICES_WITH_FALLBACK', key, cache))
    return result


def refinement_refs(root, definition, cache):
    key = definition['identity']['key']
    data = definition.get('source_data', {})
    graphs = {g['identity']['key']: g for g in data.get('interactions', [])}
    baseline_path = DIRECTORY + 'samples/interactions/interactions.json'
    baseline = cached_read(root, baseline_path, cache)
    refs = []
    path = DATA + 'refinements/conditions.json'
    packet = verified_output(root, path, DATA + 'refinements/summary.json', DATA + 'refinements/', cache)
    proof = cached_read(root, DATA + 'refinements/conditions-qualification.json', cache)
    if proof['state'] != 'PASS' or proof['packet_sha256'] != cache['digests'][path]:
        raise ValueError('truthiness condition qualification differs')
    for n, value in enumerate(packet['records']):
        target = value['interaction']
        if target not in graphs:
            continue
        witness = value['baseline_gap']
        graph_location = '/'.join(witness['json_pointer'].split('/')[:3])
        if witness['file_sha256'] != cache['digests'][baseline_path] or pointer(baseline, graph_location) != graphs[target]:
            raise ValueError('truthiness condition Source variant differs')
        if key not in value['quest_keys']:
            raise ValueError('truthiness condition owner index is stale')
        refs.append(reference(root, path, f'/records/{n}', value, 'condition',
                              value['status'], target, cache))
    specs_path = DATA + 'refinements/guard-specs.json'
    specs = verified_output(root, specs_path, DATA + 'refinements/summary.json',
                            DATA + 'refinements/', cache)
    original = {digest(record): record for record in packet['records']}
    for n, value in enumerate(specs['records']):
        base = original.get(value['baseline_record_sha256'])
        if base is None or base['status'] != 'PARTIAL_SOURCE_GUARD':
            raise ValueError('operational guard has no exact partial Source baseline')
        for field in ('interaction', 'baseline_gap', 'provenance', 'source_ref'):
            if value[field] != base[field]:
                raise ValueError('operational guard Source witness differs: ' + field)
        target = base['interaction']
        if target not in graphs:
            continue
        witness = base['baseline_gap']
        graph_location = '/'.join(witness['json_pointer'].split('/')[:3])
        if witness['file_sha256'] != cache['digests'][baseline_path] or pointer(baseline, graph_location) != graphs[target]:
            raise ValueError('operational guard Source variant differs')
        if key not in base['quest_keys'] or value['quest_keys'] != base['quest_keys']:
            raise ValueError('operational guard owner index differs')
        if value['status'] != 'SOURCE_SPEC_COMPLETE_EXECUTION_UNPROVEN' or value['native_admission'] or value['runtime_activation'] or value['unresolved_dependencies']:
            raise ValueError('unqualified operational guard cannot be attached')
        refs.append(reference(root, specs_path, f'/records/{n}', value, 'condition',
                              value['status'], target, cache))
    for filename, scope in (('mission-progress.json', 'SOURCE_MISSION_WRITE_SPEC_EXECUTION_UNPROVEN'),
                            ('mission-progress-config.json', 'SOURCE_CONFIG_MISSION_WRITE_SPEC_EXECUTION_UNPROVEN')):
        progress_path = DATA + 'refinements/' + filename
        progress = verified_output(root, progress_path, DATA + 'refinements/summary.json',
                                   DATA + 'refinements/', cache)
        for n, value in enumerate(progress['records']):
            if key not in value['quest_keys']:
                continue
            owners = [owner for owner in value['canonical_mission_refs'] if owner['quest_key'] == key]
            if not owners:
                raise ValueError('mission write has no canonical Quest owner witness')
            for owner in owners:
                quest = data.get('quest', {})
                canonical = cached_read(root, owner['path'], cache)
                definition_pointer = owner['json_pointer'].split('/source_data/')[0]
                if pointer(canonical, definition_pointer)['identity']['key'] != key or pointer(canonical, owner['json_pointer']) != owner['mission']:
                    raise ValueError('mission write canonical owner path differs')
                if 'source_quest_identity' in owner and owner['source_quest_identity'] != quest.get('identity'):
                    raise ValueError('mission write Source Quest identity differs')
                missions = [mission for mission in quest.get('missions', [])
                            if mission['key'] == owner['mission_key']]
                if missions != [owner['mission']] or digest(missions[0]) != owner['mission_sha256']:
                    raise ValueError('mission write canonical mission witness differs')
            if value['status'] != scope or value['native_admission'] or value['runtime_enabled']:
                raise ValueError('mission write cannot promote runtime')
            refs.append(reference(root, progress_path, f'/records/{n}', value, 'progress',
                                  value['status'], value['source_target'], cache))
    path = DATA + 'refinements/joins.json'
    packet = verified_output(root, path, DATA + 'refinements/summary.json', DATA + 'refinements/', cache)
    for n, value in enumerate(packet['records']):
        if key in value['quest_keys']:
            if not any(p['quest_key'] == key for p in value['proofs']):
                raise ValueError('component association has no Quest proof')
            refs.append(reference(root, path, f'/records/{n}', value, 'component',
                                  'SOURCE_ASSOCIATION_DISPATCH_UNPROVEN', value['source_component_id'], cache))
            for lane in ('boss', 'events', 'other'):
                component_path = DATA + f'components248/{lane}.json.gz'
                definitions = verified_output(root, component_path, DATA + 'components248/summary.json',
                                              DATA + 'components248/', cache)['records']
                match = [(i, record) for i, record in enumerate(definitions)
                         if record['source_component_id'] == value['source_component_id']]
                for i, record in match:
                    if record['provenance'] != value['provenance']:
                        raise ValueError('component definition provenance differs')
                    refs.append(reference(root, component_path, f'/records/{i}', record, 'component',
                                          'SOURCE_COMPONENT_DEFINITION_EXECUTION_UNPROVEN', value['source_component_id'], cache))
    path = DATA + 'refinements/dialogue.json.gz'
    packet = verified_output(root, path, DATA + 'refinements/summary.json', DATA + 'refinements/', cache)
    for n, value in enumerate(packet['quest_dialogue_links']):
        if value['quest']['key'] != key or value['join_status'] != 'EXACT_AST_STORAGE_WRITE':
            continue
        tracks = [p for p in data.get('progress', []) if p['key'] == value['progress_key']]
        occurrences = [o for p in tracks for t in p['transitions'] if t['key'] == value['transition_key']
                       for i, o in enumerate(t.get('source_occurrences', [])) if i == value['source_occurrence_index']]
        if value['quest'] != definition['identity'] or occurrences != [value['occurrence']]:
            raise ValueError('dialogue link belongs to another Quest progress occurrence')
        refs.append(reference(root, path, f'/quest_dialogue_links/{n}', value, 'npc_dialogue',
                              'QUEST_DIALOGUE_SOURCE_LINK_EXECUTION_UNPROVEN', value['progress_key'], cache))
    return refs


def attach(root, records):
    result = copy.deepcopy(records)
    schema = read(Path(__file__).with_name('quest_donor_attachment.schema.json'))
    validator = jsonschema.Draft202012Validator(schema)
    cache = {}
    for row in result:
        definition = row['definition']
        if 'oteryn_recipe' not in definition:
            continue
        refs = semantic_refs(root, definition.get('source_data', {}), cache)
        refs.extend(refinement_refs(root, definition, cache))
        if not refs:
            continue
        supplement = {'profile': 'qualified_donor_source_refs_v1',
                      'quest_key': definition['identity']['key'], 'refs': refs,
                      'original_holds_preserved': True, 'native_admission': False,
                      'whole_quest_complete': False}
        validator.validate(supplement)
        definition['oteryn_recipe']['donor_source_data'] = supplement
    return result
