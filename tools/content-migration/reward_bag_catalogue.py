"""Lower captured W19 outcome evidence into the executable reward-bag catalogue."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEST = 'tools/content-schema/native-gameplay/reward-bag-catalogue.json'
SOURCES = ('reward-bag-profiles.json', 'event-item-actions.json', 'qengine-reward-bag-profiles.json')


def catalogue(root):
    inputs = [root / 'tools/content-schema/native-gameplay' / name for name in SOURCES]
    original, events, quests = [json.loads(path.read_bytes()) for path in inputs]
    if (original['schema'] != 'OTERYN_NATIVE_REWARD_BAG_PROFILES/v1'
            or events['schema'] != 'OTERYN_EVENT_ITEM_REWARDS/v1'
            or quests['schema'] != 'OTERYN_QENGINE_REWARD_BAG_PROFILES/v1'):
        raise ValueError('REWARD_BAG_CATALOGUE_SCHEMA')
    bags = original['bags']
    for row in events['rules']:
        if row['selection'] not in ('one', 'weighted', 'all', 'without_replacement'):
            raise ValueError('REWARD_BAG_SELECTION')
        bags.append({'constant': f"EVENT_{row['item_id']}",
                     'bag': f"oteryn:item.tibia.i{row['item_id']}", 'playable': True,
                     'selection': row['selection'], 'picks': row['picks'],
                     'roll_max': row.get('roll_max'), 'donor': row['donor'],
                     'rewards': [{'item': f"oteryn:item.tibia.i{reward['item_id']}",
                                  'name': '', 'weight': reward['weight'],
                                  'minimum': reward['minimum'], 'maximum': reward['maximum']}
                                 for reward in row['rewards']]})
    bags.extend(quests['bags'])
    if len({bag['bag'] for bag in bags}) != len(bags):
        raise ValueError('REWARD_BAG_DUPLICATE')
    return {'schema': 'OTERYN_REWARD_BAG_CATALOGUE/v1',
            'sources': [{'path': str(path.relative_to(root)),
                         'sha256': hashlib.sha256(path.read_bytes()).hexdigest()} for path in inputs],
            'bags': bags}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(catalogue(ROOT), indent=2, ensure_ascii=False) + '\n').encode()
    path = ROOT / DEST
    if args.check:
        if path.read_bytes() != raw:
            raise SystemExit('REWARD_BAG_CATALOGUE_DRIFT')
    else:
        path.write_bytes(raw)


if __name__ == '__main__':
    main()
