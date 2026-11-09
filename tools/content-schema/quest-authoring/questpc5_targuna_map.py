"""M16: source-bound Sterling map dialogue; no whole-Targuna qualification claim."""
from __future__ import annotations
import argparse,hashlib,json,re,tarfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
REVISION="9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
SOURCE_PATH="data-global/npc/sterling.lua"
SOURCE_SHA256="3069d6f8265d9b414aedc6c50460268d5c3e98c0a65257cc48c54750e44516a9"
DOCUMENTS={'content/quests/npc-callbacks/m16-targuna-marker.json': {'schema': 1,
                                                            'callbacks': [{'key': 'oteryn:quest-reward/targuna/sterling-mark-treasure',
                                                                           'quest': 'oteryn:quest.targuna_quest',
                                                                           'npc': 'oteryn:npc.sterling',
                                                                           'keywords': ['yes'],
                                                                           'topic': 2,
                                                                           'next_topic': 0,
                                                                           'storage': [{'track': 'oteryn:quest-progress/crystalserver/quest/u15_24/targuna/long_lost_treasure/mission',
                                                                                        'minimum': 3,
                                                                                        'maximum': 3,
                                                                                        'set': 4},
                                                                                       {'track': 'oteryn:quest-progress/crystalserver/quest/u15_24/targuna/long_lost_treasure/treasure_marked',
                                                                                        'minimum': 0,
                                                                                        'maximum': 0,
                                                                                        'set': 1}],
                                                                           'requires': [{'track': 'oteryn:quest-progress/crystalserver/quest/u15_24/targuna/long_lost_treasure/morla_done',
                                                                                         'minimum': 1,
                                                                                         'maximum': 1}],
                                                                           'take': [],
                                                                           'give': None,
                                                                           'outfit': None,
                                                                           'achievement': None,
                                                                           'reply': ['Thank you for bringing '
                                                                                     'me that map. If I see '
                                                                                     'that correctly the X '
                                                                                     'that marks the spot '
                                                                                     'seems to be here on '
                                                                                     'Targuna.',
                                                                                     'Let me mark it for '
                                                                                     "you, don't wait too "
                                                                                     "long to grab it. It'd "
                                                                                     'be a shame if you were '
                                                                                     'too late ...'],
                                                                           'sources': ['https://raw.githubusercontent.com/zimbadev/crystalserver/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/npc/sterling.lua#L142-L155'],
                                                                           'map_mark': {'position': {'x': 31948,
                                                                                                     'y': 31924,
                                                                                                     'floor': -7},
                                                                                        'kind': 'cross',
                                                                                        'text': 'Treasure'}}]},
 'content/quests/npc-offers/m16-targuna-marker.json': {'schema': 1,
                                                         'offers': [{'npc': 'oteryn:npc.sterling',
                                                                     'keywords': ['mission'],
                                                                     'topic': 0,
                                                                     'any_topic': True,
                                                                     'next_topic': 2,
                                                                     'storage': [{'track': 'oteryn:quest-progress/crystalserver/quest/u15_24/targuna/long_lost_treasure/mission',
                                                                                  'minimum': 3,
                                                                                  'maximum': 3,
                                                                                  'set': 3,
                                                                                  'preserve': True}],
                                                                     'reply': ['Oh, would you look at that, '
                                                                               'you really helped us out '
                                                                               "here. I'm glad to see you "
                                                                               'alive and well!',
                                                                               'Did Morla hand you anything '
                                                                               'for your service?']},
                                                                    {'npc': 'oteryn:npc.sterling',
                                                                     'keywords': ['yes'],
                                                                     'topic': 2,
                                                                     'next_topic': 0,
                                                                     'storage': [{'track': 'oteryn:quest-progress/crystalserver/quest/u15_24/targuna/long_lost_treasure/morla_done',
                                                                                  'minimum': 0,
                                                                                  'maximum': 0,
                                                                                  'set': 0,
                                                                                  'preserve': True}],
                                                                     'reply': ["It seems Morla hasn't handed "
                                                                               'you anything yet. Make sure '
                                                                               'to speak with her first.']}]},
 'content/dialogues/quests/questpc5-targuna-map-greetings.json': {'schema': 1,
                                                                  'npcs': [{'npc': 'oteryn:npc.sterling',
                                                                            'greet': 'Ahoyyy there, you '
                                                                                     "might be just who I'm "
                                                                                     'looking for. I need '
                                                                                     'some help on an '
                                                                                     'adventure ...',
                                                                            'farewell': 'Safe travels, '
                                                                                        'adventurer!',
                                                                            'walkaway': 'Safe travels, '
                                                                                        'adventurer!'}]}}

def source_bytes():
    archive=ROOT/"tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz"
    with tarfile.open(archive) as corpus:
        manifest=json.load(corpus.extractfile("corpus-manifest.json"))
        entry=next(row for row in manifest["files"] if row["source"]=="crystalserver" and row["revision"]==REVISION and row["path"]==SOURCE_PATH)
        raw=corpus.extractfile(entry.get("blob_path",entry.get("cache_path"))).read()
    if hashlib.sha256(raw).hexdigest()!=SOURCE_SHA256: raise ValueError("Sterling source digest mismatch")
    text=raw.decode()
    if not re.search(r"TREASURE_SPOT\s*=\s*Position\(31948,\s*31924,\s*7\)",text): raise ValueError("treasure position changed")
    if 'player:addMapMark(TREASURE_SPOT, MAPMARK_CROSS, "Treasure")' not in text: raise ValueError("native map effect changed")
    if 'player:getStorageValue(MORLA_DONE) ~= 1' not in text: raise ValueError("Morla admission changed")
    if 'player:setStorageValue(TREASURE_MISSION, 4)' not in text: raise ValueError("mission effect changed")
    for row in DOCUMENTS.values():
        for callback in row.get("callbacks",[])+row.get("offers",[]):
            for line in callback["reply"]:
                if line not in text: raise ValueError("dialogue text is not in pinned source")
    return raw

def write(check=False):
    source_bytes()
    for name,document in DOCUMENTS.items():
        path=ROOT/name
        expected=json.dumps(document,ensure_ascii=False,indent=2)+"\n"
        if check:
            if not path.exists() or path.read_text()!=expected: raise ValueError(f"stale Sterling binding: {name}")
        elif not path.exists() or path.read_text()!=expected:
            path.parent.mkdir(parents=True,exist_ok=True);path.write_text(expected)

if __name__=="__main__":
    parser=argparse.ArgumentParser();parser.add_argument("--check",action="store_true")
    write(parser.parse_args().check)
