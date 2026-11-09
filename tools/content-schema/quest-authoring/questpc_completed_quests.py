"""Resolve exact chosen prerequisite titles without discarding unknown guards."""
import re


def resolve(quest, names, definitions):
    if len(names) > 16 or len(set(names)) != len(names):
        raise ValueError('Completed quests need a bounded distinct prerequisite list')
    result = []
    for name in names:
        matches = [d['identity']['key'] for d in definitions
                   if d.get('display_name') == name]
        if len(matches) != 1:
            raise ValueError('Completed quests need one canonical prerequisite: ' + name)
        key, = matches
        if (key == quest or len(key) > 256
                or not re.fullmatch(r'oteryn:quest\.[a-z0-9_.-]+', key)):
            raise ValueError('Prerequisite must name a different canonical quest')
        result.append(key)
    return sorted(result)
