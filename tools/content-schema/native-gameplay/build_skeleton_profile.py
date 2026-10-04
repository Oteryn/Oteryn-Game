#!/usr/bin/env python3
"""Export the exact formal Skeleton source profile used by Animate Dead.

Byte identity against current Canary is evidence; actual formal47 DefinitionRefs
remain unchanged. This source candidate neither activates content nor claims AI.
"""
from __future__ import annotations
import argparse, hashlib, io, json, tarfile, tempfile
from pathlib import Path
import build_familiar_profiles as formal

PATH = 'data-otservbr-global/monster/undeads/skeleton.lua'


def build(source: Path, output: Path):
    if formal.git(source, 'rev-parse', 'HEAD').decode().strip() != formal.CURRENT:
        raise ValueError('current Canary source HEAD mismatch')
    unchanged = formal.byte_identity(source, PATH)
    shared = ['data/items/items.xml', 'data/items/appearances.dat', 'src/utils/utils_definitions.hpp',
              'src/creatures/creatures_definitions.hpp', 'data/scripts/lib/register_monster_type.lua',
              'src/creatures/monsters/monsters.hpp', 'src/creatures/monsters/monsters.cpp']
    shared_proof = [{'path': p, 'revision': formal.FORMAL,
                     'sha256': hashlib.sha256(formal.git(source, 'show', f'{formal.FORMAL}:{p}')).hexdigest()}
                    for p in shared]
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='oteryn-skeleton-formal-') as tmp:
        pinned = Path(tmp)
        archive = formal.git(source, 'archive', formal.FORMAL,
                             'data/scripts', 'data-otservbr-global/scripts', PATH, *shared)
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
            tar.extractall(pinned, filter='data')
        objects = formal.canary_batch.load_appearance_objects(pinned/'data/items/appearances.dat')
        items = formal.canary_batch.load_items_xml(pinned/'data/items/items.xml')
        names, index = formal.canary_batch.name_index(objects, items)
        converter = formal.canary_batch.Converter(pinned, objects, items, names, index)
        slug, monster, deps, catalog, manifest, origin = converter.convert('undeads/skeleton')
        if slug != 'skeleton' or monster['creature']['identity'] != {
                'key': 'canary:creature/skeleton', 'revision': 'canary-47dfd51f'}:
            raise ValueError('Skeleton exact registered source identity mismatch')
        issues = formal.validate_monster.validate(monster, deps, catalog, None)
        if issues:
            raise ValueError('; '.join(issues))
        unresolved = [row for row in manifest['entries'] if row['status'].startswith('unresolved')]
        if unresolved:
            raise ValueError(f'Skeleton source conversion unresolved: {unresolved}')
        mapper = formal.SourceIdentityMapper({})
        converted = formal.stage.Stage(mapper)
        creature = formal.stage.profile(mapper.identity('Creature', monster['creature']['identity']),
                                        'Creature', converted.creature_profile(monster))
        behavior = formal.stage.profile(mapper.identity('Behavior', monster['behavior']['identity']),
                                        'Behavior', converted.behavior_profile(monster['behavior']))
        presentation = formal.stage.profile(mapper.identity('Presentation', monster['presentation']['identity']),
                                            'Presentation', converted.presentation_profile(monster['presentation']))
        converted.stage_dependencies(deps, slug)
        documents = {
            'skeleton-creature-profiles.json': {'schema': 'OTERYN_NATIVE_CREATURE_PROFILES/v1', 'records': [
                {'profile': creature, 'presentation': dict(monster['creature']['presentation']), 'behavior': behavior}]},
            'skeleton-presentation-profiles.json': {'schema': 'OTERYN_NATIVE_PRESENTATION_PROFILES/v1', 'records': [presentation]},
            'skeleton-dependency-profiles.json': {'schema': 'OTERYN_SKELETON_SOURCE_DEPENDENCY_PROFILES/v1',
                'records': list(converted.records.values()), 'authoring_profiles': list(converted.profiles.values())},
        }
        bundle = output/'formal-bundles'/'skeleton'
        bundle.mkdir(parents=True, exist_ok=True)
        for name, value in [('monster.json', monster), ('dependencies.json', deps),
                            ('catalog.json', catalog), ('manifest.json', manifest)]:
            (bundle/name).write_text(json.dumps(value, ensure_ascii=False, indent=2)+'\n')
        hashes = {}
        for name, value in documents.items():
            raw = (json.dumps(value, ensure_ascii=False, indent=2)+'\n').encode()
            (output/name).write_bytes(raw)
            hashes[name] = hashlib.sha256(raw).hexdigest()
        proof = {'schema': 'OTERYN_SKELETON_PROFILE_SOURCE_PROOF/v1',
                 'definition_revision': formal.FORMAL, 'current_revision': formal.CURRENT,
                 'unchanged_full_definitions': [unchanged], 'formal_shared_sources': shared_proof,
                 'source': origin, 'bundle_sha256': formal.stage.bundle_digest(bundle), 'outputs': hashes,
                 'converter_sources': [{'path': str(p.relative_to(formal.ROOT)),
                     'sha256': hashlib.sha256(p.read_bytes()).hexdigest()} for p in
                     [Path(__file__), formal.MONSTERS/'canary_batch.py',
                      formal.ROOT/'tools/content-migration/creature_admission_stage.py']],
                 'scope': 'Complete exact formal Skeleton profiles/dependencies; source qualification only, no activation or full AI claim.'}
        (output/'skeleton-profile-source-proof.json').write_text(json.dumps(proof, indent=2)+'\n')
        return proof

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canary', type=Path, default=Path('/workspace/spell-sources/canary'))
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    proof = build(args.canary, args.out)
    print(json.dumps({'outputs': proof['outputs'], 'unchanged_full_definitions': proof['unchanged_full_definitions']}))
