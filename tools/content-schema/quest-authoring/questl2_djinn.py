"""Djinn faction mission data; reuse the existing exchange and world-reward owners."""
import hashlib
import json
from pathlib import Path
import re

NPCS = ('baa_leal', 'partos', 'alesar', 'yaman', 'bo_ques', 'rata_mari',
        'fa_hradin', 'gabel', 'malor', 'the_orc_king', 'haroun', 'nah_bob')
ITEMS = {3029, 3035, 3231, 3232, 3233, 3234, 3243, 3607}
ROOT = 'oteryn:quest-progress/quest/u7_4/djinn_war/'


def append(add, state, callbacks, offers, inputs, pin):
    def path(npc):
        return f'data-global/npc/{npc}.lua'

    def text(npc, needle):
        rows = inputs[path(npc)].decode().splitlines()
        for number, line in enumerate(rows, 1):
            for value in re.findall(r'"([^"\n]*)"', line):
                if needle in value:
                    return value, f'crystal:{pin}:{path(npc)}:{number}:sha256:{hashlib.sha256(inputs[path(npc)]).hexdigest()}'
        raise ValueError('Djinn source speech changed: ' + npc + ':' + needle)

    def s(faction, suffix, before, after=None):
        return state(ROOT + faction + '_faction/' + suffix, before,
                     before if after is None else after, preserve=after is None)

    def requirement(faction, suffix, value):
        return {'track': ROOT + faction + '_faction/' + suffix,
                'minimum': value, 'maximum': value}

    def grant(key, faction, npc, words, states, needle, topic=0, requires=(), **values):
        reply, witness = text(npc, needle)
        add('l2-' + faction + '-' + key, 'the_djinn_war_' + faction + '_faction',
            npc, words, states, topic=topic, requires=requires, **values)
        callbacks[-1].update(reply=[reply], sources=[witness])

    def offer(faction, npc, words, topic, states, needle, requires=(), before=0):
        reply, _ = text(npc, needle)
        offers.append({'npc': 'oteryn:npc.' + npc, 'keywords': words,
                       'topic': before, 'next_topic': topic, 'storage': states,
                       'requires': list(requires), 'reply': [reply]})

    # The two start flags are checked atomically by the existing fenced writer.
    for faction, npc, needle in [('efreet', 'baa_leal', 'Are you interested, human?'),
                                 ('marid', 'bo_ques', 'Are you interested?')]:
        other = 'marid' if faction == 'efreet' else 'efreet'
        exclusive = [requirement(other, 'start', 0)]
        pending = [s(faction, 'start', 0), s(faction, 'mission01', 0)]
        offer(faction, npc, ['mission'], 1, pending, needle, exclusive)
        grant('enlist', faction, npc, ['yes'],
              [s(faction, 'start', 0, 1), s(faction, 'mission01', 0, 1)],
              'Listen! Since our base' if faction == 'efreet' else 'Fine! Even though',
              topic=1, requires=exclusive)

    grant('supplies', 'efreet', 'partos', ['supplies'], [s('efreet', 'mission01', 1, 2)],
          "I won't tell you anything!", requires=[requirement('efreet', 'start', 1)])
    offer('efreet', 'baa_leal', ['mission'], 2, [s('efreet', 'mission01', 2)], 'Did you find the thief')
    offer('efreet', 'baa_leal', ['yes'], 3, [s('efreet', 'mission01', 2)], 'What is his name', before=2)
    grant('thief-report', 'efreet', 'baa_leal', ['partos'], [s('efreet', 'mission01', 2, 3)],
          'You found the thief!', topic=3, give=(3035, 6))
    offer('efreet', 'alesar', ['mission'], 1, [s('efreet', 'mission02', 0)],
          'Are you prepared to embark', [requirement('efreet', 'mission01', 3)])
    grant('tear-mission', 'efreet', 'alesar', ['yes'],
          [s('efreet', 'mission02', 0, 1), s('efreet', 'door_to_marid_territory', 0, 1)],
          "All right then, human.", topic=1, requires=[requirement('efreet', 'mission01', 3)])
    offer('efreet', 'alesar', ['mission'], 2, [s('efreet', 'mission02', 2)], 'Did you find the tear')
    grant('tear-return', 'efreet', 'alesar', ['yes'], [s('efreet', 'mission02', 2, 3)],
          'You have really managed to steal', topic=2, take=[(3233, 1)])

    offer('marid', 'bo_ques', ['cookbook'], 2, [s('marid', 'mission01', 1)],
          'Can I have it?')
    grant('cookbook', 'marid', 'bo_ques', ['yes'], [s('marid', 'mission01', 1, 2)],
          'The book! You have it!', topic=2, take=[(3234, 1)], give=(3029, 3))
    grant('spy-mission', 'marid', 'fa_hradin', ['mission', 'spy report'],
          [s('marid', 'mission02', 0, 1), s('marid', 'door_to_efreet_territory', 0, 1)],
          'The password is {PIEDPIPER}', requires=[requirement('marid', 'mission01', 2)])
    grant('cheese-request', 'marid', 'rata_mari', ['spyreport', 'mission'],
          [s('marid', 'rata_mari', 0, 1)], 'I need some cheese!',
          requires=[requirement('marid', 'mission02', 1)])
    offer('marid', 'rata_mari', ['spyreport', 'mission'], 1, [s('marid', 'rata_mari', 1)],
          'have you brought me the cheese', [requirement('marid', 'mission02', 1)])
    grant('spy-report-issue', 'marid', 'rata_mari', ['yes'], [s('marid', 'rata_mari', 1, 2)],
          'Here is the spyreport', topic=1, take=[(3607, 1)], give=(3232, 1),
          requires=[requirement('marid', 'mission02', 1)])
    offer('marid', 'fa_hradin', ['mission', 'spy report'], 1, [s('marid', 'mission02', 1)],
          'Did you already retrieve the spyreport?')
    grant('spy-report', 'marid', 'fa_hradin', ['yes'], [s('marid', 'mission02', 1, 2)],
          'You have the report?', topic=1, take=[(3232, 1)],
          requires=[requirement('marid', 'rata_mari', 2)])

    for faction, npc, prior, enemy_door in [
            ('efreet', 'malor', 3, 'door_to_marid_territory'),
            ('marid', 'gabel', 2, 'door_to_efreet_territory')]:
        offer(faction, npc, ['mission'], 1, [s(faction, 'mission03', 0)],
              'Are you prepared to', [requirement(faction, 'mission02', prior)])
        grant('lamp-mission', faction, npc, ['yes'], [s(faction, 'mission03', 0, 1)],
              'Once you have' if faction == 'efreet' else 'Once you have acquired', topic=1,
              requires=[requirement(faction, 'mission02', prior)])
        # The source has one global ReceivedLamp flag; exclusive faction starts
        # make the active faction's exact DoorToLamp flag an equivalent once key.
        lamp_topic = 31 if faction == 'efreet' else 32
        offer(faction, 'the_orc_king', ['lamp'], lamp_topic, [s(faction, 'door_to_lamp', 0)],
              'Who do you want to trap', [requirement(faction, 'mission03', 1)])
        grant('lamp-issue', faction, 'the_orc_king', ['malor'],
              [s(faction, 'door_to_lamp', 0, 1)], 'Take the lamp', topic=lamp_topic,
              requires=[requirement(faction, 'mission03', 1)], give=(3231, 1))
        offer(faction, npc, ['mission'], 2, [s(faction, 'mission03', 2)], 'Have you found')
        grant('finish', faction, npc, ['yes'],
              [s(faction, 'mission03', 2, 3), s(faction, enemy_door, 1)],
              'permission to trade' if faction == 'efreet' else 'welcome to trade', topic=2,
              requires=[requirement(faction, 'start', 1)],
              achievement=f'oteryn:achievement/{faction}_ally')
        _, achievement_witness = text(npc, faction.capitalize() + ' Ally')
        callbacks[-1]['sources'].append(achievement_witness)


def world_callbacks(inputs, pin):
    path = 'data-global/scripts/actions/other/others/quest_system2.lua'
    fountain = 'data-global/scripts/quests/the_djinn_war_quest/action_water_fountain.lua'
    rows = []
    for faction, unique in [('efreet', 2285), ('marid', 3024)]:
        track = ROOT + faction + '_faction/mission03'
        rows.append({
            'key': 'oteryn:quest-callback/l2-djinn-' + faction + '-lamp',
            'quest': 'oteryn:quest.the_djinn_war_' + faction + '_faction',
            'trigger': 'use',
            'selector': {'position': None, 'unique': unique, 'action': None, 'appearance': None},
            'lever': None, 'reversible': False, 'passages': [], 'teleport': None,
            'storage': [{'track': track, 'minimum': 1, 'maximum': 1, 'set': 2}],
            'offerings': [], 'item_on_target': {
                'item': 'oteryn:item.tibia.i3231', 'count': 1,
                'replacement': 'oteryn:item.tibia.i3243',
                'target_replacement': None},
            'return_on_failure': False, 'failure_message': None,
            'effects': {},
            'sources': [f'crystalserver:{pin}:{path}:2-484:sha256:{hashlib.sha256(inputs[path]).hexdigest()}'],
        })
    rows.append({
        'key': 'oteryn:quest-callback/l2-djinn-tear-fountain',
        'quest': 'oteryn:quest.the_djinn_war_efreet_faction', 'trigger': 'use',
        'selector': {'position': None, 'unique': None, 'action': 12105, 'appearance': None},
        'lever': None, 'reversible': False, 'passages': [], 'teleport': None,
        'storage': [{'track': ROOT + 'efreet_faction/mission02',
                     'minimum': 1, 'maximum': 1, 'set': 2}],
        'offerings': [], 'item_on_target': None, 'return_on_failure': False,
        'failure_message': None,
        'effects': {'item_reward': {'item': 'oteryn:item.tibia.i3233',
                    'count': 1, 'contents': []}},
        'sources': [f'crystalserver:{pin}:{fountain}:3-13:sha256:{hashlib.sha256(inputs[fountain]).hexdigest()}'],
    })
    return {'schema': 1, 'callbacks': rows}


def shops(inputs, pin):
    profiles = json.loads((Path(__file__).resolve().parents[3] /
                          'content/items/definitions/spell-native-profiles.json').read_bytes())
    charges = {r['production_definition']['production_key']:
               r['semantics'].get('charges', {}).get('value', {}).get('count', {}).get('value')
               for r in profiles['records']
               if r['semantics'].get('charges', {}).get('state') == 'KNOWN'
               and r['semantics']['charges']['value'].get('count', {}).get('state') == 'KNOWN'
               and type(r['semantics']['charges']['value']['count'].get('value')) is int}
    result = []
    for faction, npcs in [('efreet', ('alesar', 'yaman')), ('marid', ('haroun', 'nah_bob'))]:
        for npc in npcs:
            path = f'data-global/npc/{npc}.lua'
            raw = inputs[path].decode()
            match = re.search(r'npcConfig\.shop\s*=\s*\{(.*?)\n\}', raw, re.S)
            if match is None:
                raise ValueError('Djinn shop source missing: ' + npc)
            offers = []
            for row in re.findall(r'\{([^{}]*clientId\s*=\s*\d+[^{}]*)\}', match.group(1)):
                fields = {k: int(v) for k, v in re.findall(r'(\w+)\s*=\s*(\d+)', row)}
                for field in ('count', 'subType'):
                    if re.search(r'\b' + field + r'\s*=', row) and field not in fields:
                        raise ValueError('Unresolved Djinn source scalar: ' + npc + ':' + field)
                if 'count' in fields:
                    expected = charges.get(f"oteryn:item.tibia.i{fields['clientId']}")
                    if expected != fields['count'] or fields.get('subType', expected) != expected:
                        raise ValueError('Djinn full-charge source mismatch: ' + npc + ':' + row)
                for verb, direction in [('buy', 'SellToPlayer'), ('sell', 'BuyFromPlayer')]:
                    if verb not in fields:
                        continue
                    offers.append({
                        'item': {'family': 'Item', 'key': f"oteryn:item.tibia.i{fields['clientId']}",
                                 'revision': 'definition-r1'},
                        'direction': direction, 'unit_price': fields[verb],
                        'count': 1, 'sub_type': fields.get('subType', fields.get('count')),
                        'requirements': [{
                            'quest': 'oteryn:quest.the_djinn_war_' + faction + '_faction',
                            'track': ROOT + faction + '_faction/mission03',
                            'initial': 0, 'minimum': 3, 'maximum': 3}],
                    })
            if not offers:
                raise ValueError('Empty Djinn source menu: ' + npc)
            first_line = raw[:match.start()].count('\n') + 1
            result.append({'npc': 'oteryn:npc.' + npc,
                           'service': 'oteryn:service.quest.djinn.' + npc,
                           'offers': offers,
                           'sources': [f'crystal:{pin}:{path}:{first_line}:sha256:{hashlib.sha256(inputs[path]).hexdigest()}']})
    return result
