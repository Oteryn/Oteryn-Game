"""Declare the additional source-backed locked doors, independently of output."""
import re

from key_door_item_profiles import CLASSIC_QUEST_DOORS, IDS


def refresh(source, packet):
    identities = [row['production_definition']['production_key'] for row in packet['records']]
    if (packet['schema'] != 'OTERYN_NATIVE_KEY_DOOR_ITEM_PROFILES/v1'
            or len(identities) != len(IDS)
            or set(identities) != {f'oteryn:item.tibia.i{i}' for i in IDS}
            or not {5097, 5106, 6248, 6251, 7711, 7714} <= IDS):
        raise ValueError('M01C_KEY_DOOR_COHORT')
    for cohort, count in [('M01C-KEY-DOORS', 4), ('CLASSIC-QUEST-DOORS', len(CLASSIC_QUEST_DOORS))]:
        line = f'    ("{cohort}", {count}),'
        matches = list(re.finditer(rf'^[ \t]*\("{cohort}",\s*\d+\),$', source, re.MULTILINE))
        if len(matches) > 1:
            raise ValueError('M01C_KEY_DOOR_DUPLICATE_COHORT')
        if matches:
            source = source[:matches[0].start()] + line + source[matches[0].end():]
        else:
            anchor = source.find('    // M06:')
            if anchor < 0:
                raise ValueError('M01C_KEY_DOOR_COHORT_ANCHOR')
            source = source[:anchor] + line + '\n' + source[anchor:]
    return source
