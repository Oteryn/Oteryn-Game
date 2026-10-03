"""Prepare all five familiar cores; owner lifecycle remains a separate contract.

Monk combat follows pinned Crystal, while the accepted 15000 HP and existing
mitigation remain unchanged. Challenge is recorded as an explicit omission.
"""
import argparse
import copy
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/monster-authoring'))
import canary_batch as cb
import crystal_batch as xb
import prepare_familiar_abilities as familiar
import spell_scripts
import validate_monster as validator
import creature_admission_stage as admission

FILES = ('monster.json', 'dependencies.json', 'catalog.json', 'manifest.json')
VOCATIONS = ('knight', 'paladin', 'monk', 'druid', 'sorcerer')
FLAGS = ['PLAYER_FAMILIAR_RUNTIME_INTEGRATION_PENDING', 'SOURCE_BACKED_CORE_SUMMON']


def read(path):
    return json.loads(path.read_text())


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def digest(directory):
    result = hashlib.sha256()
    for name in FILES:
        data = (directory / name).read_bytes()
        result.update(f'{name}\0{len(data)}\0'.encode() + data)
    return result.hexdigest()


def retain_stats(monster, manifest, original_monster, original_manifest):
    """Keep numeric acceptance and its original provenance, not Crystal HP 20000."""
    monster = copy.deepcopy(monster)
    manifest = copy.deepcopy(manifest)
    monster['creature']['stats'] = copy.deepcopy(original_monster['creature']['stats'])
    prefix = '/monster/creature/stats/'
    manifest['entries'] = [e for e in manifest['entries']
                           if not e.get('destination', '').startswith(prefix)]
    for entry in original_manifest['entries']:
        if not entry.get('destination', '').startswith(prefix):
            continue
        entry = copy.deepcopy(entry)
        source = original_manifest['sources'][entry['source_index']]
        if source not in manifest['sources']:
            manifest['sources'].append(copy.deepcopy(source))
        entry['source_index'] = manifest['sources'].index(source)
        manifest['entries'].append(entry)
    return monster, manifest


def omit_challenge(manifest):
    manifest = copy.deepcopy(manifest)
    unresolved = [e for e in manifest['entries'] if e['status'].startswith('unresolved')]
    if len(unresolved) != 1 or unresolved[0]['source_field'] != 'attacks[4]':
        raise ValueError('unexpected unresolved familiar behavior')
    entry = unresolved[0]
    original = copy.deepcopy(entry)
    entry.update(status='approved_omission', resolution=(
        'Owner-authorized practical core: summon challenge at interval 2000 ms, '
        'chance 40 percent is omitted. Owner-aware provoke/target reassignment '
        'has no accepted native content operation; tracked by '
        'FAMILIAR_CHALLENGE_NOT_IMPLEMENTED. Damage and self-healing retained.'))
    return manifest, original


def prepare(baseline, canary, crystal, output):
    protected = [ROOT.resolve(), baseline.resolve(), canary.resolve(), crystal.resolve()]
    if any(output.resolve() == p or p in output.resolve().parents for p in protected):
        raise ValueError('output would modify source inputs')
    if output.exists() and any(output.iterdir()):
        raise ValueError('output must be new or empty')
    converter = cb.Converter(canary, {}, {}, {}, {})
    crystal_converter = cb.Converter(canary, {}, {}, {}, {})
    crystal_converter.monster_root = crystal
    crystal_converter.monster_dir = xb.MONSTER_ROOT
    crystal_converter.source = {'repository': xb.REPOSITORY, 'revision': xb.REVISION}
    crystal_converter.spell_scripts = spell_scripts.SpellScripts(canary, extra_roots=(crystal,))
    rows = {r['monster']: r for r in read(baseline / 'population-index.json')['monsters']}
    report = {'format_version': 1, 'runtime_qualified': False, 'actors': [],
              'baseline_index_sha256': hashlib.sha256((baseline / 'population-index.json').read_bytes()).hexdigest(),
              'additional_index_rows': [], 'index_monsters': [],
              'adapter_packet': {'owning_lane': 'FAMILIARS-0',
                  'status': 'PROPOSED_NOT_IMPLEMENTED',
                  'required_semantics': familiar.UNREPRESENTED,
                  'acceptance_tests': [
                      'Level/vocation/premium and no-other-summon admission rejects atomically.',
                      'Accepted cast deducts mana once and installs one owned familiar.',
                      'Selected appearance and owner speed transfer to familiar.',
                      'Expiry at 900000 ms removes once; warning events at 60 and 10 seconds.',
                      'Cooldown follows accepted VIP/rate policy and 2000 ms support group.',
                      'Owner death/logout/login and generation fence never duplicate familiar.',
                      'Druid/Sorcerer Challenge changes only eligible nearby hostile targets.']}}
    for vocation in VOCATIONS:
        name = vocation + '_familiar'
        source = baseline / 'bundles' / name
        exists = name in rows
        flags = list(rows[name].get('completion_flags', [])) if exists else []
        original = [read(source / f) for f in FILES] if exists else None
        facts = familiar.source_facts(canary, cb.REVISION, vocation)
        for root, revision, relative in [(canary, cb.REVISION, cb.MONSTER_DIR + '/familiars/' + name + '.lua')]:
            familiar.pinned(root, revision, relative)
        omitted = None
        if vocation in ('knight', 'paladin'):
            values = copy.deepcopy(original)
        else:
            selected = crystal_converter if vocation == 'monk' else converter
            if vocation == 'monk':
                familiar.pinned(crystal, xb.REVISION, xb.MONSTER_ROOT + '/familiars/' + name + '.lua')
                familiar.pinned(crystal, xb.REVISION, 'data-global/scripts/spells/monster/monk_familiar_wave.lua')
            _, monster, dependencies, catalog, manifest, _ = selected.convert('familiars/' + name)
            if vocation == 'monk':
                monster, manifest = retain_stats(monster, manifest, original[0], original[3])
                flags.append('MONK_CRYSTAL_COMBAT_CORE_RESTORED')
            else:
                manifest, omitted = omit_challenge(manifest)
                flags += ['FAMILIAR_CHALLENGE_NOT_IMPLEMENTED', 'SOURCE_BEHAVIOR_PARTIAL', 'MITIGATION_UNKNOWN']
            values = familiar.supplement(monster, dependencies, catalog, manifest, vocation, facts)
        flags = sorted(set(flags + FLAGS))
        errors = validator.validate(*values)
        if errors:
            raise ValueError(name + ': ' + '\n'.join(errors))
        stage = admission.Stage(admission.Mapper({}))
        stage.stage_dependencies(values[1], name)
        stage.stage_monster(values[0], name)
        identities = {(family, key, 'definition-r1') for family, key in stage.records}
        for value in list(stage.profiles.values()) + list(stage.records.values()):
            if any(ref not in identities for ref in admission.exact_definition_refs(value)):
                raise ValueError(name + ': native definition closure incomplete')
        target = output / 'bundles' / name
        for filename, value in zip(FILES, values):
            write(target / filename, value)
        row = {**rows.get(name, {'monster': name, 'file': 'familiars/' + name}),
               'sha256': digest(target), 'completion_flags': flags}
        report['index_monsters'].append(row)
        actor = {'monster': name, 'bundle_digest': row['sha256'],
                 'original_bundle_digest': digest(source) if exists else None,
                 'completion_flags': flags, 'player_spell': facts,
                 'native_records': len(stage.records), 'native_missing_references': 0,
                 'original_omission_rows': [omitted] if omitted else []}
        report['actors'].append(actor)
        if not exists:
            report['additional_index_rows'].append(row)
    report['counts'] = {'familiars': 5, 'new_definitions': 2, 'corrected_monks': 1,
                        'challenge_omissions': 2, 'owner_lifecycle_pending': 5}
    write(output / 'completion.json', report)
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('baseline', 'canary', 'crystal', 'output'):
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(prepare(**vars(args))['counts']))
