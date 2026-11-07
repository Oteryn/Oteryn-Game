"""Explicit Oteryn plain collectible policies for selected personal quest rewards.

This is an Oteryn identity namespace, never a fabricated donor server binding.
The retained client and canonical Known facts qualify identity and portability.
The selected programme supplies a documented carrying weight where no source
does. Nothing here admits an action, charge, timer, equipment or container.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CLIENT_SHA = '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'
CLIENT = f'content/assets/files/appearances-{CLIENT_SHA}.dat'
PACKET = 'tools/content-schema/native-gameplay/quest-collectible-bindings.json'
PIN = 'crates/gameplay/content/src/content/native_gameplay/quest_collectible_pin.rs'
# Rule: a plain, once-reward, nonstacking collectible uses a conservative10oz
# carrying cost until an accepted exact weight is available. No use is inferred.
SELECTIONS = [(44179, 'bottled dragon breath', 1000,
               'oteryn:quest.authored.nimmersatt_s_lair',
               'content/quests/definitions/quests-00300-00351.json')]


def encoded(value):
    return (json.dumps(value, indent=2, ensure_ascii=False) + '\n').encode()


def capture(root=ROOT):
    sys.path.insert(0, str(root / 'tools/content-schema/item-authoring'))
    import engine_items
    import grizzly_shop_profiles as accepted
    raw = (root / CLIENT).read_bytes()
    if hashlib.sha256(raw).hexdigest() != CLIENT_SHA:
        raise ValueError('QUEST_COLLECTIBLE_CLIENT_PIN')
    objects = engine_items.load_appearance_objects(raw)
    selected = {f'oteryn:item.tibia.i{item}' for item, *_ in SELECTIONS}
    definitions = {}
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        source = path.read_bytes()
        for row in json.loads(source)['records']:
            definition = row['definition']
            key = definition['identity']['key']
            if key in selected:
                if key in definitions:
                    raise ValueError('QUEST_COLLECTIBLE_DUPLICATE_CANONICAL')
                definitions[key] = (definition, str(path.relative_to(root)), hashlib.sha256(source).hexdigest())
    entries = []
    known = lambda value: dict(state='KNOWN', value=value)
    na = dict(state='NOT_APPLICABLE')
    for item, name, weight, quest, path in SELECTIONS:
        key = f'oteryn:item.tibia.i{item}'
        definition, canonical_path, canonical_sha = definitions[key]
        obj = objects[item]
        flags = obj['flags']
        if (obj.get('name', '').lower() != name or flags.get('flags.take') is not True
                or any(flags.get(flag, False) for flag in
                       ['flags.unmove', 'flags.cumulative', 'flags.container',
                        'flags.liquidcontainer', 'flags.liquidpool'])):
            raise ValueError('QUEST_COLLECTIBLE_CLIENT_CUSTODY:' + key)
        recipe_raw = (root / path).read_bytes()
        recipe, = [row['definition']['recipe'] for row in json.loads(recipe_raw)['records']
                   if row['definition']['identity']['key'] == quest]
        rewards = [r for r in recipe['reward_intents'] if r.get('kind') == 'item' and r['item']['key'] == key]
        if len(rewards) != 1 or rewards[0]['count'] != 1 or recipe['repeat']['kind'] != 'once':
            raise ValueError('QUEST_COLLECTIBLE_NOT_SELECTED_ONCE_REWARD:' + key)
        proposed = copy.deepcopy(definition['semantics'])
        proposed['physical'] = known(dict(weight=known(weight),movable=known(True),pickupable=known(True)))
        proposed['stack'] = known(dict(stackable=known(False),stack_max=na))
        for group in ['container','charges','temporal']:
            proposed[group] = na
        semantics = accepted.accepted_first(definition['semantics'], proposed)
        profile = dict(authoring=dict(item=definition['identity']),semantics=semantics,
            attributes=dict(speed_bonus=0,blocks_movement=False,blocks_projectile=False,
                immovable_block_solid=False,field_condition=None,rune_consumption=None,
                field_replaceable=None,has_height=False),
            admission=dict(materializable=True,stack_class='NonStackable',
                legal_destinations=['CharacterInventory','Ground']),
            production_definition=dict(family='Item',production_key=key,revision_ref=definition['identity']['revision']),
            production_binding=dict(source_key='oteryn:source.oteryn',source_revision='quest-collectibles-v1',
                identity_namespace='oteryn/quest_collectible_id',external_id=str(item),
                target=definition['identity'],disposition='EXACT'),production_binding_qualification=None)
        entries.append(dict(profile=profile,witnesses=[
            f'{canonical_path}:sha256:{canonical_sha}',f'{path}:quest:{quest}:sha256:{hashlib.sha256(recipe_raw).hexdigest()}',
            f'{CLIENT}:sha256:{CLIENT_SHA}'],
            assumption='Oteryn plain collectible carrying weight10oz when exact source weight is unknown; no action, charge, timer or container.'))
    return dict(schema=1,entries=entries)


def proof_outputs(root=ROOT):
    raw = encoded(capture(root))
    digest = hashlib.sha256(raw).hexdigest()
    pin = ('// Generated by tools/content-migration/quest_collectible_profiles.py.\n'
           f'pub(super) const SHA256: &str = "{digest}";\n').encode()
    return {PACKET: raw, PIN: pin}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    outputs = proof_outputs(root)
    packet = json.loads(outputs[PACKET])
    document = json.loads(generated[profile_path])
    indexed = {row['production_definition']['production_key']: row for row in document['records']}
    externals = {row['production_binding']['external_id'] for row in document['records']}
    for entry in packet['entries']:
        row = entry['profile']
        key = row['production_definition']['production_key']
        if key in indexed or row['production_binding']['external_id'] in externals:
            raise ValueError('QUEST_COLLECTIBLE_EXISTING_NATIVE_CONFLICT:' + key)
        indexed[key] = row
    document['records'] = sorted(indexed.values(), key=order)
    raw = encoded(document)
    generated.update(outputs)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for path, raw in proof_outputs().items():
        target = ROOT / path
        if args.check:
            if not target.is_file() or target.read_bytes() != raw:
                raise ValueError('QUEST_COLLECTIBLE_PROOF_DRIFT:' + path)
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(raw)
