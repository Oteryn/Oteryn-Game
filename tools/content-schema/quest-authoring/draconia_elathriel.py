"""Generate Elathriel's exact paid-key source programme from pinned offline bytes."""
import argparse
import hashlib
import json
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
SOURCE = 'data-global/npc/elathriel.lua'
SHA = '24dbbbe1ee2bcb74f74462ce131dd10cdf45801d53ecb586bd63993e1450cd7d'
NPC = 'oteryn:npc.elathriel'
QUEST = 'oteryn:quest.draconia_quest'


def outputs():
    with tarfile.open(ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as corpus:
        manifest = json.load(corpus.extractfile('corpus-manifest.json'))
        source = next(f for f in manifest['files'] if f['path'] == SOURCE and f['sha256'] == SHA)
        raw = corpus.extractfile(source['cache_path']).read()
    assert hashlib.sha256(raw).hexdigest() == SHA
    assert hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest() == source['git_blob_sha1']
    assert all(s in raw for s in [b'removeMoneyBank(5000)', b'addItem(2970, 1)', b'setActionId(3012)', b'getTopic(playerId) == 1'])
    reference = lambda line: f'crystal:{REVISION}:{SOURCE}:{line}:sha256:{SHA}'
    question = "If you are that curious, do you want to buy a key for 5000 gold? Don't blame me if you get sucked in."
    no = "Believe me, it's better for you that way."
    failure = 'Come back when you have enough money.'
    greeting = 'Be greeted |PLAYERNAME|. What is your {business} near the {hellgate}?'
    farewell = 'Asha Thrazi, stranger!'
    assert all(s.encode() in raw for s in [no, failure, greeting, farewell, 'Here it is.'])
    callback = dict(key='oteryn:quest-reward/draconia-key3012', quest=QUEST, npc=NPC, keywords=['yes'], topic=1, next_topic=0, storage=[], take=[], fee=dict(gold=5000, include_bank=True), repeatable=True, present=[], give=dict(item='oteryn:item.tibia.i2970',count=1,contents=[],key_number=3012), outfit=None, achievement=None, reply=['Here it is.'], failure_reply=[failure], failure_next_topic=0, sources=[reference(line) for line in [193,195,201,204,206,208,211,213]])
    offers = [dict(npc=NPC, keywords=['key','keys'], topic=topic, next_topic=1, storage=[], reply=[question]) for topic in [0,1]]
    offers.append(dict(npc=NPC, keywords=['no'], topic=1, next_topic=0, storage=[], reply=[no]))
    messages = dict(npc=NPC,greet=greeting,farewell=farewell,walkaway=farewell,sources=[reference(line) for line in [236,237,238]])
    return {'content/quests/questpc-draconia-reward-callbacks.json':dict(schema=1,callbacks=[callback]),'content/quests/questpc-draconia-reward-offers.json':dict(schema=1,offers=offers),'content/dialogues/quests/questpc-draconia-reward-greetings.json':dict(schema=1,npcs=[messages])}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for path, value in outputs().items():
        text = json.dumps(value, indent=2) + '\n'
        target = ROOT / path
        if args.check:
            assert target.read_text() == text, path
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(text)
    print('Elathriel exact5000 cash/bank ->copper3012 source sale verified; no invented storage')


if __name__ == '__main__':
    main()
