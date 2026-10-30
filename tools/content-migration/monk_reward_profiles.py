"""Qualify the seven original Monk rewards through the existing native Item importer."""
import argparse
import copy
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from build_spell_item_identities import XML_SHA, CRYSTAL_PIN

PATH = 'tools/content-schema/native-gameplay/monk-reward-profiles.json'
IDS = (50267, 50268, 50269, 50271, 50272, 50273, 50274)


def capture(canary, crystal):
    xml = (canary / 'data/items/items.xml').read_bytes()
    appearances = (canary / 'data/items/appearances.dat').read_bytes()
    other = (crystal / 'data/items/items.xml').read_bytes()
    proof = json.loads((ROOT / 'tools/content-schema/native-gameplay/canary-ground-proof.json').read_bytes())
    appearance_hash = next(r['sha256'] for r in proof['sources'] if r['path'] == 'data/items/appearances.dat')
    if (hashlib.sha256(xml).hexdigest() != XML_SHA['canary']
            or hashlib.sha256(other).hexdigest() != XML_SHA['crystal']
            or hashlib.sha256(appearances).hexdigest() != appearance_hash):
        raise ValueError('MONK_REWARD_SOURCE_PIN')
    index, other_index = source.xml_index(xml), source.xml_index(other)
    bindings = source.binding_index(source.BINDINGS.read_bytes())
    objects = source.engine_items.load_appearance_objects(appearances)
    physical = {r['server_item_id']: r for r in source.policies(appearances, xml, set(IDS))}
    canonical = {}
    canonical_inputs = []
    for path in sorted((ROOT / 'content/items/definitions').glob('items-*.json')):
        selected = {r['definition']['identity']['key']: r['definition']
                    for r in json.loads(path.read_bytes())['records']
                    if r['definition']['identity']['key'] in {f'oteryn:item.tibia.i{i}' for i in IDS}}
        if selected:
            canonical.update(selected)
            canonical_inputs.append({'path':path.relative_to(ROOT).as_posix(),
                                     'sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
    records = []
    for item in IDS:
        # Both pinned donors name the same reward with the same physical weight.
        if (index[item].get('name') != other_index[item].get('name')
                or source.attrs(index[item]).get('weight') != source.attrs(other_index[item]).get('weight')):
            raise ValueError('MONK_REWARD_DONOR_DRIFT')
        row = source.profile(item, index[item], objects[item], physical[item], bindings, None)
        if (row['admission'] != {'materializable': True, 'stack_class': 'NonStackable',
                                'legal_destinations': ['CharacterInventory', 'Ground']}
                or row['semantics']['container'] != source.NA or row['semantics']['temporal'] != source.NA):
            raise ValueError('MONK_REWARD_NOT_A_PORTABLE_LEAF')
        # Keep already qualified Global armor, imbuement and Monk modifiers.
        # The generic XML importer deliberately leaves these groups unknown.
        existing = canonical[f'oteryn:item.tibia.i{item}']['semantics']
        for group in ('equipment', 'weapon', 'protection', 'skill_modifiers', 'imbuement'):
            if existing[group]['state'] == 'KNOWN' and (row['semantics'][group]['state'] == 'UNKNOWN'
                    or group in ('protection', 'skill_modifiers', 'imbuement')):
                row['semantics'][group] = copy.deepcopy(existing[group])
        records.append(row)
    bound = source.bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':records}, bindings, set(IDS))
    return {'schema':'OTERYN_NATIVE_MONK_REWARD_PROFILES/v1',
            'producer':'tools/content-migration/monk_reward_profiles.py',
            'source_revisions':{'canary':source.CANARY_PIN, 'crystal':CRYSTAL_PIN},
            'canonical_inputs':canonical_inputs,
            'sources':[{'path':path,'sha256':hashlib.sha256(raw).hexdigest()} for path,raw in
                       [('canary/data/items/items.xml',xml),('canary/data/items/appearances.dat',appearances),
                        ('crystal/data/items/items.xml',other)]], 'records':bound['records']}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    for descriptor in overlay['canonical_inputs']:
        if hashlib.sha256((root / descriptor['path']).read_bytes()).hexdigest() != descriptor['sha256']:
            raise ValueError('MONK_REWARD_CANONICAL_DRIFT')
    if (overlay['schema'] != 'OTERYN_NATIVE_MONK_REWARD_PROFILES/v1'
            or [r['production_definition']['production_key'] for r in records] != [f'oteryn:item.tibia.i{i}' for i in IDS]):
        raise ValueError('MONK_REWARD_PROFILE_SET')
    document = json.loads(generated[profile_path])
    keys = {r['production_definition']['production_key'] for r in document['records']}
    if any(r['production_definition']['production_key'] in keys for r in records):
        raise ValueError('MONK_REWARD_PROFILE_DUPLICATE')
    document['records'] = sorted(document['records'] + records, key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canary',type=Path,required=True)
    parser.add_argument('--crystal',type=Path,required=True)
    parser.add_argument('--check',action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(capture(args.canary,args.crystal), indent=2, ensure_ascii=False) + '\n').encode()
    path = ROOT / PATH
    if args.check:
        if path.read_bytes() != raw: raise SystemExit('MONK_REWARD_PROFILE_DRIFT')
    else: path.write_bytes(raw)
