"""Reproducible reference crop crosswalk; no asset bytes or runtime admission."""
import hashlib
import json
import sys
import struct
from pathlib import Path
from wheel_authoring import ROOT, read


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def references(candidate):
    for vocation, data in candidate['vocations'].items():
        for slot in data['slots']:
            base = f'/vocations/{vocation}/slots/{slot["state_slot"] - 1}'
            yield base + '/dedication_icon', slot['dedication_icon']
            yield base + '/conviction/icon', slot['conviction']['icon']
        for index, revelation in enumerate(data['revelations']):
            yield f'/vocations/{vocation}/revelations/{index}/icon', revelation['icon']
    for field in ('basic_mods', 'supreme_mods'):
        for index, mod in enumerate(candidate['gems'][field]):
            yield f'/gems/{field}/{index}/icon', mod['icon']


def build_manifest(candidate=None):
    candidate = candidate or read(ROOT / 'samples/wheel-candidate.json')
    source = read(ROOT / 'samples/source-icon-reference.json')
    if candidate['icon_evidence']['reference_sheet_sha256'] != digest(ROOT / 'samples/source-icon-reference.json'):
        raise ValueError('ICON_SOURCE_DIGEST')
    sheets = {sheet['category']: sheet for sheet in source['sheets']}
    expected = {'dedication', 'conviction', 'revelation', 'basic_mod', 'supreme_mod'}
    if set(sheets) != expected or len(source['sheets']) != len(expected):
        raise ValueError('ICON_SHEET_COVERAGE')
    for sheet in sheets.values():
        if any(type(sheet[key]) is not int or sheet[key] <= 0 for key in ('width', 'height', 'bytes')):
            raise ValueError('ICON_SHEET_DIMENSIONS')
    icons, bindings = {}, {}
    for pointer, reference in references(candidate):
        category, index = reference['sprite'], reference['source_index']
        sheet = sheets[category]
        size = sheet['height']
        if type(index) is not int or size <= 0 or sheet['width'] % size or not 0 <= index < sheet['width'] // size:
            raise ValueError('ICON_CROP_BOUNDS')
        key = f'{category}:{index}'
        icons[key] = {'sheet': category, 'source_index': index,
                      'rect': [index * size, 0, size, size]}
        bindings[pointer] = key
    return {'schema': 'OTERYN_WHEEL_CLIENT_ICON_REFERENCE/v1',
            'runtime_admitted': False, 'redistribution_authorized': False,
            'mapping_basis': 'CURRENT_PINNED_PLANNER_SOURCE_IDS; NOT_OLDER_CLIENT_SLOT_LAYOUT',
            'candidate_sha256': digest(ROOT / 'samples/wheel-candidate.json'),
            'source_sha256': digest(ROOT / 'samples/source-icon-reference.json'),
            'sheets': sheets, 'icons': dict(sorted(icons.items())), 'bindings': bindings}


def validate_manifest(manifest, candidate=None):
    if manifest != build_manifest(candidate):
        raise ValueError('ICON_MANIFEST_DRIFT')


def verify_sheet_bytes(directory):
    for sheet in read(ROOT / 'samples/source-icon-reference.json')['sheets']:
        path = directory / (sheet['category'] + '.png')
        data = path.read_bytes()
        if hashlib.sha256(data).hexdigest() != sheet['sha256'] or len(data) != sheet['bytes']:
            raise ValueError('ICON_SHEET_DIGEST')
        if data[:8] != b'\x89PNG\r\n\x1a\n' or data[12:16] != b'IHDR':
            raise ValueError('ICON_SHEET_FORMAT')
        if struct.unpack('>II', data[16:24]) != (sheet['width'], sheet['height']):
            raise ValueError('ICON_SHEET_DIMENSIONS')


def validate_selection(candidate):
    selection = read(ROOT / 'samples/reference-selection.json')
    source_path = ROOT.parent / 'spell-authoring/wheel-augments.json'
    source = read(source_path)
    if selection['spell_input_sha256'] != digest(source_path):
        raise ValueError('SELECTION_SOURCE_DIGEST')
    if selection['target_date'] != source['target_date'] or selection['primary_sources'] != source['sources']:
        raise ValueError('SELECTION_SNAPSHOT')
    correction = next(c for c in candidate['gems']['reference_corrections']
                      if c['key'] == 'augmented_mystic_repulse' and c['stage'] == 2)
    if selection['mystic_repulse_ii']['selected_percent'] != correction['selected_value']:
        raise ValueError('SELECTION_VALUE')
    if selection['live_global_parity_confirmed'] or selection['runtime_admitted']:
        raise ValueError('SELECTION_ADMISSION')


if __name__ == '__main__':
    if '--assets-dir' in sys.argv:
        verify_sheet_bytes(Path(sys.argv[sys.argv.index('--assets-dir') + 1]))
    path = ROOT / 'samples/client-icon-manifest.json'
    manifest = build_manifest()
    rendered = json.dumps(manifest, indent=2, allow_nan=False) + '\n'
    if '--check' in sys.argv:
        validate_manifest(read(path))
        if path.read_text() != rendered:
            raise ValueError('ICON_MANIFEST_REBUILD_DRIFT')
    else:
        path.write_text(rendered)
    validate_selection(read(ROOT / 'samples/wheel-candidate.json'))
    print(f'PASS: {len(manifest["icons"])} reference crops, {len(manifest["bindings"])} bindings; source selection qualified.')
