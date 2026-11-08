#!/usr/bin/env python3
"""SRX-3: qualify exact Soul War/Rotten Blood corpse decay closures natively.

Canonical reference-only shards remain unchanged. Replay uses pinned Crystal XML,
appearances and ItemType loader defaults through the existing source Item decoder.
"""
from __future__ import annotations
import argparse
import copy
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from build_spell_item_identities import binding_index

PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
PATH = 'tools/content-schema/native-gameplay/boss-corpse-item-profiles.json'
PIN_PATH = 'tools/content-schema/native-gameplay/boss-corpse-item-profiles-pin.json'
SCHEMA = 'OTERYN_NATIVE_BOSS_CORPSE_ITEM_PROFILES/v1'
ROOT_IDS = frozenset({39344,39530,23564,111,5973,5993,5995,6028,6081,23478,32610,33792,33797,33801,33805,33809,33813,33817,33859,33863,33867,33871,33875,33876,33889,33891,33897,33901,33905,33909,33949,33984,34121,34125,34133,43555,43559,43567,43571,43579,43583,43669,43816,43820,43824,43828,43832,43836,43840,43844,44012,44015,44018,44021,44024})
SOURCE_HASHES = {
    'data/items/appearances.dat': '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1',
    'data/items/items.xml': '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509',
    'src/items/items.hpp': '06d52ee83250ba852fe804c6b006b0aac150ceb26893e9079fa8c93502d209ca',
    'src/items/items.cpp': 'f39d8fc6c1c81b8ec6f749e1f152dd945472ffd5a36b9cf3cd9a3a64b2f19b1d',
}


def encode(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))+'\n').encode()


def load_source_blobs(cache: Path):
    blobs = {path: (cache / Path(path).name).read_bytes() for path in SOURCE_HASHES}
    for path, raw in blobs.items():
        if hashlib.sha256(raw).hexdigest() != SOURCE_HASHES[path]:
            raise ValueError(f'BOSS_CORPSE_SOURCE_DIGEST:{path}')
    if not all(token in blobs['src/items/items.hpp'] for token in
               (b'int32_t weight = 0;', b'uint16_t maxItems = 8;', b'bool stopTime = false;')):
        raise ValueError('BOSS_CORPSE_LOADER_DEFAULTS')
    return blobs


def xml_source_lines(raw, wanted):
    found = {}
    for number, line in enumerate(raw.decode().splitlines(),1):
        match = re.search(r'<item\s+id="([0-9]+)"',line)
        if match and int(match[1]) in wanted:
            if match[1] in found:raise ValueError('BOSS_ITEM_DUPLICATE_SOURCE_LINE')
            found[match[1]] = number
    return found


CARRIED_CORPSE_STAGES = frozenset({4094,4158,4159,4242,4246,33891})

def corpse_admission(item_id):
    return {'materializable':True,'stack_class':'NonStackable','legal_destinations':(['CharacterInventory','Ground'] if item_id in CARRIED_CORPSE_STAGES else ['Ground'])}


def complete_source_top_order(record, appearance):
    # Crystal 201189f items.cpp:203-208; items.hpp:343 defaults to zero.
    # The pinned DAT flags also own the stack order composed by House.
    # Capture those facts here so complete-body qualification stays exact.
    top = next((order for flag, order in [('clip', 1), ('top', 3), ('bottom', 2)]
                if appearance['flags'].get('flags.' + flag)), 0)
    record['attributes']['always_on_top'] = top != 0
    record['attributes']['always_on_top_order'] = top


def capture(cache: Path, root: Path = ROOT):
    blobs = load_source_blobs(cache)
    xml = source.xml_index(blobs['data/items/items.xml'])
    bindings = binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
    import boss_corpse_terminal_reference as terminal
    packet = terminal.checked_packet(root)
    if '43666' in bindings:raise ValueError('ELDER_TERMINAL_ALREADY_PROTECTED')
    bindings['43666'] = packet['bindings'][0]
    chains, wanted = [], set()
    for first in sorted(ROOT_IDS):
        chain, item_id = [], first
        while item_id:
            if len(chain) >= 8 or item_id in chain or (item_id not in xml and item_id != 43666):
                raise ValueError(f'BOSS_CORPSE_DECAY_CLOSURE:{first}')
            chain.append(item_id)
            if item_id == 43666 and (first != 43669 or chain != [43669,43668,43667,43666]):
                raise ValueError('ELDER_TERMINAL_WRONG_SOURCE_CHAIN')
            attributes = source.attrs(xml[item_id]) if item_id in xml else {}
            item_id = int(attributes.get('decayto', '0'))
        chains.append(chain)
        wanted.update(chain)
    if len(wanted) != 199 or any(str(item_id) not in bindings for item_id in wanted):
        raise ValueError('BOSS_CORPSE_EXACT_BOUND_SET')
    objects = source.engine_items.load_appearance_objects(blobs['data/items/appearances.dat'])
    physics = {row['server_item_id']: row for row in source.policies(
        blobs['data/items/appearances.dat'], blobs['data/items/items.xml'], wanted)}
    records = []
    for item_id in sorted(wanted):
        record = source.profile(item_id, xml.get(item_id), objects[item_id], physics[item_id], bindings, None)
        if item_id == 43666:
            complete_source_top_order(record, objects[item_id])
        if record['admission'] != corpse_admission(item_id):
            raise ValueError(f'BOSS_CORPSE_GROUND_ONLY:{item_id}')
        binding = bindings[str(item_id)]
        if binding['disposition'] != 'EXACT':
            raise ValueError(f'BOSS_CORPSE_BINDING:{item_id}')
        record['authoring']['item']['revision'] = 'srx3-boss-corpses-crystal-201189f'
        record['production_binding'] = copy.deepcopy(binding)
        record['production_binding_qualification'] = copy.deepcopy(packet) if item_id == 43666 else None
        record['production_definition'] = {'family':'Item', 'production_key':binding['target']['key'], 'revision_ref':binding['target']['revision']}
        records.append(record)
    return {'schema':SCHEMA, 'producer':'tools/content-migration/boss_corpse_profiles.py',
            'source_revision':PIN, 'sources':[{'repository':'zimbadev/crystalserver','revision':PIN,'path':path,'sha256':digest} for path,digest in {**SOURCE_HASHES, **terminal.EXTRA_SOURCES}.items()],
            'roots':sorted(ROOT_IDS), 'decay_chains':chains, 'xml_source_lines':xml_source_lines(blobs['data/items/items.xml'],wanted), 'records':records}


def admit(root, generated, destination, manifest_path, order, canonical):
    raw = (root/PATH).read_bytes()
    pin = json.loads((root/PIN_PATH).read_bytes())
    if pin != {'path':PATH, 'sha256':hashlib.sha256(raw).hexdigest()}:
        raise ValueError('BOSS_CORPSE_CAPTURE_DRIFT')
    data = json.loads(raw)
    if data['schema'] != SCHEMA or data['source_revision'] != PIN or data['roots'] != sorted(ROOT_IDS):
        raise ValueError('BOSS_CORPSE_SOURCE_SET')
    import boss_corpse_terminal_reference as terminal
    packet = terminal.checked_packet(root)
    expected_sources = [{'repository':'zimbadev/crystalserver','revision':PIN,'path':path,'sha256':digest} for path,digest in {**SOURCE_HASHES, **terminal.EXTRA_SOURCES}.items()]
    if data['sources'] != expected_sources or len(data['records']) != 199:
        raise ValueError('BOSS_CORPSE_SOURCE_AUTHORITY')
    bindings = binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
    if '43666' in bindings:raise ValueError('ELDER_TERMINAL_ALREADY_PROTECTED')
    bindings['43666'] = packet['bindings'][0]
    wanted = {item for chain in data['decay_chains'] for item in chain}
    if len(wanted) != 199 or {chain[0] for chain in data['decay_chains']} != ROOT_IDS:
        raise ValueError('BOSS_CORPSE_DECAY_SET')
    document = json.loads(generated[destination])
    existing = {row['production_definition']['production_key']:row for row in document['records']}
    seen = set()
    for record in data['records']:
        binding = record['production_binding']
        item_id = int(binding['external_id'])
        key = record['production_definition']['production_key']
        if (item_id not in wanted or item_id in seen or binding != bindings.get(str(item_id))
                or binding['disposition'] != 'EXACT'
                or record['production_binding_qualification'] != (packet if item_id == 43666 else None)
                or record['production_definition'] != {'family':'Item','production_key':binding['target']['key'],'revision_ref':binding['target']['revision']}
                or record['admission'] != corpse_admission(item_id)):
            raise ValueError(f'BOSS_CORPSE_NATIVE_AUTHORITY:{key}')
        seen.add(item_id)
        if key in existing:
            previous = existing[key]
            expected = copy.deepcopy(record)
            # Gathering already owns these two exact source bodies. Share the
            # complete policy and retain that existing authored reference; an
            # overlapping key alone never authorizes another metadata value.
            if item_id in (5973,4026,4240,4241,4242,4246,4247,4248,5995,4098):
                expected['authoring']['item']['revision'] = 'spell-p2-r21'
            # NativeItemAttributes::spell_augments is serde-default empty and
            # omitted when empty (native_gameplay.rs331-333). Keep the exact
            # pre-existing six Player-body records; no nonempty augment differs.
            if item_id in (4240,4241,4242,4246,4247,4248) and 'spell_augments' not in previous['attributes'] and expected['attributes'].get('spell_augments') == []:
                del expected['attributes']['spell_augments']
            if previous != expected:
                raise ValueError(f'BOSS_CORPSE_PROFILE_CONFLICT:{key}')
        else:
            document['records'].append(record)
    document['records'].sort(key=order)
    generated[destination] = canonical(document)
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(generated[destination]).hexdigest()
    generated[manifest_path] = canonical(manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-cache', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    import boss_corpse_terminal_reference as terminal
    terminal.publish(args.source_cache, ROOT, args.check)
    raw = encode(capture(args.source_cache))
    pin = encode({'path':PATH, 'sha256':hashlib.sha256(raw).hexdigest()})
    for path, content in ((PATH,raw),(PIN_PATH,pin)):
        if args.check:
            if (ROOT/path).read_bytes() != content:
                raise SystemExit(f'BOSS_CORPSE_GENERATED_DRIFT:{path}')
        else:
            (ROOT/path).write_bytes(content)
    rust=('// Generated only by tools/content-migration/boss_corpse_profiles.py.\n'
        'pub(super) const CAPTURE_SHA256: &str =\n    \"'+hashlib.sha256(raw).hexdigest()+'\";\n'
        'pub(super) const PACKET_SHA256: &str =\n    \"'+hashlib.sha256((ROOT/terminal.PACKET).read_bytes()).hexdigest()+'\";\n').encode()
    if args.check:
        if (ROOT/terminal.RUST_PIN).read_bytes()!=rust:raise ValueError('ELDER_TERMINAL_RUST_PIN_DRIFT')
    else:(ROOT/terminal.RUST_PIN).write_bytes(rust)
    print('BOSS_CORPSE_NATIVE_SOURCE_PROFILES', len(json.loads(raw)['records']), 'CHECK' if args.check else 'GENERATED')

if __name__ == '__main__':
    main()
