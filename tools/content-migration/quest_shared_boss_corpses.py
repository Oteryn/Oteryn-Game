"""Exact source bodies of the already active shared reward-boss policies."""
BODIES = {
    'an_observer_eye': 5995,
    'armenius_creature': 6006,
    'grand_mother_foulscale': 5973,
    'koshei_the_deathless': 7535,
    'orshabaal': 5995,
    'the_weakened_count': 6006,
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
