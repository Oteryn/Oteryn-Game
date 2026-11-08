#!/usr/bin/env python3
"""Refresh reviewed collector-merge fixture inventories without opening allowlists.

The allowlists remain closed. New entries require an explicit reviewed path below;
source scanning only verifies their Type2 tuple, never authorizes arbitrary sites.
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def replace(path, before, after):
    target = ROOT / path
    text = target.read_text()
    if after in text:
        return
    if text.count(before) != 1:
        raise ValueError(f'inventory context drift: {path}: {before!r}')
    target.write_text(text.replace(before, after))


def main():
    base = 'crates/gameplay/durability/src/durability/'
    for name in ('tutorial_viola.rs',):
        source = (ROOT / base / name).read_text()
        assert 'INSERT INTO game_item_audit_outbox(' in source, name
        assert 'tuple.schema_revision()' in source and 'tuple.retention_profile_id()' in source, name
    path = base + 'item_mint_audit.rs'
    anchor = '                "crates/gameplay/durability/src/durability/reward_claim_mint.rs",\n'
    replace(path, anchor, anchor + ''.join(f'                "{base}{name}",\n' for name in (
        'tutorial_viola.rs',)))
    dispatch = 'crates/gameplay/dispatch/src/gameplay_transport/'
    fixture = (ROOT / dispatch / 'postgres_tests/newhaven_corruptor.rs').read_text()
    assert fixture.count('SeamRooms::new(None, Some(&room))') == 1
    anchor = '    ("gameplay_transport/postgres_tests/item_cosmetics.rs", 1),\n'
    replace(dispatch + 'bundle_gates_tests.rs', anchor, anchor +
            '    ("gameplay_transport/postgres_tests/newhaven_corruptor.rs", 1),\n')

    # Derive the quest-profile inventory from its closed, source-pinned producer.
    sys.path.insert(0, str(ROOT / 'tools/content-migration'))
    import qengine_reward_item_profiles as profiles
    proof = json.loads((ROOT / 'tools/content-schema/native-gameplay/qengine-reward-item-profiles.json').read_text())
    keys = {r['production_definition']['production_key'] for r in proof['records']}
    assert keys == {f'oteryn:item.tibia.i{i}' for i in profiles.IDS}
    replace(dispatch + 'equipment_effects_owner_tests/ability_tests.rs',
            'assert_eq!(added_keys.len(), 53);', f'assert_eq!(added_keys.len(), {len(keys)});')
    creatures = json.loads((ROOT / 'content/creatures/definitions/spell-native-profiles.json').read_text())['records']
    bosses = [r for r in creatures if r['profile']['data']['profile'].get('details', {}).get('system_eligibility', {}).get('reward_boss')]
    missing = sum(not r['profile']['data']['profile']['details'].get('corpse_item') for r in bosses)
    assert missing == 52 and len(bosses) == 387
    replace('crates/gameplay/content/src/content/native_gameplay/test_support/activation_fixture.rs',
            '(52, 329, 1)', f'({missing}, {len(bosses) - missing - 1}, 1)')

    # Count the merged registered union once: older cohort totals omit the
    # lane's Newhaven/recipe/source-custody overlays and overlap the pick.
    equipment = ROOT / dispatch / 'equipment_effects_owner_tests/ability_tests.rs'
    text = equipment.read_text()
    body = text.split('const PROFILE_ADDITIONS:')[1].split('];')[0]
    other = sum(int(count) for name, count in re.findall(r'\("([^"\n]+)", (\d+)\)', body)
                if name != 'QUESTPC7-REGISTERED-UNION')
    document = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_text())
    actual_keys = {r['production_definition']['production_key'] for r in document['records']}
    assert len(actual_keys) == len(document['records']) and keys <= actual_keys
    residual = len(actual_keys) - 432 - other - len(keys) + 1
    assert residual == 95, 'review source-cohort changes before refreshing this closed union'
    row = f'    ("QUESTPC7-REGISTERED-UNION", {residual}),\n'
    if row not in text:
        anchor = '    ("SRV9-MARRIAGE", 2),\n'
        replace(str(equipment.relative_to(ROOT)), anchor, anchor +
                '    // Merged source/custody cohorts, refreshed by questpc7_merge_audit_inventories.py.\n' + row)


if __name__ == '__main__':
    main()
