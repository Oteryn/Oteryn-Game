"""Owner ITEMHOLD-Q continuation: exact full-definition-charge projection.

Keep OTS raw arguments and their conflicting source disposition as witnesses.
Wiki quest consensus selects the reward entity; its current Item definition supplies
full charges. This never admits partial charges or another placement/quantity.
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CASES = {
    # Crystal201189f chest UID6063 passes raw1 for the Garlic Necklace.
    # Its one-charge instance belongs to charge_native, not full defaults.
    'canary:reward-claim/kv/powerring2': (3056, 200, 200, {'x': 32601, 'y': 31776, 'z': 9},
        'Power Ring Quest', 'Bronze Amulet', 2466, 1194240,
        '5fa91126008b798f1c9e3950dcd01757133a7aa0d0d23a4ae6baf3c4f9bb71c9'),
}


def qualifies(item, pilot_key, position, key, raw_count, root=ROOT):
    case = CASES.get(pilot_key)
    if case is None:
        return False
    number, raw, charges, expected_position, title, entity, page, revision, digest = case
    if (key != f'oteryn:item.tibia.i{number}' or type(raw_count) is not int or raw_count != raw
            or position != expected_position or not item or not item.get('materializable')
            or item.get('identity') != {'family': 'Item', 'key': key, 'revision': 'definition-r1'}
            or item.get('stack_class') != 'NonStackable'
            or item.get('semantics', {}).get('charges') != {
                'state': 'KNOWN', 'value': {'count': {'state': 'KNOWN', 'value': charges}}}):
        return False
    observed, = json.loads((root / 'imports/tibiawiki/facts/items-stats.json').read_bytes())['records'][key]['observations']
    if (observed['page_id'] != page or observed['revision_id'] != revision
            or observed['content_sha256'] != digest or observed['fields']['charges'] != str(charges)):
        raise ValueError('ITEMHOLD_CHARGE_ITEM_WIKI_PIN')
    witnesses = []

    def visit(value):
        if isinstance(value, dict):
            if value.get('title') == title and value.get('provider') in ('tibia_fandom', 'tibiawiki_br'):
                refs = value.get('source_fields', {}).get('reward_entity_references', [])
                if any(r['entity'] == entity for r in refs):
                    witnesses.append((value['provider'], value['revid'], value['content_sha256']))
            for child in value.values():
                visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)

    visit(json.loads((root / 'tools/content-schema/quest-authoring/wiki_quest_facts.json').read_bytes()))
    expected = {('tibia_fandom', 683946, 'b5ef6edb39a0926308fb6bf73d3b51a9ee853ca96a810ce8ce040c03907be287'),
                ('tibiawiki_br', 201567, 'fcaf455986599ae199df12901f1b2193998046d71a43e47d550c20bd87393241')}
    if not expected <= set(witnesses):
        raise ValueError('ITEMHOLD_CHARGE_QUEST_CONSENSUS')
    return True


def variant_override(claim, charge, items, root=ROOT):
    if charge['reward_field'] != 'items':
        return False
    placement = claim['placements'][charge['placement_index']]
    source = placement['reward']['items'][charge['reward_index']]
    if source['count'] != charge['source_raw_argument']:
        return False
    number = int(source['item']['key'].split(':item/')[1])
    key = f'oteryn:item.tibia.i{number}'
    return qualifies(items.get(key), claim['identity']['key'], placement['position'], key,
                     charge['source_raw_argument'], root)
