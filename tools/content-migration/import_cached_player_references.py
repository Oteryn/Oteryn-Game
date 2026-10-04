#!/usr/bin/env python3
"""Import cached player header observations without replacing source or candidate authority."""
import argparse
from collections import Counter
import gzip
import json
from pathlib import Path
import re

from jsonschema import Draft202012Validator

from import_source_spell_package import ROOT, bundle_member_path, digest, require
from source_spell_import_guards import read_base, write_source_only_set

BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
PACKAGE_SHA = '7a0c45fd8010878561a3b1e64ac1a998c39e669d1d34d538ee9b10829624a2c8'
OUTPUTS = {'cached-wiki-page-facts.jsonl.gz': ('pages', 'OTERYN_CACHED_WIKI_HEADER_FACT/v1'),
           'source-wiki-header-comparisons.jsonl.gz': ('comparisons', 'OTERYN_SOURCE_WIKI_HEADER_COMPARISON/v1'),
           'supplemental-header-facts.jsonl.gz': ('supplemental_facts', 'OTERYN_CACHED_WIKI_SUPPLEMENTAL_HEADER_FACT/v1')}
FILES = set(OUTPUTS) | {'coverage.json', 'cached-player-wiki.schema.json', 'targeted-research-gaps.json', 'targeted-research-gaps.schema.json'}
INPUTS = {'docs/reference/spells/r22-audit/tibiopedia/capture-manifest.json', 'docs/reference/spells/r22-audit/tibiopedia/tibiopedia-spell-facts-2026-10-02.json',
          'docs/reference/spells/r22-audit/tibiopedia/unmatched-direct-lookup.json', 'tools/content-schema/spell-authoring/samples/wiki-spell-facts-br-2026-09-27.json',
          'tools/content-schema/spell-authoring/samples/wiki-spell-facts-fandom-2026-09-27.json', 'imports/spells/r28/player-source-bundles/source-callback-facts.jsonl.gz',
          *(f'imports/spells/r{r}/import-manifest.json' for r in (28, 29, 30, 34))}


def key(value): return re.sub(r'\s+', ' ', value.strip()).casefold()


def verify_source_joins(groups, headers, callbacks, statuses, overlays):
    pages = {r['page_key']: r for r in groups['pages']}; comparisons = {r['registration_key']: r for r in groups['comparisons']}
    require(len(pages) == len(groups['pages']) and len(comparisons) == len(groups['comparisons']) and set(comparisons) == set(callbacks), 'PLAYER_REFERENCE_SOURCE_POPULATION_MISMATCH')
    for reg, row in comparisons.items():
        body, header = headers[reg]; callback = callbacks[reg]
        require(row['source_header_sha256'] == digest(body) and row['source_sha256'] == callback['source_sha256'] and row['source_revision'] == callback['source_revision']
                and row['name'] == header['name'] and row['carrier'] == header.get('carrier', header.get('source_carrier_symbol'))
                and row['source_rune_item_id'] == header.get('reference_rune_item_id')
                and row['candidate_status'] == statuses[reg] and row['candidate_overlays'] == overlays.get(reg, [])
                and row['runtime_activation'] is False and row['source_values_overwritten'] is False and row['source_execution_gaps_remain'] is True, 'PLAYER_REFERENCE_FROZEN_SOURCE_JOIN_MISMATCH')
        carrier = header.get('carrier', header.get('source_carrier_symbol', ''))
        expected_context = 'disabled_symbolic' if carrier.startswith('@') else 'rune_use' if carrier == 'rune' else 'rune_creation' if callback['source_callback_facts']['cast'].get('conjure', {}).get('reagent_item_id') == 3147 else 'spell_cast'
        require(row['context'] == expected_context, 'PLAYER_REFERENCE_SOURCE_CONTEXT_MISMATCH')
        matched = row['matched_page_keys']; proof_rows = row['matching_evidence']
        require([p['page_key'] for p in proof_rows] == matched and all(p in pages and pages[p]['context'] == row['context'] for p in matched), 'PLAYER_REFERENCE_PAGE_CONTEXT_JOIN_MISMATCH')
        anchors = [p for p in pages.values() if row['context'] == 'rune_use' and p['context'] == 'rune_use' and row['source_rune_item_id'] is not None and p['reference_item_id'] == row['source_rune_item_id']]
        for proof in proof_rows:
            page = pages[proof['page_key']]; basis = proof['basis']; anchor_keys = proof['identity_anchor_page_keys']
            if basis == 'direct_recorded_name': valid = key(header['name']) in page['recorded_identity_names'] and not anchor_keys
            elif basis == 'exact_instant_words':
                words = next(f['normalized'] for f in page['facts'] if f['field'] == 'words')
                valid = row['context'] != 'rune_use' and bool(header.get('words')) and words is not None and words['value'] == key(header['words']) and not anchor_keys
            elif basis == 'source_rune_item_id': valid = page in anchors and anchor_keys == [page['page_key']]
            else:
                expected = [a['page_key'] for a in anchors if set(a['recorded_identity_names']) & set(page['recorded_identity_names'])]
                valid = row['context'] == 'rune_use' and bool(expected) and anchor_keys == expected
            require(valid, 'PLAYER_REFERENCE_IDENTITY_ANCHOR_MISMATCH')
        for field in row['fields']:
            require([e['page_key'] for e in field['evidence']] == matched and all(e['page_key'] in pages for e in field['evidence']), 'PLAYER_REFERENCE_FIELD_EVIDENCE_JOIN_MISMATCH')
        require(row['reference_coverage'] == [{'reference': ref, 'matching_page_count': sum(pages[p]['reference'] == ref for p in matched)} for ref in ('br', 'fandom', 'tibiopedia')], 'PLAYER_REFERENCE_COVERAGE_JOIN_MISMATCH')
    expected_supplements = {}
    for reg, row in comparisons.items():
        for field in row['fields']:
            if field['status'] == 'SOURCE_FIELD_ABSENT': expected_supplements[(reg, field['field'])] = [e for e in field['evidence'] if e['value'] is not None]
    actual_supplements = {(r['registration_key'], r['field']): r for r in groups['supplemental_facts']}
    require(len(actual_supplements) == len(groups['supplemental_facts']) and set(actual_supplements) == set(expected_supplements), 'PLAYER_REFERENCE_SUPPLEMENTAL_COHORT_MISMATCH')
    for identity, row in actual_supplements.items():
        require(row['evidence'] == expected_supplements[identity] and row['source_field_present'] is False and row['source_values_overwritten'] is False and row['runtime_activation'] is False, 'PLAYER_REFERENCE_SUPPLEMENTAL_OVERRIDE_MISMATCH')


def prepare_import(root, source=None):
    source = source or root / 'docs/reference/spells/r40-source-enrichment/cached-player-wiki'
    base, base_body, baseline, schemas, _ = read_base(root, BASE_SHA)
    manifest_body = bundle_member_path(source, 'package-manifest.json').read_bytes(); package_manifest = json.loads(manifest_body)
    require(digest(manifest_body) == PACKAGE_SHA and set(package_manifest['files']) == FILES, 'PLAYER_REFERENCE_QUALIFIED_PACKAGE_MISMATCH')
    packet = {'package-manifest.json': manifest_body}
    require({p.relative_to(source).as_posix() for p in source.rglob('*') if p.is_file()} == FILES | {'package-manifest.json'}, 'PLAYER_REFERENCE_PACKAGE_MEMBERSHIP_MISMATCH')
    for name, expected in package_manifest['files'].items():
        body = bundle_member_path(source, name).read_bytes(); require(digest(body) == expected, 'PLAYER_REFERENCE_PACKAGE_FILE_HASH_MISMATCH'); packet[name] = body
    coverage = json.loads(packet['coverage.json']); require(set(coverage['input_proofs']) == INPUTS, 'PLAYER_REFERENCE_INPUT_COHORT_MISMATCH')
    for path, expected in coverage['input_proofs'].items(): require(digest(bundle_member_path(root, path).read_bytes()) == expected, 'PLAYER_REFERENCE_INPUT_HASH_MISMATCH')
    require(coverage['runtime_activation'] is False and coverage['source_values_overwritten'] is False and coverage['network_fetched_this_run'] is False and coverage['source_execution_gaps_remain'] is True, 'PLAYER_REFERENCE_COVERAGE_OVERRIDE_MISMATCH')
    validators = Draft202012Validator(json.loads(packet['cached-player-wiki.schema.json'])); groups = {}
    for name, (group, schema_name) in OUTPUTS.items():
        body = packet[name]; payload = gzip.decompress(body); expected = coverage['outputs'][group]
        require(digest(body) == expected['gzip_sha256'] and digest(payload) == expected['payload_sha256'], 'PLAYER_REFERENCE_PAYLOAD_PIN_MISMATCH')
        rows = [json.loads(line) for line in payload.splitlines()]; require(len(rows) == expected['records'], 'PLAYER_REFERENCE_ROW_COUNT_MISMATCH')
        for row in rows: validators.validate(row); require(row['schema'] == schema_name, 'PLAYER_REFERENCE_RECORD_FAMILY_MISMATCH')
        groups[group] = rows
    gaps = json.loads(packet['targeted-research-gaps.json']); Draft202012Validator(json.loads(packet['targeted-research-gaps.schema.json'])).validate(gaps)
    callbacks = {r['registration_key']: r for r in map(json.loads, gzip.decompress((base / 'player-source-bundles/source-callback-facts.jsonl.gz').read_bytes()).splitlines())}
    headers = {}; statuses = {}; overlays = {}
    for reg in callbacks:
        folder = base / 'player-source-bundles' / reg.split('/')[0] / digest(reg.encode())[:16]
        body = (folder / 'source-header.json').read_bytes(); headers[reg] = body, json.loads(body)['spell']; statuses[reg] = json.loads((folder / 'receipt.json').read_bytes())['status']
    for revision in (29, 30, 34):
        for reg in json.loads((root / f'imports/spells/r{revision}/import-manifest.json').read_bytes())['source_keys']:
            statuses[reg] = 'CANDIDATE_SCHEMA_VALID'; overlays.setdefault(reg, []).append('r' + str(revision))
    verify_source_joins(groups, headers, callbacks, statuses, overlays)
    for page in groups['pages']:
        require(page['cached_input_path'] in coverage['input_proofs'] and page['cached_input_sha256'] == coverage['input_proofs'][page['cached_input_path']] and page['network_fetched_this_run'] is False, 'PLAYER_REFERENCE_PAGE_CACHE_PROVENANCE_MISMATCH')
    require(all(row['registration_key'] in callbacks for row in gaps['records']), 'PLAYER_REFERENCE_GAP_SOURCE_IDENTITY_MISMATCH')
    comparisons = groups['comparisons']; matched = sum(bool(r['matched_page_keys']) for r in comparisons); candidates = sum(r['candidate_status'] == 'CANDIDATE_SCHEMA_VALID' for r in comparisons)
    counts = {'source_records': len(comparisons), 'wiki_page_fact_records': len(groups['pages']), 'page_records_by_reference': dict(Counter(r['reference'] for r in groups['pages'])),
              'field_comparisons': sum(len(r['fields']) for r in comparisons), 'status_counts': dict(Counter(f['status'] for r in comparisons for f in r['fields'])),
              'source_records_with_context_compatible_page': matched, 'source_records_without_context_compatible_page': len(comparisons) - matched,
              'supplemental_source_absent_fields': len(groups['supplemental_facts']), 'candidate_records_unchanged': candidates, 'full_spell_candidates': 0, 'targeted_research_gap_records': len(gaps['records'])}
    require(all(counts[k] == coverage[k] for k in counts if k not in ('full_spell_candidates', 'targeted_research_gap_records')), 'PLAYER_REFERENCE_AGGREGATE_CONSERVATION_MISMATCH')
    extra_refs = [{'uri': 'evidence/' + name, 'path': 'evidence/' + name, 'sha256': digest(packet[name])} for name in ('cached-player-wiki.schema.json', 'targeted-research-gaps.schema.json')]
    source_path = source.relative_to(root).as_posix(); data = {**schemas, 'base-r28/import-manifest.json': base_body, **{'evidence/' + n: b for n, b in packet.items()}}
    def origin(path): return source_path + '/' + path.split('/', 1)[1] if path.startswith('evidence/') else 'imports/spells/r28/import-manifest.json' if path.startswith('base-r28/') else 'imports/spells/r28/' + path
    manifest = {'schema': 'OTERYN_CACHED_PLAYER_REFERENCE_IMPORT/v1', 'revision': 40, 'admission_status': 'source_only_not_active',
                'runtime_activation': False, 'source_override': False, 'source_values_overwritten': False, 'source_execution_gaps_remain': True,
                'native_identity_allocation': False, 'canonical_selection_changed': False, 'native_execution_qualified': False, 'execution_qualified': False,
                'input_provider_equivalence': False, 'full_spell_candidates': 0, 'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': BASE_SHA, 'snapshot': 'base-r28/import-manifest.json'},
                'source_pins': baseline['source_pins'], 'reference_inputs': coverage['input_proofs'], 'counts': counts, 'source_metadata_path': 'evidence/coverage.json',
                'schemaRefs': baseline['schemaRefs'] + extra_refs,
                'artifacts': [{'path': p, 'sourcePath': origin(p), 'sha256': digest(b), 'bytes': len(b), 'role': 'supplemental_reference_evidence' if p.startswith('evidence/') else 'reference_schema' if p.startswith('schemas/') else 'base_manifest',
                               'schemaRefs': ['evidence/cached-player-wiki.schema.json'] if p.endswith('.jsonl.gz') else ['evidence/targeted-research-gaps.schema.json'] if p == 'evidence/targeted-research-gaps.json' else []} for p, b in sorted(data.items())],
                'limits': coverage['limitations'] + ['Source candidates remain scoped to the sealed 308-candidate baseline; external supplementation admits no new Spell candidate.']}
    return data, manifest


def write_import(root, destination, data, manifest):
    require(manifest['revision'] == 40 and manifest['source_override'] is False and manifest['source_values_overwritten'] is False and manifest['full_spell_candidates'] == 0, 'PLAYER_REFERENCE_OVERRIDE_CLAIM_REFUSED')
    return write_source_only_set(root, destination, 'imports/spells/r40', data, manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument('--source', type=Path); args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.source); print(json.dumps(write_import(ROOT, ROOT / 'imports/spells/r40', data, manifest), indent=2))


if __name__ == '__main__': main()
