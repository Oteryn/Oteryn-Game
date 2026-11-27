"""Exact source bodies of the already active shared reward-boss policies."""
BODIES = {
    'an_observer_eye': 5995,
    'arbaziloth': 50029,
    'arachir_the_ancient_one': 8109,
    'armenius_creature': 6006,
    'diblis_the_fair': 8109,
    'grand_mother_foulscale': 5973,
    'koshei_the_deathless': 7535,
    'orshabaal': 5995,
    'sir_valorcrest': 8109,
    'the_pale_count': 18953,
    'the_weakened_count': 6006,
    'zevelon_duskbringer': 8109,
}

def bind_koshei(creatures):
    """Crystal 201189f: the fresh remains accept the amulet before decaying."""
    rows = [row['profile'] for row in creatures['records']
            if row['profile']['target']['key'] == 'oteryn:creature.koshei_the_deathless']
    if len(rows) != 1:
        raise ValueError('KOSHEI_CORPSE_CREATURE_BINDING')
    details = rows[0]['data']['profile']['details']
    corpse = details.get('corpse_item')
    if corpse not in (
        {'family': 'Item', 'key': 'oteryn:item.tibia.i7538', 'revision': 'definition-r1'},
        {'family': 'Item', 'key': 'oteryn:item.tibia.i7535', 'revision': 'definition-r1'},
    ):
        raise ValueError('KOSHEI_CORPSE_PREDECESSOR')
    details['corpse_item'] = {**corpse, 'key': 'oteryn:item.tibia.i7535'}


def roots(creatures):
    selected = set()
    for name, number in BODIES.items():
        key = 'oteryn:creature.' + name
        record = creatures.get(key)
        if record is None:
            continue
        expected = {'family': 'Creature', 'key': key, 'revision': 'definition-r1'}
        corpse = {'family': 'Item', 'key': f'oteryn:item.tibia.i{number}',
                  'revision': 'definition-r1'}
        if (record['target'] != expected
                or record['data']['profile']['details'].get('corpse_item') != corpse):
            raise ValueError('SHARED_REWARD_BOSS_CORPSE_BINDING:' + key)
        selected.add(number)
    return selected


def apply_arbaziloth_corpse_facts(xml):
    # Crystal 201189f8 data/items/items.xml, items 50029 and 50028.
    # Keep the pinned Canary records' exact identity, decay targets and physical facts.
    for number in (50029, 50028):
        overrides = {'containersize': ('15', '8'), 'duration': ('300', '60')}
        for attribute in xml[number].findall('attribute'):
            key = attribute.get('key', '').lower()
            if key in overrides:
                expected, value = overrides.pop(key)
                if attribute.get('value') != expected:
                    raise ValueError(f'ARBAZILOTH_CORPSE_CANARY_FACT_DRIFT:{number}:{key}')
                attribute.set('value', value)
        if overrides:
            raise ValueError(f'ARBAZILOTH_CORPSE_CANARY_FACT_MISSING:{number}')
