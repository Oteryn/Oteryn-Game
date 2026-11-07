"""SRV7 literal horse-station greetings, source-pinned without executing Lua."""
import argparse
import hashlib
import json
from pathlib import Path
import re

from quests_2_greetings import message, source, PIN

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = 'crates/gameplay/npc/src/quest_greetings/horse_rental.json'
HASHES = {'data/npclib/npc_system/npc_handler.lua': '8479e2aa7904915ef23bcbc50eef3ac9c9a8b2588cc16dc8713f33fa0cfec35a', 'data-global/npc/appaloosa.lua': '2972dcf9913edd16f693bf265a97acbdec9acde1de8487141ee776ed98efce19', 'data-global/npc/palomino.lua': '595387918194d62554ba02e66c381e945fc065a0c0b3b2658ce773fe52f293da'}


def build(crystal):
    raw = {path: (crystal/path).read_bytes() for path in HASHES}
    if any(hashlib.sha256(raw[path]).hexdigest() != expected for path,expected in HASHES.items()):
        raise ValueError('HORSE_RENTAL_GREETING_SOURCE_DIGEST')
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
        if path.read_bytes()!=raw: raise ValueError('HORSE_RENTAL_GREETING_DRIFT')
    else:
        path.parent.mkdir(parents=True,exist_ok=True)
        path.write_bytes(raw)
