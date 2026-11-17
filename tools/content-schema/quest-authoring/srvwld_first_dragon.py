"""Reconstruct Tamoril's seasonal dialogue from the already pinned retained source."""
import argparse
import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(HERE / 'batch3_a'))
from capture import Capture
from lower_dialogue import Lowering

QUEST = 'oteryn:quest.the_first_dragon_quest'
SHA = 'd5d55a74c2bb9ade413c23442924b46ee6e5b05c9daf73ed8259e67f04576969'
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
TARGET = HERE / 'batch3_c/npc/the_first_dragon.json'


class RetainedCapture(Capture):
    def __init__(self, text):
        self.text = text
        self.quest = 'the_first_dragon'
        self.key = QUEST
        self.prefix = 'oteryn:quest-progress/quest/u11_02/the_first_dragon'
        self.rules, self.tracks, self.files = [], {}, {'data-global/npc/tamoril.lua': SHA}

    def read(self, npc):
        if npc != 'tamoril':
            raise ValueError('unreviewed NPC')
        return self.text

    def track(self, name, maximum=2147483647, line=None, npc=None):
        needle = 'Storage.Quest.U11_02.TheFirstDragon.' + name
        line = next(i for i, text in enumerate(self.text.splitlines(), 1) if needle in text)
        return super().track(name, maximum, line, 'tamoril')


def document(root=ROOT):
    source = json.loads((root / 'content/dialogues/quest-sources/the_first_dragon.json').read_text())
    if source['source_repository_revision'] != 'zimbadev/crystalserver@' + PIN:
        raise ValueError('unpinned source revision')
    entry = next(e for e in source['entries'] if e['npc_source_name'] == 'tamoril')
    text = entry['full_source_lua']
    if hashlib.sha256(text.encode()).hexdigest() != SHA:
        raise ValueError('Tamoril source changed')
    c = RetainedCapture(text)
    window = {'kind': 'calendar_window', 'start_month': 1, 'start_day': 14,
              'end_month': 2, 'end_day': 12, 'within': True}
    outside = dict(window, within=False)
    # Crystal data-global/npc/tamoril.lua:60-63 returns before every dialogue branch.
    c.rule('tamoril', 'outside-event', 60, guards=[outside], any_message=True,
           replies=c.strings('tamoril', 61, 61))
    Lowering(c, 'tamoril', 'Storage.Quest.U11_02.TheFirstDragon').lower(65, 108, 'conversation')
    for rule in c.rules[1:]:
        rule['guards'].insert(0, dict(window))
    return {'schema': 1, 'program_key': 'oteryn:quest-npc-program.first-dragon-calendar', 'quest': QUEST, 'tracks': list(c.tracks.values()), 'rules': c.rules,
            'source_files': [{'path': 'data-global/npc/tamoril.lua', 'revision': PIN, 'sha256': SHA}]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    encoded = json.dumps(document(), ensure_ascii=False, indent=2) + '\n'
    if args.check:
        if not TARGET.is_file() or TARGET.read_text() != encoded:
            raise SystemExit('stale Tamoril seasonal source programme')
    else:
        TARGET.write_text(encoded)
    print('Tamoril seasonal source programme ' + ('verified' if args.check else 'generated'))


if __name__ == '__main__':
    main()
