"""Source-only carried quest disguises; no inventory or quest-state mutation."""
from pathlib import Path
import hashlib
import json
import re
import tarfile

ROOT = Path(__file__).resolve().parents[3]
PIN = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'


def build(root=ROOT):
    paths = [f'data-global/scripts/quests/wrath_of_the_emperor/{name}.lua'
             for name in ('actions_mission01_crate', 'movements_crate')] + ['src/utils/utils_definitions.hpp']
    sources, texts = [], []
    with tarfile.open(root / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as corpus:
        manifest = json.load(corpus.extractfile('corpus-manifest.json'))
        for path in paths:
            row = next(r for r in manifest['files'] if r['source'] == 'crystalserver'
                       and r['revision'] == PIN and r['path'] == path)
            raw = corpus.extractfile(row.get('blob_path', row.get('cache_path'))).read()
            digest = hashlib.sha256(raw).hexdigest()
            if digest != row['sha256']:
                raise ValueError('QUEST_DISGUISE_SOURCE_DRIFT')
            sources.append({'path': path, 'revision': PIN, 'sha256': digest})
            texts.append(raw.decode())
    action, movement, constants = texts
    look = re.search(r'lookTypeEx\s*=\s*(\d+)', action)
    item = re.search(r'wrathEmperorMiss1Crate:id\((\d+)\)', action)
    gate = re.search(r'getStorageValue\(Storage.Quest.U8_6.WrathOfTheEmperor.Questline\)\s*==\s*(\d+)', action)
    exit_pos = re.search(r'-- check\s*\{ x = (\d+), y = (\d+), z = (\d+) \}', movement)
    if not all((look, item, gate, exit_pos)) or look[1] != item[1] or 'condition:setTicks(-1)' not in action:
        raise ValueError('QUEST_DISGUISE_UNSUPPORTED_SOURCE')
    if 'elseif j == 205 then' not in movement or 'player:removeCondition(CONDITION_OUTFIT)' not in movement:
        raise ValueError('QUEST_DISGUISE_RESTORE_SOURCE_DRIFT')
    use_text = re.search(r'player:say\("([^"\n]+)", TALKTYPE_MONSTER_SAY\)', action)
    exit_text = re.search(r'elseif j == 205 then[\s\S]*?player:say\("([^"\n]+)", TALKTYPE_MONSTER_SAY\)', movement)
    effect = re.search(r'CONST_ME_POFF\s*=\s*(\d+)', constants)
    speech = re.search(r'TALKTYPE_MONSTER_SAY\s*=\s*(\d+)', constants)
    if not all((use_text, exit_text, effect, speech)):
        raise ValueError('QUEST_DISGUISE_CUE_SOURCE_DRIFT')
    return [{
        'key': 'quest.wrath.crate',
        'sources': [f"crystal:{row['revision']}:{row['path']}:sha256:{row['sha256']}" for row in sources],
        'item': 'oteryn:item.tibia.i' + item[1],
        'use_text': use_text[1], 'restore_text': exit_text[1],
        'use_effect': int(effect[1]), 'speech_type': int(speech[1]),
        'appearance': int(look[1]),
        'track': 'oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/questline',
        'required': int(gate[1]),
        'restore': {'x': int(exit_pos[1]), 'y': int(exit_pos[2]), 'floor': -int(exit_pos[3])},
    }]

APPEARANCE_PROOFS = 'tools/content-schema/native-gameplay/quest-item-disguise-appearances.json'


def capture_appearances(root, appearances):
    """The same canonical-definition/decoded-object proof used by Item spell looks."""
    import sys
    sys.path.insert(0, str(root / 'tools/content-schema/item-authoring'))
    import engine_items
    raw = appearances.read_bytes()
    sources = json.loads((root / 'tools/content-schema/native-gameplay/event-item-profiles.json').read_bytes())['sources']
    expected = next(row['sha256'] for row in sources
                    if row['path'] == 'canary-47dfd51f/data/items/appearances.dat')
    if hashlib.sha256(raw).hexdigest() != expected:
        raise ValueError('QUEST_DISGUISE_APPEARANCE_SOURCE_DRIFT')
    objects = engine_items.load_appearance_objects(raw)
    import last_creep_native
    ids = ({rule['appearance'] for rule in build(root)}
           | {form['appearance'] for form in last_creep_native.build(root=root)['forms']})
    keys = {f'oteryn:item.tibia.i{item}' for item in ids}
    definitions = {}
    for path in json.loads((root / 'content/items/index.json').read_bytes())['shards']:
        for row in json.loads((root / path).read_bytes())['records']:
            definition = row['definition']
            if definition['identity']['key'] in keys:
                definitions[definition['identity']['key']] = definition
    def digest(value):
        return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True,
                                         separators=(',', ':')).encode()).hexdigest()
    records = []
    for item in sorted(ids):
        key = f'oteryn:item.tibia.i{item}'
        if key not in definitions or item not in objects:
            raise ValueError('QUEST_DISGUISE_APPEARANCE_IDENTITY_MISSING')
        record = {'item': definitions[key]['identity'],
                  'source_revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
                  'appearances_sha256': expected,
                  'canonical_definition_sha256': digest(definitions[key]),
                  'appearance_object_sha256': digest(objects[item])}
        record['qualification_sha256'] = digest(record)
        records.append(record)
    return records


def admit_appearances(root, generated, canonical):
    records = json.loads((root / APPEARANCE_PROOFS).read_bytes())
    path = 'content/presentations/bindings/spell-appearances.json'
    document = json.loads(generated[path])
    existing = {row['item']['key']: row for row in document.get('item_appearances', [])}
    for record in records:
        key = record['item']['key']
        if key in existing and existing[key] != record:
            raise ValueError('QUEST_DISGUISE_APPEARANCE_CONFLICT')
        if key not in existing:
            document.setdefault('item_appearances', []).append(record)
    generated[path] = canonical(document)
    manifest = json.loads(generated['content/spells.manifest.json'])
    manifest['spell_appearances']['sha256'] = hashlib.sha256(generated[path]).hexdigest()
    generated['content/spells.manifest.json'] = canonical(manifest)


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--capture-appearances', type=Path, required=True)
    args = parser.parse_args()
    records = capture_appearances(ROOT, args.capture_appearances)
    (ROOT / APPEARANCE_PROOFS).write_text(json.dumps(records, indent=2) + '\n')
