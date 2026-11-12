"""Restore Cake Golem's source-complete bundle through the normal Creature stage.

The retained source capture has the same Lua blob as Crystal 201189f. Its only
unresolved entry is the formerly unrepresented Undefined combat family. This
producer verifies that provenance and restores that exact entry; it does not
admit the separately unavailable Cake bridge Item 48227.
"""
import argparse
import hashlib
import json
import sys
from pathlib import Path

from creature_admission_stage import Mapper, Stage
from compact_record_json import encode

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/monster-authoring'))
import validate_monster

PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
BASE = ROOT / 'docs/reference/content-prep/prey-content-1/cake-golem'
SOURCE = 'data-global/monster/quests/a_piece_of_cake/cake_golem.lua'
SOURCE_HASHES = {
    SOURCE: '48471262a6d87ca938218890cc10f3839bd072dbf9f71301e63df155933595cf',
    'src/creatures/monsters/monsters.hpp': '24c5b9830d89c4150ccdaecf12f6a5d892e926b5e81c1f812d0081df32220426',
    'src/creatures/creatures_definitions.hpp': '8fbc70bf2416b6e76f17cce3dc1b075f8a5451f0f85b522b0a4ae34cf2cccf94',
    'src/game/game.cpp': '5f7ba67a158af607bacb9d75451ca490817f189ca7d3dcf9e774d55dc1ccaab6',
    'src/items/functions/item/item_parse.cpp': '08687bc0e096ebfcce55daa6a523b78bca0191f25ce4134c133927d4a10dfce7',
}
BASE_HASHES = {
    'monster.json': '65a7bec5cc5134aed1e0fcfa40a3ffe8eb10bb48057f19103c5c4d30df76aa70',
    'dependencies.json': '93ddf83ef183b9ed2c0299f4139a814ff165a9a2899860ff6a888bfb90c9baf6',
    'catalog.json': '8c1c7a5aa49afd0f78964c5b4707d09772658a861ee70f32a74ee533cf7bdae5',
    'manifest.json': '10736aa2a4c891a4340e798bb0eb6a0b89dd10e30183aab314159de3aa678a24',
}
OUTPUT = 'tools/content-schema/native-gameplay/cake-creature-admission.json'
PIN_OUTPUT = 'tools/content-schema/native-gameplay/cake_creature_admission_pin.rs'


def complete_bundle():
    bundle = {}
    for name, expected in BASE_HASHES.items():
        raw = (BASE / name).read_bytes()
        if hashlib.sha256(raw).hexdigest() != expected:
            raise ValueError('CAKE_BASE_CAPTURE_DRIFT:' + name)
        bundle[name] = json.loads(raw)
    monster, deps = bundle['monster.json'], bundle['dependencies.json']
    revision = 'canary-47dfd51f'

    def identity(kind, suffix):
        return {'key': f'canary:{kind}/cake_golem/{suffix}', 'revision': revision}

    def reference(kind, suffix):
        return {'family': kind.title(), **identity(kind, suffix)}

    # Crystal Cake Golem:79 and monsters.hpp:244: omitted combat type is
    # Undefined, with a 0..40 range, not an Earth inference from poison visuals.
    attack = reference('ability', 'attack-2')
    monster['behavior']['attacks'].insert(1, {
        'ability': attack, 'interval_ms': 2000, 'chance_percent': 15})
    deps['abilities'].append({'identity': identity('ability', 'attack-2'), 'kind': 'spell',
                             'range_tiles': 7, 'needs_target': True, 'needs_direction': False,
                             'effects': [reference('effect', 'attack-2')]})
    deps['effects'].append({'identity': identity('effect', 'attack-2'), 'operation': 'damage',
                           'damage_type': 'undefined', 'formula': reference('formula', 'attack-2-1'),
                           'presentation': {'projectile_asset_binding': 'canary.appearance:missile/cake',
                                            'impact_asset_binding': 'canary.appearance:effect/poisonarea'}})
    deps['formulas'].append({'identity': identity('formula', 'attack-2-1'), 'kind': 'range',
                            'magnitude': {'minimum': 0, 'maximum': 40}})
    for field in ('abilities', 'effects', 'formulas'):
        deps[field].sort(key=lambda row: row['identity']['key'])
    bundle['catalog.json']['assets'].append('canary.appearance:effect/poisonarea')
    bundle['catalog.json']['assets'] = sorted(set(bundle['catalog.json']['assets']))
    manifest = bundle['manifest.json']
    manifest['sources'][0]['revision'] = PIN
    entry, = [row for row in manifest['entries'] if row.get('source_field') == 'attacks[2]']
    entry.update(status='mapped', destination='/monster/behavior/attacks/1', resolution=(
        'Crystal src/creatures/monsters/monsters.hpp:244 defaults omitted type to Undefined; '
        'src/creatures/creatures_definitions.hpp:805 names family4; src/game/game.cpp:8142 '
        'bypasses mana shield; src/items/functions/item/item_parse.cpp:534 excludes item all-absorb.'))
    errors = validate_monster.validate(monster, deps, bundle['catalog.json'], manifest)
    if errors:
        raise ValueError('CAKE_BUNDLE_NOT_CLOSED:' + '; '.join(errors[:8]))
    return bundle


def stage_bundle(bundle, item_map):
    stage = Stage(Mapper(item_map))
    stage.stage_dependencies(bundle['dependencies.json'], 'cake_golem')
    stage.stage_monster(bundle['monster.json'], SOURCE, {
        'source_key': 'oteryn:source.crystalserver',
        'source_revision': 'crystalserver-creature-1530:' + PIN,
        'identity_namespace': 'crystalserver/monster-file', 'external_id': SOURCE})
    return {'schema': 'OTERYN_CREATURE_ADMISSION_STAGED/v1',
            'records': [value for _, value in sorted(stage.records.items())],
            'authoring_profiles': [value for _, value in sorted(stage.profiles.items())],
            'source_identity_bindings': stage.bindings}


def capture(crystal):
    sources = []
    for path, expected in SOURCE_HASHES.items():
        raw = (crystal / path).read_bytes()
        if hashlib.sha256(raw).hexdigest() != expected:
            raise ValueError('CAKE_DONOR_SOURCE_DRIFT:' + path)
        sources.append({'path': path, 'sha256': expected})
    raw = (crystal / SOURCE).read_bytes()
    blob = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
    baseline = json.loads((BASE / 'validation.json').read_bytes())
    if blob != baseline['source']['git_blob']:
        raise ValueError('CAKE_SOURCE_NOT_IDENTICAL_TO_CAPTURE')
    bindings = json.loads((ROOT / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    reference = json.loads((ROOT / 'content/world/definitions/reference.json').read_bytes())
    items = {(row['identity']['key'], row['identity']['revision'])
             for row in reference['records'] if row['identity']['family'] == 'Item'}
    item_map = {int(row['external_id']): row['target']['key'] for row in bindings
                if row['disposition'] == 'EXACT' and row['external_id'].isdecimal()
                and row['target']['family'] == 'Item'
                and (row['target']['key'], row['target']['revision']) in items}
    bundle = complete_bundle()
    return {'schema': 'OTERYN_SEASONAL_CREATURE_ADMISSION/v1', 'crystal_commit': PIN,
            'sources': sources, 'source_blob': blob, 'qualification': 'OTS_HYPOTHESIS_ONLY',
            'capture_tool_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            'stage_tool_sha256': hashlib.sha256((ROOT / 'tools/content-migration/creature_admission_stage.py').read_bytes()).hexdigest(),
            'bundle': bundle, 'staged': stage_bundle(bundle, item_map)}


def pin_output(packet, raw):
    counts = [('RECORD_COUNT', len(packet['staged']['records'])),
              ('PROFILE_COUNT', len(packet['staged']['authoring_profiles']))]
    text = '// Generated by tools/content-migration/cake_creature_admission.py. Do not edit.\n'
    text += f'const SEASONAL_CAKE_PACKET_SHA256: &str = "{hashlib.sha256(raw).hexdigest()}";\n'
    for suffix, count in counts:
        text += f'const SEASONAL_CAKE_{suffix}: usize = {count};\n'
    return text.encode()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    packet = capture(args.crystal)
    raw = encode(packet)
    for path, data in [(OUTPUT, raw), (PIN_OUTPUT, pin_output(packet, raw))]:
        target = ROOT / path
        if args.check:
            if target.read_bytes() != data:
                raise SystemExit('CAKE_ADMISSION_PACKET_DRIFT:' + path)
        else:
            target.write_bytes(data)
