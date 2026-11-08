#!/usr/bin/env python3
"""Refresh reviewed collector-merge fixture inventories without opening allowlists.

The allowlists remain closed. New entries require an explicit reviewed path below;
source scanning only verifies their Type2 tuple, never authorizes arbitrary sites.
"""
from collections import Counter
import hashlib
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
    equipment_path = ROOT / dispatch / 'equipment_effects_owner_tests/ability_tests.rs'
    previous = equipment_path.read_text()
    refreshed, matches = re.subn(r'assert_eq!\(added_keys\.len\(\), [0-9]+\);',
                                 f'assert_eq!(added_keys.len(), {len(keys)});', previous)
    assert matches == 1, 'quest-profile inventory assertion is missing or ambiguous'
    if refreshed != previous:
        equipment_path.write_text(refreshed)
    # Full regeneration restores five source-qualified shared corpse bodies.
    # Preserve the original closed fixture; the partial-provider count was stale.
    replace('crates/gameplay/content/src/content/native_gameplay/test_support/activation_fixture.rs',
            '(52, 334, 1)', '(52, 329, 1)')

    # Count the merged registered union once: older cohort totals omit the
    # lane's Newhaven/recipe/source-custody overlays and overlap the pick.
    equipment = ROOT / dispatch / 'equipment_effects_owner_tests/ability_tests.rs'
    text = equipment.read_text()
    marsh_proof = ROOT / 'tools/content-schema/native-gameplay/questpc7-marsh-corpse-profiles.json'
    if marsh_proof.exists():
        marsh = json.loads(marsh_proof.read_bytes())['records']
        assert {r['production_definition']['production_key'] for r in marsh} == {
            f'oteryn:item.tibia.i{i}' for i in (17428, 17439, 17440, 17441)}
        assert len(marsh) == 4 and all(r['semantics']['temporal']['state'] == 'KNOWN' for r in marsh)
        row = '    ("QUESTPC7-MARSH-CORPSES", 4),\n'
        anchor = '    ("SRV9-MARRIAGE", 2),\n'
        replace(str(equipment.relative_to(ROOT)), anchor, anchor + row)
        text = equipment.read_text()
    body = text.split('const PROFILE_ADDITIONS:')[1].split('];')[0]
    other = sum(int(count) for name, count in re.findall(r'\("([^"\n]+)", (\d+)\)', body)
                if name != 'QUESTPC7-REGISTERED-UNION')
    document = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_text())
    actual_keys = {r['production_definition']['production_key'] for r in document['records']}
    assert len(actual_keys) == len(document['records']) and keys <= actual_keys
    # Production from_manifest composes these pinned providers before admission.
    manifest = json.loads((ROOT / 'content/spells.manifest.json').read_text())
    for field in ('item_profile_updates', 'starting_loadout_item_profiles'):
        descriptor = manifest[field]
        path = (ROOT / 'content' / descriptor['path']).resolve()
        assert path.is_relative_to(ROOT)
        raw = path.read_bytes()
        assert hashlib.sha256(raw).hexdigest() == descriptor['sha256']
        records = json.loads(raw)['records']
        additions = {r['production_definition']['production_key'] for r in records}
        assert len(additions) == len(records)
        actual_keys |= additions
    residual = len(actual_keys) - 432 - other - len(keys) + 1
    assert residual == 130, 'review source-cohort changes before refreshing this closed union'
    row = f'    ("QUESTPC7-REGISTERED-UNION", {residual}),\n'
    replace(str(equipment.relative_to(ROOT)),
            '    ("QUESTPC7-REGISTERED-UNION", 95),\n', row)
    text = equipment.read_text()
    if row not in text:
        anchor = '    ("SRV9-MARRIAGE", 2),\n'
        replace(str(equipment.relative_to(ROOT)), anchor, anchor +
                '    // Merged source/custody cohorts, refreshed by questpc7_merge_audit_inventories.py.\n' + row)

    # Three reviewed native sign sites extend the lane's closed callback inventory.
    callbacks = json.loads((ROOT / 'content/quests/questpc7-world-callbacks.json').read_text())['callbacks']
    sign_keys = {f'oteryn:quest-callback/questpc7-goblin-sign-{i}' for i in range(1, 4)}
    present = {row['key'] for row in callbacks}
    if present & sign_keys:
        pond_keys = {'oteryn:quest-callback/questpc7-goblin-pond'} & present
        trunk_keys = {f"oteryn:quest-callback/questpc7-goblin-trunk-{i}" for i in range(1, 4)} & present
        assert len(trunk_keys) in (0, 3), "all three source trunks must be registered together"
        expected = 106 + len(pond_keys) + len(trunk_keys)
        assert sign_keys <= present and len(present) == len(callbacks) == expected
        assert len(present - sign_keys - pond_keys - trunk_keys) == 103, 'review the original closed lane inventory'
        path = ROOT / 'crates/gameplay/quests/src/quest/world_callbacks/outfit_tests.rs'
        text = path.read_text()
        replacement = f'assert_eq!(parse(source)?.len(), {expected});'
        if replacement not in text:
            old = next(value for value in (103, 106, 107) if f'assert_eq!(parse(source)?.len(), {value});' in text)
            replace(str(path.relative_to(ROOT)), f'assert_eq!(parse(source)?.len(), {old});', replacement)

    defaults = Counter()
    for path in sorted((ROOT / 'content/dialogues/quest-rules').glob('quest-batch3-*.json')):
        for track in json.loads(path.read_bytes())['tracks']:
            defaults[track['initial']] += 1
    assert defaults == {-1: 192, 0: 52}, 'review registered NPC source corpus changes'
    path = ROOT / 'crates/gameplay/quests/src/quest/npc_rules/tests.rs'
    text = path.read_text()
    marker = 'fn lane_npc_activation_keeps_exact_legacy_hashes_defaults_and_rejects_incompatible_rollback'
    prefix, body = text.split(marker, 1)
    old = 'assert_eq!(defaults, BTreeMap::from([(-1, 180), (0, 50)]));'
    new = f'assert_eq!(defaults, BTreeMap::from([(-1, {defaults[-1]}), (0, {defaults[0]})]));'
    if old in body:
        assert body.count(old) == 1
        body = body.replace(old, new, 1)
    else:
        assert new in body
    rendered = prefix + marker + body
    if rendered != text:
        path.write_text(rendered)


if __name__ == '__main__':
    main()
