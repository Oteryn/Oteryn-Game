"""Exact source bodies of the already active shared reward-boss policies."""
BODIES = {
    'an_observer_eye': 5995,
    'armenius_creature': 6006,
    'grand_mother_foulscale': 5973,
    'orshabaal': 5995,
    'the_weakened_count': 6006,
}


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
