"""Exact retained Targuna NPC names/positions; no source services are inferred."""
import copy
import json

FACTS = 'imports/tibiawiki/npc-br/2026-09-28/tibiawiki-br-npc-facts.json'
SPECS = {
    'sterling': (66775, 433368, '05419a96c99353684b97e258a59cc0c51520a9c9802d12e6be2cb670ef474b62', 'Sterling', [[31928,31903,7]]),
    'emiliana': (66776, 433429, '51aec4aedd7018e8346f5ee26b4cbe6b870985cafeccbfaff84e564b7f68b742', 'Emiliana', [[31960,31901,6],[32412,32687,12]]),
    'morla': (66777, 433367, '1f666ba6139502082f14134423ef324775ef134332f0b36e3b17d4034c266a1f', 'Morla', [[33514,32748,8]]),
    'lizzie': (66778, 433420, 'e57050a26ba9e6a219d47a199cfb5a10b7397c36be8dc04916887b55fddca870', 'Lizzie', [[31941,31920,7]]),
}


def extend(declarations, root):
    pages = json.loads((root / FACTS).read_bytes())['pages']
    result = copy.deepcopy(declarations)
    for slug, (pageid, revid, digest, name, positions) in SPECS.items():
        page, = [p for p in pages if p.get('pageid') == pageid]
        if (page['revid'], page['sha256'], page['name'], page['positions']) != (revid, digest, name, positions):
            raise ValueError('TARGUNA_NPC_RETAINED_FACT_DRIFT:' + slug)
        npc, = [r for r in result['records'] if r.get('kind') == 'NPC'
                and r['identity']['key'] == 'oteryn:npc.' + slug]
        facts = {
            'oteryn:source.npc.bulk.name': name,
            'oteryn:source.npc.bulk.wiki_positions': json.dumps(positions, separators=(',', ':')),
            'oteryn:source.npc.questpc5.targuna_position_evidence': json.dumps(dict(
                path=FACTS, pageid=pageid, revid=revid, sha256=digest,
                url=f'https://www.tibiawiki.com.br/index.php?curid={pageid}&oldid={revid}'), sort_keys=True, separators=(',', ':')),
        }
        for field, value in facts.items():
            expected = dict(field_path=field, value=dict(type='Text', value=value))
            existing = [f for f in npc['fields'] if f['field_path'] == field]
            if existing and existing != [expected]:
                raise ValueError('TARGUNA_NPC_EXISTING_FACT_CONFLICT:' + field)
            if not existing:
                npc['fields'].append(expected)
        npc['fields'].sort(key=lambda f: f['field_path'])
    return result
