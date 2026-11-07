"""SRV7 literal seasonal greetings, source-pinned without executing Lua."""
import argparse
import hashlib
import json
from pathlib import Path
import re

from quests_2_greetings import message, source, PIN

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = 'crates/gameplay/npc/src/quest_greetings/seasonal.json'
HASHES = {
'data-global/npc/messenger_of_santa.lua': 'cb91c928e0d5dfd8dbf4cd327667a2456be8bcc57461e226cde3af36260939b1',
'data-global/npc/balduin_the_baker.lua': '3d372bb0e99b02bf66bb5ebe4a0c6319730f566e6935467322c40d552dedbe76',
'data-global/npc/benny_the_baker.lua': '43781aaf77c9b076cae8841412335bb2d62c2f4f3b86a0ef78fcc8759f7e59a0',
'data-global/npc/biff_the_baker.lua': 'a590a49ebec09956779a7ca9776c449dc4e6d849e9821c5ddcd6df9da0b36a9d',
'data-global/npc/billy_the_baker.lua': 'f9e98b1042edd5011f95c1ca8779cdad5c30326f5047ca57e2dd764c4294ea4f',
'data-global/npc/brian_the_baker.lua': 'a49795eeb75ab6b3b9c7730162156546035e8e5872fb947116714ce8924c0b3a',
'data-global/npc/brutus_the_baker.lua': 'f81918f6205263762f0604092deb5aff1f78f69c500566cdd33efdd7e1529ae1',
'data-global/npc/santa_claus.lua': '16f900a55116b6298044c4105b2f1550f9b91c797aefc56f201d60f61596c06c',
'data/npclib/npc_system/npc_handler.lua': '8479e2aa7904915ef23bcbc50eef3ac9c9a8b2588cc16dc8713f33fa0cfec35a',
}


def build(crystal):
    raw = {path: (crystal/path).read_bytes() for path in HASHES}
    if any(hashlib.sha256(raw[path]).hexdigest() != expected for path,expected in HASHES.items()):
        raise ValueError('SEASONAL_GREETING_SOURCE_DIGEST')
    default = 'data/npclib/npc_system/npc_handler.lua'
    entries = []
    for path in HASHES:
        if path == default: continue
        text = raw[path].decode()
        entry = {'npc': 'oteryn:npc.'+Path(path).stem, 'variants': {}}
        for field, kind in [('greet','MESSAGE_GREET'),('farewell','MESSAGE_FAREWELL'),('walkaway','MESSAGE_WALKAWAY')]:
            selected = path if re.search(r'setMessage\(\s*'+kind+r'\s*,',text) else default
            entry[field] = message(raw[selected],kind,selected)
        entries.append(entry)
    return {'schema': 1, 'source_revision': PIN, 'npcs': entries, 'placeholder_source': source(default,383,raw[default])}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', required=True, type=Path)
    parser.add_argument('--check',action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(build(args.crystal),indent=2,ensure_ascii=False)+'\n').encode()
    path = ROOT/OUTPUT
    if args.check:
        if path.read_bytes()!=raw: raise ValueError('SEASONAL_GREETING_DRIFT')
    else:
        path.parent.mkdir(parents=True,exist_ok=True)
        path.write_bytes(raw)
