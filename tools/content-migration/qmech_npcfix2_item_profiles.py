"""M22: qualify the plain antler talisman using the retained XML/client importer."""
import argparse
import json
from pathlib import Path

import qengine_reward_item_profiles as shared

PATH = 'tools/content-schema/native-gameplay/qmech-npcfix2-item-profiles.json'
SCHEMA = 'OTERYN_QMECH_NPCFIX2_ITEM_PROFILES/v1'
KEY = 'oteryn:item.tibia.i22008'


def capture(root=shared.ROOT):
    packet = shared.capture(root, {22008})
    packet.update(schema=SCHEMA, producer='tools/content-migration/qmech_npcfix2_item_profiles.py',
                  scope='M22: physical custody for one non-stackable antler talisman per captured transformed antler stack.')
    (record,) = packet['records']
    if record['admission']['stack_class'] != 'NonStackable':
        raise ValueError('M22_TALISMAN_OUTPUT_STACK_CONFLICT')
    return packet


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root / PATH).read_bytes())
    if packet['schema'] != SCHEMA or [r['production_definition']['production_key'] for r in packet['records']] != [KEY]:
        raise ValueError('M22_ITEM_PROFILE_SCOPE')
    shared.admit_records(root, generated, profile_path, manifest_path, order, canonical, packet['records'])


def equipment_cohort(root):
    target = root / 'crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/ability_tests.rs'
    text = target.read_text()
    begin = '    // BEGIN GENERATED NPCFIX2 M22 PROFILE ADDITIONS\n'
    end = '    // END GENERATED NPCFIX2 M22 PROFILE ADDITIONS\n'
    block = begin + '    // Source-qualified antler talisman 22008; qmech_npcfix2_item_profiles.py\n    ("NPCFIX2-M22", 1),\n' + end
    if begin in text:
        first = text.index(begin)
        last = text.index(end, first) + len(end)
    else:
        first = last = text.index('    // BEGIN GENERATED QREADY PROFILE ADDITIONS')
    return target, text[:first] + block + text[last:]


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--equipment-cohort', action='store_true',
                        help='refresh the generated fixture after this profile is admitted')
    args = parser.parse_args()
    raw = json.dumps(capture(), indent=2, ensure_ascii=False).encode() + b'\n'
    target = shared.ROOT / PATH
    if args.check:
        if target.read_bytes() != raw:
            raise SystemExit('M22_ITEM_PROFILE_DRIFT')
    else:
        target.write_bytes(raw)
    if args.equipment_cohort:
        equipment, generated = equipment_cohort(shared.ROOT)
        if args.check:
            if equipment.read_text() != generated:
                raise SystemExit('M22_EQUIPMENT_COHORT_DRIFT')
        else:
            equipment.write_text(generated)
