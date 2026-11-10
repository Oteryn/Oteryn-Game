"""Bound Newhaven item overlay; historical starter definitions remain independent.

Current Wiki item identities and portable/equipment facts win over Crystal's
older server IDs. Wiki revision IDs below are the exact source cuts selected by
QUESTPC. The sole appearance translation is explicit: current Wiki Simple Jo
Staff Wiki Item51166 names current15.30 appearance51119, `simple jo staff`,
take/clothes, sprite239746. Crystal201189f8 independently agrees on appearance
51119 (sprite238626). Appearance51166 is an unrelated immovable object. Existing
Game identity and presentation bindings retain i51119; Wiki external ID51166
is a separate namespace.

Viola: Newhaven Quest/Spoiler111294/revid1205117 selects Book51977 -> Scarf3572
and800 XP. This module supplies item facts only, never a claim or starter grant.
"""
from __future__ import annotations

import copy

from native_profile_composition import encode_native_profile

CLIENT_SHA256 = '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'

CORRUPTOR_ITEMS = {
    'knight': (3359, 3285, 3375), 'paladin': (3571, 51760),
    'sorcerer': (3074, 7991), 'druid': (3066, 7991),
    'monk': (50195, 51119),
}
APPEARANCE_TRANSLATIONS = {51166: 51119}
# Crystal201189f8 items.xml:87366-84: source48 on the first three
# containers. Owner-approved native30 projection; final stage has no container.
CORPSE_CHAIN = (52113, 52114, 52115, 52116)
CORPSE_SOURCE_CAPACITY = 48
CORPSE_NATIVE_CAPACITY = 30
# Already-admitted canonical ordinary loot needs an exact native carrying policy
# too. Preserve all its source semantics, including stack100 and food108s.
CANONICAL_LOOT_ITEMS = {3723: 'white mushroom'}
# id: name, weight in hundredths of an ounce, primary slot, vocations,
# minimum level (None = source did not specify), Wiki page, revision.
# Crystal corpus9f5a72c6 items.xml SHA256 c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb
# declares no level gate on these armor/helmet/cloak/robe/scarf moveevents,
# and no vocation gate on armor/helmet/sword/bow/scarf. Empty means unrestricted;
# preserve the current Wiki's explicit level0 choice for the Monk items.
SPEC = {
    3359: ('brass armor', 8000, 'TORSO', (), 0, 1615, 1186985),
    3285: ('longsword', 4200, 'WEAPON', (), 0, 2404, 1186983),
    3375: ('soldier helmet', 3200, 'HEAD', (), 0, 2304, 1147311),
    3571: ("ranger's cloak", 4800, 'TORSO', ('PALADIN',), 0, 25353, 1186981),
    51760: ('refined bow', 3250, 'WEAPON', (), 0, 109332, 1148726),
    3074: ('wand of vortex', 1900, 'WEAPON', ('SORCERER',), 6, 4939, 1186984),
    7991: ("magician's robe", 2500, 'TORSO', ('DRUID', 'SORCERER'), 0, 25343, 1186986),
    3066: ('snakebite rod', 1900, 'WEAPON', ('DRUID',), 6, 4937, 1147165),
    50195: ('harmony amulet', 510, 'AMULET', ('MONK',), 0, 108150, 1194222),
    51119: ('simple jo staff', 1800, 'WEAPON', ('MONK',), 0, 108393, 1186982),
    51977: ('book of wisdom', 100, None, (), None, 109842, 1143205),
    3572: ('scarf', 200, 'AMULET', (), 0, 2473, 1194228),
}
UNKNOWN_GROUPS = ('classification', 'container', 'equipment', 'weapon', 'protection',
                  'skill_modifiers', 'charges', 'imbuement', 'temporal', 'use_transform',
                  'trade_restrictions', 'fluid', 'readable_writeable')


def known(value):
    return {'state': 'KNOWN', 'value': value}


def unknown():
    return {'state': 'UNKNOWN'}


def apply(definition):
    """Apply one named Item overlay; fail before mutation on identity conflicts."""
    identity = definition.get('identity', {})
    key = identity.get('key', '')
    if identity.get('family') != 'Item' or not key.startswith('oteryn:item.tibia.i'):
        return False
    try:
        item = int(key.removeprefix('oteryn:item.tibia.i'))
    except ValueError:
        return False
    if item in CORPSE_CHAIN:
        name, weight, slot, vocations, level = 'dead the corruptor', None, None, (), None
    elif item in SPEC:
        name, weight, slot, vocations, level, _, _ = SPEC[item]
    else:
        return False
    semantics = definition.get('semantics', {})
    previous_name = semantics.get('presentation', {}).get('value', {}).get('name', {})
    if previous_name.get('state') == 'KNOWN' and previous_name.get('value', '').lower() != name:
        raise ValueError(f'NEWHAVEN_ITEM_NAME:{item}')
    replacement = copy.deepcopy(definition)
    semantics = replacement.setdefault('semantics', {})
    for group in UNKNOWN_GROUPS:
        semantics.setdefault(group, unknown())
    semantics['physical'] = known({'weight': known(weight) if weight is not None else unknown(),
        'movable': known(item not in CORPSE_CHAIN), 'pickupable': known(item not in CORPSE_CHAIN)})
    if item in CORPSE_CHAIN:
        semantics['container'] = (known({'capacity': known(CORPSE_NATIVE_CAPACITY)})
                                  if item != CORPSE_CHAIN[-1] else {'state': 'NOT_APPLICABLE'})
    else:
        # The exact served client's flags are checked by verify_client below.
        semantics['container'] = {'state': 'NOT_APPLICABLE'}
    semantics['stack'] = known({'stackable': known(False), 'stack_max': {'state': 'NOT_APPLICABLE'}})
    semantics['presentation'] = known({'name': known(name),
        'description': semantics.get('presentation', {}).get('value', {}).get('description', unknown())})
    if slot:
        # Bow permits a quiver in the other hand; a jo staff reserves both hands.
        semantics['equipment'] = known({'patterns': known([{
            'pattern_id': 1, 'primary_slot': known(slot),
            'additional_reserved_slots': known(['SHIELD'] if item == 51119 else []),
            'mutually_exclusive_groups': known(['oteryn:equipment-group.non_quiver_left_hand']
                                              if item == 51760 else []),
            'vocations': known(list(vocations)),
            'level': known(level) if level is not None else unknown(),
            'compatibility_rule': unknown(),
        }])})
    if item == 51119:
        weapon = {field: unknown() for field in ('ammunition', 'elemental', 'extra_defense',
            'hit_chance', 'max_hit_chance', 'range')}
        weapon.update({'weapon_type': known('FIST'), 'attack': known(12), 'defense': known(8)})
        semantics['weapon'] = known(weapon)
        semantics['skill_modifiers'] = known({'modifiers': known([{
            'kind': 'SKILL_FIST', 'parameter': known({'kind': 'SIGNED_POINTS', 'value': 1}),
            'evaluation_phase': unknown(), 'priority': unknown(), 'target_domain': unknown(),
        }])})
    replacement['materializable'] = True
    replacement['stack_class'] = 'NonStackable'
    definition.clear()
    definition.update(replacement)
    return True


def verify_client(root):
    """Prove each selected Item's portability and name on the exact served client."""
    import hashlib
    import sys

    path = root / f'content/assets/files/appearances-{CLIENT_SHA256}.dat'
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != CLIENT_SHA256:
        raise ValueError('NEWHAVEN_CLIENT_PIN')
    sys.path.insert(0, str(root / 'tools/content-schema/item-authoring'))
    from engine_items import load_appearance_objects
    objects = load_appearance_objects(raw)
    names = {item: spec[0] for item, spec in SPEC.items()}
    names.update(CANONICAL_LOOT_ITEMS)
    for item, name in names.items():
        record = objects.get(item, {})
        flags = record.get('flags', {})
        if (record.get('name', '').lower() != name
                or flags.get('flags.take') is not True
                or flags.get('flags.unmove', False)
                or flags.get('flags.container', False)):
            raise ValueError(f'NEWHAVEN_CLIENT_IDENTITY:{item}')
    for item in CORPSE_CHAIN:
        flags = objects.get(item, {}).get('flags', {})
        if (flags.get('flags.corpse') is not True
                or bool(flags.get('flags.container')) != (item != CORPSE_CHAIN[-1])
                or (item == CORPSE_CHAIN[0] and flags.get('flags.unmove') is not True)):
            raise ValueError(f'NEWHAVEN_CLIENT_CORPSE:{item}')


def apply_native(root, generated, profile_path, manifest_path, provider_order, canonical_bytes):
    """Mount exact named Newhaven Items in the existing native provider document."""
    import hashlib
    import json

    verify_client(root)
    canonical = {}
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            key = definition['identity']['key']
            if key in {f'oteryn:item.tibia.i{item}' for item in (*SPEC, *CORPSE_CHAIN, *CANONICAL_LOOT_ITEMS)}:
                if key in canonical:
                    raise ValueError('NEWHAVEN_DUPLICATE_ITEM')
                canonical[key] = copy.deepcopy(definition)
    bindings = {int(row['external_id']): row for row in json.loads(
        (root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']}
    taxonomy = {row['target']['key']: row['source_taxonomy'] for row in json.loads(
        (root / 'content/items/taxonomy/items.json').read_bytes())['records']}
    document = json.loads(generated[profile_path])
    indexed = {row['production_definition']['production_key']: row for row in document['records']}
    for item in sorted((*SPEC, *CORPSE_CHAIN, *CANONICAL_LOOT_ITEMS)):
        key = f'oteryn:item.tibia.i{item}'
        definition = canonical[key]
        if item in CANONICAL_LOOT_ITEMS:
            if definition['materializable'] is not True:
                raise ValueError(f'NEWHAVEN_CANONICAL_LOOT_INACTIVE:{item}')
        else:
            apply(definition)
        external = item
        binding = copy.deepcopy(bindings[external])
        if binding['target'] != definition['identity']:
            raise ValueError('NEWHAVEN_ITEM_BINDING')
        previous = indexed.get(key)
        if previous is not None and item in CANONICAL_LOOT_ITEMS:
            # Canonical ordinary loot may already have a richer source owner.
            # Reuse its exact native policy instead of replacing source-derived
            # semantics or authoring metadata with the chest projection.
            import grizzly_shop_profiles
            if (previous['production_binding'] != binding
                    or not previous['admission']['materializable']
                    or 'CharacterInventory' not in previous['admission']['legal_destinations']
                    or grizzly_shop_profiles.known_conflict(
                        definition['semantics'], previous['semantics'])):
                raise ValueError(f'NEWHAVEN_NATIVE_LOOT_CONFLICT:{item}')
            continue
        row = copy.deepcopy(previous) if previous else {
            'authoring': {'item': copy.deepcopy(definition['identity'])},
            'attributes': {'speed_bonus': 0},
            'production_binding': binding, 'production_binding_qualification': None,
            'production_definition': {'family': 'Item', 'production_key': key,
                                      'revision_ref': definition['identity']['revision']},
        }
        row['semantics'] = copy.deepcopy(definition['semantics'])
        row['admission'] = {'materializable': True, 'stack_class': definition['stack_class'],
            'legal_destinations': ['Ground'] if item in CORPSE_CHAIN else ['CharacterInventory', 'Ground']}
        if key in taxonomy:
            row['authoring']['taxonomy'] = copy.deepcopy(taxonomy[key])
        if item == 51119:
            row['authoring']['taxonomy'] = {'primary': 'Fist Fighting Weapons'}
        indexed[key] = row
    document['records'] = sorted(indexed.values(), key=provider_order)
    raw = encode_native_profile(document)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical_bytes(manifest)
