"""Admit the source-qualified Arbaziloth encounter, loot and Doomforge items."""
import hashlib
import json
from fractions import Fraction

REWARD_IDS = (49520, 49522, 49523, 49524, 49525, 49526, 49527, 49528, 49529, 49530, 50250)


def admit(root, generated, destination, manifest_path, order, canonical):
    bindings = {
        row['external_id']: row for row in json.loads(
            (root / 'imports/crystalserver/bindings/items.json').read_bytes()
        )['bindings']
    }
    definitions = {}
    for shard in ('items-27000-27499.json', 'items-27500-27999.json'):
        records = json.loads((root / 'content/items/definitions' / shard).read_bytes())['records']
        definitions.update((r['definition']['identity']['key'], r['definition']) for r in records)
    document = json.loads(generated[destination])
    existing = {r['production_definition']['production_key'] for r in document['records']}
    for item in REWARD_IDS:
        key = f'oteryn:item.tibia.i{item}'
        definition = definitions[key]
        binding = bindings[str(item)]
        semantics = definition['semantics']
        physical = semantics['physical']['value']
        if (key in existing or binding['target'] != definition['identity']
                or any(physical[field] != {'state': 'KNOWN', 'value': True}
                       for field in ('movable', 'pickupable'))):
            raise ValueError(f'Doomforge reward source facts changed: {item}')
        document['records'].append({
            'authoring': {'item': binding['target']},
            'production_binding': binding,
            'production_binding_qualification': None,
            'production_definition': {
                'family': 'Item', 'production_key': key, 'revision_ref': binding['target']['revision'],
            },
            'admission': {
                'materializable': True, 'stack_class': 'NonStackable',
                'legal_destinations': ['CharacterInventory', 'Ground'],
            },
            'attributes': {'speed_bonus': 0, 'spell_augments': []},
            'semantics': semantics,
        })
    if FIRE in existing:
        raise ValueError('Duplicate Arbaziloth fire profile')
    document['records'].append(fire_profile(root))
    document['records'].sort(key=order)
    generated[destination] = canonical(document)
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(generated[destination]).hexdigest()
    generated[manifest_path] = canonical(manifest)

FACTS = 'docs/reference/content-prep/nrwicked/encounter-facts.json'
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
FIRE = 'oteryn:item.tibia.i49362'
ROLES = ('enraged_arbaziloth', 'the_forgemaster')
HARD_CREATURE = 'oteryn:creature.enraged_arbaziloth'
HARD_LOOT = 'oteryn:loot.creature.enraged_arbaziloth'


def bind_hard_loot(root, creatures, definitions, tables):
    """The phase replacement has a native profile and a separate donor loot table."""
    if not any(row['profile']['target']['key'] == HARD_CREATURE for row in creatures['records']):
        return
    if HARD_CREATURE in definitions or HARD_LOOT in tables:
        raise ValueError('Arbaziloth hard loot already has an owner')
    source = json.loads((root / FACTS).read_bytes())['enraged_arbaziloth']
    identity = {'family': 'Loot', 'key': HARD_LOOT, 'revision': 'definition-r1'}
    entries = []
    # Crystal201189f enraged_arbaziloth.lua uses a 100,000-point chance scale.
    # Preserve ordered independent rows, including the two draws for source ID 237.
    for row in source['loot']:
        item, chance, maximum = row['id'], row['chance'], row['max_count']
        if not (1 <= item <= 65535 and 1 <= chance <= 100_000 and 1 <= maximum <= 100):
            raise ValueError('Arbaziloth hard loot source row is out of bounds')
        entries.append({
            'item': {'family': 'Item', 'key': f'oteryn:item.tibia.i{item}', 'revision': 'definition-r1'},
            'probability_ppm': chance * 10,
            'min_count': 1,
            'max_count': maximum,
        })
    definitions[HARD_CREATURE] = {
        'identity': {'family': 'Creature', 'key': HARD_CREATURE, 'revision': 'definition-r1'},
        'loot': identity,
    }
    tables[HARD_LOOT] = {'identity': identity, 'algorithm': 'IndependentBernoulliPpm', 'entries': entries}



def known(value):
    return {'state': 'KNOWN', 'value': value}


def fire_definition(root):
    from boss_conditional_item_reference import definition
    facts = json.loads((root / FACTS).read_bytes())['fire']
    if facts['appearance'] != 49362 or any(facts[k] for k in (
            'movable', 'pickupable', 'solid', 'blocks_projectile')):
        raise ValueError('Arbaziloth fire source facts changed')
    row = definition(FIRE)
    row['semantics']['physical'] = known({
        'weight': known(0), 'movable': known(False), 'pickupable': known(False),
    })
    row['semantics']['stack'] = known({
        'stackable': known(False), 'stack_max': {'state': 'NOT_APPLICABLE'},
    })
    return row


def extend_records(records, root):
    row = fire_definition(root)
    matches = [r for r in records if r['identity']['key'] == FIRE]
    if not matches:
        records.append(row)
    elif matches != [row]:
        raise ValueError('Arbaziloth fire definition changed')


def fire_profile(root):
    row = fire_definition(root)
    return {
        'authoring': {'item': row['identity']},
        'production_binding': {
            'source_key': 'oteryn:source.crystalserver', 'source_revision': REVISION,
            'identity_namespace': 'ots/item_server_id', 'external_id': '49362',
            'target': row['identity'], 'disposition': 'EXACT',
        },
        'production_binding_qualification': None,
        'production_definition': {'family': 'Item', 'production_key': FIRE, 'revision_ref': 'definition-r1'},
        'admission': {'materializable': True, 'stack_class': 'NonStackable', 'legal_destinations': ['Ground']},
        'attributes': {'speed_bonus': 0, 'spell_augments': [], 'blocks_movement': False,
                       'blocks_projectile': False},
        'semantics': row['semantics'],
    }


def creature_profiles(root):
    from engenc2_fugue_profile import profile
    facts = json.loads((root / FACTS).read_bytes())
    result = []
    for role in ROLES:
        source = facts[role]
        boss = role == 'enraged_arbaziloth'

        def ref(family, suffix=''):
            key = role if family == 'Creature' else f'creature.{role}{suffix}'
            return {'family': family, 'key': f'oteryn:{family.lower()}.{key}', 'revision': 'definition-r1'}

        attacks, defenses, abilities, records, profiles = [], [], [], [], []
        for i, attack in enumerate(source['attacks'], start=1):
            # The existing actor spell dialect does not lower Crystal's relative Haste.
            if attack['kind'] == 'speed':
                continue
            melee = attack['kind'] == 'melee'
            healing = attack['damage_type'] == 'healing'
            channel = 'physical' if melee else attack['damage_type']
            suffix = f'.attack-{i}'
            ability, effect, formula = ref('Ability', suffix), ref('Effect', suffix), ref('Formula', suffix + '-1')
            reach = attack.get('range', 1 if melee else 0)
            details = {'kind': 'Melee' if melee else 'Spell', 'needs_direction': 'length' in attack,
                       'needs_target': reach > 0, 'range_tiles': reach,
                       'effects': [{'kind': 'Executable', 'effect': effect}]}
            if 'length' in attack:
                details['area'] = {'shape': 'Beam', 'length_tiles': attack['length'], 'spread_tiles': attack['spread']}
            elif 'radius' in attack:
                details['area'] = {'shape': 'Circle', 'radius_tiles': attack['radius']}
            low, high = sorted((abs(attack['minDamage']), abs(attack['maxDamage'])))
            records.extend([
                {'identity': ability, 'kind': 'Ability', 'effects': [effect]},
                {'identity': effect, 'kind': 'Effect', 'client_projection': 'ServerOnly',
                 'effect_family': 'Heal' if healing else 'Damage', 'formula': formula},
                {'identity': formula, 'kind': 'Formula'},
            ])
            effect_body = {'damage_type': channel}
            if melee:
                effect_body['mitigated_by'] = ['Armor', 'Shield']
            profiles.extend([
                profile(ability, 'Ability', {'details': details}),
                profile(effect, 'Effect', effect_body),
                profile(formula, 'Formula', {'formula': 'Range', 'minimum': low, 'maximum': high}),
            ])
            (defenses if healing else attacks).append({
                'ability': ability, 'interval_ms': attack['interval'], 'chance_ppm': attack['chance'] * 10_000,
            })
            abilities.append(ability)

        name = 'Arbaziloth' if boss else 'The Forgemaster'
        details = {
            'display_name': name, 'inspection': name, 'reference_level': None,
            'critical_chance_ppm': 0, 'defense': source['defense'],
            'condition_immunities': ['invisible', 'paralyze'],
            'flags': {'attackable': True, 'health_hidden': False, 'illusionable': False},
            'summoning': {'summonable': False, 'convinceable': False, 'is_familiar': False},
            'spawn_eligibility': {'period': 'All', 'blocked_by_nearby_players': False, 'ignore_period_underground': False},
            'system_eligibility': {'prey': False, 'forge': False, 'exclusive_prey': not boss, 'reward_boss': boss},
        }
        if source['corpse']:
            details['corpse_item'] = {'family': 'Item', 'key': f"oteryn:item.tibia.i{source['corpse']}", 'revision': 'definition-r1'}
        mitigation = Fraction(source['mitigation'], 100)
        creature = profile(ref('Creature'), 'Creature', {
            'health': source['health'], 'experience': source['experience'],
            'speed': source['speed'], 'armor': source['armor'], 'abilities': abilities,
            'mitigation': {'numerator': mitigation.numerator, 'denominator': mitigation.denominator},
            'resistances': [{'damage_type': element, 'percent': {'numerator': value, 'denominator': 1}}
                            for element, value in source['resistances']],
            'details': details,
        })
        interval, chance = source['change_target']
        behavior = profile(ref('Behavior'), 'Behavior', {
            'attacks': attacks, 'defenses': defenses,
            'movement': {'can_walk': True, 'pass_through': False, 'push_creatures': True, 'push_items': True,
                         'pushable': False, 'walks_on_energy': True, 'walks_on_fire': True, 'walks_on_poison': True},
            'targeting': {'can_target': True, 'hostile': True, 'sense_invisible': True,
                          'change_target': {'interval_ms': interval, 'chance_ppm': chance * 10_000},
                          'flee_health': 0, 'static_attack_chance_ppm': source['static_attack_chance'] * 10_000,
                          'strategy_weights': {k: source['targeting'].get(k, 0) for k in ('nearest', 'damage', 'health', 'random')},
                          'target_distance_tiles': 1},
        })
        records.extend([
            {'identity': ref('Creature'), 'kind': 'Creature', 'client_projection': 'ClientSafe',
             'behavior': ref('Behavior'), 'presentation': ref('Presentation')},
            {'identity': ref('Behavior'), 'kind': 'Generic', 'client_projection': 'ServerOnly'},
        ])
        profiles.extend([creature, behavior])
        presentation = profile(ref('Presentation'), 'Presentation', {
            'asset_binding': f"canary.appearance:outfit/{source['look_type']}", 'light_level': 0,
        })
        result.append(({
            'profile': creature, 'behavior': behavior, 'presentation': ref('Presentation'),
            'attack_definitions': {'records': records, 'profiles': profiles},
        }, presentation))
    return result


def admit_creatures(root, creatures, generated, path, manifest_path, canonical):
    presentations = json.loads(generated[path])
    def insert(records, row, key):
        wanted = key(row)
        offset = next((i for i, existing in enumerate(records) if key(existing) >= wanted), len(records))
        if offset < len(records) and key(records[offset]) == wanted:
            raise ValueError(f'Duplicate Arbaziloth encounter definition: {wanted}')
        records.insert(offset, row)

    for row, presentation in creature_profiles(root):
        insert(creatures['records'], row, lambda r: r['profile']['target']['key'])
        insert(presentations['records'], presentation, lambda r: r['target']['key'])
    generated[path] = canonical(presentations)
    manifest = json.loads(generated[manifest_path])
    manifest['presentation_profiles']['sha256'] = hashlib.sha256(generated[path]).hexdigest()
    generated[manifest_path] = canonical(manifest)


def inventory_extension(root, rows):
    keys = {f'oteryn:creature.{role}' for role in ROLES}
    selected = [r for r in rows if r['profile']['target']['key'] in keys]
    expected = [r for r, _ in creature_profiles(root)]
    if selected != expected:
        raise ValueError('Arbaziloth native creature source facts changed')
    return len(selected), sum(len(r['behavior']['data']['profile'][kind])
                              for r in selected for kind in ('attacks', 'defenses'))
