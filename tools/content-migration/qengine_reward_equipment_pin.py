"""Pin QENGINE's exact declared source identities without weakening item checks."""
import re

from qengine_reward_item_profiles import IDS


def refresh(source, packet):
    identities = [row['production_definition']['production_key'] for row in packet['records']]
    if (packet['schema'] != 'OTERYN_QENGINE_REWARD_ITEM_PROFILES/v1'
            or len(identities) != len(IDS)
            or set(identities) != {f'oteryn:item.tibia.i{i}' for i in IDS}):
        raise ValueError('QENGINE_REWARD_SOURCE_COHORT')
    pattern = r'assert_eq!\(added_keys\.len\(\),\s*\d+\);'
    if len(re.findall(pattern, source)) != 1:
        raise ValueError('QENGINE_REWARD_PIN_ANCHOR')
    source = re.sub(pattern, f'assert_eq!(added_keys.len(), {len(IDS)});', source)
    return re.sub(r'// QENGINE2 captures [^\n]+',
                  f'// QENGINE2 captures {len(IDS)} source-declared shared Items.', source)
