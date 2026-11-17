"""Reproduce questpc4's reward execution fixtures from the pinned Crystal table.

The fixtures certify the chest path only. NPC missions and access puzzles need
their own execution evidence before a whole quest can be declared complete.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / 'tools/content-schema/reward-claim-authoring'))
import capture_container_native_source as capture
import container_native
import lua_tables
import reward_claim_authoring as plain

TITLES = (
    'Thais Lighthouse Quest',
    'The Annihilator Quest', 'The Ape City Quest', 'The Desert Dungeon Quest',
    'The Explorer Society Quest', 'The Hidden City of Beregar Quest',
    'The Ice Islands Quest', 'The Inquisition Quest', 'The New Frontier Quest',
    'The Outlaw Camp Quest', 'The Paradox Tower Quest', 'The Pits of Inferno Quest',
    'The Shattered Isles Quest', 'The Thieves Guild Quest',
    'The White Raven Monastery Quest',
    'Threatened Dreams Quest', 'Throwing Star Quest', 'Time Ring Quest', 'To Blind the Enemy Quest',
    'To Outfox a Fox Quest', 'Tomes of Knowledge Quest', 'Triangle Tower Quest',
    'Troll Cave Quest', 'Vampire Hunter Quest', 'Vampire Shield Quest',
    'Waterfall Quest', 'Wedding Ring Quest', 'What a Foolish Quest',
)
OUTPUT = Path(__file__).with_name('reward-cases.json')


def build(crystal):
    _, table = capture.read(crystal, capture.TABLE)
    entries = {field['key']: (field['line'], lua_tables.as_python(field['value']))
               for field in lua_tables.assignments(table, {'ChestUnique'})['ChestUnique']['fields']}
    items = plain.load_items()
    evidence, _ = container_native.source()
    texts = {row['unique_id']: row for row in evidence['text_entries']}
    achievements = {row['unique_id']: row['name'] for row in evidence['achievement_entries']}
    definitions = [r['definition']
                   for path in sorted((ROOT / plain.CONTENT_DIR).glob('reward-claim*.json'))
                   for r in json.loads(path.read_bytes())['records']]
    result = []
    for title in TITLES:
        quest = ('canary:quest/desert_dungeon_quest' if title == 'The Desert Dungeon Quest'
                 else 'canary:quest/' + title.lower().replace(' ', '_'))
        chests = []
        for definition in definitions:
            if definition.get('quest', {}).get('key') != quest:
                continue
            for placement in definition['placements']:
                binding = placement['source_binding']
                unique, = [r['unique_id'] for r in binding['legacy_unique_ids']
                           if r['server'] == 'crystalserver']
                line, source = entries[unique]
                point = binding['project_position']
                if source['itemPos'] != point or source['itemId'] != placement['appearance_tibia_id']:
                    raise ValueError(f'{title}: source placement mismatch UID{unique}')
                charge_rows = []
                key_number = None
                binding = placement['reward'].get('key_binding')
                if binding is not None:
                    prefix, number = binding.rsplit('/', 1)
                    if (prefix != 'canary:door-key' or not number.isascii() or not number.isdecimal()
                            or not 1 <= int(number) <= 65535 or source.get('isKey') is not True
                            or source.get('storage') != {'expr': f'Storage.Quest.Key.ID{number}'}):
                        raise ValueError(f'{title}: key binding mismatch UID{unique}')
                    key_number = int(number)
                elif source.get('isKey'):
                    raise ValueError(f'{title}: missing source key binding UID{unique}')
                rewards = []
                for item, count in source['reward']:
                    key = f'oteryn:item.tibia.i{item}'
                    if item in (3082, 3083, 3084, 3085):
                        if container_native.fact(items[key], 'charges', 'count') != count:
                            raise ValueError(f'{title}: partial source charges UID{unique}')
                        charge_rows.append([key, count])
                        count = 1
                    rewards.append([key, count])
                override = None
                if unique == 6078:
                    # PR2770 Q5=A explicitly retains canonical12510 over donor3208.
                    # This exact deviation is also in quest_callback_coverage.py.
                    accepted = 'oteryn:item.tibia.i12510'
                    if (definition['identity']['key'] !=
                            'oteryn:reward-claim.quest.u7_6.explorer_society.giant_smith_hammer.hammer'
                            or source['reward'] != [[3208, 1]]
                            or placement['reward'] != {
                                'items': [{'count': 1, 'item': plain.item_ref(accepted)}]}):
                        raise ValueError(f'{title}: accepted hammer deviation changed UID{unique}')
                    rewards = [[accepted, 1]]
                    override = 'https://github.com/Oteryn/Oteryn-Game/pull/2770#issuecomment-6542716086 Q5=A (donor3208, accepted12510)'
                text = texts.get(unique)
                achievement = None
                if unique in achievements:
                    achievement = 'oteryn:achievement/annihilator'
                    if (achievements[unique] != 'Annihilator'
                            or placement.get('achievement_grant') != {'state': 'KNOWN', 'value': {
                                'owner': 'Achievement', 'request': 'grant', 'achievement': {
                                    'family': 'Achievement', 'key': achievement, 'revision': '1'}}}):
                        raise ValueError(f'{title}: achievement mismatch UID{unique}')
                elif 'achievement_grant' in placement:
                    raise ValueError(f'{title}: unverified achievement UID{unique}')
                body = text['text'].strip('\n') if text else None
                root_text = None
                if source.get('container'):
                    native, = [p for p in definition['native_contents']['placements']
                               if p['unique_id'] == unique]
                    root = f"oteryn:item.tibia.i{source['container']}"
                    children = [[r['item']['key'], r['count']] for r in native['contents']]
                    if native['root']['key'] != root or children != rewards:
                        raise ValueError(f'{title}: incomplete container UID{unique}')
                    if text is not None or any('text' in child for child in native['contents']):
                        raise ValueError(f'{title}: needs an authored-child-text case UID{unique}')
                    quantity = 1
                else:
                    root, quantity = rewards[0]
                    native_leaf = None
                    if achievement is not None:
                        native_leaf, = [p for p in definition['native_contents']['placements']
                                        if p['unique_id'] == unique]
                        if (quantity != 1 or native_leaf['root'] != plain.item_ref(root)
                                or native_leaf['capacity'] is not None or native_leaf['contents']
                                or native_leaf.get('achievement') != {
                                    'family': 'Achievement', 'key': achievement, 'revision': '1'}):
                            raise ValueError(f'{title}: native choice mismatch UID{unique}')
                    if body is not None:
                        native, = [p for p in definition['native_contents']['placements']
                                   if p['unique_id'] == unique]
                        written = {'item': plain.item_ref(root), 'text_ref': {
                            'length': len(body), 'placeholders': [],
                            'sha256': hashlib.sha256(body.encode()).hexdigest()}}
                        if (quantity != 1 or native['root'] != plain.item_ref(root)
                                or native['capacity'] is not None or native['contents']
                                or native.get('root_text') != body
                                or text['item_id'] != int(root.rsplit('.i', 1)[1])
                                or placement['reward'].get('written_text') != written):
                            raise ValueError(f'{title}: leaf text mismatch UID{unique}')
                        root_text = body
                    if (len(rewards) != 1 or (definition['readiness'] != 'ready'
                            and key_number is None and root_text is None and native_leaf is None)
                            or placement['reward']['items'] != [
                                {'count': quantity, 'item': plain.item_ref(root)}]):
                        raise ValueError(f'{title}: leaf mismatch UID{unique}')
                    children = []
                chests.append({
                    'claim': definition['identity']['key'],
                    'position': [point['x'], point['y'], -point['z']],
                    'unique': unique, 'appearance': source['itemId'],
                    'root': root, 'quantity': quantity, 'children': children,
                    'charges': charge_rows,
                    'key_number': key_number, 'root_text': root_text,
                    'achievement': achievement,
                    'source': f'crystal:{capture.REVISION}:{capture.TABLE}:{line}'
                              + (f'; {override}' if override else ''),
                })
        if not chests:
            raise ValueError(f'{title}: no executable source chest')
        result.append({'title': title, 'chests': sorted(chests, key=lambda c: c['claim'])})
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    text = json.dumps(build(args.crystal), indent=2) + '\n'
    if args.check:
        if OUTPUT.read_text() != text:
            parser.exit(1, 'questpc4 reward cases are stale\n')
    else:
        OUTPUT.write_text(text)
    print(f'questpc4 reward cases: {len(TITLES)} source chest cohorts')


if __name__ == '__main__':
    main()
