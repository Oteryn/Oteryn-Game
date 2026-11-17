"""Bind selected Oteryn TALK starters; phrases are authored route choices.

These are the selected recipes' briefings, not claims of original Lua dialogue
parity or completed quests. Later stages retain their separate native owners.
"""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = 'content/quests/authored-bindings/m01a-starter-talk-routes.json'
ROUTES = [
    ('goblin_merchant_quest', 'rafzan', 'six jobs'),
    ('machinery_of_war_quest', 'baxter', 'orc-war commission'),
    ('authored.annual_autumn_vintage', 'emilie', 'cellar assignment'),
    ('barbarian_test', 'sven', 'barbarian test'),
]

def build():
    index = json.loads((ROOT / 'content/quests/definitions/index.json').read_text())
    definitions = {row['definition']['identity']['key']: (path, row['definition'])
                   for path in index['shards']
                   for row in json.loads((ROOT / path).read_text())['records']}
    npc_index = json.loads((ROOT / 'content/npcs/definitions/index.json').read_text())
    npcs = {row['declaration']['identity']['key'] for path in npc_index['shards']
            for row in json.loads((ROOT / path).read_text())['records']}
    quests = []
    for slug, npc, keyword in ROUTES:
        key = 'oteryn:quest.' + slug
        assert 'oteryn:npc.' + npc in npcs, 'briefing NPC must exist in the real catalogue'
        path, definition = definitions[key]
        imported = definition.get('definition_profile') != 'oteryn_authored_v1'
        if imported:
            selected = definition['oteryn_recipe']
            assert selected['profile'] == 'chosen_source_completion_v1'
            assert selected['chosen_data_complete']
            assert selected['payload']['identity'] == definition['identity']
            recipe = selected['payload']['recipe']
        else:
            assert definition['data_complete']
            recipe = definition['recipe']
        stage = recipe['stages'][0]
        assert stage['key'] == 's1' and stage['kind'] == 'talk' and stage['count'] == 1
        assert npc.capitalize() in stage['targets']
        assert not recipe['requirements']['prerequisites']
        prefix = 'chosen-source/' if imported else 'authored/'
        local = slug.removeprefix('authored.')
        track = 'oteryn:quest-progress/' + prefix + local + '/s1'
        sources = [path + ': selected recipe s1',
                   'tools/content-schema/quest-authoring/m01a_talk_routes.py: authored briefing choices']
        callback = {'key': 'oteryn:quest-reward/native-talk/' + local + '/s1',
                    'quest': key, 'npc': 'oteryn:npc.' + npc,
                    'keywords': [keyword], 'topic': 0, 'next_topic': 0,
                    'storage': [{'track': track, 'minimum': 0, 'maximum': 0, 'set': 1}],
                    'take': [], 'give': None, 'outfit': None, 'achievement': None,
                    'reply': ['Next: ' + recipe['stages'][1]['objective']], 'sources': sources}
        if recipe['requirements']['min_level']:
            callback['minimum_level'] = recipe['requirements']['min_level']
        if recipe['requirements']['premium']:
            callback['requires_premium'] = True
        quests.append({'quest': key,
                       'recipe_sha256': hashlib.sha256(json.dumps(recipe, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest(),
                       'tracks': [{'key': track, 'initial': 0, 'minimum': 0, 'maximum': 1}],
                       'stages': [{'key': 's1', 'completes': False,
                                   'trigger': {'kind': 'npc_dialogue', 'callback': callback}}],
                       'sources': sources})
    return (json.dumps({'schema': 1, 'quests': quests}, indent=2, ensure_ascii=False) + '\n').encode()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    expected = build()
    path = ROOT / OUTPUT
    if args.check:
        if not path.is_file() or path.read_bytes() != expected:
            raise ValueError('stale M01a starter TALK bindings')
    else:
        path.write_bytes(expected)

if __name__ == '__main__':
    main()
