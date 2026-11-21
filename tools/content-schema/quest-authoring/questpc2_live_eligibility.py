#!/usr/bin/env python3
"""Materialize M08 eligibility guards from captured source and wiki requirements."""
import argparse
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
PIN = 'crystal:201189f871d23597b82caf88fd611454a2ddefb8:'


def prerequisites(document):
    quest = document['quest']
    guards = [{'kind': 'premium', 'value': True}]
    if quest == 'oteryn:quest.hot_cuisine_quest':
        guards.insert(0, {'kind': 'level', 'op': 'ge', 'value': 35})
    elif quest != 'oteryn:quest.the_hidden_city_of_beregar_quest':
        return document
    for rule in document['rules']:
        for guard in guards:
            if guard not in rule['guards']:
                rule['guards'].append(guard.copy())
    return document


def elemental():
    capture = json.loads((ROOT / 'content/dialogues/quest-sources/elemental_spheres.json').read_text())
    entry = next(e for e in capture['entries'] if e['npc_source_name'] == 'arkulius')
    source = entry['full_source_lua']
    digest = hashlib.sha256(source.encode()).hexdigest()
    if digest != entry['source_blob_sha256']:
        raise ValueError('Arkulius source checksum')
    lines = source.splitlines()
    quest = 'oteryn:quest.elemental_spheres_quest'
    track = 'oteryn:quest-progress/elemental_spheres/quest_line'
    ref = lambda n: f'{PIN}data-global/npc/arkulius.lua:{n}'
    storage = lambda op, value: {'kind': 'storage', 'track': track, 'op': op, 'value': value}
    effects = lambda value: [{'kind': 'set', 'track': track, 'value': value}, {'kind': 'set_topic', 'value': 0}]
    rules = []

    def say(first, last):
        # These donor rows contain only literal say strings; JSON decoding preserves Lua's escapes here.
        return [json.loads(line.strip().removesuffix(',')) for line in lines[first-1:last]
                if line.strip().startswith('"')]

    def rule(key, line, guards, actions, reply, topics=None):
        row = {'key': f'oteryn:quest-npc/elemental_spheres/arkulius/{key}',
               'quest': quest, 'npc': 'oteryn:npc.arkulius', 'trigger': 'say',
               'keywords': ['mission', 'quest'], 'guards': guards + [{'kind':'premium','value':True}],
               'effects': actions, 'reply': reply, 'sources': [ref(line)]}
        if topics is not None:
            row['keywords'] = ['yes']; row['topics'] = topics
        rules.append(row)

    for vocation, first, soil, reward in [('sorcerer',110,946,8039), ('druid',117,947,8041),
                                          ('paladin',124,942,8025), ('knight',131,948,8055)]:
        vocation_guard = {'kind':'vocation','value':f'oteryn:vocation.{vocation}'}
        level = {'kind':'level','op':'ge','value':80}
        rule(vocation+'-offer', first, [storage('lt',1), level, vocation_guard],
             [{'kind':'set_topic','value':1}], say(first,first+3))
        rule(vocation+'-accept',169,[storage('lt',1),level,vocation_guard],effects(1),
             ["Good, don't waste time! Come back here when you have the elemental object!"],[1])
        # The donor inspects the soil but does not consume it.
        rule(vocation+'-soil',144,[storage('eq',1),vocation_guard,
             {'kind':'item','item':f'oteryn:item.tibia.i{soil}','count':1,'op':'ge'}],effects(2),
             [json.loads(lines[146].strip()) + json.loads(re.search(r'".*"', lines[148 + ["sorcerer","druid","paladin","knight"].index(vocation)]).group()), *say(154,155)])
        rule(vocation+'-reward',162,[storage('eq',2),vocation_guard],
             [{'kind':'take_item','item':'oteryn:item.tibia.i954','count':1,'optional':False},
              {'kind':'give_item','item':f'oteryn:item.tibia.i{reward}','count':1},*effects(3)],
             ['AMAZING!! I\'m going to start immediately with the research. If it turns out the way I expect it, Alverus will be revived soon!! Here, take this as a reward and try to collect more of this substance. I\'ll make you a good offer, I promise. '])
    return {'schema':1,'quest':quest,'tracks':[{'key':track,'quest':quest,'initial':-1,
             'min':-1,'max':3,'sources':[ref(105)]}], 'rules':rules,
             'source_files':[{'path':'data-global/npc/arkulius.lua','sha256':digest,
                              'revision':'201189f871d23597b82caf88fd611454a2ddefb8'}]}


def run(check=False):
    outputs = {}
    for name in ['hot_cuisine', 'the_hidden_city_of_beregar']:
        src = HERE / 'batch3_a/npc' / (name+'.json')
        document = prerequisites(json.loads(src.read_text()))
        outputs[src] = document
        outputs[ROOT / 'content/dialogues/quest-rules' / ('quest-batch3-a-'+name+'.json')] = document
    outputs[ROOT / 'content/dialogues/quest-rules/quest-batch3-a-elemental_spheres.json'] = elemental()
    for path, document in outputs.items():
        text = json.dumps(document, ensure_ascii=False, indent=2)+'\n'
        if check:
            if not path.exists() or path.read_text() != text:
                raise ValueError(f'Stale live eligibility: {path}')
        else:
            path.write_text(text)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    run(parser.parse_args().check)
