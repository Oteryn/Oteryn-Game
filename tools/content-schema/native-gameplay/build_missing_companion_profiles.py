#!/usr/bin/env python3
"""Export six missing companion policies through the pinned formal converter.

The original formal DefinitionRefs, adopted reference captures, full bundles and
Stage dependency records are retained. Current Lua byte equality is evidence,
not an identity or revision rewrite. This producer does not activate content.
"""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import tarfile
import tempfile
from pathlib import Path

import build_familiar_profiles as formal
import official_library
import population_census as census

FILES = (
    'birds/tame_terror_bird',
    'mammals/white_tiger',
    'quests/rotten_blood/bloated_man-maggot',
    'quests/rotten_blood/rotten_man-maggot',
    'quests/rottin_wood_and_the_married_men_quest/travelling_merchant',
    'vermins/enpa_yolo',
)
SHARED = (
    'data/items/items.xml', 'data/items/appearances.dat',
    'src/utils/utils_definitions.hpp', 'src/creatures/creatures_definitions.hpp',
    'data/scripts/lib/register_monster_type.lua',
    'src/creatures/monsters/monsters.hpp', 'src/creatures/monsters/monsters.cpp',
)


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, indent=2) + '\n').encode()


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def build(source: Path, output: Path):
    if formal.git(source, 'rev-parse', 'HEAD').decode().strip() != formal.CURRENT:
        raise ValueError('Current Canary HEAD differs from the pinned source')
    index_raw = census.INDEX.read_bytes()
    indexed = {row['file']: row for row in json.loads(index_raw)['monsters']}
    paths = [f'data-otservbr-global/monster/{file}.lua' for file in FILES]
    proofs = [formal.byte_identity(source, path) for path in paths]
    shared = [{'path': path, 'revision': formal.FORMAL,
               'sha256': sha(formal.git(source, 'show', f'{formal.FORMAL}:{path}'))}
              for path in SHARED]
    captures = [('wiki', census.WIKI), ('br', census.BR_FILL),
                ('official', official_library.SAMPLE)]
    capture_proofs = []
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='oteryn-six-companions-formal-') as temporary:
        pinned = Path(temporary)
        archive = formal.git(source, 'archive', formal.FORMAL,
                             'data/scripts', 'data-otservbr-global/scripts', *paths, *SHARED)
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
            tar.extractall(pinned, filter='data')
        objects = formal.canary_batch.load_appearance_objects(pinned / 'data/items/appearances.dat')
        items = formal.canary_batch.load_items_xml(pinned / 'data/items/items.xml')
        names, index = formal.canary_batch.name_index(objects, items)
        converter = formal.canary_batch.Converter(pinned, objects, items, names, index)
        for attribute, path in captures:
            raw = path.read_bytes()
            setattr(converter, attribute, {row['monster']: row for row in json.loads(raw)['monsters']})
            capture_proofs.append({'policy': attribute, 'path': str(path.relative_to(formal.ROOT)),
                                   'sha256': sha(raw)})
        mapper = formal.SourceIdentityMapper({})
        converted = formal.stage.Stage(mapper)
        creatures, presentations, bundles, appearance_paths = [], [], [], {}
        for file, path, proof in zip(FILES, paths, proofs):
            if file not in indexed:
                raise ValueError(f'Source file absent from the formal population census: {file}')
            converter.pending_definitions = set()
            slug, monster, dependencies, catalog, manifest, origin = converter.convert(file)
            expected = indexed[file]
            identity = monster['creature']['identity']
            if slug != expected['monster'] or identity != {
                    'key': f'canary:creature/{slug}', 'revision': 'canary-47dfd51f'}:
                raise ValueError(f'Exact registered source identity differs: {file}')
            local = {row['identity']['key'] for row in dependencies['items']}
            for family, key in sorted(converter.pending_definitions):
                reference = formal.canary_batch.ref(family, key)
                if reference not in catalog['definitions'] and key != identity['key'] and key not in local:
                    catalog['definitions'].append(reference)
            issues = formal.validate_monster.validate(monster, dependencies, catalog, None)
            open_rows = [row for row in manifest['entries'] if row['status'] in census.OPEN]
            if issues or open_rows:
                raise ValueError(f'{file}: invalid or unresolved source conversion: {issues}; {open_rows}')
            registered, raw_monster, _ = formal.canary_batch.load_monster(pinned / path)
            initial, maximum = raw_monster['health'], raw_monster['maxHealth']
            if not isinstance(initial, int) or initial <= 0 or initial != maximum:
                raise ValueError(f'{file}: source initial health is not exactly maximum health')
            appearance = monster['presentation']['appearance']
            binding = appearance.get('asset_binding', '')
            if not binding.startswith('canary.appearance:outfit/'):
                raise ValueError(f'{file}: source object/unknown appearance cannot become an Outfit')
            summoning = monster['creature']['summoning']
            if any(not isinstance(summoning[key], bool) for key in ('summonable', 'convinceable')):
                raise ValueError(f'{file}: source eligibility flags are not closed booleans')
            if (summoning['summonable'] or summoning['convinceable']) and 'mana_cost' not in summoning:
                raise ValueError(f'{file}: eligible normalized policy lacks exact source mana cost')
            target = output / 'formal-bundles' / slug
            target.mkdir(parents=True, exist_ok=True)
            for name, value in zip(census.BUNDLE_FILES, (monster, dependencies, catalog, manifest)):
                (target / name).write_bytes(encoded(value))
            digest = formal.stage.bundle_digest(target)
            if digest != expected['sha256']:
                raise ValueError(f'{file}: regenerated formal bundle differs from its population census: {digest}')
            converted.stage_dependencies(dependencies, slug)
            converted.stage_monster(monster, file)
            creature = formal.stage.profile(mapper.identity('Creature', identity), 'Creature',
                                             converted.creature_profile(monster))
            behavior = formal.stage.profile(mapper.identity('Behavior', monster['behavior']['identity']),
                                             'Behavior', converted.behavior_profile(monster['behavior']))
            presentation = formal.stage.profile(mapper.identity('Presentation', monster['presentation']['identity']),
                                                 'Presentation', converted.presentation_profile(monster['presentation']))
            creatures.append({'profile': creature, 'presentation': dict(monster['creature']['presentation']),
                              'behavior': behavior})
            presentations.append(presentation)
            appearance_paths[identity['key']] = path
            proof.update({'source_initial_health': initial, 'source_maximum_health': maximum,
                          'registered_name': registered, 'canonical_target': creature['target'],
                          'raw_source_summoning': {key: raw_monster['flags'][key]
                              for key in ('summonable', 'convinceable')},
                          'raw_source_mana_cost': raw_monster['manaCost'],
                          'normalized_summoning': summoning,
                          'formal_bundle_sha256': digest, 'population_census_sha256_matched': True})
            bundles.append({'identity': creature['target'], 'source': origin, 'bundle_sha256': digest,
                            'source_initial_health': initial, 'source_maximum_health': maximum,
                            'normalized_initial_health': monster['creature']['stats']['initial_health'],
                            'normalized_maximum_health': creature['data']['profile']['health'],
                            'appearance_asset_binding': binding, 'summoning': summoning,
                            'loot_entry_count': len((monster.get('loot') or {}).get('entries', [])),
                            'unresolved_manifest_rows': []})
        documents = {
            'missing-companion-creature-profiles.json': {'schema': 'OTERYN_NATIVE_CREATURE_PROFILES/v1', 'records': creatures},
            'missing-companion-presentation-profiles.json': {'schema': 'OTERYN_NATIVE_PRESENTATION_PROFILES/v1', 'records': presentations},
            'missing-companion-dependency-profiles.json': {'schema': 'OTERYN_MISSING_COMPANION_SOURCE_DEPENDENCY_PROFILES/v1',
                'records': list(converted.records.values()), 'authoring_profiles': list(converted.profiles.values()),
                'exact_source_bindings': converted.bindings},
        }
        hashes = {}
        for name, document in documents.items():
            raw = encoded(document)
            (output / name).write_bytes(raw)
            hashes[name] = sha(raw)
        evidence = {'schema': 'OTERYN_MISSING_COMPANION_SOURCE_PROOF/v1',
                    'classification': 'Explicit local source-qualified candidate; no activation or AI execution claim',
                    'definition_revision': formal.FORMAL, 'current_revision': formal.CURRENT,
                    'formal_population_index_sha256': sha(index_raw), 'reference_captures': capture_proofs,
                    'formal_shared_sources': shared, 'unchanged_full_definitions': proofs,
                    'appearance_source_paths': appearance_paths, 'bundles': bundles, 'outputs': hashes,
                    'creature_count': len(creatures), 'presentation_count': len(presentations),
                    'normalized_eligible_count': sum(bool(row['summoning']['summonable'] or
                        row['summoning']['convinceable']) for row in bundles),
                    'source_record_count': len(converted.records), 'authoring_profile_count': len(converted.profiles),
                    'conversion_tools': [{'path': str(path.relative_to(formal.ROOT)), 'sha256': sha(path.read_bytes())}
                        for path in [Path(__file__), Path(formal.__file__), formal.MONSTERS / 'canary_batch.py',
                                     Path(census.__file__), formal.ROOT / 'tools/content-migration/creature_admission_stage.py']]}
        (output / 'missing-companion-source-proof.json').write_bytes(encoded(evidence))
        return evidence


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canary', type=Path, default=Path('/workspace/spell-sources/canary'))
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    proof = build(args.canary, args.out)
    print(json.dumps({key: proof[key] for key in ('creature_count', 'presentation_count', 'source_record_count', 'outputs')}))
