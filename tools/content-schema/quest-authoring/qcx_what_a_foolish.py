"""Compile the existing native entry binding for What a Foolish Quest."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
QUEST = 'oteryn:quest.what_a_foolish_quest'
PREFIX = 'oteryn:quest-progress/quest/u8_1/what_afoolish_quest/'
SOURCES = [
    'https://tibia.fandom.com/?curid=19799&oldid=1094085',
    'https://tibia.fandom.com/?curid=19801&oldid=1056289#Mission_1_-_First_Mission',
    'crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/bozo.lua:624-687',
]


def build(root):
    path = root / 'content/quests/definitions/quests-00200-00299.json'
    definition = next(
        row['definition'] for row in json.loads(path.read_bytes())['records']
        if row['definition']['identity']['key'] == QUEST
    )
    recipe = definition['oteryn_recipe']['payload']['recipe']
    recipe_bytes = json.dumps(recipe, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()
    tracks = {PREFIX + 'questline': 13, PREFIX + 'mission1': 2}
    callback = {
        'key': 'oteryn:quest-reward/what-a-foolish-join',
        'quest': QUEST,
        'npc': 'oteryn:npc.bozo',
        'keywords': ['yes'],
        'topic': 2,
        'next_topic': 0,
        'storage': [{'track': track, 'minimum': 0, 'maximum': 0, 'set': 1} for track in tracks],
        'requires_premium': True,
        'take': [], 'give': None, 'outfit': None, 'achievement': None,
        'reply': [
            "What a foolish decision! You are indeed a worthy candidate! But let's talk about business ...",
            'Being a jester is not just about telling jokes. A good jester heavily relies on requisites ...',
            'Getting some requisites will be your first job. First of all we need a good supply of water squirt flowers ...',
            "I'm making them on my own in my spare time but I need the right material. South of Thais, next to the Whiteflower Temple, you will find the ideal flowers ...",
            'Take a kitchen knife, cut the thickest and healthiest flower and bring it here. Then talk to me about your mission.',
        ],
        'sources': SOURCES,
    }
    return {
        'schema': 1,
        'quests': [{
            'quest': QUEST,
            'recipe_sha256': hashlib.sha256(recipe_bytes).hexdigest(),
            'tracks': [{'key': key, 'initial': 0, 'minimum': 0, 'maximum': maximum}
                       for key, maximum in tracks.items()],
            'stages': [{'key': 'join', 'completes': False,
                        'trigger': {'kind': 'npc_dialogue', 'callback': callback}}],
            'sources': SOURCES,
        }],
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    output = ROOT / 'content/quests/authored-bindings/qcx-what-a-foolish.json'
    payload = json.dumps(build(ROOT), indent=2, ensure_ascii=False) + '\n'
    if args.check:
        if output.read_text() != payload:
            raise ValueError('Foolish Quest binding differs from its generated document')
    else:
        output.write_text(payload)
