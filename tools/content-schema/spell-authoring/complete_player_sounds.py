"""Repair only sound presentation in an existing qualified catalog.

Retain every execution/dependency/identity and selection decision. Engine defaults
are obtained through convert_spells.Execution; no absent registrar means silence.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
from types import SimpleNamespace
from build_player_presentation import HEADERS, PINS, pinned
from convert_spells import Bundle, SOUND_PREFIX
import re


def complete(sources, catalog_path, census_path):
    old = json.loads(catalog_path.read_bytes())
    result = copy.deepcopy(old)
    census = json.loads(census_path.read_bytes())
    indices, executions = {}, {}
    for source, pin in PINS.items():
        root = sources / source
        header = pinned(root, pin, 'src/creatures/combat/spells.hpp').decode()
        lua = pinned(root, pin, 'src/lua/functions/core/game/lua_enums.cpp').decode()
        registered = set(re.findall(r'registerEnumNamespace\(L,\s*soundNamespace,\s*SoundEffect_t::([A-Z0-9_]+)\s*\)', lua))
        defaults = {}
        for method, member in (('castSound', 'soundCastEffect'), ('impactSound', 'soundImpactEffect')):
            match = re.search(r'\b' + member + r'\s*=\s*SoundEffect_t::([A-Z0-9_]+)\s*;', header)
            if match is None: raise ValueError('unresolved pinned default')
            defaults[method] = SOUND_PREFIX + match.group(1)
        def sound(value, registered=registered):
            if isinstance(value, str) and value.startswith(SOUND_PREFIX):
                member = value.removeprefix(SOUND_PREFIX)
                if member in registered: return 'canary.sound:' + member.lower()
            return None
        executions[source] = SimpleNamespace(root=root, sound_defaults=defaults, sound=sound)
        indices[source] = {(r['spell_type'], r['name'].lower()): r for r in census[source]}
    changed = []
    for row in result['bundles']:
        spell, manifest = row['bundle']['spell'], row['manifest']
        records = {s: idx[(spell['carrier'], spell['name'].lower())] for s, idx in indices.items()
                   if (spell['carrier'], spell['name'].lower()) in idx}
        fake = SimpleNamespace(records=records, executions=executions, rows=[])
        def record_entry(status, field, destination=None, resolution=None, source=None, **kwargs):
            index = next(i for i, value in enumerate(manifest['sources'])
                         if value.get('revision') == PINS[source])
            fake.rows.append({'source_index': index, 'source_file': records[source]['file'],
                              'source_line': next((n for n, line in enumerate(pinned(sources / source, PINS[source], records[source]['file']).decode().splitlines(), 1) if ':' + field + '(' in line), 1), 'source_field': field, 'kind': 'field',
                              'status': status, **({'destination': destination} if destination else {}),
                              **({'resolution': resolution} if resolution else {})})
        fake.row = record_entry
        before = copy.deepcopy(spell.get('presentation', {}))
        after = Bundle.sound_cues(fake, '/spell/spell')
        if before == after: continue
        if after: spell['presentation'] = after
        else: spell.pop('presentation', None)
        manifest['entries'] = [entry for entry in manifest['entries']
                               if entry['source_field'] not in ('castSound', 'impactSound')]
        manifest['entries'].extend(fake.rows)
        digest = hashlib.sha256((json.dumps(manifest, ensure_ascii=False, indent=2) + '\n').encode()).hexdigest()
        for identity in row['source_identities']: identity['manifest_sha256'] = digest
        changed.append({'identity': spell['identity'], 'before': before, 'after': after})
    return result, changed

def synchronize_native_profiles(catalog, profiles_path):
    """Preserve closed execution equality while refreshing qualified presentation."""
    from build_native_profiles import assemble, REVISION
    if (catalog.get('schema') != 'OTERYN_EXECUTABLE_SPELL_CATALOG/v1' or
            catalog.get('revision') != REVISION or len(catalog.get('bundles', [])) != 246):
        raise ValueError('unqualified complete catalog')
    original = json.loads(profiles_path.read_bytes())
    if original.get('revision') != REVISION:
        raise ValueError('unqualified native profile revision')
    rows = {}
    for row in catalog['bundles']:
        spell = row['bundle']['spell']
        identity = (spell['identity']['key'], spell['identity']['revision'])
        if identity in rows:
            raise ValueError('duplicate catalog identity')
        rows[identity] = row
    profiles = copy.deepcopy(original['profiles'])
    changes = []
    for profile in profiles:
        before = profile['spell']
        identity = (before['identity']['key'], before['identity']['revision'])
        if identity not in rows:
            raise ValueError('native profile missing from qualified catalog')
        row = rows[identity]
        after = row['bundle']['spell']
        if ({k: v for k, v in before.items() if k != 'presentation'} !=
                {k: v for k, v in after.items() if k != 'presentation'} or
                profile['dependencies'] != row['dependencies']):
            raise ValueError('native synchronization may change presentation only')
        if before != after:
            changes.append({'identity': before['identity'],
                            'before': before.get('presentation'),
                            'after': after.get('presentation')})
            if 'presentation' in after:
                profile['spell']['presentation'] = copy.deepcopy(after['presentation'])
            else:
                profile['spell'].pop('presentation', None)
    return assemble(profiles), changes

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sources', type=Path, required=True)
    parser.add_argument('--catalog', type=Path, required=True)
    parser.add_argument('--census', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--delta', type=Path, required=True)
    parser.add_argument('--native-profiles', type=Path)
    parser.add_argument('--native-profiles-out', type=Path)
    args = parser.parse_args()
    if bool(args.native_profiles) != bool(args.native_profiles_out):
        parser.error('--native-profiles and --native-profiles-out must be supplied together')
    result, delta = complete(args.sources, args.catalog, args.census)
    args.out.write_text(json.dumps(result, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n')
    args.delta.write_text(json.dumps({'changed_definitions': len(delta), 'changes': delta}, sort_keys=True, indent=2) + '\n')
    if args.native_profiles:
        profiles, changes = synchronize_native_profiles(result, args.native_profiles)
        args.native_profiles_out.write_text(json.dumps(profiles, ensure_ascii=False, indent=2) + '\n')
        print(json.dumps({'changed_native_headers': len(changes)}))
    print(json.dumps({'changed_definitions': len(delta), 'catalog_sha256': hashlib.sha256(args.out.read_bytes()).hexdigest()}))
