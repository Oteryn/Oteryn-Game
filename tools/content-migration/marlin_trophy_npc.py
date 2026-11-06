"""Lower Pemaret's fixed once-only exchange from the retained donor corpus."""
import argparse
import hashlib
import json
from pathlib import Path
import tarfile

ROOT = Path(__file__).resolve().parents[2]
REVISION = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
SOURCE = 'data-global/npc/pemaret.lua'
SHA = '7860edb917cf36e43f2f675b9a2f9b8bbbc0821051ef20e4a9443cf04649489c'
QUEST = 'oteryn:quest.marlin_trophy_quest'
TRACK = 'oteryn:quest-progress/quest/u7_8/marlin_trophy'
TARGET = 'content/dialogues/quest-rules/quest-batch3-a-marlin_trophy.json'


def document(root=ROOT):
    archive = root / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz'
    if hashlib.sha256(archive.read_bytes()).hexdigest() != 'f3fe990f2a72e2779fb360b9c8f98be9d4174b898fa486d4251aa00137c56277':
        raise ValueError('MARLIN_CORPUS_PIN')
    with tarfile.open(archive) as corpus:
        manifest = json.load(corpus.extractfile('corpus-manifest.json'))
        files = [row for row in manifest['files'] if row['path'] == SOURCE
                 and row['revision'] == REVISION and row['sha256'] == SHA]
        if len(files) != 1:
            raise ValueError('MARLIN_SOURCE_CUSTODY')
        raw = corpus.extractfile(files[0]['cache_path']).read()
    if hashlib.sha256(raw).hexdigest() != SHA:
        raise ValueError('MARLIN_SOURCE_PIN')
    lines = raw.decode().splitlines()
    ref = lambda line: f'crystal:{REVISION}:{SOURCE}:{line}'
    storage = {'kind': 'storage', 'track': TRACK, 'op': 'lt', 'value': 1}
    fish = {'kind': 'item', 'item': 'oteryn:item.tibia.i901', 'count': 1, 'op': 'ge'}
    rules = []

    def rule(name, line, words, guards=(), effects=(), reply=(), topics=None, exclude=()):
        row = {'key': f'oteryn:quest-npc/marlin_trophy_quest/pemaret/{name}',
               'quest': QUEST, 'npc': 'oteryn:npc.pemaret', 'trigger': 'say',
               'keywords': words, 'guards': list(guards), 'effects': list(effects),
               'reply': list(reply), 'continue_matching': True, 'sources': [ref(line)]}
        if topics is not None:
            row['topics'] = topics
        if exclude:
            row['exclude_keywords'] = list(exclude)
        rules.append(row)

    def say(line):
        text = lines[line - 1].strip()
        if not text.startswith('npcHandler:say("') or not text.endswith('", npc, creature)'):
            raise ValueError('MARLIN_SOURCE_REPLY')
        return text[len('npcHandler:say("'):-len('", npc, creature)')]

    rule('offer', 61, ['marlin'], [storage, fish], [{'kind': 'set_topic', 'value': 1}], [say(64)])
    rule('offer-missing', 61, ['marlin'], [storage, dict(fish, op='lt')])
    rule('deliver', 68, ['yes'], [storage, fish], [
        {'kind': 'take_item', 'item': 'oteryn:item.tibia.i901', 'count': 1},
        {'kind': 'give_item', 'item': 'oteryn:item.tibia.i902', 'count': 1},
        {'kind': 'set', 'track': TRACK, 'value': 1}, {'kind': 'set_topic', 'value': 0}],
        [say(70)], [1], ['marlin'])
    rule('missing', 68, ['yes'], [storage, dict(fish, op='lt')],
         [{'kind': 'set_topic', 'value': 0}], [say(74)], [1], ['marlin'])
    rule('decline', 78, ['no'], effects=[{'kind': 'set_topic', 'value': 0}],
         reply=[say(79)], topics=[1])
    rules[-1]['continue_matching'] = False
    return {'schema': 1, 'quest': QUEST, 'tracks': [
        {'key': TRACK, 'quest': QUEST, 'initial': -1, 'min': -1, 'max': 1, 'sources': [ref(62)]}],
        'rules': rules, 'source_files': [{'path': SOURCE, 'sha256': SHA, 'revision': REVISION}]}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    payload = json.dumps(document(), ensure_ascii=False, indent=2) + '\n'
    path = ROOT / TARGET
    if args.check:
        if path.read_text() != payload:
            raise SystemExit('MARLIN_NPC_GENERATION_DRIFT')
    else:
        path.write_text(payload)
