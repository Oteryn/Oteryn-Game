#!/usr/bin/env python3
"""Prepare a source-bound completion population without changing original bundles.

Explicit partial source behavior stays in the quality receipt. This prepares
content; it neither activates a world nor qualifies live gameplay.
"""
import argparse
import copy
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
MONSTERS = ROOT / 'tools/content-schema/monster-authoring'
ENCOUNTERS = ROOT / 'tools/content-schema/encounter-authoring'
sys.path[:0] = [str(MONSTERS), str(ENCOUNTERS)]
import creature_admission_stage as admission
import prepare_candy_decay_item as candy
import prepare_encounter_completion as encounter
import prepare_familiar_abilities as familiar
import prepare_reference_creatures as references
import validate_monster as validator

FILES = admission.BUNDLE_FILES


def read(path):
    return json.loads(Path(path).read_text())


def write(path, value):
    Path(path).write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')


def correct_seacrest_defense(documents):
    """Retain attacks/stats/loot; explicitly omit the known unsupported defense."""
    output = copy.deepcopy(documents)
    monster, deps, _, manifest = output
    key = 'canary:ability/seacrest_serpent/defense-2'
    if monster['creature']['identity']['key'] != 'canary:creature/seacrest_serpent':
        raise ValueError('defense correction has the wrong owner')
    abilities = [a for a in deps['abilities'] if a['identity']['key'] == key]
    actions = monster['behavior']['defenses']
    if len(abilities) != 1 or abilities[0]['kind'] != 'melee' or len(actions) != 2 or actions[1]['ability']['key'] != key:
        raise ValueError('Seacrest defense differs from the qualified failing action')
    original_action = actions.pop()
    entries = [e for e in manifest['entries'] if e.get('destination') == '/monster/behavior/defenses/1']
    if len(entries) != 1:
        raise ValueError('unsupported defense lacks its source provenance')
    entry = entries[0]
    original_entry = copy.deepcopy(entry)
    entry.pop('destination')
    entry.update(status='approved_omission', resolution='Flagged completion: native ProfileScheduleState rejects melee in defense schedules. Only this defensive action is omitted; attacks, stats, loot and original Ability definition are retained.')
    return output, {'flag': 'UNSUPPORTED_DEFENSIVE_MELEE_OMITTED', 'original_action': original_action,
                    'original_manifest_entry': original_entry, 'original_ability': abilities[0],
                    'live_gameplay_verified': False}


def install_bundle(source, destination, row):
    """Reject forged index rows before installing a supplemented copy."""
    slug = row['monster']
    if Path(slug).name != slug or slug in ('', '.', '..'):
        raise ValueError('invalid monster slug')
    if admission.bundle_digest(source) != row['sha256']:
        raise ValueError('supplemental bundle digest mismatch: ' + slug)
    documents = [read(source / f) for f in FILES]
    errors = validator.validate(*documents)
    if errors:
        raise ValueError(f'{slug}: invalid source completion: {errors}')
    target = destination / slug
    if target.is_symlink():
        target.unlink()
    elif target.exists():
        raise ValueError('conflicting completion writer: ' + slug)
    target.mkdir()
    for name in FILES:
        shutil.copyfile(source / name, target / name)


def merge_population(index, bundles, output, supplements):
    """Keep every baseline row; unchanged files share read-only input bytes."""
    rows = {}
    directory = output / 'bundles'
    directory.mkdir()
    replacement_names = {row['monster'] for _, row in supplements}
    for row in index['monsters']:
        slug = row['monster']
        if slug in rows or Path(slug).name != slug:
            raise ValueError('duplicate/invalid baseline identity')
        source = bundles / slug
        if admission.bundle_digest(source) != row['sha256']:
            raise ValueError('baseline bundle digest mismatch: ' + slug)
        rows[slug] = copy.deepcopy(row)
        if slug not in replacement_names:
            target = directory / slug
            target.mkdir()
            for filename in FILES:
                try:
                    os.link(source / filename, target / filename)
                except OSError:
                    shutil.copyfile(source / filename, target / filename)
    seen = set()
    for source, row in supplements:
        slug = row['monster']
        if slug in seen:
            raise ValueError('duplicate supplemental identity: ' + slug)
        install_bundle(source, directory, row)
        rows[slug] = copy.deepcopy(row)
        seen.add(slug)
    result = copy.deepcopy(index)
    result['monsters'] = [rows[k] for k in sorted(rows)]
    result['bundles'] = len(rows)
    return result


def prepare(canary, crystal, baseline_bundles, baseline_index, baseline_stage, output,
            components=None, encounter_components=None):
    output = output.resolve()
    protected = [ROOT, canary.resolve(), crystal.resolve(), baseline_bundles.resolve()]
    if any(output == p or p in output.parents for p in protected):
        raise ValueError('output would modify repository or source inputs')
    if output.exists() and any(output.iterdir()):
        raise ValueError('output must be new/empty')
    output.mkdir(parents=True, exist_ok=True)
    index = read(baseline_index)
    if components is None:
        components = output / 'components'
        components.mkdir()
        references.prepare_playable(canary, crystal, components / 'reference-playable')
        familiar.prepare(canary, crystal, baseline_bundles, components / 'familiars')
        encounter.prepare(canary, baseline_stage, baseline_index, components / 'encounter-bundles-v2')
    reference_root = components / 'reference-playable'
    encounter_root = encounter_components or components / 'encounter-bundles-v2'
    ref_report = read(reference_root / 'reference-creatures.json')
    enc_report = read(encounter_root / 'encounter-completion.json')
    if enc_report['baseline_index_sha256'] != encounter.sha(baseline_index) or enc_report['baseline_stage_sha256'] != encounter.sha(baseline_stage):
        raise ValueError('Encounter components belong to a different baseline')
    fam_report = read(components / 'familiars/completion-report.json')
    enc_names = {r['monster'] for r in enc_report['actors']}
    supplements = [(reference_root / 'bundles' / row['monster'], row)
                   for row in ref_report['index_monsters'] if row['monster'] not in enc_names]
    supplements.extend((encounter_root / 'creatures' / row['monster'],
                        {k: row[k] for k in ('monster', 'file', 'sha256')})
                       for row in enc_report['actors'])
    original_rows = {r['monster']: r for r in index['monsters']}
    for entry in fam_report['creatures']:
        slug = entry['slug']
        source = components / 'familiars' / slug
        supplements.append((source, {**original_rows[slug], 'sha256': admission.bundle_digest(source)}))
    corrections = output / 'corrections'
    corrections.mkdir()
    proof = candy.prove_sources(canary.parent, ROOT)
    original = {Path(f).stem: read(baseline_bundles / 'candy_horror' / f) for f in FILES}
    corrected = candy.correct_bundle(original, proof)
    target = corrections / 'candy_horror'
    target.mkdir()
    for name, value in corrected.items():
        write(target / (name + '.json'), value)
    supplements.append((target, {**original_rows['candy_horror'], 'sha256': admission.bundle_digest(target)}))
    slug = 'grand_mother_foulscale'
    values = [read(baseline_bundles / slug / f) for f in FILES]
    monster, deps, catalog, correction = references.canonical_reference_correction(*values[:3])
    for row in values[3]['entries']:
        if str(row.get('destination', '')).startswith('/monster/behavior/summons'):
            row['resolution'] += ' Source correction: both donor summon scripts create Dragon Hatchling singular; no plural alias is created.'
    target = corrections / slug
    target.mkdir()
    for filename, value in zip(FILES, [monster, deps, catalog, values[3]]):
        write(target / filename, value)
    supplements.append((target, {**original_rows[slug], 'sha256': admission.bundle_digest(target)}))
    slug = 'seacrest_serpent'
    values, defense_correction = correct_seacrest_defense([read(baseline_bundles / slug / f) for f in FILES])
    target = corrections / slug
    target.mkdir()
    for filename, value in zip(FILES, values):
        write(target / filename, value)
    supplements.append((target, {**original_rows[slug], 'sha256': admission.bundle_digest(target)}))
    merged = merge_population(index, baseline_bundles, output, supplements)
    row_by_name = {r['monster']: r for r in merged['monsters']}
    def flag(name, *values):
        row = row_by_name[name]
        row['completion_flags'] = sorted(set(row.get('completion_flags', [])) | set(values))
    for row in ref_report['creatures']:
        if row['monster'] in enc_names:
            continue
        flag(row['monster'], *row.get('quality_flags', []))
        if row.get('omitted_source_mechanics'):
            flag(row['monster'], 'SOURCE_BEHAVIOR_PARTIAL')
    for row in enc_report['actors']:
        flag(row['monster'], *row.get('flags', []))
        if row['classification'] == 'SOURCE_DONOR_PARTIAL':
            flag(row['monster'], 'SOURCE_BEHAVIOR_PARTIAL')
    partial = {r['encounter']: r['flags'] for r in enc_report['encounters'] if r['omitted_mechanics']}
    for row in enc_report['held_monsters']:
        for name in row['encounters']:
            if name in partial:
                flag(row['monster'], 'ENCOUNTER_PARTIAL_IMPORT', *partial[name])
    for name, flags in partial.items():
        sample = read(encounter_root / 'encounters' / name / 'manifest.json')
        for key in {key for keys in sample['covers'].values() for key in keys}:
            slug = key.rsplit('/', 1)[-1]
            if slug in row_by_name:
                flag(slug, 'ENCOUNTER_PARTIAL_IMPORT', *flags)
    for row in fam_report['creatures']:
        flag(row['slug'], *row['flags'])
    flag('candy_horror', *proof['flags'])
    flag('grand_mother_foulscale', 'SOURCE_CONFIRMED_SUMMON_NAME_CORRECTION')
    flag('seacrest_serpent', defense_correction['flag'])
    quality = {'format_version': 1, 'classification': 'SOURCE_BACKED_FLAGGED_COMPLETION_CANDIDATE',
               'live_gameplay_verified': False, 'runtime_activated': False,
               'baseline_stage_sha256': encounter.sha(baseline_stage),
               'baseline_index_sha256': encounter.sha(baseline_index),
               'baseline_count': len(index['monsters']), 'prepared_count': len(merged['monsters']),
               'supplemented_or_corrected': sorted(row['monster'] for _, row in supplements),
               'reference_completion': ref_report, 'encounter_completion': enc_report,
               'familiar_completion': fam_report, 'candy_correction': proof,
               'grand_mother_correction': correction,
               'seacrest_defense_correction': defense_correction,
               'shared_input_note': 'Unchanged files use read-only hard links to verified inputs (copy fallback); replacements are independent copies.'}
    write(output / 'population-index.json', merged)
    write(output / 'completion-quality.json', quality)
    return quality


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('canary', 'crystal', 'bundles', 'index', 'stage', 'item-map', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--components', type=Path, help='Reuse already generated source-pinned component outputs')
    parser.add_argument('--encounter-components', type=Path, help='Use a newer recursive Encounter component output')
    args = parser.parse_args()
    quality = prepare(args.canary, args.crystal, args.bundles, args.index, args.stage, args.output,
                      args.components, args.encounter_components)
    subprocess.run([sys.executable, str(Path(admission.__file__)), '--bundles', str(args.output / 'bundles'),
                    '--index', str(args.output / 'population-index.json'), '--encounters',
                    str((args.encounter_components or (args.components or args.output / 'components') / 'encounter-bundles-v2') / 'encounters'),
                    '--item-map', str(args.item_map), '--out', str(args.output / 'creature-admission-stage.json')], check=True)
    result = read(args.output / 'creature-admission-stage.json')
    before = read(args.stage)
    baseline_names = {r['monster'] for r in read(args.index)['monsters']}
    held = {name for group in ('encounter', 'initial_health', 'reference_loot_contract') for name in result['deferred'][group]}
    held.update(row['monster'] for group in ('unresolved_reference', 'unregistered_items') for row in result['deferred'][group])
    quality['native_counts'] = result['counts']
    quality['original_population_admitted'] = len(baseline_names - held)
    quality['original_population_held'] = sorted(baseline_names & held)
    quality['previous_counts'] = before['counts']
    write(args.output / 'completion-quality.json', quality)
    print(json.dumps({k: quality[k] for k in ('prepared_count', 'original_population_admitted', 'original_population_held', 'native_counts')}))


if __name__ == '__main__':
    main()
