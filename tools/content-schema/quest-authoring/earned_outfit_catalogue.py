"""Promote the retained Wiki-only looks and the Crystal Task Board reward targets."""
import hashlib
import json


OUTFIT_EVIDENCE = (
    'docs/reference/content-prep/outfits-content-1/'
    'admission-evidence/evidence-00000-00135.json'
)
WIKI_OUTFITS = (
    ('Aerial Disciple', 'aerial_disciple'),
    ('Captains', 'captains'),
    ('Illuminated Warrior', 'illuminated_warrior'),
    ('Moon Guardian', 'moon_guardian'),
    ('Phoenix Evoker', 'phoenix_evoker'),
    ('Vampire Noble', 'vampire_noble'),
)


def append_wiki_outfits(root, outfits, looks, addon_patterns):
    raw = (root / OUTFIT_EVIDENCE).read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    records = json.loads(raw)['records']
    by_name = {row['facts']['name']['value']: (index, row['facts'])
               for index, row in enumerate(records)}
    for name, slug in WIKI_OUTFITS:
        index, facts = by_name[name]
        for field in ('male_look_type', 'female_look_type', 'premium'):
            if facts[field]['classification'] != 'PROVEN':
                raise ValueError(f'unproven outfit field: {name}/{field}')
        key = f'oteryn:outfit.{slug}'
        male = facts['male_look_type']['value']
        female = facts['female_look_type']['value']
        if any(row['key'] == key or (row['male']['id'], row['female']['id']) == (male, female)
               for row in outfits):
            raise ValueError(f'duplicate Wiki outfit: {name}')
        if min(addon_patterns[male], addon_patterns[female]) < 2:
            raise ValueError(f'client cannot render both addons: {name}')
        source = {
            'reference': f'{OUTFIT_EVIDENCE}#/records/{index}',
            'revision': facts['male_look_type']['sources'][0]['source'],
            'sha256': digest,
        }
        outfits.append({
            'key': key, 'revision': 'wiki-content-1',
            'female': {'id': female, 'source_record_sha256': looks[female]},
            'male': {'id': male, 'source_record_sha256': looks[male]},
            'addon_count': 2, 'starter_female': False, 'starter_male': False,
            'starter_addons_female': 0, 'starter_addons_male': 0,
            'premium': facts['premium']['value'], 'unlock_sources': [source], 'source': source,
        })
    outfits.sort(key=lambda row: row['key'])


def append_task_board_outfit_mappings(outfits, mappings):
    # Crystal 201189f data/scripts/lib/task_board_shop.lua, in shop-offer order.
    targets = []
    for slug, look in (('feral_trapper', 1907), ('falconer', 1282)):
        key = f'oteryn:outfit.{slug}'
        matches = [row for row in outfits if look in (row['male']['id'], row['female']['id'])]
        if len(matches) != 1 or matches[0]['key'] != key or matches[0]['addon_count'] != 2:
            raise ValueError(f'Task Board outfit identity drift: {key}')
        targets.append({'kind': 'Outfit', 'outfit_key': key})
        targets.extend({'kind': 'Addon', 'outfit_key': key, 'addon': addon} for addon in (1, 2))
    mappings.extend({
        'grant_key': f'oteryn:taskboard/shop/offer-{offer}',
        'target': target,
        'compatible_provenance': ['task-board-shop-v1'],
    } for offer, target in enumerate(targets))


def append_consumptive_outfit_grants(item_packet, outfits):
    # Retained TibiaWiki outfit quest revisions. Crystal has the item records
    # but no corresponding ordinary-USE action in its global cosmetic table.
    specs = (
        ('ancient_aucar', '101463:1045728', ((40578, 0),)),
        ('draccoon_herald', '104371:1046106', ((44753, 2), (44754, 1))),
        ('rootwalker', '107079:1065255', ((48514, 1), (48515, 2))),
    )
    definitions = {row['key']: row for row in outfits}
    grants = {row['key']: row for row in item_packet['grants']}
    for slug, revision, items in specs:
        outfit = definitions[f'oteryn:outfit.{slug}']
        looks = sorted([outfit['male']['id'], outfit['female']['id']])
        citation = f'wiki:{revision}'
        pair = f'oteryn:item-grant.outfit.{looks[0]}-{looks[1]}'
        base = f'{pair}.base'
        grants.setdefault(base, {'key': base, 'kind': 'outfit', 'look_types': looks,
                                 'addon': 0, 'donor': citation})
        for item, addon in items:
            key = f'{pair}.addon{addon}' if addon else base
            if addon:
                grants.setdefault(key, {'key': key, 'kind': 'addon', 'look_types': looks,
                                        'addon': addon, 'donor': citation})
            item_packet['rules'].append({
                'item_id': item, 'donor': citation, 'mode': 'direct', 'premium': True,
                'choices': [{'grants': [key], 'owned': key, 'required': [base] if addon else [],
                             'cues': []}],
            })
    item_packet['grants'] = [grants[key] for key in sorted(grants)]
    item_packet['rules'].sort(key=lambda row: row['item_id'])
