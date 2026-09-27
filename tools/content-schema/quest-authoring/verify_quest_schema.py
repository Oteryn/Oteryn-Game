"""Focused positive/negative checks of the quest content schema and semantic validator (synthetic fixtures only)."""
import json
import sys

from validate_quest_content import BLOCKED, validate, validate_gates, validate_interactions, validate_storylines


def ref(family, name):
    return {'family': family, 'key': 'oteryn:' + name, 'revision': 'r1'}


def fixture():
    claim = {
        'identity': {'key': 'oteryn:reward-claim/annihilator', 'revision': 'r1'}, 'label': 'Demon armor',
        'section': 'Annihilator quest', 'quest': ref('Quest', 'quest/annihilator'), 'quest_link_basis': 'storage_key',
        'quest_candidate_from_section': None, 'claim': {'per': 'character', 'repeat': {'kind': 'once'}},
        'placements': [
            {'position': {'x': 100, 'y': 200, 'z': 7}, 'appearance': ref('Item', 'item/chest'),
             'reward': {'items': [{'item': ref('Item', 'item/demon_armor'), 'count': 1}]}, 'achievement': ref('Achievement', 'achievement/annihilator')},
            {'position': {'x': 102, 'y': 200, 'z': 7}, 'appearance': ref('Item', 'item/chest'),
             'reward': {'items': [], 'random_one_of': [{'item': ref('Item', 'item/gold'), 'count': 10}, {'item': ref('Item', 'item/key'), 'count': 1}],
                        'container': ref('Item', 'item/bag'), 'key_binding': 'oteryn:door-key/3800',
                        'written_text': {'item': ref('Item', 'item/bag'),
                                         'text_ref': {'sha256': '0' * 64, 'length': 8, 'placeholders': []}}}}]}
    quest = {'identity': {'key': 'oteryn:quest/annihilator', 'revision': 'r1'}, 'display_name': 'The Annihilator',
             'kind': 'reward_only', 'shown_in_quest_log': False, 'wiki': {'title': 'The Annihilator', 'pageid': 1, 'revid': 2},
             'requirements_from_wiki': {'premium': 'yes', 'lvl': '100'}, 'claims': [ref('RewardClaim', 'reward-claim/annihilator')]}
    catalog = {'definitions': [ref('Item', f'item/{n}') for n in ('chest', 'demon_armor', 'gold', 'key', 'bag')] +
               [ref('Achievement', 'achievement/annihilator')]}
    manifest = {'entries': [{'position': [100, 200, 7], 'status': 'mapped', 'destination': 'oteryn:reward-claim/annihilator'},
                            {'position': [102, 200, 7], 'status': 'conflict', 'destination': 'oteryn:reward-claim/annihilator'},
                            {'position': [1, 1, 7], 'status': 'approved_omission'}]}
    return {'claims': [claim]}, {'quests': [quest]}, catalog, manifest


results = []


def case(name, mutate=None, expected=False):
    claims, quests, catalog, manifest = fixture()
    if mutate:
        mutate(claims['claims'][0], quests['quests'][0], catalog, manifest)
    errors = validate(claims, quests, catalog, manifest)
    results.append({'name': name, 'expected_valid': expected, 'passed': (not errors) == expected,
                    'first_error': errors[0] if errors else None})


def placement(i):
    return lambda c, q, cat, m: c['placements'][i]


case('fixture accepted', expected=True)
case('cooldown claim accepted', lambda c, q, cat, m: c['claim'].update(repeat={'kind': 'cooldown', 'hours': 24}), expected=True)
case('claims are per character only', lambda c, q, cat, m: c['claim'].update(per='account'))
case('cooldown needs hours', lambda c, q, cat, m: c['claim'].update(repeat={'kind': 'cooldown'}))
case('cooldown hours are positive', lambda c, q, cat, m: c['claim'].update(repeat={'kind': 'cooldown', 'hours': 0}))
case('placements are required', lambda c, q, cat, m: c.update(placements=[]))
case('a placement hands out something', lambda c, q, cat, m: c['placements'][0]['reward'].update(items=[]))
case('counts are positive', lambda c, q, cat, m: c['placements'][0]['reward']['items'][0].update(count=0))
case('random reward needs two options', lambda c, q, cat, m: c['placements'][1]['reward']['random_one_of'].pop())
case('floor is bounded', lambda c, q, cat, m: c['placements'][0]['position'].update(z=16))
case('two claims cannot share a position', lambda c, q, cat, m: c['placements'][1]['position'].update(x=100))
case('unknown reward field', lambda c, q, cat, m: c['placements'][0]['reward'].update(weight=5.0))
case('key binding names a door key', lambda c, q, cat, m: c['placements'][1]['reward'].update(key_binding='oteryn:storage/1'))
case('no committed narrative text', lambda c, q, cat, m: c['placements'][1]['reward']['written_text'].update(text='Hardek *'))
case('written text on an item not handed out', lambda c, q, cat, m: c['placements'][1]['reward']['written_text'].update(item=ref('Item', 'item/gold')))
case('link basis without a quest', lambda c, q, cat, m: c.update(quest=None))
case('section candidate on a linked claim', lambda c, q, cat, m: c.update(quest_candidate_from_section=ref('Quest', 'quest/x')))
case('link basis is closed', lambda c, q, cat, m: c.update(quest_link_basis='section'))
case('quest lists an unknown claim', lambda c, q, cat, m: q['claims'].append(ref('RewardClaim', 'reward-claim/ghost')))
case('claim and quest disagree', lambda c, q, cat, m: c.update(quest=ref('Quest', 'quest/other')))
case('quest kind is closed', lambda c, q, cat, m: q.update(kind='storyline'))
case('duplicate claim key', lambda c, q, cat, m: None)
case('item missing from catalog', lambda c, q, cat, m: cat['definitions'].pop(1))
case('achievement missing from catalog', lambda c, q, cat, m: cat['definitions'].pop())
case('manifest status is closed', lambda c, q, cat, m: m['entries'][2].update(status='skipped'))
case('manifest destination must exist', lambda c, q, cat, m: m['entries'][0].update(destination='oteryn:reward-claim/ghost'))
case('mapped entry needs a destination', lambda c, q, cat, m: m['entries'][0].pop('destination'))
case('every claim is mapped from a source', lambda c, q, cat, m: m.update(entries=m['entries'][2:]))

# 'duplicate claim key' needs two claims: rebuild it by hand
claims, quests, catalog, manifest = fixture()
claims['claims'].append(json.loads(json.dumps(claims['claims'][0])))
for p in claims['claims'][1]['placements']:
    p['position']['y'] += 50
errors = validate(claims, quests, catalog, manifest)
results[[r['name'] for r in results].index('duplicate claim key')] = {
    'name': 'duplicate claim key', 'expected_valid': False, 'passed': any('duplicate claim key' in e for e in errors),
    'first_error': errors[0] if errors else None}

# one quest cannot appear under two namespaces
claims, quests, catalog, manifest = fixture()
twin = json.loads(json.dumps(quests['quests'][0]))
twin['identity']['key'] = 'other:quest/annihilator'
twin['claims'] = []
twin['kind'] = 'reward_only'
claims['claims'].append(json.loads(json.dumps(claims['claims'][0])))
claims['claims'][1]['identity']['key'] = 'oteryn:reward-claim/second'
claims['claims'][1]['quest'] = {'family': 'Quest', 'key': 'other:quest/annihilator', 'revision': 'r1'}
for p in claims['claims'][1]['placements']:
    p['position']['y'] += 50
twin['claims'] = [{'family': 'RewardClaim', 'key': 'oteryn:reward-claim/second', 'revision': 'r1'}]
quests['quests'].append(twin)
manifest['entries'].append({'position': [100, 250, 7], 'status': 'mapped', 'destination': 'oteryn:reward-claim/second'})
errors = validate(claims, quests, catalog, manifest)
results.append({'name': 'one identity per quest across namespaces', 'expected_valid': False,
                'passed': bool(errors) and 'one identity per quest' in errors[0], 'first_error': errors[0] if errors else None})

# an unlinked claim keeps its section candidate and no quest lists it
claims, quests, catalog, manifest = fixture()
claims['claims'][0].update(quest=None, quest_link_basis=None, quest_candidate_from_section=ref('Quest', 'quest/annihilator'))
quests['quests'].clear()
errors = validate(claims, quests, catalog, manifest)
results.insert(2, {'name': 'unlinked claim with section candidate accepted', 'expected_valid': True, 'passed': not errors,
                   'first_error': errors[0] if errors else None})



def gate_fixture():
    claims, _, _, _ = fixture()
    claims['claims'][0]['placements'][1]['reward']['key_binding'] = 'oteryn:door-key/3800'
    gates = {'gates': [
        {'identity': {'key': 'oteryn:door-gate/progress/reward-claim', 'revision': 'r1'}, 'label': 'The annihilator door',
         'quest': ref('Quest', 'quest/annihilator'), 'quest_link_basis': 'storage_key',
         'condition': {'kind': 'quest_progress', 'progress': 'oteryn:quest-progress/annihilator', 'claim': ref('RewardClaim', 'reward-claim/annihilator')},
         'state': 'per_character_pass', 'placements': [{'position': {'x': 90, 'y': 200, 'z': 7}, 'appearance': None}]},
        {'identity': {'key': 'oteryn:door-gate/level/100', 'revision': 'r1'}, 'label': None, 'quest': None, 'quest_link_basis': None,
         'condition': {'kind': 'min_level', 'level': 100}, 'state': 'per_character_pass',
         'placements': [{'position': {'x': 91, 'y': 200, 'z': 7}, 'appearance': ref('Item', 'item/door')}]},
        {'identity': {'key': 'oteryn:door-gate/key/3800', 'revision': 'r1'}, 'label': None, 'quest': None, 'quest_link_basis': None,
         'condition': {'kind': 'door_key', 'key_binding': 'oteryn:door-key/3800', 'key_from_claims': [ref('RewardClaim', 'reward-claim/annihilator')]},
         'state': 'shared_lock', 'placements': [{'position': {'x': 92, 'y': 200, 'z': 7}, 'appearance': None}]}]}
    manifest = {'entries': [{'position': [90 + i, 200, 7], 'status': 'mapped', 'destination': g['identity']['key']}
                            for i, g in enumerate(gates['gates'])]}
    return gates, claims, manifest


def gate_case(name, mutate=None, expected=False):
    gates, claims, manifest = gate_fixture()
    if mutate:
        mutate(gates['gates'], claims, manifest)
    errors = validate_gates(gates, claims, manifest)
    results.append({'name': name, 'expected_valid': expected, 'passed': (not errors) == expected,
                    'first_error': errors[0] if errors else None})


gate_case('gate fixture accepted', expected=True)
gate_case('condition kind is closed', lambda g, c, m: g[1]['condition'].update(kind='vocation'))
gate_case('level is positive', lambda g, c, m: g[1]['condition'].update(level=0))
gate_case('only key doors share a lock', lambda g, c, m: g[1].update(state='shared_lock'))
gate_case('key doors share a lock', lambda g, c, m: g[2].update(state='per_character_pass'))
gate_case('two gates cannot share a position', lambda g, c, m: g[1]['placements'][0]['position'].update(x=90))
gate_case('progress door reads the claim it names', lambda g, c, m: g[0]['condition'].update(progress='oteryn:quest-progress/other'))
gate_case('progress door names a known claim', lambda g, c, m: g[0]['condition'].update(claim=ref('RewardClaim', 'reward-claim/ghost')))
gate_case('key comes from a chest that hands it out', lambda g, c, m: g[2]['condition'].update(key_binding='oteryn:door-key/1'))
gate_case('gate link basis without a quest', lambda g, c, m: g[0].update(quest=None))
gate_case('every gate is mapped from a source', lambda g, c, m: m['entries'].pop())



TEXT = {'sha256': '1' * 64, 'length': 25, 'placeholders': []}


def progress(name):
    return f'oteryn:quest-progress/{name}'


def storyline_fixture():
    gates, claims, _ = gate_fixture()
    quest = {'identity': {'key': 'oteryn:quest/banshees', 'revision': 'r1'}, 'display_name': 'The Queen of the Banshees',
             'kind': 'storyline', 'shown_in_quest_log': True, 'source_name': 'The Queen of the Banshees',
             'start': {'progress': progress('first_seal'), 'at_least': 1}, 'claims': [],
             'gates': [ref('Gate', 'door-gate/level/100')],
             'missions': [
                 {'key': 'the_hidden_seal', 'name': 'The Hidden Seal', 'progress': progress('first_seal'), 'start_value': 1,
                  'end_value': 1, 'journal': {'kind': 'fixed', 'text_ref': TEXT},
                  'transitions': [{'key': 'movement_1', 'owner': 'movement', 'callback': 'onStepIn',
                                   'from': {'op': '<', 'value': 1, 'exact': True}, 'to': 1, 'servers': ['canary', 'crystalserver']}]},
                 {'key': 'the_plague_seal', 'name': 'The Plague Seal', 'progress': progress('second_seal'), 'start_value': 1,
                  'end_value': 3, 'transitions': [{'key': 'npc_1', 'owner': 'npc', 'callback': None, 'from': None, 'increment': 1,
                                                   'servers': ['canary']}],
                  'journal': {'kind': 'per_stage', 'stages': [
                      {'value': 1, 'text_ref': TEXT},
                      {'value': 2, 'template': {'parts': [dict(TEXT, placeholders=['%d'])], 'reads': ['quest/kills']}}]}}]}
    tracks = {'progress': [
        {'key': progress('first_seal'), 'missions': ['oteryn:quest/banshees#the_hidden_seal'], 'start_of': ['oteryn:quest/banshees'],
         'transitions': [{'key': 'movement_1'}]},
        {'key': progress('second_seal'), 'missions': ['oteryn:quest/banshees#the_plague_seal'], 'start_of': [],
         'transitions': [{'key': 'npc_1'}]}]}
    return {'quests': [quest]}, gates, tracks, claims


def storyline_case(name, mutate=None, expected=False):
    quests, gates, tracks, claims = storyline_fixture()
    if mutate:
        mutate(quests['quests'][0], tracks['progress'])
    errors = validate({'claims': []}, quests) + validate_storylines(quests, gates, tracks)
    results.append({'name': name, 'expected_valid': expected, 'passed': (not errors) == expected,
                    'first_error': errors[0] if errors else None})


storyline_case('storyline fixture accepted', expected=True)
storyline_case('storyline without a start accepted', lambda q, t: (q.update(start=None), t[0].update(start_of=[])) and None, expected=True)
storyline_case('storyline needs missions', lambda q, t: q.pop('missions'))
storyline_case('reward-only quest has no missions', lambda q, t: q.update(kind='reward_only', claims=[ref('RewardClaim', 'x')]))
storyline_case('journal kind is closed', lambda q, t: q['missions'][0].update(journal={'kind': 'video'}))
storyline_case('a stage has text or a template', lambda q, t: q['missions'][1]['journal']['stages'][0].pop('text_ref'))
storyline_case('a stage has not both', lambda q, t: q['missions'][1]['journal']['stages'][0].update(template={'parts': [], 'reads': []}))
storyline_case('no committed journal text', lambda q, t: q['missions'][0]['journal'].update(text='You broke the first seal.'))
storyline_case('mission keys are unique', lambda q, t: (q['missions'][1].update(key='the_hidden_seal'),
                                                        t[1]['missions'].append('oteryn:quest/banshees#the_hidden_seal')) and None)
storyline_case('start not above end', lambda q, t: q['missions'][0].update(start_value=2))
storyline_case('stages ascend', lambda q, t: q['missions'][1]['journal']['stages'].reverse())
storyline_case('stages stay in range', lambda q, t: q['missions'][1]['journal']['stages'][1].update(value=9))
storyline_case('progress track lists the mission', lambda q, t: t[1]['missions'].clear())
storyline_case('start track names the quest', lambda q, t: t[0]['start_of'].clear())
storyline_case('quest names a known gate', lambda q, t: q['gates'].append(ref('Gate', 'door-gate/ghost')))
storyline_case('a transition has one effect', lambda q, t: q['missions'][0]['transitions'][0].update(increment=1))
storyline_case('a transition has an effect', lambda q, t: q['missions'][0]['transitions'][0].pop('to'))
storyline_case('transition owner is closed', lambda q, t: q['missions'][0]['transitions'][0].update(owner='wizard'))
storyline_case('increments are positive', lambda q, t: q['missions'][1]['transitions'][0].update(increment=0))
storyline_case('transition keys are unique', lambda q, t: q['missions'][0]['transitions'].append(dict(q['missions'][0]['transitions'][0])))
storyline_case('a transition has source evidence', lambda q, t: t[0]['transitions'].clear())
storyline_case('progress names a quest-progress track', lambda q, t: q['missions'][0].update(progress='oteryn:storage/1'))


def interaction_fixture():
    seal = {'quest_stage': {'progress': progress('first_seal'), 'op': '<', 'value': 1}, 'negate': False}
    lever = {'object': {'role': 'source', 'field': 'item_type', 'op': '==', 'item': ref('Item', 'item/lever')}, 'negate': False}
    interaction = {
        'identity': {'key': 'oteryn:interaction/first_seal_flame', 'revision': 'r1'},
        'source': {'edge': 'ON_ENTER', 'callback': 'onStepIn', 'target_registrations': ['aid(25010)']},
        'rules': [{'branch': [{'when': {'all': [{'actor_is_player': True, 'negate': False}, seal]}, 'then': [
            {'owner': 'Quest', 'request': 'set_progress', 'progress': progress('first_seal'), 'to': 1,
             'transition': 'oteryn:quest/banshees#the_hidden_seal:movement_1'},
            {'owner': 'Ability', 'effect': 'summon', 'creature': ref('Creature', 'creature/banshee'), 'anchor': 'p1'},
            {'owner': 'Presentation', 'effect': 'magic_effect', 'authoritative': False}]},
            {'when': lever, 'then': [{'owner': 'WorldObject', 'status': 'blocked', 'reason': BLOCKED['WorldObject'],
                                      'source_line': 12}]}],
            'otherwise': [{'owner': 'Movement', 'status': 'blocked', 'reason': BLOCKED['Movement'], 'to_anchor': 'p2'}]}],
        'anchors': [{'key': 'p1', 'source_position': {'x': 1, 'y': 2, 'z': 7}},
                    {'key': 'p2', 'source_position': {'x': 3, 'y': 4, 'z': 7}}],
        'unresolved': []}
    manifest = {'undeclared_progress_tracks': [],
                'entries': [{'destination': 'oteryn:interaction/first_seal_flame', 'status': 'mapped',
                             'sources': [{'source': 'canary'}, {'source': 'crystalserver'}]}]}
    return {'interactions': [interaction]}, manifest


def interaction_case(name, mutate=None, expected=False):
    quests, _, tracks, _ = storyline_fixture()
    doc, manifest = interaction_fixture()
    if mutate:
        i = doc['interactions'][0]
        mutate(i, i['rules'][0]['branch'][0]['then'], manifest)
        if i.pop('twin', False):
            doc['interactions'].append(json.loads(json.dumps(i)))
    errors = validate_interactions(doc, manifest, quests, tracks)
    results.append({'name': name, 'expected_valid': expected, 'passed': (not errors) == expected,
                    'first_error': errors[0] if errors else None})


def unresolved_line(i, children, m):
    i['rules'][0]['branch'][1]['when'] = {'unresolved': {'line': 9}}


interaction_case('interaction fixture accepted', expected=True)
interaction_case('unresolved interaction accepted',
                 lambda i, c, m: (unresolved_line(i, c, m), m['entries'][0].update(status='unresolved_semantics')) and None,
                 expected=True)
interaction_case('undeclared track listed in the manifest accepted',
                 lambda i, c, m: (c[0].pop('transition'), c[0].update(progress=progress('door')),
                                  m.update(undeclared_progress_tracks=[progress('door')])) and None, expected=True)
interaction_case('item, achievement and message children accepted',
                 lambda i, c, m: c.extend([{'owner': 'Item', 'request': 'hand_out', 'item': ref('Item', 'item/key'), 'count': 1},
                                           {'owner': 'Achievement', 'request': 'grant', 'achievement': ref('Achievement', 'achievement/x')},
                                           {'owner': 'Presentation', 'effect': 'message', 'authoritative': False, 'source_line': 4}]),
                 expected=True)
interaction_case('level condition accepted',
                 lambda i, c, m: i['rules'][0]['branch'][1].update(when={'actor_level': {'op': '>=', 'value': 100}, 'negate': False}),
                 expected=True)
interaction_case('a hand-out names an item', lambda i, c, m: c.append({'owner': 'Item', 'request': 'hand_out', 'count': 1}))
interaction_case('a hand-out item is an Item',
                 lambda i, c, m: c.append({'owner': 'Item', 'request': 'hand_out', 'item': ref('Creature', 'creature/x'), 'count': 1}))
interaction_case('a message keeps no text',
                 lambda i, c, m: c.append({'owner': 'Presentation', 'effect': 'message', 'authoritative': False, 'source_line': 4,
                                           'text': 'Hello.'}))
interaction_case('a message keeps its source line',
                 lambda i, c, m: c.append({'owner': 'Presentation', 'effect': 'message', 'authoritative': False}))
interaction_case('edge is closed', lambda i, c, m: i['source'].update(edge='ON_WHISPER'))
interaction_case('child owner is closed', lambda i, c, m: c[2].update(owner='Script'))
interaction_case('no committed text in presentation', lambda i, c, m: c[2].update(text='The seal breaks.'))
interaction_case('movement is blocked', lambda i, c, m: i['rules'][0]['otherwise'][0].update(status='ready'))
interaction_case('blocked for the known reason', lambda i, c, m: i['rules'][0]['otherwise'][0].update(reason='later'))
interaction_case('world object needs its source line', lambda i, c, m: i['rules'][0]['branch'][1]['then'][0].pop('source_line'))
interaction_case('presentation is not authoritative', lambda i, c, m: c[2].update(authoritative=True))
interaction_case('item condition names an item', lambda i, c, m: i['rules'][0]['branch'][1]['when']['object'].update(value=5))
interaction_case('item condition item is an Item',
                 lambda i, c, m: i['rules'][0]['branch'][1]['when']['object'].update(item=ref('Creature', 'creature/x')))
interaction_case('uid condition has a value', lambda i, c, m: i['rules'][0]['branch'][1]['when']['object'].update(field='unique_id'))
interaction_case('summon names a creature', lambda i, c, m: c[1].pop('creature'))
interaction_case('anchor exists', lambda i, c, m: c[1].update(anchor='p9'))
interaction_case('anchor is used', lambda i, c, m: i['anchors'].append({'key': 'p3', 'source_position': {'x': 5, 'y': 5, 'z': 7}}))
interaction_case('anchor positions are unique', lambda i, c, m: i['anchors'][1].update(source_position={'x': 1, 'y': 2, 'z': 7}))
interaction_case('transition exists', lambda i, c, m: c[0].update(transition='oteryn:quest/banshees#the_hidden_seal:npc_9'))
interaction_case('transition moves its own track', lambda i, c, m: c[0].update(transition='oteryn:quest/banshees#the_plague_seal:npc_1'))
interaction_case('undeclared track is listed', lambda i, c, m: (c[0].pop('transition'), c[0].update(progress=progress('door'))) and None)
interaction_case('listed tracks are undeclared', lambda i, c, m: m.update(undeclared_progress_tracks=[progress('first_seal')]))
interaction_case('mapped means resolved', lambda i, c, m: unresolved_line(i, c, m))
interaction_case('mapped means no unresolved line',
                 lambda i, c, m: i['unresolved'].append({'line': 3, 'reason': 'statement outside the transcribed vocabulary'}))
interaction_case('conflict needs two sources',
                 lambda i, c, m: (m['entries'][0].update(status='conflict'), m['entries'][0]['sources'].pop()) and None)
interaction_case('every interaction has a manifest entry', lambda i, c, m: m['entries'].clear())
interaction_case('manifest lists an interaction once', lambda i, c, m: m['entries'].append(dict(m['entries'][0])))
interaction_case('interaction keys are unique', lambda i, c, m: i.update(twin=True))

failed = [r for r in results if not r['passed']]
if '--verbose' in sys.argv:
    for r in results:
        print(f"{r['name']:<48} {r['first_error']}")
print(json.dumps({'cases': len(results), 'passed': len(results) - len(failed), 'failed': failed}, indent=2))
sys.exit(1 if failed else 0)
