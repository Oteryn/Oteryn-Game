"""Bind a container recipe's identity and charge witnesses to the pinned source entry."""


def matches(definition, placement_index, entry):
    source = definition.get('provenance', {}).get('pilot_key', '')
    if not source.startswith(('canary:reward-claim/', 'crystalserver:reward-claim/')):
        return False
    expected = 'oteryn:reward-claim.' + source.split('reward-claim/', 1)[1].replace('/', '.')
    if definition.get('identity', {}).get('key') != expected:
        return False
    original = entry['setting'].get('reward', [])
    for charge in definition.get('source_variant', {}).get('charge_dispositions', []):
        if charge.get('placement_index') != placement_index:
            continue
        index = charge.get('reward_index')
        if (charge.get('reward_field') != 'items' or type(index) is not int
                or not 0 <= index < len(original)
                or charge.get('source_raw_argument') != original[index][1]):
            return False
    return True
