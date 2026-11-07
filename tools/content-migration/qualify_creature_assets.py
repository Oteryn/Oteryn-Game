"""Exact source/client appearance and sprite-file evidence; never native Asset admission.

Creature Admission v1 leaves asset tokens unbound until an Asset slice admits them.
This helper proves byte availability and numeric references, without name matching or
inventing a cross-version/native identity mapping.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
sys.path.insert(0, str(ROOT / 'tools/content-schema/monster-authoring'))
from client_map_reader import fields, varint
from canary_batch import FAMILIAR_DEFAULT_LOOK, blob_id, load_effect_constants, load_monster, slug
from creature_admission_stage import bundle_digest

CLIENT_SHA = '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'
SOURCES = {
    'opentibiabr/canary': ('47dfd51f45280a59a1d3e50ba7edd573d7234446', 'aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50'),
    'zimbadev/crystalserver': ('00ce02a57ca5a12e48f32a3476e37471167e4c3f', '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1'),
}
FAMILIES = {1: 'object', 2: 'outfit', 3: 'effect', 4: 'missile'}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def decode_appearances(data):
    result = {}
    for family, wire, raw in fields(data):
        if family not in FAMILIES:
            continue
        if wire != 2:
            raise ValueError('appearance is not a message')
        ids, sprites = [], set()
        for number, wire, value in fields(raw):
            if number == 1:
                if wire != 0:
                    raise ValueError('appearance ID is not an integer')
                ids.append(value)
            if number == 2:
                if wire != 2:
                    raise ValueError('frame group is not a message')
                for field, group_wire, group in fields(value):
                    if field != 3:
                        continue
                    if group_wire != 2:
                        raise ValueError('sprite info is not a message')
                    for item, sprite_wire, sprite in fields(group):
                        if item != 5:
                            continue
                        if sprite_wire == 0:
                            sprites.add(sprite)
                        elif sprite_wire == 2:
                            offset = 0
                            while offset < len(sprite):
                                sprite_id, offset = varint(sprite, offset)
                                sprites.add(sprite_id)
                        else:
                            raise ValueError('sprite IDs have an unsupported wire type')
        if len(ids) != 1 or (FAMILIES[family], ids[0]) in result:
            raise ValueError('appearance ID missing or duplicated within its family')
        result[FAMILIES[family], ids[0]] = {'record_sha256': sha(raw), 'sprite_ids': sorted(sprites)}
    return result


def binding_id(token, constants):
    match = re.fullmatch(r'canary\.appearance:(object|outfit|effect|missile)/([a-z0-9_-]+)', token)
    if not match:
        return None
    family, value = match.groups()
    if value.isdigit():
        return family, int(value)
    if value.startswith('id-') and value[3:].isdigit():
        return family, int(value[3:])
    prefix = {'effect': 'CONST_ME_', 'missile': 'CONST_ANI_'}.get(family)
    number = constants.get(prefix + value.upper()) if prefix else None
    return (family, number) if number is not None else None


def declared_binding(name, monster):
    outfit = monster.get('outfit', {})
    if outfit.get('lookTypeEx'):
        return 'object', outfit['lookTypeEx']
    if outfit.get('lookType'):
        return 'outfit', outfit['lookType']
    vocation = slug(name).split('_')[0]
    if monster.get('flags', {}).get('familiar') and vocation in FAMILIAR_DEFAULT_LOOK:
        return 'outfit', FAMILIAR_DEFAULT_LOOK[vocation]
    return None


def pinned_code(checkout, revision, path, excerpt):
    data = subprocess.check_output(['git', '-C', str(checkout), 'show', revision + ':' + path])
    if (checkout / path).read_bytes() != data:
        raise ValueError(path + ': source differs from the exact revision')
    text = data.decode()
    if excerpt not in text:
        raise ValueError(path + ': declared source semantics does not match pinned code')
    return {'path': path, 'sha256': sha(data), 'line': text[:text.index(excerpt)].count('\n') + 1,
            'excerpt': excerpt}


def sprite_files(record, ranges):
    files, missing = set(), []
    for sprite in record['sprite_ids']:
        matches = [row['file'] for row in ranges if row['firstspriteid'] <= sprite <= row['lastspriteid']]
        if len(matches) != 1:
            missing.append({'sprite_id': sprite, 'matching_files': matches})
        else:
            files.add(matches[0])
    return sorted(files), missing


def qualify(args):
    manifest_path = args.repo / 'imports/official/client-assets/15.30/manifest.json'
    manifest = json.loads(manifest_path.read_text())
    descriptors = {row['name']: row for row in manifest['files']}
    assets = args.repo / 'content/assets/files'
    checked = {}

    def verify_file(name):
        if name not in checked:
            descriptor = descriptors.get(name)
            path = assets / name
            if not descriptor or not path.is_file():
                checked[name] = {'verified': False, 'reason': 'missing file or manifest entry'}
            else:
                data = path.read_bytes()
                checked[name] = {'verified': len(data) == descriptor['bytes'] and sha(data) == descriptor['sha256'],
                                 'bytes': len(data), 'sha256': sha(data)}
        return checked[name]['verified']

    client_name = 'appearances-' + CLIENT_SHA + '.dat'
    if not verify_file(client_name) or checked[client_name]['sha256'] != CLIENT_SHA or not verify_file('catalog-content.json'):
        raise ValueError('client appearances/catalog does not match admitted file pins')
    client = decode_appearances((assets / client_name).read_bytes())
    ranges = [row for row in json.loads((assets / 'catalog-content.json').read_text()) if row['type'] == 'sprite']
    source_indexes, source_evidence = {}, {}
    for repository, checkout in [('opentibiabr/canary', args.canary), ('zimbadev/crystalserver', args.crystal)]:
        data = (checkout / 'data/items/appearances.dat').read_bytes()
        revision, expected = SOURCES[repository]
        if sha(data) != expected:
            raise ValueError(repository + ': appearances does not match pinned source bytes')
        constants_path = 'src/utils/utils_definitions.hpp'
        pinned_constants = subprocess.check_output(['git', '-C', str(checkout), 'show', revision + ':' + constants_path])
        if (checkout / constants_path).read_bytes() != pinned_constants:
            raise ValueError(repository + ': constants differ from the exact source revision')
        effect, missile = load_effect_constants(checkout / constants_path)
        semantics = [pinned_code(checkout, revision, path, excerpt) for path, excerpt in (
            ('src/server/network/protocol/protocolgame.cpp', '\t} else {\n\t\tmsg.add<uint16_t>(outfit.lookTypeEx);\n\t}'),
            ('src/items/items.cpp', 'ItemType &iType = items[object.id()];'),
            ('src/lua/functions/lua_functions_loader.cpp', 'outfit.lookTypeEx = getField<uint16_t>(L, arg, "lookTypeEx");'))]
        tree = subprocess.check_output(['git', '-C', str(checkout), 'ls-tree', '-r', revision]).decode()
        blobs = {line.split('\t', 1)[1]: line.split()[2] for line in tree.splitlines()}
        source_evidence[repository] = {'revision': revision, 'appearance_sha256': sha(data),
                                       'look_type_ex_mapping': 'direct object appearance ID; no server-to-client remap',
                                       'semantics': semantics}
        source_indexes[repository] = (decode_appearances(data), {**effect, **missile}, revision,
                                      sha((checkout / 'src/utils/utils_definitions.hpp').read_bytes()), checkout, blobs)
    index = json.loads(args.index.read_text())
    rows = {row['monster']: row for row in index['monsters']}
    staged = json.loads(args.staged.read_text())
    admitted = sorted(profile['target']['key'].removeprefix('oteryn:creature.') for profile in staged['authoring_profiles']
                      if profile['data']['kind'] == 'Creature')
    results = []
    for name in admitted:
        directory = args.bundles / name
        if bundle_digest(directory) != rows[name]['sha256']:
            raise ValueError(name + ': bundle differs from the qualified census pin')
        monster = json.loads((directory / 'monster.json').read_text())
        provenance = json.loads((directory / 'manifest.json').read_text())
        source = next((p for p in provenance['sources'] if p.get('repository') in SOURCES), None)
        base = {'monster': name, 'bundle_sha256': rows[name]['sha256'], 'source': source,
                'native_binding_status': 'UNKNOWN_UNBOUND_ASSET_SLICE'}
        presentation = monster['presentation']['appearance']
        token = presentation.get('asset_binding')
        if token is None:
            results.append({**base, 'status': 'NO_VISIBLE_BINDING', 'selection': presentation.get('selection')})
            continue
        base['asset_binding'] = token
        if source is None or source['revision'] != SOURCES[source['repository']][0]:
            results.append({**base, 'status': 'UNKNOWN_SOURCE_PIN'})
            continue
        source_index, constants, _, constants_sha, checkout, blobs = source_indexes[source['repository']]
        identity = binding_id(token, constants)
        if identity is None:
            results.append({**base, 'status': 'UNKNOWN_BINDING_TOKEN'})
            continue
        base.update(appearance_family=identity[0], appearance_id=identity[1], source_constants_sha256=constants_sha)
        entries = [entry for entry in provenance['entries'] if entry.get('source_field') == 'outfit'
                   and entry.get('destination') == '/monster/presentation/appearance/asset_binding']
        if len(entries) != 1:
            raise ValueError(name + ': no unique source outfit provenance')
        entry = entries[0]
        if provenance['sources'][entry['source_index']] != source:
            raise ValueError(name + ': outfit source does not match the selected exact engine pin')
        path = checkout / entry['source_file']
        if blob_id(path.read_bytes()) != blobs.get(entry['source_file']):
            raise ValueError(name + ': monster source differs from pinned revision')
        registered_name, source_monster, _ = load_monster(path, errors=[])
        declaration = declared_binding(registered_name, source_monster)
        base['source_outfit'] = {'path': entry['source_file'], 'line': entry['source_line'],
                                'sha256': sha(path.read_bytes()), 'identity_verified': declaration == identity,
                                'declared_identity': declaration, 'resolution': entry.get('resolution'),
                                'basis': 'explicit_engine_outfit' if any(source_monster.get('outfit', {}).get(field)
                                         for field in ('lookType', 'lookTypeEx')) else 'accepted_familiar_default'}
        original, current = source_index.get(identity), client.get(identity)
        if original is None or current is None:
            results.append({**base, 'status': 'MISSING_SOURCE_OR_CLIENT_APPEARANCE',
                            'source_present': original is not None, 'client_present': current is not None})
            continue
        files, missing = sprite_files(current, ranges)
        failures = [file for file in files if not verify_file(file)]
        results.append({**base, 'status': 'CLIENT_APPEARANCE_AND_FILES_VERIFIED' if not missing and not failures and files else 'UNRESOLVED_SPRITE_FILES',
                        'source_record_sha256': original['record_sha256'], 'client_record_sha256': current['record_sha256'],
                        'record_bytes_equal': original['record_sha256'] == current['record_sha256'],
                        'sprite_ids_equal': original['sprite_ids'] == current['sprite_ids'],
                        'sprite_ids': current['sprite_ids'], 'sprite_files': files,
                        'missing_sprite_ranges': missing, 'invalid_files': failures})
    counts = {status: sum(row['status'] == status for row in results) for status in sorted({r['status'] for r in results})}
    return {'scope': 'Exact pinned source appearance IDs and client 15.30 sprite-file bytes; no name heuristic, native binding admission, licence or runtime qualification.',
            'client_manifest_sha256': sha(manifest_path.read_bytes()), 'client_appearance_sha256': CLIENT_SHA,
            'source_protocol_evidence': source_evidence,
            'staged_sha256': sha(args.staged.read_bytes()), 'counts': counts, 'creatures_checked': len(results),
            'source_or_accepted_default_identity_verified': sum(r.get('source_outfit', {}).get('identity_verified', False) for r in results),
            'runtime_qualified': False, 'native_binding_qualified': False, 'checked_files': checked, 'creatures': results}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('index', 'staged', 'bundles', 'canary', 'crystal', 'out'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--repo', type=Path, default=ROOT)
    args = parser.parse_args()
    report = qualify(args)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'creatures_checked': report['creatures_checked'], 'counts': report['counts'],
                      'files_checked': len(report['checked_files']), 'native_binding_qualified': False}))


if __name__ == '__main__':
    main()
