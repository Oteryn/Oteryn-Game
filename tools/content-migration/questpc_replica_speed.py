"""Complete the exact jade replica's source-derived equipment speed fact."""


def qualify(record, source_items):
    if record['production_definition']['production_key'] != 'oteryn:item.tibia.i31268':
        return
    matches = [item for item in source_items if item.get('id') == '31268']
    if len(matches) != 1 or matches[0].get('name') != 'jade amulet':
        raise ValueError('QUESTPC_JADE_SOURCE_IDENTITY')
    speeds = [a for a in matches[0].findall('attribute')
              if a.get('key', '').lower() == 'speed']
    if len(speeds) > 1:
        raise ValueError('QUESTPC_JADE_DUPLICATE_SPEED')
    # Same pinned Canary Abilities default used for newly imported quest Items.
    speed = int(speeds[0].get('value')) if speeds else 0
    if record['attributes'].get('speed_bonus', speed) != speed:
        raise ValueError('QUESTPC_JADE_SPEED_CONFLICT')
    record['attributes']['speed_bonus'] = speed
