"""Refresh QENGINE proof cardinalities from the producer's declared source identities."""
import re
from qengine_reward_item_profiles import IDS


def refresh(source, packet):
    declared = {f'oteryn:item.tibia.i{i}' for i in IDS}
    observed = [r['production_definition']['production_key'] for r in packet['records']]
    if len(observed) != len(declared) or set(observed) != declared:
        raise ValueError('QENGINE_EQUIPMENT_SOURCE_SET')
    patterns = [
        r'assert_eq!\(added_keys\.len\(\),\s*[0-9_]+\);',
        r'\("qengine-reward-item-profiles\.json",\s*[0-9_]+\)',
        r'// QENGINE2 captures [^\n]+',
    ]
    replacements = [
        f'assert_eq!(added_keys.len(), {len(IDS)});',
        f'("qengine-reward-item-profiles.json", {len(IDS)})',
        f'// QENGINE2 captures {len(IDS)} source-qualified Items (qengine_reward_item_profiles.IDS).',
    ]
    for pattern, replacement in zip(patterns, replacements):
        if len(re.findall(pattern, source)) != 1:
            raise ValueError('QENGINE_EQUIPMENT_PIN_ANCHOR')
        source = re.sub(pattern, replacement, source)
    return source
