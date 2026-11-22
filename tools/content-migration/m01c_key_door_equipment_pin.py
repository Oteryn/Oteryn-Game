"""Declare the source-backed key-door Item additions, independently of output."""
import re

from key_door_item_profiles import IDS


M01C_IDS = {5097, 6248, 7033, 7034, 7035}


def refresh(source, packet):
    identities = [row['production_definition']['production_key'] for row in packet['records']]
    if (packet['schema'] != 'OTERYN_NATIVE_KEY_DOOR_ITEM_PROFILES/v1'
            or len(identities) != len(IDS)
            or set(identities) != {f'oteryn:item.tibia.i{i}' for i in IDS}
            or not M01C_IDS <= IDS):
        raise ValueError('M01C_KEY_DOOR_COHORT')
    line = f'    ("M01C-KEY-DOORS", {len(M01C_IDS)}),'
    matches = list(re.finditer(r'^[ \t]*\("M01C-KEY-DOORS",\s*\d+\),$',
                              source, re.MULTILINE))
    if len(matches) > 1:
        raise ValueError('M01C_KEY_DOOR_DUPLICATE_COHORT')
    if matches:
        return source[:matches[0].start()] + line + source[matches[0].end():]
    anchor = source.find('    // M06:')
    if anchor < 0:
        raise ValueError('M01C_KEY_DOOR_COHORT_ANCHOR')
    return source[:anchor] + line + '\n' + source[anchor:]
