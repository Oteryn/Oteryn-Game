"""Reproduce W17's bounded source inputs; this does not admit a native encounter.

Both capture and check read the retained 15.30 outfit shard/client manifest,
six original sprite sheets, and eight pinned TibiaMaps PNGs. All are offline
inputs; no downloads, client login, content activation or proprietary-asset
publication occurs. See the W17 evidence document for source acquisition.

The output supplies exact asset_binding strings to existing Presentation
authoring and Global xyz cells to a future qualified Drome room. It deliberately
does not create incomplete Creature/Behavior records or a replacement engine.
"""
from __future__ import annotations

import argparse
import hashlib
import io
import json
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = 'tools/content-schema/native-gameplay/tibiadrome-source-profile.json'
SCHEMA = 'OTERYN_TIBIADROME_SOURCE_INPUTS/v1'
OUTFITS = 'imports/cipsoft-appearances/outfits/outfits-01050-01199.json'
OUTFITS_SHA = '291f591df0016a330c7b9571c2ea5a5adac22a441073b5ea99475c93b4f4fddc'
MANIFEST = 'imports/official/client-assets/15.30/manifest.json'
MANIFEST_SHA = 'febaff9f4bd7e0f8a029736e446a81f1626805e895e7c2268018e0a9a8493fe4'
CATALOG = 'content/assets/files/catalog-content.json'
CATALOG_SHA = '4921ef60464a0baea0aa25e02c5c7843a42196767951c453400e81a67cc3b424'
CLIENT_APPEARANCE_SHA = '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'
MAP_REVISIONS = {
    'target': 'b9c0a3692f7a6608cea64c4bd599e1858e302350',
    'current': '3cf5dff5791dee70fda70525b05cf928fe20bafd',
}
# Tuple: name, lookType, raw appearance record SHA-256, first sprite, named frame.
POOL = (
    ('Domestikion', 1426, 'b45b9349825288dae6f5869dcb1921bc6629187d92b8546f00c32d950568d7f5', 97988, 385),
    ('Hoodinion', 1424, '9093d346e98f62beceb36231a84cfd641d9f37965c89536b179fb5cf8871617f', 97916, 415),
    ('Mearidion', 1425, '0aeba7874aa2ef2732f352ed76f73622d1aef92bc8245fdfd58f54f6380eedb0', 97952, 405),
    ('Murmillion', 1422, '983c71558280074aec70f4d23324a50ffb73da09fc28f180f444a265fe09ea10', 97844, 480),
    ('Scissorion', 1423, '69fba8be411a7a50abaf83bbc4060bbc48f8c0815b182e03f8313677f10ea5b7', 97880, 385),
)
# Filename digest and compressed-byte digest are different, both are retained.
SPRITES = (
    ('da806e8cb6d2f09c396ac35fa892acde094fd896dec7efcc9cb7096b6e1d97b7', '93dac89bd2885d8ca0a41d0132598935f72d25fa0f00188bd16b058190dd619c', 97811, 97846),
    ('6b133e5292fd74e8099859f69dbfda268966a48f12efdff5ec152871870e9b37', '657a2ab5573018deee0ec1fe25062f6c47eefbf270e5f83f4593c0ba65c4f1ca', 97847, 97882),
    ('5a99e7d2d7fd124f4c7a1e2ce0575088b82e4a6b59f5cf1dca985f8a424eba0c', '72403dbc6334af32476c3f4a3a04bf5d65997d875db5b81cce47d9a11b509500', 97883, 97918),
    ('cc40c5f70929f8bfa06f81e3b434f974c28a332551863d68facdb9c7bca9a2ba', '0e468236135f1ecbeb0d546d492099769810b984c6b3637abf5da733ea33e4e5', 97919, 97954),
    ('45275c3306287db32a8c5fc0b798014015576905c4cd6db6e686bb7908f4c7ba', 'c20917e1304b00d4fd0f049a4141265cc96645f113cb4f87d2b82f6645a4a410', 97955, 97990),
    ('c3b282f41d40f6cdebfba8f72c226ae996d2a9690516a569dd0f54240f47862f', 'cbb44239d842843b5abc3b6c395c1395bc36668f5b487e1a673b76cc2b6e5641', 97991, 98026),
)
MAP_HASHES = {
    'target-floor-11-map.png': '28baa7a5aeeea42a72bce58ce1c9133358dee2f52744ec54564b0db660704c93',
    'target-floor-11-path.png': 'ac564b94d99d2d70721f35c565c81f58d8b2c1740f059930062823cabd595a23',
    'target-floor-12-map.png': 'd9b387986d2305a36514be8c13e80fb0d18d4df270f3fdea4534927e6dbd8414',
    'target-floor-12-path.png': '79b8a9a6f6b4e5ce565a2090c6f23e71a418a8c730d9e843da8ed364bff074b1',
    'current-floor-11-map.png': 'c2bfa6b30f0446fff7c6b8f94e407a83983754614a547e14ceb82799bc8d0d3f',
    'current-floor-11-path.png': '29421c2ebc1476c2e526c4e97eb2b30c579ee0953dd956284dfabce05face78a',
    'current-floor-12-map.png': '6e18ff8cac3ddb7066d2b295f271f0b9f70f6b377f11cdd84d31d7349ac5ce0c',
    'current-floor-12-path.png': '2fb7865f21424fce7850fe5d81ad28e53b7fd3872258f24391ac833c5ab16261',
}
CROP_HASHES = {
    'floor-11-map.png': '4ab4538b743d3d883ce8625c6e764627840386baa5f6461e3890460064d7d735',
    'floor-11-path.png': '4493717a239af2c56b5d624a36b6c5e5c1e96f589cc35d637189376c46005574',
    'floor-12-map.png': '83ed4788985a1e892bfed21f47ec8b86e28e6006163dc9e541fe16c8bd0179ab',
    'floor-12-path.png': '895357b6d13a0a5a54dd08e552199de736c5475febdbaf5956d742bc606b8bc6',
}
FRAMES = {
    'prebattle-frame-340.jpg': '6199603918ba076ab112e98721b8020137b5eb111dcaadb65391cead34eb3d95',
    'prebattle-frame-365.jpg': '47dae814d95653534b9937d07a9a4d892f73abaeec7626af9ea4030847e091d5',
    'combat-frame-375.jpg': 'b70fdb42566b362bbfcd1359670c9b12f11474a25e3367f60a1af06f5f4ea753',
    'combat-frame-385.jpg': '2f641576b9d71dbea9fcf16a5a451de8d3a2a49e5a33bac492137327e3a77419',
    'combat-frame-400.jpg': '9e7a563117138f75657c392e628ef61780a0cec074a6268c0842439027ba36bb',
    'combat-frame-405.jpg': 'fe45b9de4eefb296f31d6c07e24e88456bec3aa79f5db8af4b4ec8e6594ceb02',
    'combat-frame-415.jpg': '74fa35fc0453935767e1ee2288286dc33f03cb845fe2389444dc6180136342db',
    'combat-frame-427.jpg': '96c81383e91484dfa90897ce7ba8f151e1a6c81be75226e435c269494448571a',
    'combat-frame-480.jpg': '803a4ecdc531cce12084b060abd2b362a8947a85ca0eaf11776fc5ee5ea8d70d',
    'combat-frame-489.jpg': '804c9f302ffd27bc8c2f45d991473b553c84ea1259737852e1a266f43a8d77a9',
}


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode()


def decode_json(raw):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError('DROME_SOURCE_DUPLICATE_KEY:' + key)
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=unique)


def verify_bytes(raw, expected, label):
    if hashlib.sha256(raw).hexdigest() != expected:
        raise ValueError('DROME_SOURCE_HASH:' + label)


def battle_cells():
    return [[x, y, 12] for y in range(32179, 32195) for x in range(32283, 32299)
            if not (x in (32283, 32298) and y in (32179, 32194))]


def expected_profile():
    return {
        'schema': SCHEMA, 'producer': 'tools/content-migration/tibiadrome_source_profile.py',
        'target_version': '15.30', 'runtime_admission': False,
        'qualification': 'SOURCE_INPUTS_ONLY',
        'sources': {
            'outfits': {'path': OUTFITS, 'sha256': OUTFITS_SHA, 'input_sha256': CLIENT_APPEARANCE_SHA},
            'client_manifest': {'path': MANIFEST, 'sha256': MANIFEST_SHA},
            'catalog': {'path': CATALOG, 'sha256': CATALOG_SHA},
            'sprites': [{'file': f'sprites-{name}.bmp.lzma', 'sha256': sha,
                         'sprite_first': first, 'sprite_last': last} for name, sha, first, last in SPRITES],
            'maps': {'repository': 'tibiamaps/tibia-map-data', 'revisions': dict(MAP_REVISIONS),
                     'target_capture_commit_date': '2026-09-10', 'current_capture_commit_date': '2026-11-23',
                     'frame': {'x_min': 31744, 'y_min': 30976, 'width': 2560, 'height': 2048},
                     'crop': {'x_min': 32275, 'y_min': 32165, 'width': 33, 'height': 55},
                     'files': [{'file': name, 'repository_path': 'data/' + name.split('-', 1)[1],
                                'revision': MAP_REVISIONS[name.split('-', 1)[0]], 'sha256': sha,
                                'crop_rgb_sha256': CROP_HASHES[name.split('-', 1)[1]]}
                               for name, sha in MAP_HASHES.items()],
                     'crop_continuity': 'IDENTICAL_RGB_AT_BOTH_PINNED_COMMITS'},
            'video': {'url': 'https://www.youtube.com/watch?v=InPPpFvdddE', 'channel': 'GM Tibia',
                      'upload_date': '2023-02-13', 'duration_seconds': 698,
                      'game_version': 'UNKNOWN', 'role': 'ORIGINAL_GAMEPLAY',
                      'timestamps': 'requested source seconds; section cuts can offset frames',
                      'frame_hashes': dict(FRAMES), 'semantic_verification': 'MANUAL_FRAME_INSPECTION'},
        },
        'pool': [{'name': name, 'look_type': look, 'asset_binding': f'canary.appearance:outfit/{look}',
                  'source_record_sha256': sha, 'idle_sprites': [first, first + 3],
                  'walking_sprites': [first + 4, first + 35],
                  'name_binding': {'classification': 'DERIVED', 'video_second': second,
                                   'frame': f'combat-frame-{second}.jpg',
                                   'basis': 'in-world name and sprite compared with decoded client artwork'}}
                 for name, look, sha, first, second in POOL],
        'room': {'name': 'Thais', 'coordinates': 'GLOBAL_XYZ',
                 'qualification': 'DERIVED_FROM_PINNED_MINIMAP_AND_ORIGINAL_VIDEO',
                 'full_donor_otbm': False, 'battle_cells': battle_cells(),
                 'battle_exit_marker': [32295, 32178, 12],
                 'lever': [32289, 32198, 12],
                 'entry_pads': [[32289, y, 12] for y in range(32199, 32204)],
                 'lobby_portal_marker': [32285, 32205, 11],
                 'stair_markers': [[32291, 32208, 11], [32291, 32208, 12]],
                 'return_anchor': 'UNKNOWN', 'first_admission_position': 'UNKNOWN',
                 'tile_items_flags_and_actions': 'UNKNOWN',
                 'other_city_rooms': 'UNKNOWN',
                 'excluded': 'floor-11 spectator roof, north exit, south barriers and waiting pads'},
        'progression': {
            'classification': 'PROVEN_HISTORICAL_GAMEPLAY', 'wave_numbering': 'DISPLAYED_UI_ONE_BASED',
            'observations': [{'video_second': second, 'frame': f'combat-frame-{second}.jpg',
                              'previous_displayed_wave': previous, 'skipped': skipped,
                              'next_displayed_wave': following}
                             for second, previous, skipped, following in ((400, 1, 3, 5), (427, 5, 2, 8), (489, 11, 1, 13))],
            'skip_threshold_seconds': 'UNKNOWN', 'skipped_wave_score': 'UNKNOWN',
            'current_version_continuity': 'UNKNOWN',
            'observation_does_not_amend_accepted_oteryn_scoring': True},
        'unknown_native_profile_fields': ['server_speed', 'attack_intervals_and_chances',
                                          'hoodinion_two_ice_wave_tile_masks', 'hoodinion_chain_falloff',
                                          'scissorion_critical_probability_and_factor'],
        'existing_consumers': {
            'presentation_authoring': 'tools/content-schema/native-gameplay/build_canonical_monster_profiles.py::union_profiles',
            'room_cells': 'crates/gameplay/entities/src/carrier/tibiadrome_modifiers.rs::DromeModifierState::new',
            'public_guard': 'crates/gameplay/dispatch/src/gameplay_transport/tibiadrome/modifiers.rs::qualified_context',
            'requirements': 'Complete exact Creature/Behavior/Presentation closure, qualified World tiles and entry/return ownership remain required; no partial native profile is emitted.'},
    }


def validate_profile(document):
    # An immutable bounded source packet has a closed schema at every depth.
    # Canonical bytes also distinguish JSON booleans from integer coordinates.
    if canonical(document) != canonical(expected_profile()):
        raise ValueError('DROME_SOURCE_PROFILE_BODY')


def battle_cells_from_crop(crop):
    if crop.size != (33, 55):
        raise ValueError('DROME_SOURCE_ROOM_CROP_SIZE')
    cells = []
    for y in range(32179, 32195):
        for x in range(32283, 32299):
            red, green, blue = crop.getpixel((x - 32275, y - 32165))
            if red == green == blue and red > 0:
                cells.append([x, y, 12])
    if cells != battle_cells():
        raise ValueError('DROME_SOURCE_ROOM_MASK')
    return cells


def verify_maps(directory):
    crops = {}
    for name, sha in MAP_HASHES.items():
        raw = (directory / name).read_bytes()
        verify_bytes(raw, sha, name)
        with Image.open(io.BytesIO(raw)) as source:
            if source.size != (2560, 2048):
                raise ValueError('DROME_SOURCE_MAP_DIMENSIONS')
            crop = source.convert('RGB').crop((531, 1189, 564, 1244))
        verify_bytes(crop.tobytes(), CROP_HASHES[name.split('-', 1)[1]], name + ':crop')
        crops[name] = crop
    for variant in MAP_REVISIONS:
        battle_cells_from_crop(crops[f'{variant}-floor-12-path.png'])
    # Exact full-file pins and crop pins prove continuity, not server tile flags.
    for name in CROP_HASHES:
        if crops['target-' + name].tobytes() != crops['current-' + name].tobytes():
            raise ValueError('DROME_SOURCE_MAP_CONTINUITY')


def verify_catalog(raw):
    verify_bytes(raw, CATALOG_SHA, 'client-catalog')
    rows = decode_json(raw)
    if [row for row in rows if row.get('type') == 'appearances'] != [
            {'type': 'appearances', 'file': f'appearances-{CLIENT_APPEARANCE_SHA}.dat'}]:
        raise ValueError('DROME_SOURCE_CATALOG_APPEARANCES')
    for name, _, first, last in SPRITES:
        filename = f'sprites-{name}.bmp.lzma'
        if [row for row in rows if row.get('file') == filename] != [
                {'type': 'sprite', 'file': filename, 'spritetype': 3,
                 'firstspriteid': first, 'lastspriteid': last, 'area': 0}]:
            raise ValueError('DROME_SOURCE_CATALOG_BINDING:' + filename)


def verify_appearances(outfits, manifest, sprites, catalog):
    outfit_raw, manifest_raw = outfits.read_bytes(), manifest.read_bytes()
    verify_bytes(outfit_raw, OUTFITS_SHA, 'outfit-shard')
    verify_bytes(manifest_raw, MANIFEST_SHA, 'client-manifest')
    rows = decode_json(outfit_raw)['records']
    client_files = decode_json(manifest_raw)['files']
    files = {r['name']: r for r in client_files}
    if len(files) != len(client_files):
        raise ValueError('DROME_SOURCE_MANIFEST_DUPLICATE')
    catalog_raw = catalog.read_bytes()
    if files.get('catalog-content.json') != {
            'name': 'catalog-content.json', 'sha256': CATALOG_SHA, 'bytes': len(catalog_raw)}:
        raise ValueError('DROME_SOURCE_CATALOG_MANIFEST')
    verify_catalog(catalog_raw)
    for name, sha, _, _ in SPRITES:
        filename = f'sprites-{name}.bmp.lzma'
        record = files.get(filename, {})
        raw = (sprites / filename).read_bytes()
        if record.get('sha256') != sha or record.get('bytes') != len(raw):
            raise ValueError('DROME_SOURCE_SPRITE_MANIFEST:' + filename)
        verify_bytes(raw, sha, filename)
    for _, look, sha, first, _ in POOL:
        selected = [r for r in rows if r.get('source_id') == look]
        if len(selected) != 1 or selected[0].get('source_record_sha256') != sha:
            raise ValueError('DROME_SOURCE_APPEARANCE_RECORD')
        groups = selected[0]['frame_groups']
        if ([(g['fixed_frame_group'], g['sprite_ids']) for g in groups]
                != [(0, list(range(first, first + 4))), (1, list(range(first + 4, first + 36)))]):
            raise ValueError('DROME_SOURCE_APPEARANCE_SPRITES')
        if any(sum(start <= sprite <= end for _, _, start, end in SPRITES) != 1
               for sprite in range(first, first + 36)):
            raise ValueError('DROME_SOURCE_SPRITE_COVERAGE')


def capture(outfits, manifest, maps, sprites, catalog=ROOT / CATALOG):
    verify_appearances(outfits, manifest, sprites, catalog)
    verify_maps(maps)
    document = expected_profile()
    validate_profile(document)
    return document


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=('capture', 'check'))
    parser.add_argument('--outfits', type=Path, default=ROOT / OUTFITS)
    parser.add_argument('--client-manifest', type=Path, default=ROOT / MANIFEST)
    parser.add_argument('--catalog', type=Path, default=ROOT / CATALOG)
    parser.add_argument('--maps', type=Path, required=True)
    parser.add_argument('--sprites', type=Path, required=True)
    parser.add_argument('--frames', type=Path, help='Also recheck the retained manually inspected frame bytes')
    parser.add_argument('--output', type=Path, default=ROOT / OUTPUT)
    args = parser.parse_args(argv)
    document = capture(args.outfits, args.client_manifest, args.maps, args.sprites, args.catalog)
    if args.frames:
        for name, sha in FRAMES.items():
            verify_bytes((args.frames / name).read_bytes(), sha, name)
    raw = canonical(document)
    if args.operation == 'check':
        retained = args.output.read_bytes()
        validate_profile(decode_json(retained))
        if retained != raw:
            raise ValueError('DROME_SOURCE_OUTPUT_DRIFT')
    else:
        args.output.write_bytes(raw)
    print('PASS Tibiadrome source inputs: five looks, 252 Thais cells; runtime admission remains false')


if __name__ == '__main__':
    main()
