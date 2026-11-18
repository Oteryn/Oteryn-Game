"""Lower the City of Gold idol to the existing carried quest reward owner."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import tarfile

PATH = 'data-global/scripts/quests/adventures_of_galthen/actions_idol_of_tukh.lua'
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
# Verified at the pinned donor revision. The retained corpus has the same Git blob.
BLOB = 'e48a5a477d43ce6a2c38e2f23fc130004662205a'
DIGEST = '944dfe3c80292a4bfe6bae209634f8a94c885cdef5db2285954e2ff1442b52ac'
QUEST = 'oteryn:quest.adventures_of_galthen_quest'
TRACK = 'oteryn:quest-progress/adventures-of-galthen/idol-redeemed'
OUTPUT = 'content/quests/authored-bindings/qcx81-adventures-of-galthen.json'
WIKI = 'https://tibia.fandom.com/?curid=94719&oldid=1199250#Ahau'


def build(root):
    corpus = root / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz'
    with tarfile.open(corpus) as archive:
        raw = archive.extractfile(f'blobs/{BLOB}').read()
    if hashlib.sha256(raw).hexdigest() != DIGEST:
        raise ValueError('Living Idol of Tukh donor blob drift')
    script = raw.decode()
    item, female, male, name = re.search(
        r'\[(\d+)\]\s*=\s*\{\s*female = (\d+),\s*male = (\d+),\s*msg = "([^"]+)"',
        script,
    ).groups()
    feedback = re.search(
        r'sendTextMessage\(MESSAGE_EVENT_ADVANCE, "([^"]+)" \.\. choice.msg \.\. "([^"]+)"',
        script,
    )
    outfits = json.loads((root / 'content/cosmetics/outfits/authoring.json').read_bytes())
    outfit = next(row for row in outfits['document']['outfits']
                  if row['key'] == 'oteryn:outfit.ancient_aucar')
    if (outfit['female']['id'], outfit['male']['id']) != (int(female), int(male)):
        raise ValueError('Living Idol of Tukh outfit pair drift')
    item_key = f'oteryn:item.tibia.i{item}'
    sources = [WIKI, f'crystal:{REVISION}:{PATH}:1-31:sha256:{DIGEST}']
    reward = {
        'key': 'oteryn:quest-reward/qcx81-living-idol-of-tukh',
        'quest': QUEST,
        # Required quest reward metadata. Carried USE excludes NPC dialogue routing.
        'npc': 'oteryn:npc.ahau_s_dead_chamberlain',
        'keywords': ['idol'],
        'topic': 0,
        'next_topic': 0,
        'storage': [{'track': TRACK, 'minimum': 0, 'maximum': 0, 'set': 1}],
        'take': [{'item': item_key, 'count': 1}],
        'give': None,
        'outfit': {'outfit': outfit['key'], 'addons': 0},
        'forbids_outfits': [outfit['key']],
        'achievement': None,
        'carried_item': item_key,
        'reply': [feedback[1] + name + feedback[2]],
        'sources': sources,
    }


    shard = root / 'content/quests/definitions/quests-00100-00199.json'
    definitions = json.loads(shard.read_bytes())['records']
    definition = next(row['definition'] for row in definitions
                      if row['definition']['identity']['key'] == QUEST)
    recipe = definition['oteryn_recipe']['payload']['recipe']
    recipe_bytes = json.dumps(recipe, ensure_ascii=False, sort_keys=True,
                              separators=(',', ':')).encode()
    return {'schema': 1, 'quests': [{
        'quest': QUEST,
        'recipe_sha256': hashlib.sha256(recipe_bytes).hexdigest(),
        'tracks': [{'key': TRACK, 'initial': 0, 'minimum': 0, 'maximum': 1}],
        'stages': [{
            'key': 'redeem-living-idol',
            'completes': False,
            'trigger': {'kind': 'carried_use', 'callback': reward},
        }],
        'sources': sources,
    }]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[3]
    path = root / OUTPUT
    output = (json.dumps(build(root), indent=2, ensure_ascii=False) + '\n').encode()
    if args.check:
        if path.read_bytes() != output:
            raise SystemExit('Living Idol of Tukh binding drift')
    else:
        path.write_bytes(output)


if __name__ == '__main__':
    main()
