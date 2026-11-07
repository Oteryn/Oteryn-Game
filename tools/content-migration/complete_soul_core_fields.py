#!/usr/bin/env python3
"""Prepare source-backed soul-core field patches; never add conditional cores to loot."""
from __future__ import annotations
import argparse
import hashlib
import importlib.util
import json
import re
from pathlib import Path

REPOSITORIES = {'canary': 'opentibiabr/canary', 'crystal': 'zimbadev/crystalserver'}
FLAGS = ['SOURCE_SUPPORTED_CORE_BINDING', 'SOUL_CORE_GLOBAL_PARITY_UNVERIFIED',
         'SOUL_CORE_RUNTIME_DROP_UNVERIFIED']

def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

def selected_project(manifest: dict) -> str:
    for source in manifest['sources']:
        for project, repository in REPOSITORIES.items():
            if source.get('repository') == repository:
                return project
    raise ValueError('No pinned monster donor; cannot choose a conflicting Item identity')

def choose_item(matches: dict, primary: str) -> tuple[str, dict, str]:
    if primary in matches:
        return primary, matches[primary], 'SELECTED_MONSTER_DONOR_ITEM'
    if len({item['id'] for item in matches.values()}) != 1:
        raise ValueError('Cross-source Item conflict without selected donor match')
    project = sorted(matches)[0]
    return project, matches[project], 'EXACT_NAME_PINNED_SECONDARY_ITEM_SUPPLEMENT'

def item_proof(project: str, match: dict, name: str, sources: Path, pins: dict) -> dict:
    path = sources / project / 'data/items/items.xml'
    lines = path.read_text().splitlines()
    line = match['line']
    if not 1 <= line <= len(lines):
        raise ValueError('Item line outside pinned XML')
    text = lines[line - 1]
    actual = re.search(r'<item\s+id="(\d+)"[^>]*name="([^"]+)"', text)
    if (actual is None or int(actual[1]) != match['id']
            or actual[2].casefold() != (name + ' soul core').casefold()):
        raise ValueError('Soul-core Item ID/name/line proof does not match')
    return {'repository': REPOSITORIES[project], 'revision': pins[project],
            'source_file': 'data/items/items.xml', 'source_line': line,
            'source_file_sha256': sha(path), 'source_item_id': match['id'],
            'source_item_name': actual[2], 'source_line_text': text.strip(),
            'wiki_confirmation': 'NOT_ASSERTED', 'global_parity': 'UNVERIFIED'}

def protected_items(repository: Path, export_path: Path) -> tuple[dict[int, str], dict]:
    spec = importlib.util.spec_from_file_location('soul_core_stage', repository / 'tools/content-migration/creature_admission_stage.py')
    stage = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(stage)
    export = json.loads(export_path.read_text())
    if export['allocation_digest_sha256'] != stage.ITEM_ALLOCATION_SHA256:
        raise ValueError('Protected allocation digest drift')
    mapping = {row['source_item_id']: row['native_key'] for row in export['records']}
    for path in stage.ITEM_REKEYS:
        row = json.loads(path.read_text())['source_identity']
        if mapping.get(row['source_item_id']) != row['current_native_key']:
            raise ValueError('Protected Item rekey drift')
        mapping[row['source_item_id']] = row['target_native_key']
    appearance_index, appearances = stage.load_admitted()
    current_ids = {entry[0] for entry in appearances[appearance_index['newest']]['entries']}
    mapping = stage.resolve_admitted_item_map(mapping,
        json.loads(stage.REFERENCE.read_text())['records'],
        json.loads(stage.ITEM_ALIASES.read_text())['entries'],
        json.loads(stage.ITEM_BINDINGS.read_text())['bindings'], current_ids)
    return mapping, {'item_map_sha256': sha(export_path),
        'allocation_digest_sha256': export['allocation_digest_sha256'],
        'reference_sha256': sha(stage.REFERENCE), 'aliases_sha256': sha(stage.ITEM_ALIASES),
        'bindings_sha256': sha(stage.ITEM_BINDINGS), 'resolved_item_count': len(mapping)}

def prepare(population: Path, audit_path: Path, sources: Path,
            mapping: dict[int, str], allocation_proof: dict) -> dict:
    index_path = population / 'population-index.json'
    index = json.loads(index_path.read_text())
    audit = json.loads(audit_path.read_text())
    if sha(index_path) != audit['index_sha256']:
        raise ValueError('Audit/population baseline SHA mismatch')
    pins = {'canary': index['source']['revision'], 'crystal': index['crystal']['revision']}
    if pins != audit['source_pins']:
        raise ValueError('Audit source pins mismatch')
    for project in REPOSITORIES:
        expected = [proof for proof in audit['source_proofs']
            if proof['project'] == project and proof['path'] == 'data/items/items.xml']
        if len(expected) != 1 or expected[0]['revision'] != pins[project]:
            raise ValueError('Pinned Item XML source proof missing or ambiguous')
        if sha(sources / project / 'data/items/items.xml') != expected[0]['sha256']:
            raise ValueError('Pinned Item XML source SHA drift')
    actors = {row['monster'] for row in index['monsters']}
    patches, unresolved, flags = [], [], {}
    seen = set()
    for candidate in audit['soul_core_mapping_candidates']:
        slug = candidate['monster']
        if slug not in actors or slug in seen:
            raise ValueError('Unknown or duplicate soul-core candidate')
        seen.add(slug)
        directory = population / 'bundles' / slug
        monster = json.loads((directory / 'monster.json').read_text())
        creature = monster['creature']
        if 'soul_core_item' in creature:
            raise ValueError('Candidate already has soul_core_item')
        if candidate['display_name'] != creature['display_name']:
            raise ValueError('Candidate identity/name drift')
        manifest = json.loads((directory / 'manifest.json').read_text())
        primary = selected_project(manifest)
        donor = next(source for source in manifest['sources']
            if source.get('repository') == REPOSITORIES[primary])
        if donor.get('revision') != pins[primary]:
            raise ValueError('Monster donor source revision drift')
        try:
            project, match, selection = choose_item(candidate['matching_soul_core_items'], primary)
            proof = item_proof(project, match, creature['display_name'], sources, pins)
        except ValueError as error:
            unresolved.append({'monster': slug, 'reason': str(error)})
            continue
        source_id = match['id']
        if source_id not in mapping:
            unresolved.append({'monster': slug, 'source': proof,
                'reason': 'ITEM_NOT_IN_PROTECTED_ADMITTED_RESOLVED_MAP',
                'requires': 'Item owner admission; no invented Item reference'})
            continue
        ref = {'family': 'Item', 'key': f'canary:item/{source_id}',
               'revision': creature['identity']['revision']}
        proof.update({'selection': selection, 'selected_monster_donor': primary,
            'native_item': {'family': 'Item', 'key': mapping[source_id], 'revision': 'definition-r1'},
            'condition': 'Fiendish monster loot callback, configured probability and same/category choice; not an unconditional ordinary loot entry'})
        reason = 'Source-supported exact-name soul-core binding; Global parity and runtime conditional drop unverified.'
        patches.append({'monster': slug, 'file': 'monster.json',
            'pointer': '/creature/soul_core_item', 'expected_present': False,
            'expected_value': None, 'value': ref, 'source': proof, 'reason': reason})
        catalog = json.loads((directory / 'catalog.json').read_text())
        if ref not in catalog['definitions']:
            patches.append({'monster': slug, 'file': 'catalog.json', 'pointer': '/definitions/-',
                'expected_present': False, 'expected_value': None, 'value': ref,
                'source': proof, 'reason': 'Declare the protected source Item reference for authoring validation.'})
        flags[slug] = FLAGS.copy()
    return {'schema': 'OTERYN_MONSTER_FIELD_PATCH_PACKET/v1', 'lane': 'soul-core',
        'baseline_index_sha256': sha(index_path), 'source_pins': pins,
        'audit_sha256': sha(audit_path), 'protected_item_proof': allocation_proof,
        'patches': patches, 'actor_flags': flags, 'unresolved': unresolved,
        'counts': {'audited_candidates': len(seen), 'patched_actors': len(flags),
            'patch_rows': len(patches), 'unresolved_actors': len(unresolved)},
        'runtime_qualification': {'native_stage_field_supported': True,
            'native_contract_item_ref_supported': True,
            'conditional_soul_core_drop_consumer_implemented': False,
            'proof': ['tools/content-migration/creature_admission_stage.py:418',
                'apps/game-server/src/content/project/v2/creature.rs:44,917'],
            'owner_contract_required': 'Fiendish/category/probability SoulPit drop dispatch; metadata import does not implement drop'}}

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['repository', 'population', 'audit', 'sources', 'item-map', 'output']:
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    mapping, proof = protected_items(args.repository, args.item_map)
    packet = prepare(args.population, args.audit, args.sources, mapping, proof)
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / 'field-patches.json').write_text(json.dumps(packet, indent=2) + '\n')
    (args.output / 'summary.json').write_text(json.dumps({key: packet[key] for key in
        ['counts', 'baseline_index_sha256', 'source_pins', 'protected_item_proof', 'runtime_qualification']}, indent=2) + '\n')
    print(json.dumps(packet['counts']))

if __name__ == '__main__':
    main()
