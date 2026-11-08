"""Finite retained Draccoon identity/position overlay for the selected Cook recipe."""
import copy
import json

FACTS = 'imports/tibiawiki/npc-br/2026-09-28/tibiawiki-br-npc-facts.json'
KEY = 'oteryn:npc.the_draccoon_npc'
PAGE_SHA = '90adb85049eab4c3dc9b8eed74e065ca552b10a0f46d6d2f8bdcd8032f251c74'


def extend(declarations, root):
    page, = [p for p in json.loads((root / FACTS).read_bytes())['pages']
             if p.get('pageid') == 61322]
    if (page['revid'], page['sha256'], page['name'], page['positions']) != (
            416298, PAGE_SHA, 'The Draccoon (NPC)', [[32973, 32078, 8]]):
        raise ValueError('COOK_DRACCOON_RETAINED_FACT_DRIFT')
    result = copy.deepcopy(declarations)
    npc, = [r for r in result['records'] if r.get('kind') == 'NPC'
            and r['identity']['key'] == KEY]
    facts = {
        'oteryn:source.npc.bulk.name': 'The Draccoon',
        'oteryn:source.npc.bulk.wiki_positions': '[[32973,32078,8]]',
        'oteryn:source.npc.questpc5.cook_position_evidence': json.dumps(dict(
            path=FACTS, pageid=61322, revid=416298, sha256=PAGE_SHA,
            url='https://www.tibiawiki.com.br/index.php?curid=61322&oldid=416298'),
            sort_keys=True, separators=(',', ':')),
    }
    for field, value in facts.items():
        expected = dict(field_path=field, value=dict(type='Text', value=value))
        existing = [f for f in npc['fields'] if f['field_path'] == field]
        if existing and existing != [expected]:
            raise ValueError('COOK_DRACCOON_EXISTING_FACT_CONFLICT:' + field)
        if not existing:
            npc['fields'].append(expected)
    npc['fields'].sort(key=lambda f: f['field_path'])
    return result
