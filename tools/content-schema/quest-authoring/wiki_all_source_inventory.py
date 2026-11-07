"""Portable all-title SOURCE inventory; preserves prior detailed facts unchanged."""
import argparse
import copy
import hashlib
import json
from collections import Counter
from pathlib import Path

import jsonschema

HERE = Path(__file__).parent
FILES = ('previous105.json', 'selection373.json', 'legacy-source-schema.json',
         'catalogue-source-schema.json', 'access-checks.json')
METRICS = ('level', 'reward_entity_references', 'requirement_entity_references',
           'location_entity_references', 'mission_heading_references', 'quest_entity_references')
SUMMARY = ('titles', 'preserved_detailed_titles', 'new_structured_titles', 'preserved_source_revisions',
    'new_structured_source_revisions', 'new_prerequisite_expressions', 'new_unparsed_requirement_holds',
    'new_mission_heading_references', 'new_reward_entity_references', 'new_requirement_entity_references',
    'new_location_entity_references', 'new_titles_with_rewards', 'new_titles_with_requirements',
    'new_titles_with_locations', 'new_titles_with_mission_headings', 'new_full_body_captures')


def read(path):
    return json.loads(path.read_text())


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=False,
                                    separators=(',', ':')).encode()).hexdigest()


def file_sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def object_schema(properties, required=None):
    return dict(type='object', properties=properties, required=list(required or properties), additionalProperties=False)


def schema(samples):
    defs = {}
    for filename, prefix in (('legacy-source-schema.json', 'legacy_'),
                             ('catalogue-source-schema.json', 'fresh_')):
        imported = json.loads(json.dumps(read(samples / filename)['$defs']).replace('#/$defs/', '#/$defs/' + prefix))
        defs.update({prefix + name: value for name, value in imported.items()})
    ref = lambda name: {'$ref': '#/$defs/' + name}
    string, number = {'type': 'string', 'minLength': 1}, {'type': 'integer', 'minimum': 0}
    false, unknown = {'const': False}, {'const': 'UNKNOWN'}
    array = lambda item: {'type': 'array', 'items': item}
    evidence = object_schema({**defs['legacy_source_ref']['properties'], 'line': {'type': 'integer', 'minimum': 1},
                             'line_sha256': defs['fresh_hash']})
    hold = object_schema({'field_group': string, 'classification': unknown, 'reason': string, 'evidence': evidence},
                        ('field_group', 'classification', 'reason'))
    spec = object_schema({'provider': defs['fresh_freshSource']['properties']['provider'],
                          'relation': defs['fresh_freshSource']['properties']['relation'],
                          'semantic_enrichment': ref('fresh_semanticEnrichment')})
    new_entry = object_schema({'wiki_title': string, 'source_refs': array(ref('legacy_source_ref')),
        'source_specification': array(spec), 'source_fieldsets': array(ref('fresh_freshSource')),
        'migration_status': {'const': 'PARTIAL_SOURCE_SPECIFICATION'}, 'definition_complete': false,
        'runtime_readiness': unknown, 'unresolved': array(hold), 'available_model_projection': string,
        'source_field_listing_complete': false, 'source_capture_complete_for_selected_revisions': false})
    count_map = object_schema({name: number for name in METRICS})
    coverage = object_schema({'wiki_title': string,
        'proof_level': {'enum': ['RECORDED_PRIOR_BODY_FACTS', 'PINNED_STRUCTURED_FIELDS_ONLY']},
        'source_fieldsets': number, 'known_field_counts': count_map, 'source_binding_records': number,
        'mission_labels_are_execution_steps': false, 'full_walkthrough_complete': false,
        'raw_body_rechecked': false, 'provider_conflict_resolution': {'const': 'NOT_ATTEMPTED'},
        'source_approximation_status': {'const': 'REFERENCE_FACTS_ONLY'}, 'oteryn_behavior_selected': false,
        'runtime_readiness': unknown})
    properties = {'schema': {'const': 'OTERYN_ALL_WIKI_SOURCE_SPECIFICATIONS/v1'}, 'titles': number,
        'entries': array({'oneOf': [ref('legacy_entry'), new_entry]}), 'coverage': array(coverage),
        'summary': object_schema({key: number for key in SUMMARY}),
        'source_definition_complete': false, 'runtime_readiness': unknown, 'no_runtime_promotion': {'const': True},
        'baseline_inventory_sha256': defs['fresh_hash'],
        'input_provenance': array(object_schema({'path': string, 'sha256': defs['fresh_hash']})),
        'inventory_scope': object_schema({'repository': string, 'revision': {'type': 'string', 'pattern': '^[0-9a-f]{40}$'},
            'catalogue_path': string, 'catalogue_sha256': defs['fresh_hash'],
            'selection': {'const': 'all_titles_at_pinned_catalogue'}, 'preserved_prior_entry_digests':
            {'type': 'object', 'additionalProperties': defs['fresh_hash']},
            'selection_does_not_require_absence_of_current_binding': {'const': True}}),
        'source_access_checks': {'const': read(samples / 'access-checks.json')}}
    return {'$schema': 'https://json-schema.org/draft/2020-12/schema',
            '$id': 'oteryn:source-schema/all-wiki-specifications/v1', '$defs': defs, **object_schema(properties)}


def validate_sources(sources, title):
    seen = set()
    for source in sources:
        key = tuple(source.get(field) for field in ('provider', 'relation', 'pageid', 'revid'))
        if key in seen or not source.get('available') or not source.get('revid'):
            raise ValueError('Unavailable/duplicate/ambiguous source: ' + title)
        seen.add(key)
        semantic = source['source_fields']['semantic_enrichment']
        values = {k: v for k, v in semantic.items() if k != 'semantic_sha256'}
        if semantic['source_revision_sha256'] != source['content_sha256'] or digest(values) != semantic['semantic_sha256']:
            raise ValueError('Stale semantic source digest: ' + title)
        if source['source_fields']['listing_complete'] or semantic['runtime_promotion']:
            raise ValueError('False source completeness/runtime authority')
        if any(r['entity'].startswith(('Image:', 'File:')) for r in semantic['reward_entity_references']):
            raise ValueError('Media reference cannot be an authored reward')
    if not seen:
        raise ValueError('Missing source fieldsets: ' + title)


def added_entry(row):
    sources = copy.deepcopy(row['fresh_sources'])
    refs = [{k: s[k] for k in ('provider', 'pageid', 'revid', 'content_sha256', 'url', 'relation')} for s in sources]
    holds = [{'field_group': field, 'classification': 'UNKNOWN', 'reason': reason} for field, reason in (
        ('Complete walkthrough and conditional step ordering', 'Full public bodies were not cached for this added title; headings are references only.'),
        ('NPC identity and dialogue branches', 'Structured field capture does not separately prove all NPC identities or dialogue branches.'),
        ('Canonical reward and prerequisite bindings', 'Wiki entity references are not item, outfit, mount or achievement identity admission.'),
        ('Execution and Quest Log policy', 'Source requirements and source-scoped log fields do not choose Oteryn execution or publication policy.'))]
    for source, evidence in zip(sources, refs):
        for hold in source['source_fields']['semantic_enrichment']['unparsed_requirements']:
            holds.append({'field_group': 'Source requirement not parsed', 'classification': 'UNKNOWN',
                          'reason': hold['reason'], 'evidence': {**evidence, 'line': hold['line'], 'line_sha256': hold['line_sha256']}})
    return {'wiki_title': row['wiki_title'], 'source_refs': refs,
        'source_specification': [{'provider': s['provider'], 'relation': s['relation'],
            'semantic_enrichment': copy.deepcopy(s['source_fields']['semantic_enrichment'])} for s in sources],
        'source_fieldsets': sources, 'migration_status': 'PARTIAL_SOURCE_SPECIFICATION',
        'definition_complete': False, 'runtime_readiness': 'UNKNOWN', 'unresolved': holds,
        'available_model_projection': 'Pinned source requirements, rewards, locations and mission labels; complete walkthrough and executable journey UNKNOWN.',
        'source_field_listing_complete': False, 'source_capture_complete_for_selected_revisions': False}


def build(selection, previous, access, provenance):
    rows = selection['quests']; titles = [row['wiki_title'] for row in rows]
    if len(set(titles)) != len(titles) or titles != selection['catalogue_title_inventory']:
        raise ValueError('Duplicate or incomplete pinned all-title selection')
    prior = {entry['wiki_title']: entry for entry in previous['entries']}
    if len(prior) != len(previous['entries']) or not set(prior) <= set(titles):
        raise ValueError('Duplicate/extraneous prior detailed specification')
    entries, coverage, counts = [], [], Counter(titles=len(rows), preserved_detailed_titles=len(prior),
        new_structured_titles=len(rows)-len(prior), preserved_source_revisions=sum(len(e['source_refs']) for e in prior.values()),
        new_full_body_captures=0)
    for row in rows:
        title = row['wiki_title']; sources = row['fresh_sources']; validate_sources(sources, title)
        if title in prior:
            entry = copy.deepcopy(prior[title])
            if entry['definition_complete'] or entry['runtime_readiness'] != 'UNKNOWN':
                raise ValueError('Prior inventory cannot assert runtime completeness')
            expected = [{k: s[k] for k in ('provider', 'pageid', 'revid', 'content_sha256', 'url', 'relation')} for s in sources]
            if sorted(entry['source_refs'], key=digest) != sorted(expected, key=digest):
                raise ValueError('Prior detailed source pin differs from selection: ' + title)
        else:
            entry = added_entry(row)
            counts['new_structured_source_revisions'] += len(sources)
            for field in ('prerequisite_expressions', 'unparsed_requirements'):
                metric = 'new_prerequisite_expressions' if field == 'prerequisite_expressions' else 'new_unparsed_requirement_holds'
                counts[metric] += sum(len(s['source_fields']['semantic_enrichment'][field]) for s in sources)
            for field in METRICS[1:4] + ('mission_heading_references',):
                counts['new_' + field] += sum(len(s['source_fields'][field]) for s in sources)
            for field, label in (('reward_entity_references', 'rewards'), ('requirement_entity_references', 'requirements'),
                                 ('location_entity_references', 'locations'), ('mission_heading_references', 'mission_headings')):
                counts['new_titles_with_' + label] += any(s['source_fields'][field] for s in sources)
        entries.append(entry)
        coverage.append({'wiki_title': title, 'proof_level': 'RECORDED_PRIOR_BODY_FACTS' if title in prior else 'PINNED_STRUCTURED_FIELDS_ONLY',
            'source_fieldsets': len(sources), 'known_field_counts': {name: sum(bool(s['source_fields'].get(name)) for s in sources) for name in METRICS},
            'source_binding_records': len(row['authored_candidates']) + bool(row['family_representation']),
            'mission_labels_are_execution_steps': False, 'full_walkthrough_complete': False, 'raw_body_rechecked': False,
            'provider_conflict_resolution': 'NOT_ATTEMPTED', 'source_approximation_status': 'REFERENCE_FACTS_ONLY',
            'oteryn_behavior_selected': False, 'runtime_readiness': 'UNKNOWN'})
    return {'schema': 'OTERYN_ALL_WIKI_SOURCE_SPECIFICATIONS/v1', 'titles': len(entries), 'entries': entries,
        'coverage': coverage, 'summary': {key: counts[key] for key in SUMMARY}, 'source_definition_complete': False, 'runtime_readiness': 'UNKNOWN',
        'no_runtime_promotion': True, 'baseline_inventory_sha256': digest(previous), 'input_provenance': provenance,
        'inventory_scope': {k: selection[k] for k in ('repository', 'revision', 'catalogue_path', 'catalogue_sha256')} |
            {'selection': 'all_titles_at_pinned_catalogue', 'preserved_prior_entry_digests': {k: digest(v) for k, v in prior.items()},
             'selection_does_not_require_absence_of_current_binding': True}, 'source_access_checks': copy.deepcopy(access)}


def generate(samples):
    manifest = read(samples / 'inventory-inputs.json')
    if set(manifest['inputs']) != set(FILES):
        raise ValueError('Incomplete portable input manifest')
    for filename, expected in manifest['inputs'].items():
        if file_sha(samples / filename) != expected:
            raise ValueError('Pinned input digest differs: ' + filename)
    result = build(read(samples / 'selection373.json'), read(samples / 'previous105.json'), read(samples / 'access-checks.json'),
                   [{'path': name, 'sha256': manifest['inputs'][name]} for name in FILES])
    jsonschema.Draft202012Validator(schema(samples)).validate(result)
    return result


def validate(value, samples):
    jsonschema.Draft202012Validator(schema(samples)).validate(value)
    if value != generate(samples):
        raise ValueError('SOURCE dataset differs from exact portable regeneration')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--samples', type=Path, default=HERE / 'samples/wiki-source-all373')
    parser.add_argument('--schema-out', type=Path)
    parser.add_argument('--validate', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    if args.schema_out:
        content = json.dumps(schema(args.samples), indent=2) + '\n'
        if args.check:
            if args.schema_out.read_text() != content:
                raise SystemExit('Portable all-title SOURCE schema differs')
        else:
            args.schema_out.write_text(content)
    elif args.validate:
        validate(read(args.validate), args.samples)
    else:
        value = generate(args.samples); content = json.dumps(value, ensure_ascii=False, indent=2) + '\n'
        target = args.samples / 'source-specs-373.json'
        if args.check:
            if target.read_text() != content:
                raise SystemExit('Portable all-title SOURCE inventory differs')
        else:
            target.write_text(content)
        print(json.dumps(value['summary']))
