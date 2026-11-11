"""Refresh the independently declared cooking cohort, then let the normal pin tool verify it."""
import json
from pathlib import Path
import re
import subprocess
import questpc7_hot_corpse_profiles as corpses

BASE = '47b9de3c9'
PATHS = ('content/items/definitions/spell-native-profiles.json',
         'content/items/definitions/srx1-item-updates.json',
         'content/items/definitions/starting-loadout-profiles.json')


def refresh(root):
    keys = lambda doc: {r['production_definition']['production_key'] for r in doc['records']}
    before = set().union(*(keys(json.loads(subprocess.check_output(
        ['git', 'show', f'{BASE}:{p}'], cwd=root))) for p in PATHS))
    recipes = json.loads((root / 'content/quests/hot-cuisine-batch3.json').read_bytes())['recipes']
    selected = {i['item'] for r in recipes for i in [*r['ingredients'], r['reward']]}
    selected |= {f'oteryn:item.tibia.i{i}' for i in corpses.IDS}
    actual = set().union(*(keys(json.loads((root / p).read_bytes())) for p in PATHS))
    if not selected <= actual:
        raise ValueError('M42_COOKBOOK_NATIVE_CLOSURE')
    count = len(selected - before)
    if count != 45:
        raise ValueError('M42_REVIEW_EXACT_COHORT_DELTA')
    path = root / 'crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/ability_tests.rs'
    text = path.read_text()
    row = f'    ("M42-HOT-CUISINE", {count}),\n'
    if '"M42-HOT-CUISINE"' in text:
        text, n = re.subn(r'    \("M42-HOT-CUISINE", \d+\),\n', row, text)
        assert n == 1
    else:
        anchor = '    ("SEASON-W05D", 35),\n'
        assert text.count(anchor) == 1
        text = text.replace(anchor, anchor + '    // Finite cookbook operands and exact corpse stages, refreshed by m42_hot_profile_inventory.py.\n' + row)
    path.write_text(text)
    print(f'PASS M42 exact native cooking cohort {count}')


if __name__ == '__main__':
    refresh(Path(__file__).resolve().parents[2])
