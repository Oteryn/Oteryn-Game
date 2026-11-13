"""Declare M41's independent six-key source cohort; never derive a pin from actual union size."""
import re
from m41_quest_item_profiles import IDS


def register(source):
    count = len(IDS)
    marker = 'const PROFILE_ADDITIONS: &[(&str, usize)] = &[\n'
    if source.count(marker) != 1:
        raise ValueError('M41_EQUIPMENT_COHORT_TABLE')
    pattern = r'\("M41",\s*[0-9_]+\)'
    if len(re.findall(pattern, source)) > 1:
        raise ValueError('M41_EQUIPMENT_COHORT_DUPLICATE')
    if re.search(pattern, source):
        return re.sub(pattern, f'("M41", {count})', source)
    row = ('    // M41 source operands: sculpture 31574, beak 52632 and corpse stages 52677..=52680.\n'
           '    // Regenerated from m41_quest_item_profiles.IDS, independently of the profile union.\n'
           f'    ("M41", {count}),\n')
    return source.replace(marker, marker + row, 1)
