"""Prepare bounded source-backed missing Creature dependencies, preserving unresolved mechanics.

Outputs are donor evidence, not authority to admit content. Original population bundles
are immutable. Grand Mother Foulscale's plural reference is corrected only in a copy,
using the exact singular registered type and her pinned summon spell as evidence.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path

import canary_batch as cb
import crystal_batch
import validate_monster as vm
import spell_scripts

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]
BUNDLE_FILES = ('monster.json', 'dependencies.json', 'catalog.json', 'manifest.json')
SAMPLE = ROOT / 'samples/reference-creature-completion-20261002.json'
TARGETS = (
    ('bone_bear', 'crystal', 'winter_update_2025/bone_bear'),
    ('parasite', 'canary', 'vermins/parasite'),
    ('carnisylvan_sapling', 'canary', 'humans/carnisylvan_sapling'),
    ('eruption_of_destruction', 'canary', 'quests/ferumbras_ascension/traps/eruption_of_destruction'),
    ('wormling', 'canary', 'quests/feaster_of_souls/wormling'),
    ('the_baron_from_below', 'canary', 'quests/dangerous_depth/bosses/the_baron_from_below'),
)
OPEN = {'unresolved_semantics', 'unresolved_dependency', 'unsupported_source_field', 'partial_text'}


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def dump(value):
    return json.dumps(value, ensure_ascii=False, indent=2) + '\n'


def digest(directory):
    result = hashlib.sha256()
    for name in BUNDLE_FILES:
        data = (directory / name).read_bytes()
        result.update(f'{name}\0{len(data)}\0'.encode('ascii') + data)
    return result.hexdigest()


def canonical_reference_correction(monster, dependencies, catalog):
    """Bounded correction of one proven source typo; never define a plural alias."""
    result = copy.deepcopy((monster, dependencies, catalog))
    if monster['creature']['identity']['key'] != 'canary:creature/grand_mother_foulscale':
        raise ValueError('correction is restricted to Grand Mother Foulscale')
    old, new = 'canary:creature/dragon_hatchlings', 'canary:creature/dragon_hatchling'
    count = 0

    def correct(value):
        nonlocal count
        if isinstance(value, dict):
            if value.get('family') == 'Creature' and value.get('key') == old:
                value['key'] = new
                count += 1
            for child in value.values():
                correct(child)
        elif isinstance(value, list):
            for child in value:
                correct(child)

    for value in result:
        correct(value)
    if count == 0:
        raise ValueError('expected exact plural Creature reference is absent')
    return (*result, {'original_reference': old, 'corrected_reference': new,
                      'changed_typed_references': count,
                      'source_conflict': 'Donor monster table uses plural; donor summon spell creates exact singular.',
                      'evidence_sample': str(SAMPLE.relative_to(REPO)),
                      'global_alias_created': False})


def prepare(canary, crystal, out):
    out = out.resolve()
    if out == REPO or REPO in out.parents:
        raise ValueError('generated bundles must be outside the repository')
    if out.exists() and any(out.iterdir()):
        raise ValueError('output must be empty; no overwriting qualified donor bundles')
    for root in (canary, crystal):
        if out == root.resolve() or root.resolve() in out.parents:
            raise ValueError('output must not modify source checkouts')
    sample = json.loads(SAMPLE.read_text())
    expected = {(slug, source, relative) for slug, source, relative in TARGETS}
    if expected != {(r['slug'], r['source'], r['relative']) for r in sample['creatures']}:
        raise ValueError('source evidence allow-list does not match bounded preparation')
    crystal_batch.require_revision(canary, crystal)
    crystal_batch.require_pinned(canary, 'Canary', cb.REVISION,
        ('data/items/items.xml', 'data/items/appearances.dat', 'data/scripts/lib/register_monster_type.lua'), extra=False)
    source_paths = [f'{crystal_batch.MONSTER_ROOT}/{rel}.lua' for _, source, rel in TARGETS if source == 'crystal']
    crystal_batch.require_pinned(crystal, 'CrystalServer', crystal_batch.REVISION, source_paths, extra=False)
    converters = {'crystal': crystal_batch.converter(canary, crystal)}
    objects = cb.load_appearance_objects(canary / 'data/items/appearances.dat')
    items = cb.load_items_xml(canary / 'data/items/items.xml')
    names, index = cb.name_index(objects, items)
    converters['canary'] = cb.Converter(canary, objects, items, names, index)
    out.mkdir(parents=True, exist_ok=True)
    report = {'schema': 'OTERYN_REFERENCE_CREATURE_PREPARATION/v1',
              'classification': 'OTS_HYPOTHESIS_ONLY', 'native_admission_authorized': False,
              'runtime_qualified': False, 'evidence_sample_sha256': sha256(SAMPLE), 'creatures': [], 'index_monsters': [],
              'canonical_reference_corrections': sample['canonical_reference_corrections']}
    for slug, source, relative in TARGETS:
        conv = converters[source]
        path = conv.monster_root / conv.monster_dir / (relative + '.lua')
        proof = next(r for r in sample['creatures'] if r['slug'] == slug)
        if sha256(path) != proof['sha256'] or cb.blob_id(path.read_bytes()) != proof['blob_sha1']:
            raise ValueError('donor source changed: ' + slug)
        conv.pending_definitions = set()
        actual, monster, deps, catalog, manifest, _ = conv.convert(relative)
        if actual != slug:
            raise ValueError('registered donor name differs from dependency identity')
        local = {item['identity']['key'] for item in deps['items']}
        pending = sorted(conv.pending_definitions)
        for family, key in pending:
            reference = cb.ref(family, key)
            if reference not in catalog['definitions'] and key != monster['creature']['identity']['key'] and key not in local:
                catalog['definitions'].append(reference)
        structure = vm.validate(monster, deps, catalog, None)
        if structure:
            raise ValueError(f'{slug} structure invalid: {structure}')
        target = out / slug
        target.mkdir()
        for name, value in zip(BUNDLE_FILES, (monster, deps, catalog, manifest)):
            (target / name).write_text(dump(value), encoding='utf-8', newline='\n')
        readiness = vm.validate(monster, deps, catalog, manifest)
        index_row = {'monster': slug, 'file': relative, 'sha256': digest(target)}
        if source == 'crystal':
            index_row['file'] = f'{crystal_batch.MONSTER_ROOT}/{relative}'
            index_row['binding'] = {'source_key': 'oteryn:source.crystalserver',
                'source_revision': crystal_batch.REVISION, 'identity_namespace': 'crystalserver/monster-file',
                'external_id': f'{crystal_batch.MONSTER_ROOT}/{relative}.lua'}
        report['index_monsters'].append(index_row)
        report['creatures'].append({'monster': slug, 'bundle_directory': slug, 'digest': digest(target),
            'source': proof, 'structure_errors': structure, 'readiness_errors': readiness,
            'status': 'source_schema_valid' if not readiness else 'source_mechanics_pending',
            'open_rows': [row for row in manifest['entries'] if row['status'] in OPEN],
            'stats': monster['creature']['stats'], 'loot_entry_count': len(monster.get('loot', {}).get('entries', [])),
            'pending_definition_references': [cb.ref(family, key) for family, key in pending]})
    (out / 'reference-creatures.json').write_text(dump(report), encoding='utf-8', newline='\n')
    return report



PENDING_MECHANICS = {
    'parasite': 'events=ParasiteDeath',
    'carnisylvan_sapling': 'attacks[1]',
    'eruption_of_destruction': 'attacks[2]',
    'wormling': 'events=WormlingDeath',
    'the_baron_from_below': 'events=TheBaronFromBelowThink',
}
SAPLING_BODY = 'addEvent(removeSapling, 1, creature.uid) return combat:execute(creature, var)'


def prepare_playable(canary, crystal, out):
    """Explicit user-accepted approximations, retaining source originals and omission ledger."""
    out = out.resolve()
    if out.exists() and any(out.iterdir()):
        raise ValueError('output must be empty')
    if out == REPO or REPO in out.parents:
        raise ValueError('playable preparation must stay outside the repository')
    source_report = prepare(canary, crystal, out / 'source-bundles')
    sample = json.loads(SAMPLE.read_text())
    script = canary / 'data-otservbr-global/scripts/spells/monster/sapling_explode.lua'
    if sha256(script) != sample['sapling_partial_source']['sha256']:
        raise ValueError('sapling partial cast source changed')
    objects = cb.load_appearance_objects(canary / 'data/items/appearances.dat')
    items = cb.load_items_xml(canary / 'data/items/items.xml')
    names, index = cb.name_index(objects, items)
    conv = cb.Converter(canary, objects, items, names, index)
    conv.spell_scripts = spell_scripts.SpellScripts(canary, accepted_guards={'sapling explode': {SAPLING_BODY}})
    report = copy.deepcopy(source_report)
    report['classification'] = 'SOURCE_BACKED_FLAGGED_PLAYABLE_APPROXIMATION'
    report['source_report_sha256'] = sha256(out / 'source-bundles/reference-creatures.json')
    report['index_monsters'] = []
    target_root = out / 'bundles'
    target_root.mkdir()
    for entry in report['creatures']:
        slug = entry['monster']
        original = out / 'source-bundles' / slug
        monster, deps, catalog, manifest = [json.loads((original / name).read_text()) for name in BUNDLE_FILES]
        strict_manifest = copy.deepcopy(manifest)
        original_stats, original_loot = copy.deepcopy(monster['creature']['stats']), copy.deepcopy(monster.get('loot'))
        omitted = []
        if slug == 'carnisylvan_sapling':
            conv.pending_definitions = set()
            _, monster, deps, catalog, manifest, _ = conv.convert('humans/carnisylvan_sapling')
            if not monster['behavior']['attacks']:
                raise ValueError('source sapling fire combat was not preserved')
            row = next(r for r in strict_manifest['entries'] if r['source_field'] == 'attacks[1]')
            omitted.append({'source_row': row, 'omitted_component': 'Delayed caster removal after 1ms',
                            'preserved_component': 'Exact fire combat area, presentation, magnitude and source cast schedule'})
            manifest['entries'].append({**row, 'source_field': 'attacks[1].delayed_self_remove',
                'status': 'approved_omission', 'resolution': 'FLAGGED_PLAYABLE_APPROXIMATION: caster removal after1ms omitted; exact fire combat preserved.'})
        else:
            for row in manifest['entries']:
                if row['status'] in OPEN:
                    if row['source_field'] != PENDING_MECHANICS.get(slug) or row['status'] != 'unresolved_semantics':
                        raise ValueError('unapproved new pending mechanic: ' + slug)
                    omitted.append({'source_row': copy.deepcopy(row), 'omitted_component': row['resolution']})
                    row['status'] = 'approved_omission'
                    row['resolution'] = 'FLAGGED_PLAYABLE_APPROXIMATION: source mechanic omitted; exact original row retained in preparation ledger. ' + row['resolution']
        if monster['creature']['stats'] != original_stats or monster.get('loot') != original_loot:
            raise ValueError('approximation changed source stats or loot')
        errors = vm.validate(monster, deps, catalog, manifest)
        if errors:
            raise ValueError(f'{slug} flagged playable structure/readiness invalid: {errors}')
        target = target_root / slug
        target.mkdir()
        for name, value in zip(BUNDLE_FILES, (monster, deps, catalog, manifest)):
            (target / name).write_text(dump(value), encoding='utf-8', newline='\n')
        entry['original_bundle_digest'] = entry['digest']
        entry['digest'] = digest(target)
        entry['status'] = 'flagged_playable_schema_valid' if omitted else 'source_schema_valid'
        entry['readiness_errors'] = []
        entry['omitted_source_mechanics'] = omitted
        entry['source_stats_and_loot_unchanged'] = True
        entry['quality_flags'] = ['OTS_HYPOTHESIS_ONLY', 'GAMEPLAY_UNVERIFIED']
        if omitted:
            entry['quality_flags'].append('SOURCE_MECHANICS_OMITTED')
        if 'mitigation_percent' not in monster['creature']['stats']:
            entry['quality_flags'].append('MITIGATION_UNKNOWN')
        index_row = copy.deepcopy(next(r for r in source_report['index_monsters'] if r['monster'] == slug))
        index_row['sha256'] = entry['digest']
        report['index_monsters'].append(index_row)
    (out / 'reference-creatures.json').write_text(dump(report), encoding='utf-8', newline='\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    for name in ('canary', 'crystal', 'out'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--playable', action='store_true', help='explicit flagged approximations, retaining source originals')
    args = parser.parse_args()
    producer = prepare_playable if args.playable else prepare
    report = producer(args.canary, args.crystal, args.out)
    print(json.dumps({'prepared': len(report['creatures']),
                      'readiness_valid': sum(not r['readiness_errors'] for r in report['creatures']),
                      'native_admission_authorized': False}))


if __name__ == '__main__':
    main()
