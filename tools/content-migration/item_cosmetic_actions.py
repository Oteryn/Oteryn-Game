"""Capture source item cosmetic grants without promoting canonical cosmetics authoring."""
import argparse
import hashlib
import json
import re
import xml.etree.ElementTree as ET
from pathlib import Path

from event_item_actions import PIN, ITEMS, table

DEST = 'tools/content-schema/native-gameplay/item-cosmetic-actions.json'
PROVENANCE = 'crystal-item-cosmetics-201189f-v1'


def capture(crystal):
    sources, grants, rules = {}, {}, []

    def read(path):
        raw = (crystal / path).read_bytes()
        sources[path] = hashlib.sha256(raw).hexdigest()
        return raw.decode()

    mount_xml = read('data/XML/mounts.xml')
    mounts = {int(row.attrib['id']): int(row.attrib['clientid'])
              for row in ET.fromstring(mount_xml).findall('mount')}
    constants = {name: int(value) for name, value in re.findall(
        r'(CONST_ME_\w+)\s*=\s*(\d+)', read('src/utils/utils_definitions.hpp'))}

    def achievement(name):
        return 'oteryn:achievement/' + re.sub(r'[^a-z0-9]+', '_', name.lower()).strip('_')

    def cue(symbol):
        return {'binding': 'canary.appearance:effect/' + symbol.removeprefix('CONST_ME_').lower(),
                'effect_id': constants[symbol]}

    def outfit(looks, addon, donor):
        looks = sorted(looks)
        key = f'oteryn:item-grant.outfit.{looks[0]}-{looks[1]}.' + (f'addon{addon}' if addon else 'base')
        value = {'key': key, 'kind': 'addon' if addon else 'outfit',
                 'look_types': looks, 'addon': addon, 'donor': donor}
        grants.setdefault(key, value)
        return key

    def mount(mount_id, donor):
        key = f'oteryn:item-grant.mount.{mount_id}'
        source_line = next(i for i, line in enumerate(mount_xml.splitlines(), 1)
                           if re.search(r'<mount\s+id="' + str(mount_id) + r'"', line))
        grants.setdefault(key, {'key': key, 'kind': 'mount', 'mount_id': mount_id,
                              'look_type': mounts[mount_id], 'donor': donor,
                              'identity_source': f'data/XML/mounts.xml:{source_line}'})
        return key

    def add(item, donor, choices, premium=False, mode='direct', **fields):
        rules.append({'item_id': item, 'donor': donor, 'mode': mode,
                      'premium': premium, 'choices': choices, **fields})

    path = ITEMS + 'usable_outfit_items.lua'
    raw = read(path)
    entries, _ = table(raw, 'outfitConfig')
    for item, values in entries.items():
        line = next(i for i, row in enumerate(raw.splitlines(), 1) if f'[{item}]' in row)
        donor = f'{path}:{line}'
        base = outfit([values['female'], values['male']], 0, donor)
        addon = values.get('addon', 0)
        selected = outfit([values['female'], values['male']], addon, donor)
        choice = {'grants': [selected], 'owned': selected, 'required': [base] if addon else [],
                  'cues': [cue(values.get('effect', 'CONST_ME_GIFT_WRAPS'))]}
        if 'achievement' in values:
            choice['achievement_when_full'] = achievement(values['achievement'])
            choice['complete_grants'] = [outfit([values['female'], values['male']], a, donor) for a in (1, 2)]
        if 'orangeText' in values:
            choice['speech'] = values['orangeText']
        elif 'whiteText' in values:
            choice['feedback'] = values['whiteText']
        add(item, donor, [choice], premium=True)

    path = ITEMS + 'usable_mount_items.lua'
    raw = read(path)
    entries, _ = table(raw, 'config')
    for item, values in entries.items():
        line = next(i for i, row in enumerate(raw.splitlines(), 1) if f'[{item}]' in row)
        donor = f'{path}:{line}'
        grant = mount(values['mountId'], donor)
        choice = {'grants': [grant], 'owned': grant, 'required': [], 'cues': [],
                  'speech': values['tameMessage'], 'achievements': [achievement('Natural Born Cowboy')]}
        if 'achievement' in values:
            choice['achievements'].insert(0, achievement(values['achievement']))
        if item in (23538, 23684, 23685):  # usable_mount_items.lua:26-27.
            choice['achievement_progress'] = [{'key': achievement('Vortex Tamer'), 'target': 3}]
        add(item, donor, [choice], premium=True)

    path = ITEMS + 'usable_afflicted_outfit_items.lua'
    raw = read(path)
    required, line = table(raw, 'requiredItemIds')
    pieces = list(required.values())
    base = outfit([430, 431], 0, f'{path}:49-50')
    for item in pieces:
        add(item, f'{path}:{line}', [{'grants': [base], 'owned': base, 'required': [],
            'cues': [cue('CONST_ME_POFF')], 'speech': 'You have restored an outfit.',
            'private_speech': True}],
            premium=True, mode='pieces', required_items=pieces)
    for item, addon, line in ((12786, 2, 10), (12787, 1, 21)):
        selected = outfit([430, 431], addon, f'{path}:{line}')
        add(item, f'{path}:{line}', [{'grants': [selected], 'owned': selected, 'required': [base],
            'cues': [cue('CONST_ME_POFF')],
            'speech': 'You gained a plague ' + ('mask' if addon == 2 else 'bell') + ' for your outfit.',
            'private_speech': True,
            'achievement_progress': [{'key': achievement('Beak Doctor'), 'target': 2}]}], premium=True)

    for name in ('phantasmal_jade', 'singeing_steed', 'spirit_of_purity'):
        path = ITEMS + f'usable_{name}_items.lua'
        raw = read(path)
        values, line = table(raw, 'config')
        grant = mount(values['mountId'], f'{path}:{line}')
        components = [{'item_id': item, 'key': row['key'], 'count': row['count']}
                      for item, row in values['requiredItems'].items()]
        achievements = [achievement(name) for name in re.findall(r'player:addAchievement\("([^"]+)"\)', raw)]
        effect = re.search(r'sendMagicEffect\((CONST_ME_\w+)\)', raw)[1]
        completion_feedback = re.search(r'player:sendTextMessage\(MESSAGE_EVENT_ADVANCE, "([^"]+)"\)', raw)[1]
        for component in components:
            add(component['item_id'], f'{path}:{line}', [{'grants': [grant], 'owned': grant,
                'required': [], 'cues': [cue(effect)], 'achievements': achievements,
                'completion_feedback': completion_feedback}],
                mode='parts', components=components, progress_key=component['key'])

    path = 'data-crystal/scripts/special/addon_doll.lua'
    raw = read(path)
    entries, line = table(raw, 'outfits')
    choices = []
    for index, row in entries.items():
        donor = f'{path}:{line + index}'
        for addon in (1, 2):
            selected = outfit([row[1], row[2]], addon, donor)
            # Player::addOutfit (player.cpp:6840-6851) inserts the base with the addon.
            choices.append({'grants': [outfit([row[1], row[2]], 0, donor), selected],
                            'owned': selected, 'required': [], 'cues': [cue('CONST_ME_GIFT_WRAPS')]})
    add(int(re.search(r'local itemDollId = (\d+)', raw)[1]), f'{path}:53-71', choices, mode='random_outfit')

    path = 'data-crystal/scripts/special/mount_random.lua'
    raw = read(path)
    entries, line = table(raw, 'storeMounts')
    choices = []
    for index, row in entries.items():
        selected = mount(row[1], f'{path}:{line + index}')
        choices.append({'grants': [selected], 'owned': selected, 'required': [],
                        'cues': [cue('CONST_ME_FIREWORK_RED')]})
    add(int(re.search(r'local itemMountId = (\d+)', raw)[1]), f'{path}:44-68', choices, mode='random_mount')
    rules.sort(key=lambda row: row['item_id'])
    if len({r['item_id'] for r in rules}) != len(rules):
        raise ValueError('duplicate item cosmetic action')
    return {'schema': 'OTERYN_ITEM_COSMETICS/v1', 'crystal_commit': PIN,
            'provenance': PROVENANCE,
            'sources': [{'path': path, 'sha256': digest} for path, digest in sorted(sources.items())],
            'grants': [grants[key] for key in sorted(grants)], 'rules': rules}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(capture(args.crystal), indent=2, ensure_ascii=False) + '\n').encode()
    path = args.root / DEST
    if args.check:
        if path.read_bytes() != raw:
            raise SystemExit('ITEM_COSMETIC_ACTIONS_DRIFT')
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)


if __name__ == '__main__':
    main()
