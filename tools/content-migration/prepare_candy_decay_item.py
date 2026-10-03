#!/usr/bin/env python3
"""Source-proven Candy corpse correction: keep corpse, omit broken no-op timer.

This prepares an explicitly flagged approximation, never a new Item identity.
Canonical registry, bindings and source bundles are not modified.
"""
from __future__ import annotations
import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import sys

import verify_spyrat_corpse_source_boundaries as source
import prepare_canonical_corpse_item_candidates as item_xml

SCHEMA = 'OTERYN_CANDY_DECAY_COMPLETION/v1'
CORPSE = 'canary:item/48267'
MISSING = 'canary:item/48296'


def prove_sources(checkouts: Path, repo: Path) -> dict:
    donors = {}
    for label, (revision, _) in source.SOURCES.items():
        checkout = checkouts / label
        paths = ['data/items/items.xml', 'data/items/appearances.dat',
                 'src/game/game.cpp', 'src/items/decay/decay.cpp']
        data = {p: source.pinned(checkout, revision, p) for p in paths}
        table = item_xml.parse_items(data[paths[0]])
        objects = set()
        for family, wire, raw in source.fields(data[paths[1]]):
            if family == 1 and wire == 2:
                objects.update(value for number, _, value in source.fields(raw) if number == 1)
        if len(table.get(48267, [])) != 1 or 48296 in table or 48267 not in objects or 48296 in objects:
            raise ValueError('Donor source identity/absence differs from correction precondition')
        row = table[48267][0]
        attrs = {k: v[0]['value'] for k, v in row['fields'].items() if len(v) == 1}
        if attrs.get('decayto') != '48296' or attrs.get('duration') != '5':
            raise ValueError('Donor corpse decay differs')
        transform = source.function_slice(data[paths[2]].decode(), 'std::shared_ptr<Item> Game::transformItem(')
        decay = source.function_slice(data[paths[3]].decode(), 'void Decay::internalDecayItem(')
        if not re.search(r'if \(newType.id == 0\) \{\s*return item;', transform['text']):
            raise ValueError('Absent-target transform is no longer a no-op')
        if 'g_game().transformItem(item, static_cast<uint16_t>(it.decayTo));' not in decay['text']:
            raise ValueError('Donor decay dispatch differs')
        donors[label] = {'revision': revision, 'access': 'local_pinned_git_source_read',
                         'files': {p: source.sha(raw) for p, raw in data.items()},
                         'corpse_xml': row, 'appearance_48267_present': True,
                         'appearance_48296_present': False, 'xml_48296_present': False,
                         'missing_target_guard': transform, 'decay_dispatch': decay}
    membership = []
    for path in sorted((repo / 'imports/official/appearance-membership').glob('appearances-*.json')):
        document = json.loads(path.read_text())
        identifiers = {row[0] for row in document['entries']}
        if 48296 in identifiers:
            raise ValueError('48296 now has admitted appearance evidence; reassess instead of approximating')
        membership.append({'path': str(path.relative_to(repo)), 'sha256': source.sha(path.read_bytes()),
                           '48296_present': False, '48267_present': 48267 in identifiers})
    bindings_path = repo / 'imports/crystalserver/bindings/items.json'
    bindings = json.loads(bindings_path.read_text())['bindings']
    if any(row['external_id'] == '48296' for row in bindings):
        raise ValueError('48296 now has an accepted binding; reassess')
    corpse_bindings = [r for r in bindings if r['external_id'] == '48267']
    if not corpse_bindings:
        raise ValueError('Corpse lacks exact accepted binding')
    return {'schema': SCHEMA, 'sources': donors, 'appearance_membership': membership,
            'corpse_exact_bindings': corpse_bindings,
            'bindings_sha256': source.sha(bindings_path.read_bytes()),
            'new_item_identity_authorized': False,
            'flags': ['APPROXIMATION_SOURCE_NOOP_DECAY_TIMER_OMITTED'],
            'behavior': 'Retain corpse48267; no decay. Donor attempted missing-target transform after5s retains original corpse.',
            'difference': 'The five-second no-op timer and its observable duration/decaying state are omitted.',
            'wiki_needed_for_this_correction': False, 'live_gameplay_verified': False}


def correct_bundle(bundle: dict, proof: dict) -> dict:
    """Return corrected documents; caller owns output placement and digest refresh."""
    if proof.get('schema') != SCHEMA or proof.get('new_item_identity_authorized') is not False:
        raise ValueError('Invalid source proof')
    if set(proof.get('sources', {})) != {'canary', 'crystal'} or not proof.get('corpse_exact_bindings'):
        raise ValueError('Incomplete source proof')
    result = copy.deepcopy(bundle)
    creature = result['monster']['creature']
    if creature['identity']['key'] != 'canary:creature/candy_horror' or creature['corpse_item']['key'] != CORPSE:
        raise ValueError('Unexpected Creature/corpse identity')
    items = result['dependencies']['items']
    corpses = [row for row in items if row['identity']['key'] == CORPSE]
    if len(corpses) != 1 or corpses[0]['temporal'].get('decay_target', {}).get('key') != MISSING:
        raise ValueError('Unexpected dependency decay contract')
    if corpses[0]['temporal'].get('duration_ms') != 5000:
        raise ValueError('Unexpected no-op duration')
    corpses[0]['temporal'] = {'decay_action': 'none', 'stop_duration': False}
    result['dependencies']['items'] = [r for r in items if r['identity']['key'] != MISSING]
    catalog = result['catalog']
    catalog['assets'] = [r for r in catalog['assets'] if r != 'canary.appearance:object/48296']
    catalog['definitions'] = [r for r in catalog['definitions'] if r['key'] != MISSING]
    for entry in result['manifest']['entries']:
        if entry.get('destination') == '/monster/creature/corpse_item':
            entry['resolution'] += ' Correction: both pinned donors lack Item/appearance48296; missing-target transform retains corpse48267. Flagged approximation omits the5s no-op timer, preserves corpse, stats and loot.'
    # Reject any other use; cannot silently drop an unrelated loot/effect dependency.
    if MISSING in json.dumps(result):
        raise ValueError('Missing Item is referenced outside the bounded decay correction')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--sources', type=Path, required=True)
    parser.add_argument('--bundle', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    proof = prove_sources(args.sources, args.repo)
    bundle = {p.stem: json.loads(p.read_text()) for p in args.bundle.glob('*.json')}
    corrected = correct_bundle(bundle, proof)
    args.out.mkdir(parents=True, exist_ok=False)
    for name, document in corrected.items():
        (args.out / (name + '.json')).write_text(json.dumps(document, indent=2) + '\n')
    (args.out / 'correction-proof.json').write_text(json.dumps(proof, indent=2) + '\n')


if __name__ == '__main__':
    main()
