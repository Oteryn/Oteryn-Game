"""Offline SOURCE reconstruction; public body verification is not performed here."""
import argparse
import copy
import hashlib
import json
from collections import Counter
from pathlib import Path


def digest(value):
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def file_sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    return json.loads(path.read_text())


def identity(source):
    return tuple(source[field] for field in ('provider', 'relation', 'pageid', 'revid'))


def union(previous, additions):
    result = copy.deepcopy(previous); seen = {digest(value) for value in result}
    for value in additions:
        marker = digest(value)
        if marker not in seen:
            result.append(copy.deepcopy(value)); seen.add(marker)
    return result


def reconstruct(baseline, supplements, receipt, selection):
    if receipt.get('qualification') != 'RAW_BODIES_VERIFIED_IN_PRIOR_ACQUISITION_ONLY':
        raise ValueError('Missing recorded acquisition qualification')
    if digest(baseline) != receipt.get('baseline_inventory_sha256') or digest(selection) != receipt.get('selection_receipt_sha256'):
        raise ValueError('Baseline/selection differs from recorded acquisition anchor')
    original = {entry['wiki_title']: entry for entry in baseline['entries']}
    records = {(row['wiki_title'], identity(row)): row for row in receipt['source_records']}
    if len(original) != len(baseline['entries']) or len(records) != len(receipt['source_records']):
        raise ValueError('Ambiguous baseline/acquisition identity')
    if set(selection['selected_titles']) != set(original) or len(selection['selected_titles']) != len(original):
        raise ValueError('Pinned selection differs from SOURCE inventory')
    selected = {}
    for package in supplements:
        if package.get('schema') != 'OTERYN_BOUNDED_WIKI_FACT_SUPPLEMENT/v1' or package.get('runtime_promotion') is not False:
            raise ValueError('Unsupported authored supplement or runtime promotion')
        for entry in package['entries']:
            if entry['wiki_title'] in selected or entry['wiki_title'] not in original:
                raise ValueError('Duplicate/extraneous title')
            selected[entry['wiki_title']] = entry
    if set(selected) != set(original):
        raise ValueError('Incomplete selected inventory')
    entries, seen_records, counts = [], set(), Counter()
    for old in baseline['entries']:
        entry = copy.deepcopy(old); title = old['wiki_title']
        if old['definition_complete'] is not False or old['runtime_readiness'] != 'UNKNOWN':
            raise ValueError('SOURCE inventory cannot assert native readiness')
        entry['baseline_entry_sha256'] = digest(old)
        refs = {identity(ref): ref for ref in old['source_refs']}
        sources, seen = [], set()
        for source in selected[title]['sources']:
            key = identity(source); record_key = title, key
            if key in seen or key not in refs or record_key not in records:
                raise ValueError('Missing/duplicate/ambiguous acquisition source')
            if any(source.get(field) != value for field, value in refs[key].items()):
                raise ValueError('Source pin differs from baseline')
            record = records[record_key]
            if any(record.get(field) != value for field, value in refs[key].items()) or digest(source) != record['source_fact_sha256']:
                raise ValueError('Authored facts differ from recorded acquisition digest')
            if source.get('source_field_listing_complete') is not False or source.get('runtime_promotion') is not False:
                raise ValueError('False completeness/runtime authority')
            if source.get('source_revision_verified') is not False or source.get('historical_acquisition_verified') is not True:
                raise ValueError('Offline regeneration cannot reverify public body hashes')
            sources.append(copy.deepcopy(source))
            semantic = next(value['semantic_enrichment'] for value in entry['source_specification']
                            if value['provider'] == source['provider'] and value['relation'] == source['relation'])
            for field, additions in source['semantic_enrichment_delta'].items():
                if isinstance(additions, list):
                    semantic[field] = union(semantic.get(field, []), additions)
            semantic.pop('semantic_sha256', None); semantic['semantic_sha256'] = digest(semantic)
            for hold in source['semantic_enrichment_delta']['unparsed_requirements']:
                entry['unresolved'] = union(entry['unresolved'], [{'field_group': 'Source requirement not parsed',
                    'classification': 'UNKNOWN', 'reason': hold['reason'], 'evidence': {**refs[key],
                    'line': hold['line'], 'line_sha256': hold['line_sha256']}}])
            counts.update(source_revisions=1, **{field: len(source[field]) for field in
                ('source_spans', 'heading_witnesses', 'journal_field_witnesses', 'source_fact_entries')})
            counts['curated_source_fact_groups'] += sum(fact['kind'] == 'curated_source_fact' for fact in source['source_fact_entries'])
            counts['unparsed_requirements'] += len(source['semantic_enrichment_delta']['unparsed_requirements'])
            counts['provider:' + source['provider']] += 1
            seen.add(key); seen_records.add(record_key)
        if seen != set(refs):
            raise ValueError('Missing source for selected title')
        entry.update(source_fact_supplements=sources, source_field_listing_complete=False,
                     source_capture_complete_for_selected_revisions=True)
        entries.append(entry)
    if seen_records != set(records):
        raise ValueError('Extraneous acquisition record')
    counts['titles'] = len(entries)
    scope = {key: copy.deepcopy(value) for key, value in selection.items() if key not in ('schema', 'selected_titles')}
    scope.update(selected_title_count=len(entries), current_binding_observation=None)
    return {**copy.deepcopy(baseline), 'entries': entries, 'supplement_summary': dict(counts),
        'source_field_listing_complete': False, 'selected_source_capture_complete': True,
        'source_definition_complete': False, 'baseline_inventory_sha256': digest(baseline), 'inventory_scope': scope,
        'source_access': {'repositories': 'normal_github_and_local_pinned_read',
            'tibia_fandom': 'remote_desktop_browser_after_normal_http_402',
            'tibiawiki_br': 'remote_desktop_browser_after_normal_http_403', 'tibiopedia': 'UNAVAILABLE'},
        'offline_verification': {'mode': 'AUTHORED_FACTS_AND_RECORDED_ACQUISITION_ONLY',
            'raw_body_rechecked': False, 'line_spans_rechecked': False, 'source_membership_checked': True,
            'authored_fact_digests_checked': True, 'acquisition_receipt_sha256': digest(receipt),
            'frozen_acquisition_handoff_sha256': receipt['frozen_handoff_sha256']}}


def generate(samples, schema_path):
    from jsonschema import Draft202012Validator
    schema = read(schema_path); Draft202012Validator.check_schema(schema)
    def validate(value, definition):
        bound = {'$schema': schema['$schema'], '$defs': schema['$defs'], '$ref': '#/$defs/' + definition}
        error = next(Draft202012Validator(bound).iter_errors(value), None)
        if error:
            raise ValueError('SOURCE ' + definition + ': ' + error.message)
    manifest = read(samples / 'publication-inputs.json'); validate(manifest, 'input_manifest')
    for filename, expected in manifest['inputs'].items():
        if Path(filename).name != filename or file_sha(samples / filename) != expected:
            raise ValueError('Portable input file digest differs: ' + filename)
    paths = [samples / f'supplement-{number}.json' for number in range(1, 6)]
    packages = [read(path) for path in paths]
    for package in packages:
        validate(package, 'authored_supplement')
    receipt = read(samples / 'acquisition-receipt.json'); validate(receipt, 'acquisition_receipt')
    output = reconstruct(read(samples / 'source-specs-105.json'), packages, receipt, read(samples / 'selection-receipt.json'))
    output['supplement_inputs'] = [{'artifact': path.name, 'sha256': file_sha(path)} for path in paths]
    error = next(Draft202012Validator(schema).iter_errors(output), None)
    if error:
        raise ValueError('SOURCE schema at ' + '/'.join(map(str, error.absolute_path)) + ': ' + error.message)
    return output


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--samples', type=Path, default=Path(__file__).parent / 'samples/wiki-source-specs')
    parser.add_argument('--schema', type=Path, default=Path(__file__).with_name('wiki_source_specs.schema.json'))
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    output = args.samples / 'source-specs-105.enriched.json'
    content = json.dumps(generate(args.samples, args.schema), ensure_ascii=False, indent=2) + '\n'
    if args.check:
        if output.read_text() != content:
            raise SystemExit('Offline authored SOURCE reconstruction differs')
    else:
        output.write_text(content)
    print('Offline authored SOURCE reconstruction passed; raw bodies and spans NOT rechecked')
