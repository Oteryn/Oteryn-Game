"""Source-pinned optional partial Encounter successors and missing actor bundles.

Original samples and donors are immutable. Approved omissions are explicit and
retained in a receipt; schema validity is not a claim of runtime qualification.
"""
import argparse
import copy
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parent / 'monster-authoring'))
import canary_batch as cb
import validate_monster as vm
import validate_encounter as ve

OMISSIONS = {
    'ferumbras_mortal_shell': 'QUEST_GLOBAL_CRYSTAL_RESET_NOT_IMPLEMENTED',
    'soul_war_taint_zones': 'SW6_POOL_CREATION_POLICY_NOT_IMPLEMENTED',
}
BLOCKING = {'unsupported_source_field', 'unresolved_semantics', 'unresolved_dependency', 'partial_text'}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def bundle_digest(directory):
    digest = hashlib.sha256()
    for filename in ('monster.json', 'dependencies.json', 'catalog.json', 'manifest.json'):
        data = (directory / filename).read_bytes()
        digest.update(f'{filename}\0{len(data)}\0'.encode('ascii') + data)
    return digest.hexdigest()


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')


def omission_source_evidence(canary, omitted):
    paths = {row['source_file'] for row in omitted}
    for row in omitted:
        paths.update(re.findall(r'\((data[^) ]+\.lua)\)', row.get('resolution', '')))
    result = []
    for path in sorted(paths):
        data = subprocess.check_output(['git', '-C', str(canary), 'show', cb.REVISION + ':' + path])
        result.append({'repository': cb.REPOSITORY, 'revision': cb.REVISION, 'path': path,
                       'sha256': hashlib.sha256(data).hexdigest(), 'git_blob': cb.blob_id(data)})
    return result


def approved_actor_omissions(manifest):
    omitted = []
    for entry in manifest['entries']:
        if entry['status'] not in BLOCKING:
            continue
        spell_omission = (entry['kind'] == 'field'
                          and re.fullmatch(r'(attacks|defenses)\[\d+\]', entry['source_field'])
                          and 'registered instant spell' in entry.get('resolution', '')
                          and 'custom logic' in entry.get('resolution', ''))
        if (entry['kind'] != 'script' and not spell_omission
                or entry['source_field'] == 'top-level script before mType:register'):
            continue
        omitted.append(copy.deepcopy(entry))
        entry['status'] = 'approved_omission'
        entry.pop('destination', None)
        entry['resolution'] = ('Explicit owner-authorized partial source actor: executable mechanic omitted; '
                               'original source and unresolved row retained in completion receipt. '
                               + entry.get('resolution', ''))
    return omitted


def source_index(canary):
    head = subprocess.check_output(['git', '-C', str(canary), 'rev-parse', 'HEAD'], text=True).strip()
    if head != cb.REVISION:
        raise ValueError('Canary checkout is not the pinned revision')
    index = {}
    for path in sorted((canary / cb.MONSTER_DIR).rglob('*.lua')):
        data = path.read_bytes()
        match = re.search(r'Game\.createMonsterType\("([^"]+)"', data.decode('utf-8'))
        if match:
            key = 'canary:creature/' + cb.slug(match[1])
            if key in index:
                raise ValueError('duplicate source identity: ' + key)
            index[key] = path
    return index


def partial_successor(encounter, manifest, name):
    """Keep every encoded rule; omit only the not-yet-encoded documented gap."""
    output = copy.deepcopy(manifest)
    omitted = []
    for entry in output['entries']:
        if entry['status'] != 'unresolved_semantics':
            continue
        if name not in OMISSIONS or entry.get('destination'):
            raise ValueError('cannot omit an encoded or unrecognized mechanic')
        original = copy.deepcopy(entry)
        omitted.append(original)
        entry['status'] = 'approved_omission'
        entry['resolution'] = ('Explicit owner-authorized partial import: ' + OMISSIONS[name]
                               + '. Original unresolved mechanic retained in completion receipt: '
                               + original['resolution'])
    if name in OMISSIONS and len(omitted) != 1:
        raise ValueError('expected exactly one documented unresolved mechanic')
    return copy.deepcopy(encounter), output, omitted


def prepare(canary, baseline_stage, baseline_index, output, crystal=None):
    if output.exists() and any(output.iterdir()):
        raise ValueError('output must be new or empty')
    index = source_index(canary)
    crystal = crystal or canary.parent / 'crystal'
    stage = json.loads(baseline_stage.read_text())
    population = json.loads(baseline_index.read_text())
    existing = {row['monster'] for row in population['monsters']}
    required = set()
    for path in (ROOT / 'samples').glob('*/encounter.json'):
        required.update(re.findall(r'canary:creature/[^" ]+', path.read_text()))
    missing = sorted(key for key in required if key.split('/')[-1] not in existing)
    objects = cb.load_appearance_objects(canary / 'data/items/appearances.dat')
    items = cb.load_items_xml(canary / 'data/items/items.xml')
    names, names_index = cb.name_index(objects, items)
    converter = cb.Converter(canary, objects, items, names, names_index)
    # Capture declarations only: Zone positions are never consumed by executable
    # callbacks in this evaluator. The omitted callback is separately recorded.
    original_prelude = cb.LUA_PRELUDE
    library_path = 'data-otservbr-global/lib/quests/soul_war.lua'
    library = subprocess.check_output(['git', '-C', str(canary), 'show', cb.REVISION + ':' + library_path])
    interval = re.findall(rb'goshnarsCrueltyWaveInterval\s*=\s*(\d+)', library)
    if interval != [b'7']:
        raise ValueError('Soul War source interval changed')
    zone_stub = ("Zone = {getByName = function(name) return {getPositions = function() return {} end} end}\n"
                 "SoulWarQuest = {goshnarsCrueltyWaveInterval = 7}\n")
    cb.LUA_PRELUDE = zone_stub + original_prelude
    actors = []
    try:
        for key in missing:
            path = index.get(key)
            if path is None:
                raise ValueError('required source actor is unavailable: ' + key)
            source_path = str(path.relative_to(canary))
            pinned = subprocess.check_output(['git', '-C', str(canary), 'show', cb.REVISION + ':' + source_path])
            if pinned != path.read_bytes():
                raise ValueError('source actor differs from pinned Git object: ' + key)
            relative = str(path.relative_to(canary / cb.MONSTER_DIR).with_suffix(''))
            converter.pending_definitions = set()
            slug, monster, deps, catalog, manifest, evidence = converter.convert(relative)
            original_manifest = copy.deepcopy(manifest)
            source_summons = copy.deepcopy(monster['behavior'].get('summons'))
            flags = []
            failed_summon = None
            if slug == 'professor_maxxen':
                entries = monster['behavior']['summons']['entries']
                invalid = [n for n, row in enumerate(entries)
                           if row['creature']['key'] == 'canary:creature/glooth_smasher']
                if invalid != [2] or 'canary:creature/glooth_smasher' in index:
                    raise ValueError('Professor source failed-summon census changed')
                original_action = entries.pop(2)
                crystal_revision = '00ce02a57ca5a12e48f32a3476e37471167e4c3f'
                crystal_path = 'data-global/monster/quests/hero_of_rathleton/professor_maxxen.lua'
                crystal_source = subprocess.check_output(['git', '-C', str(crystal), 'show',
                                                          crystal_revision + ':' + crystal_path])
                if not re.search(rb'name\s*=\s*"glooth smasher"', crystal_source):
                    raise ValueError('Crystal does not corroborate unregistered summon literal')
                failed_summon = {'original_action': original_action,
                                 'failure': 'requested source identity is unregistered; no Creature was fabricated',
                                 'alias_created': False,
                                 'corroborating_source': {'repository': 'zimbadev/crystalserver',
                                     'revision': crystal_revision, 'path': crystal_path,
                                     'sha256': hashlib.sha256(crystal_source).hexdigest(),
                                     'git_blob': cb.blob_id(crystal_source)}}
                for row in manifest['entries']:
                    pointer = row.get('destination', '')
                    prefix = '/monster/behavior/summons/entries/'
                    if not pointer.startswith(prefix):
                        continue
                    number, *tail = pointer[len(prefix):].split('/')
                    if int(number) == 2:
                        row['status'] = 'approved_omission'
                        row.pop('destination', None)
                        row['resolution'] = ('Pinned source requests unregistered Glooth Smasher; no registered '
                                             'type of this identity exists. Failed summon omitted, no alias to Glooth Masher invented.')
                    elif int(number) > 2:
                        row['destination'] = prefix + str(int(number) - 1) + ('/' + '/'.join(tail) if tail else '')
                flags.append('SOURCE_UNREGISTERED_SUMMON_OMITTED')
            # Missing numeric data and pre-registration failures always remain
            # blocking; only bounded executable mechanics can be omitted.
            omitted = approved_actor_omissions(manifest)
            for family, target in sorted(converter.pending_definitions):
                reference = cb.ref(family, target)
                if reference not in catalog['definitions'] and target != key:
                    catalog['definitions'].append(reference)
            errors = vm.validate(monster, deps, catalog, manifest)
            directory = output / 'creatures' / slug
            for filename, value in [('monster.json', monster), ('dependencies.json', deps),
                                    ('catalog.json', catalog), ('manifest.json', manifest)]:
                write(directory / filename, value)
            actors.append({'monster': slug, 'file': relative, 'source': evidence,
                           'sha256': bundle_digest(directory),
                           'source_sha256': sha(path), 'validation_errors': errors,
                           'schema_valid': not errors, 'omitted_mechanics': omitted,
                           'flags': flags, 'original_summons': source_summons,
                           'failed_summon_omission': failed_summon,
                           'omission_source_evidence': omission_source_evidence(canary, omitted),
                           'original_manifest': original_manifest,
                           'classification': 'SOURCE_DONOR_PARTIAL' if omitted or failed_summon else 'SOURCE_DONOR_PREPARED',
                           'authoring_capture': {'callbacks_executed': False,
                                                'zone_positions_inferred': False,
                                                'soul_war_interval_seconds': 7,
                                                'library_path': library_path,
                                                'library_sha256': hashlib.sha256(library).hexdigest()},
                           'runtime_qualified': False})
            # Iterate the growing list to close exact summon/transform actor
            # dependencies without manufacturing aliases or removing all summons.
            referenced = set(re.findall(r'canary:creature/[^" ]+', json.dumps([monster, deps])))
            for target in sorted(referenced):
                if target.split('/')[-1] not in existing and target not in missing:
                    if target not in index:
                        raise ValueError('recursive actor has no registered source identity: ' + target)
                    missing.append(target)
    finally:
        cb.LUA_PRELUDE = original_prelude
    encounters = []
    for directory in sorted(p.parent for p in (ROOT / 'samples').glob('*/encounter.json')):
        encounter = json.loads((directory / 'encounter.json').read_text())
        manifest = json.loads((directory / 'manifest.json').read_text())
        catalog = json.loads((directory / 'catalog.json').read_text())
        successor, next_manifest, omitted = partial_successor(encounter, manifest, directory.name)
        errors = ve.validate(successor, catalog, next_manifest)
        if errors:
            raise ValueError(directory.name + ': invalid successor: ' + str(errors))
        target = output / 'encounters' / directory.name
        for filename, value in [('encounter.json', successor), ('manifest.json', next_manifest), ('catalog.json', catalog)]:
            write(target / filename, value)
        encounters.append({'encounter': directory.name, 'original_encounter_sha256': sha(directory / 'encounter.json'),
                           'original_manifest_sha256': sha(directory / 'manifest.json'),
                           'omitted_mechanics': omitted,
                           'flags': [OMISSIONS[directory.name]] if omitted else [],
                           'encoded_rules_preserved': True, 'schema_valid': True,
                           'full_encounter_qualified': False, 'runtime_qualified': False})
    covered_by = {}
    for row in encounters:
        manifest = json.loads((output / 'encounters' / row['encounter'] / 'manifest.json').read_text())
        for key in {k for keys in manifest['covers'].values() for k in keys}:
            covered_by.setdefault(key.split('/')[-1], []).append(row['encounter'])
    result = {'format_version': 1, 'source_revision': cb.REVISION,
              'baseline_stage_sha256': sha(baseline_stage), 'baseline_index_sha256': sha(baseline_index),
              'source_access': 'pinned local Git checkouts; no Remote Desktop',
              'actors': actors, 'encounters': encounters,
              'held_monsters': [{'monster': slug, 'encounters': covered_by.get(slug, []),
                                 'reason': 'source_actor_dependency_or_explicit_partial_encounter',
                                 'original_behavior_preserved': True}
                                for slug in stage['deferred']['encounter']],
              'native_reference_closure_verified': False, 'runtime_qualified': False}
    write(output / 'encounter-completion.json', result)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canary', type=Path, required=True)
    parser.add_argument('--crystal', type=Path, help='Pinned corroborating donor; default sibling crystal checkout')
    parser.add_argument('--stage', type=Path, required=True)
    parser.add_argument('--index', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = prepare(args.canary, args.stage, args.index, args.output, args.crystal)
    print(json.dumps({'actors': len(result['actors']), 'encounters': len(result['encounters']),
                      'schema_valid_actors': sum(r['schema_valid'] for r in result['actors'])}))


if __name__ == '__main__':
    main()
