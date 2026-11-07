"""Emit source-bound Corym dialogue using the existing reviewed literal reader."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
import tarfile

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(Path(__file__).resolve().parent / 'batch3_a'))
from capture import Capture, inventory, topic, snake
from lua_tables import Parser

CAPSULE = 'tools/content-schema/quest-authoring/samples/native-rewards/hidden-threats-npcs.json'
OUTPUT = 'content/dialogues/quest-rules/quest-batch3-c-hidden_threats.json'
REVISION = '04b83b512114bfd888000d6e1433ed8ecaec7c5b'
CORPUS = 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz'
CORPUS_SHA = 'f3fe990f2a72e2779fb360b9c8f98be9d4174b898fa486d4251aa00137c56277'
NPCS = ['corym_ratter', 'corym_servant', 'corym_slave', 'corym_butler', 'corym_footman'] + [f'corym_worker_{n}' for n in range(1, 6)]


def filename(npc):
    return re.sub(r'_([1-5])$', r'_0\1', npc) + '.lua'


def sources(root):
    capsule = json.loads((root / CAPSULE).read_bytes())
    archive = root / CORPUS
    if capsule['revision'] != REVISION or hashlib.sha256(archive.read_bytes()).hexdigest() != CORPUS_SHA:
        raise ValueError('HIDDEN_THREATS_SOURCE_PIN')
    with tarfile.open(archive) as donor:
        manifest = json.load(donor.extractfile('corpus-manifest.json'))['files']
    wanted = {'data-otservbr-global/npc/' + filename(npc): npc for npc in NPCS}
    records = {row['cache_path']: row for row in manifest
               if row['revision'] == REVISION and row['path'] in wanted}
    if len(records) != 10 or len(capsule['sources']) != 10:
        raise ValueError('HIDDEN_THREATS_SOURCE_COUNT')
    fresh = {row['path']: row for row in capsule['sources']}
    raw = {}
    with tarfile.open(archive, mode='r|gz') as donor:
        for member in donor:
            if member.name not in records:
                continue
            record = records[member.name]
            data = donor.extractfile(member).read()
            if (hashlib.sha256(data).hexdigest() != record['sha256']
                    or fresh[record['path']]['sha256'] != record['sha256']
                    or fresh[record['path']]['revision'] != REVISION):
                raise ValueError('HIDDEN_THREATS_SOURCE_CUSTODY')
            raw[wanted[record['path']]] = data.decode()
    return raw


class CorymCapture(Capture):
    def __init__(self, raw):
        self.raw = raw
        self.quest = 'hidden_threats'
        self.key = 'oteryn:quest.tibia_tales'
        self.prefix = 'oteryn:quest-progress/quest/u11_50/hidden_threats'
        self.rules, self.tracks, self.files = [], {}, {}

    def read(self, npc):
        text = self.raw[npc]
        self.files['data-otservbr-global/npc/' + filename(npc)] = hashlib.sha256(text.encode()).hexdigest()
        return text

    def ref(self, npc, line):
        return f'canary:{REVISION}:data-otservbr-global/npc/{filename(npc)}:{line}'

    def track(self, name, maximum=1, line=None, npc=None):
        key = ('oteryn:quest-progress/quest/u8_1/tibia_tales/default_start'
               if name == 'DefaultStart' else self.prefix + '/' + snake(name))
        if key not in self.tracks:
            if line is None or npc is None:
                raise ValueError('Register the exact source track before using it: ' + name)
            initial = 0 if name in ['DefaultStart', 'QuestLine'] else -1
            self.tracks[key] = dict(key=key, quest=self.key, initial=initial, min=initial,
                                    max=maximum, sources=[self.ref(npc, line)])
        return key

    def greeting(self, npc, ordinal):
        text = self.read(npc)
        matches = list(re.finditer(r'npcHandler:setMessage\(MESSAGE_GREET,\s*', text))
        match = matches[ordinal]
        value = Parser(text[match.end():]).value()
        reply = [value] if isinstance(value, str) else [field['value'] for field in value['fields']]
        return reply, text[:match.start()].count('\n') + 1

    def greet(self, npc, label, ordinal, guards=(), effects=()):
        reply, line = self.greeting(npc, ordinal)
        return self.rule(npc, label, line, guards=guards, effects=effects,
                         replies=reply, trigger='greet')


def document(root=ROOT):
    c = CorymCapture(sources(root))
    for npc in NPCS:
        c.read(npc)
    c.track('QuestLine', 5, 84, 'corym_ratter')
    c.track('DefaultStart', 1, 83, 'corym_ratter')
    c.track('RatterDoor', 1, 85, 'corym_ratter')
    c.track('ServantDoor', 1, 101, 'corym_servant')
    c.track('CorymRescueMission', 2147483647, 102, 'corym_servant')
    rescuers = [('corym_worker_1', 1, None, 3030), ('corym_slave', 2, 'pay back', None),
                ('corym_worker_2', 3, 'repression', None), ('corym_worker_3', 4, None, 3032),
                ('corym_butler', 5, 'repression', None), ('corym_worker_4', 6, None, None),
                ('corym_worker_5', 7, None, 3029), ('corym_footman', 8, 'hunger', None)]
    for npc, flag, _, _ in rescuers:
        c.track(f'CorymRescued{flag:02}', 1, 59, npc)
    g, e, r = c.guard, c.effect, c.rule
    npc = 'corym_ratter'
    c.greet(npc, 'greet-new', 0, [g('QuestLine', 'lt', 1)])
    c.greet(npc, 'greet-started', 1)
    r(npc, 'help', 78, ['help'], effects=[topic(1)], say=79)
    r(npc, 'accept', 81, ['yes'], [g('QuestLine', 'lt', 1)],
      [e('DefaultStart', 1), e('QuestLine', 1), e('RatterDoor', 1), topic(2)], say=86, topics=[1])
    r(npc, 'accept-repeat', 81, ['yes'], [g('QuestLine', 'ge', 1)],
      [e('DefaultStart', 1), e('RatterDoor', 1), topic(2)], say=86, topics=[1])
    npc = 'corym_servant'
    c.greet(npc, 'greet-start', 0, [g('QuestLine', 'eq', 1)])
    c.greet(npc, 'all-rescued', 1, [g('CorymRescueMission', 'eq', 8), g('QuestLine', 'eq', 3)], [e('QuestLine', 4)])
    c.greet(npc, 'greet-rare-earth', 2, [g('QuestLine', 'eq', 4)])
    c.greet(npc, 'greet-rescues', 3, [g('QuestLine', 'eq', 3)])
    c.greet(npc, 'greet-other', 4)
    r(npc, 'resources', 91, ['decreasing resources'], effects=[topic(1)], say=92)
    r(npc, 'defy', 97, ['defy'], [g('QuestLine', 'eq', 1)],
      [e('QuestLine', 2), e('ServantDoor', 1), e('CorymRescueMission', 0), topic(2)], say=104, topics=[1])
    r(npc, 'defy-repeat', 97, ['defy'], effects=[topic(2)], say=104, topics=[1])
    r(npc, 'rare-earth', 110, ['yes'], [g('QuestLine', 'eq', 4), inventory('item', 27301, 20) | {'op': 'ge'}],
      [inventory('take_item', 27301, 20), inventory('give_item', 3040, 2), e('QuestLine', 5), e('CorymRescueMission', 9)], say=112)
    r(npc, 'rare-earth-missing', 110, ['yes'], [g('QuestLine', 'eq', 4), inventory('item', 27301, 20) | {'op': 'lt'}],
      [{'kind': 'end_conversation', 'delay_ms': 0}, topic(0)], say=120)
    for npc, flag, keyword, gift in rescuers:
        name = f'CorymRescued{flag:02}'
        guards = [g(name, 'lt', 1), g('CorymRescueMission', 'between', 0, 7)]
        effects = [e('CorymRescueMission', 1, 'add'), e(name, 1)]
        if gift is not None:
            effects.append(inventory('give_item', gift))
        if keyword is None:
            c.greet(npc, 'rescue', 0, guards, effects)
            c.greet(npc, 'greet-repeat', 1)
        else:
            c.greet(npc, 'greet-new', 0, [g(name, 'lt', 1)])
            c.greet(npc, 'greet-repeat', 1)
            r(npc, 'rescue', 76, [keyword], guards, effects, say=77)
            r(npc, 'rescue-repeat', 76, [keyword], say=77)
    return dict(schema=1, program_key='oteryn:quest-npc-program.hidden_threats', quest=c.key, tracks=list(c.tracks.values()), rules=c.rules,
                source_files=[dict(path=path, sha256=sha, revision=REVISION) for path, sha in sorted(c.files.items())])


GREETINGS = 'content/dialogues/quests-2/corym-npc-greetings.json'


def greeting_document(root=ROOT):
    c = CorymCapture(sources(root))
    handler = dict(repository='opentibiabr/canary', revision=REVISION,
                   path='data/npclib/npc_system/npc_handler.lua', line=64,
                   sha256='632be3d5b445ec233d80ed2e60c3cd6bc43fec0691330d35aea10f5a21551f55')
    capsule = json.loads((root / CAPSULE).read_bytes())
    if any(capsule['handler_source'][key] != handler[key] for key in ['path', 'revision', 'sha256']):
        raise ValueError('HIDDEN_THREATS_HANDLER_PIN')
    rows = []
    for npc in NPCS:
        text = c.read(npc)
        reply, line = c.greeting(npc, 0)
        farewell = re.search(r'npcHandler:setMessage\(MESSAGE_FAREWELL,\s*', text)
        if farewell is None:
            raise ValueError('HIDDEN_THREATS_FAREWELL')
        message = lambda value, anchor: dict(text=value, source=dict(repository='opentibiabr/canary',
            revision=REVISION, path='data-otservbr-global/npc/' + filename(npc), line=anchor,
            sha256=hashlib.sha256(text.encode()).hexdigest()))
        rows.append(dict(npc='oteryn:npc.' + npc, greet=message(' '.join(reply), line),
            farewell=message(Parser(text[farewell.end():]).value(), text[:farewell.start()].count('\n') + 1),
            walkaway=dict(text='Good bye.', source=handler | dict(line=97)), variants={}))
    return dict(schema=1, source_revision=REVISION, npcs=rows, placeholder_source=handler)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for name, value in [(OUTPUT, document()), (GREETINGS, greeting_document())]:
        text = json.dumps(value, ensure_ascii=False, indent=2) + '\n'
        path = ROOT / name
        if args.check:
            if not path.exists() or path.read_text() != text:
                raise ValueError('HIDDEN_THREATS_NPC_GENERATION_DRIFT: ' + name)
        else:
            path.write_text(text)
    print('Hidden Threats10 NPC sources, programme and greeting shard verified')
