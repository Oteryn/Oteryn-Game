"""NPCFIX source compositions over the shared fenced reward owner."""
import hashlib
import re

NPCS = ('al_dee', 'a_weakened_forest_fury', 'a_wandering_soul')
ITEMS = {6528,3560,12804,3456, 8225, 8453, 8746, 23812, 43854, 43855, 7528, 7529, 7530, 7531, 7532}


def append(callbacks, offers, inputs, pin):
    def text(npc, phrase):
        raw = inputs[f'data-global/npc/{npc}.lua'].decode()
        for line in raw.splitlines():
            if phrase in line:
                matches = re.findall(r'"([^"\n]*)"', line)
                found = [m for m in matches if phrase in m]
                if len(found) == 1:
                    return found[0]
        raise ValueError('NPCFIX source literal missing: ' + npc + ':' + phrase)

    def source(npc, phrase):
        path = f'data-global/npc/{npc}.lua'
        raw = inputs[path]
        line = next(i for i, row in enumerate(raw.decode().splitlines(), 1) if phrase in row)
        return f'crystal:{pin}:{path}:{line}:sha256:{hashlib.sha256(raw).hexdigest()}'

    def state(track, minimum, maximum, value, preserve=False):
        return dict(track='oteryn:quest-progress/' + track, minimum=minimum,
                    maximum=maximum, set=value, **({'preserve': True} if preserve else {}))

    def item(number, count=1):
        return dict(item=f'oteryn:item.tibia.i{number}', count=count)

    def offer(npc, words, topic, next_topic, storage, reply):
        offers.append(dict(npc='oteryn:npc.' + npc, keywords=words, topic=topic,
                           next_topic=next_topic, storage=storage, reply=reply))

    def callback(name, quest, npc, words, topic, storage, take, give, phrase, repeat=False,
                 achievement=None):
        row = dict(key='oteryn:quest-reward/npcfix/' + name, quest='oteryn:quest.' + quest,
                   npc='oteryn:npc.' + npc, keywords=words, topic=topic, next_topic=0,
                   storage=storage, take=take, give={**give, 'contents': []}, outfit=None,
                   achievement=achievement, reply=[text(npc, phrase)], sources=[source(npc, phrase)])
        if repeat:
            row['repeatable'] = True
        if achievement:
            row['achievement_once'] = True
        callbacks.append(row)

    npc = 'al_dee'
    receipt = [state('npcfix/pick_quest/exchange', -1, -1, -1, True)]
    offer(npc, ['pick', 'small axe'], 0, 1, receipt, [text(npc, 'Picks are hard to come by.')])
    offer(npc, ['no'], 1, 0, receipt, [text(npc, "Well, then don't.")])
    callback('al-dee-pick', 'pick_quest', npc, ['yes'], 1, receipt,
             [item(3462)], item(3456), 'Splendid! Here, take your pick.', True)

    npc = 'a_weakened_forest_fury'
    cage = 'quest/u11_02/forgotten_knowledge/bird_cage'
    waiting = [state(cage, 0, 0, 1)]
    raw = inputs[f'data-global/npc/{npc}.lua'].decode()
    blocks = re.findall(r'npcHandler:say\(\{(.*?)\}, npc, creature\)', raw, re.S)
    introduction = next(b for b in blocks if 'Will you be our guardian?' in b)
    offer(npc, ['distress', 'mission'], 0, 1, waiting,
          re.findall(r'"([^"\n]*)"', introduction))
    success = next(b for b in blocks if 'Indeed, you will.' in b)
    callback('forest-fury-cage', 'forgotten_knowledge_quest', npc, ['yes'], 1,
             waiting, [], item(23812), 'Indeed, you will.')
    callbacks[-1]['reply'] = re.findall(r'"([^"\n]*)"', success)
    held = [state(cage, 1, 1, 1, True)]
    offer(npc, ['cages'], 0, 2, held,
          [text(npc, 'Crafted generations ago'), text(npc, 'Are you in need of another one?')])
    offer(npc, ['yes'], 2, 0, held, [text(npc, 'I already handed a cage to you.')])
    for keyword, phrase in [('seeds', 'Seeds to give life'), ('intruders', 'The intruders appeared'),
                            ('water', 'The purest water flows'), ('birds', 'Take care, guardian.'),
                            ('phials', 'Phials for the purest water')]:
        offer(npc, [keyword], 1, 0, waiting, [text(npc, phrase)])

    npc = 'a_wandering_soul'
    mission = [state('quest/u8_4/blood_brothers/mission05', 2, 4, 2, True)]
    callback('wandering-crystal', 'blood_brothers_quest', npc, ['blood crystal'], 0,
             mission, [item(8453)], item(8225), "What the...? I don't know anything",
             True, 'oteryn:achievement/ghostwhisperer')
    callbacks[-1]['give']['decay'] = dict(duration_millis=604800000,
        target=dict(family='Item', key='oteryn:item.tibia.i8453', revision='definition-r1'))
    offer(npc, ['blood crystal'], 0, 0,
          [state('quest/u8_4/blood_brothers/mission05', 0, 1, 0, True)],
          [text(npc, 'A crystal? My mind is too clouded')])

    greetings = {}
    for npc in NPCS:
        default = 'data/npclib/npc_system/npc_handler.lua'
        messages = {}
        citations = []
        raw = inputs[f'data-global/npc/{npc}.lua'].decode()
        for field, constant in [('greet', 'GREET'), ('farewell', 'FAREWELL'), ('walkaway', 'WALKAWAY')]:
            lines = re.findall(r'npcHandler:setMessage\(MESSAGE_' + constant + r',\s*"([^"\n]*)"', raw)
            if npc == 'al_dee' and field == 'greet':
                block = re.search(r'npcHandler:setMessage\(MESSAGE_GREET,\s*\{(.*?)\}\)', raw, re.S)
                lines = [' '.join(re.findall(r'"([^"\n]*)"', block.group(1)))]
            elif npc == 'a_wandering_soul' and field == 'greet':
                lines = [text(npc, 'Is that you...? No...')]
            if not lines:
                lines = re.findall(r'\[MESSAGE_' + constant + r'\]\s*=\s*"([^"\n]*)"', inputs[default].decode())
                citations.append(f'crystal:{pin}:{default}:sha256:{hashlib.sha256(inputs[default]).hexdigest()}')
            else:
                citations.append(source(npc, 'MESSAGE_' + constant))
            if len(lines) != 1:
                raise ValueError('NPCFIX ambiguous message: ' + npc + ':' + field)
            messages[field] = lines[0]
        greetings['oteryn:npc.' + npc] = dict(npc='oteryn:npc.' + npc, **messages, sources=citations)
    greetings['oteryn:npc.a_wandering_soul']['conditional'] = [
        dict(track='oteryn:quest-progress/quest/u8_4/blood_brothers/mission05', minimum=2,
             item='oteryn:item.tibia.i8453', text=text('a_wandering_soul', 'I saw you...')),
        dict(track='oteryn:quest-progress/quest/u8_4/blood_brothers/mission05', minimum=2,
             item='oteryn:item.tibia.i8225', text=text('a_wandering_soul', 'The pain is slightly less...')),
    ]
    fading = json.loads(INPUT.read_text())
    workshop = json.loads(INPUT.with_name('npcfix_workshop.json').read_text())
    existing = {row['key']: row for row in callbacks}
    for row in workshop['callbacks']:
        if row['key'] in existing:
            if existing[row['key']] != row:
                raise ValueError('NPCFIX_REUSED_CALLBACK_CONFLICT:' + row['key'])
        else:
            callbacks.append(row)
            existing[row['key']] = row
    for key, topic in workshop.get('refusal_topics', {}).items():
        if key not in existing:
            raise ValueError('NPCFIX_UNKNOWN_REFUSAL_CALLBACK:' + key)
        existing[key]['refusal_topic'] = topic
    offers.extend(workshop['offers'])
    greetings.update({row['npc']: row for row in fading['npcs']})
    # Explicit conservative defaults for uncustomized farewells, with source provenance.
    for npc, greet in [('altar', 'Kneel before the all-devouring power of blooded decay.'),
                       ('a_sweaty_cyclops', 'Hum Humm! Welcume lil\' |PLAYERNAME|.')]:
        citation = list(next(c['sources'] for c in workshop['callbacks'] if c['npc'] == 'oteryn:npc.' + npc))
        if npc == 'a_sweaty_cyclops':
            default = 'data/npclib/npc_system/npc_handler.lua'
            citation.append(f'crystal:{pin}:{default}:90:sha256:{hashlib.sha256(inputs[default]).hexdigest()}')
        greetings['oteryn:npc.' + npc] = dict(npc='oteryn:npc.' + npc, greet=greet,
            farewell=('Good bye lil\' one.' if npc == 'a_sweaty_cyclops' else 'Bye, |PLAYERNAME|.'),
            walkaway=('Good bye.' if npc == 'a_sweaty_cyclops' else 'Bye.'), sources=citation)
    return greetings

import argparse
import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[3]
INPUT = Path(__file__).with_suffix('.json')


def encoded(value):
    return json.dumps(value, ensure_ascii=False, indent=2) + '\n'


def outputs(root=ROOT, generated=None):
    source = json.loads(INPUT.read_text())
    if source['schema'] != 1:
        raise ValueError('NPCFIX_REWARD_SCHEMA')
    owners = set(source['owns'])
    def read(relative, empty):
        if generated is not None and relative in generated:
            return json.loads(generated[relative])
        path = root / relative
        return json.loads(path.read_text()) if path.is_file() else empty

    callbacks = root / 'content/quests/shared-reward-callbacks.json'
    document = read('content/quests/shared-reward-callbacks.json', {'schema': 1, 'callbacks': []})
    document['callbacks'] = [row for row in document['callbacks']
                             if row['npc'] not in owners] + source['callbacks']
    declared = {row['key'] for row in source['tracks']}
    document['tracks'] = [row for row in document.get('tracks', [])
                          if row['key'] not in declared] + source['tracks']
    schema = json.loads((root / 'tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json').read_text())
    Draft202012Validator(schema).validate(document)
    for callback in document['callbacks']:
        if callback.get('repeatable') and (not callback.get('give') or callback.get('outfit')
                or not (callback['take'] or callback.get('fee') or callback.get('cooldown'))):
            raise ValueError('NPCFIX_UNBOUNDED_REPEAT')
    result = {callbacks: encoded(document)}
    for relative, field in [('content/quests/shared-reward-offers.json', 'offers'),
                            ('content/dialogues/quests/shared-reward-greetings.json', 'npcs')]:
        path = root / relative
        current = read(relative, {'schema': 1, field: []})
        current[field] = [row for row in current[field] if row['npc'] not in owners] + source[field]
        result[path] = encoded(current)
    return result


def overlay_generated(generated, root=ROOT):
    """Compose owned rows after the shared producer, preserving its current generation."""
    result = dict(generated)
    for path, value in outputs(root, generated).items():
        result[str(path.relative_to(root))] = value.encode()
    return result


def compose(generated):
    """Layer only NPCFIX-owned facts onto the authoritative shared generation."""
    import sys
    import qengine_rewards
    sys.path.insert(0, str(ROOT / 'tools/content-migration'))
    import npcfix_item_profiles
    import register_spell_families
    inputs = qengine_rewards.donor_inputs()
    import tarfile
    with tarfile.open(ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as corpus:
        manifest = json.load(corpus.extractfile('corpus-manifest.json'))
        for npc in NPCS:
            path = f'data-global/npc/{npc}.lua'
            record = next(row for row in manifest['files'] if row['source'] == 'crystalserver'
                          and row['revision'] == qengine_rewards.PIN and row['path'] == path)
            raw = corpus.extractfile(record.get('blob_path', record.get('cache_path'))).read()
            if hashlib.sha256(raw).hexdigest() != record['sha256']:
                raise ValueError('NPCFIX_SOURCE_PIN:' + path)
            inputs[path] = raw
    callbacks, offers = [], []
    greetings = append(callbacks, offers, inputs, qengine_rewards.PIN)
    for relative, field, rows in (
            ('content/quests/shared-reward-callbacks.json', 'callbacks', callbacks),
            ('content/quests/shared-reward-offers.json', 'offers', offers),
            ('content/dialogues/quests/shared-reward-greetings.json', 'npcs', list(greetings.values()))):
        document = json.loads(generated[relative])
        if field == 'callbacks':
            keys = {row['key'] for row in rows}
            document[field] = [row for row in document[field] if row['key'] not in keys] + rows
        elif field == 'npcs':
            keys = {row['npc'] for row in rows}
            document[field] = [row for row in document[field] if row['npc'] not in keys] + rows
        else:
            document[field].extend(rows)
        generated[relative] = qengine_rewards.encoded(document)
    generated = overlay_generated(generated)
    qengine_rewards.validate_callbacks(json.loads(generated['content/quests/shared-reward-callbacks.json']))
    npcfix_item_profiles.admit(
        ROOT, generated, npcfix_item_profiles.PROFILE, 'content/spells.manifest.json',
        register_spell_families.provider_order, register_spell_families.spell_import.canonical_bytes)
    return generated


def build():
    import qengine_rewards
    return qengine_rewards.build()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for relative, raw in build().items():
        path = ROOT / relative
        if path.is_file() and path.read_bytes() == raw:
            continue
        if args.check:
            raise SystemExit('NPCFIX_REWARD_DRIFT: ' + relative)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)


if __name__ == '__main__':
    main()
