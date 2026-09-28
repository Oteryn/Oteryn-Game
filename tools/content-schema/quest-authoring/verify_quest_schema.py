"""Focused positive/negative checks of the quest content schema and semantic validator (synthetic fixtures only)."""
import json
import sys
import tempfile
from pathlib import Path

from validate_quest_content import BLOCKED, validate, validate_gates, validate_interactions, validate_storylines
import ots_interactions as oi


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
gate_case('condition kind is closed', lambda g, c, m: g[1]['condition'].update(kind='wizard'))
gate_case('level is positive', lambda g, c, m: g[1]['condition'].update(level=0))
gate_case('only key doors share a lock', lambda g, c, m: g[1].update(state='shared_lock'))
gate_case('key doors share a lock', lambda g, c, m: g[2].update(state='per_character_pass'))
gate_case('two gates cannot share a position', lambda g, c, m: g[1]['placements'][0]['position'].update(x=90))
gate_case('progress door reads the claim it names', lambda g, c, m: g[0]['condition'].update(progress='oteryn:quest-progress/other'))
gate_case('progress door names a known claim', lambda g, c, m: g[0]['condition'].update(claim=ref('RewardClaim', 'reward-claim/ghost')))
gate_case('key comes from a chest that hands it out', lambda g, c, m: g[2]['condition'].update(key_binding='oteryn:door-key/1'))
gate_case('gate link basis without a quest', lambda g, c, m: g[0].update(quest=None))
gate_case('every gate is mapped from a source', lambda g, c, m: m['entries'].pop())
gate_case('lever gate accepted',
          lambda g, c, m: g[2].update(condition={'kind': 'lever', 'lever_position': {'x': 1, 'y': 2, 'z': 7}}), expected=True)
gate_case('lever needs a position', lambda g, c, m: g[2].update(condition={'kind': 'lever'}))
gate_case('lever gate shares a lock',
          lambda g, c, m: g[2].update(condition={'kind': 'lever', 'lever_position': {'x': 1, 'y': 2, 'z': 7}}, state='per_character_pass'))



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
                                                   'requested_by': {'npc': 'canary:npc/the_queen', 'keywords': ['mission', 'yes'],
                                                                    'topics': [2]},
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
storyline_case('script-only quest accepted',
               lambda q, t: (q.update(kind='script_only', wiki={'title': 'X', 'pageid': 1, 'revid': 1}),
                             q.pop('missions'), q.pop('start'), q.pop('gates')) and None, expected=True)
storyline_case('script-only quest needs wiki',
               lambda q, t: (q.update(kind='script_only'), q.pop('missions'), q.pop('start'), q.pop('gates')) and None)
storyline_case('script-only quest has no missions',
               lambda q, t: q.update(kind='script_only', wiki={'title': 'X', 'pageid': 1, 'revid': 1}))
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
def auxiliary(owner):
    return {'key': progress('seal_door'), 'missions': [], 'start_of': [], 'read_by_gates': [], 'auxiliary_of': [owner],
            'owner_basis': 'mission track prefix', 'writes': {'canary': 1}, 'transitions': []}


storyline_case('auxiliary track accepted', lambda q, t: t.append(auxiliary('oteryn:quest/banshees')), expected=True)
storyline_case('auxiliary track names a known quest', lambda q, t: t.append(auxiliary('oteryn:quest/ghost')))
storyline_case('an NPC transition names its dialogue', lambda q, t: q['missions'][1]['transitions'][0].pop('requested_by'))
storyline_case('only NPC transitions name dialogue',
               lambda q, t: q['missions'][0]['transitions'][0].update(requested_by={'npc': 'canary:npc/x', 'keywords': [], 'topics': []}))
storyline_case('dialogue names an NPC bundle key',
               lambda q, t: q['missions'][1]['transitions'][0]['requested_by'].update(npc='npc/the_queen'))
storyline_case('dialogue keywords are short words',
               lambda q, t: q['missions'][1]['transitions'][0]['requested_by'].update(keywords=['three short words']))
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
            {'when': lever, 'then': [{'owner': 'WorldObject', 'operation': 'TRANSFORM', 'value_source_line': 12},
                                     {'owner': 'WorldObject', 'status': 'blocked', 'reason': BLOCKED['WorldObject'],
                                      'source_line': 13}]}],
            'otherwise': [{'owner': 'Movement', 'request': 'relocate', 'scope': 'in_scope',
                           'target': {'kind': 'anchor', 'anchor': 'p2'}},
                          {'owner': 'Movement', 'status': 'blocked', 'reason': BLOCKED['Movement'], 'to_source_line': 20}]}],
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
interaction_case('item count condition accepted',
                 lambda i, c, m: i['rules'][0]['branch'][1].update(
                     when={'actor_item_count': {'item': ref('Item', 'item/key'), 'op': '>=', 'value': 1}, 'negate': False}),
                 expected=True)
interaction_case('item count condition names an item',
                 lambda i, c, m: i['rules'][0]['branch'][1].update(
                     when={'actor_item_count': {'op': '>=', 'value': 1}, 'negate': False}))
interaction_case('item count condition item is an Item',
                 lambda i, c, m: i['rules'][0]['branch'][1].update(
                     when={'actor_item_count': {'item': ref('Creature', 'creature/x'), 'op': '>=', 'value': 1}, 'negate': False}))
interaction_case('consumption of the used item accepted',
                 lambda i, c, m: c.append({'owner': 'Item', 'request': 'consume', 'object': 'used_item'}), expected=True)
interaction_case('a consumption names what it consumes', lambda i, c, m: c.append({'owner': 'Item', 'request': 'consume'}))
interaction_case('a hand-out names an item', lambda i, c, m: c.append({'owner': 'Item', 'request': 'hand_out', 'count': 1}))
interaction_case('a hand-out item is an Item',
                 lambda i, c, m: c.append({'owner': 'Item', 'request': 'hand_out', 'item': ref('Creature', 'creature/x'), 'count': 1}))
interaction_case('a hand-out container may list its contents',
                 lambda i, c, m: c.append({'owner': 'Item', 'request': 'hand_out', 'item': ref('Item', 'item/backpack'), 'count': 1,
                                           'contents': [{'item': ref('Item', 'item/rope'), 'count': 1}]}),
                 expected=True)
interaction_case('contents name an item and a count',
                 lambda i, c, m: c.append({'owner': 'Item', 'request': 'hand_out', 'item': ref('Item', 'item/backpack'), 'count': 1,
                                           'contents': [{'count': 1}]}))
interaction_case('an achievement grant names an achievement',
                 lambda i, c, m: c.append({'owner': 'Achievement', 'request': 'grant', 'achievement': ref('Achievement', 'achievement/y')}),
                 expected=True)
interaction_case('a table-driven achievement id keeps its source line',
                 lambda i, c, m: c.append({'owner': 'Achievement', 'request': 'grant', 'value_source_line': 9}), expected=True)
interaction_case('an achievement grant is not empty', lambda i, c, m: c.append({'owner': 'Achievement', 'request': 'grant'}))
interaction_case('an outfit grant names a looktype',
                 lambda i, c, m: c.append({'owner': 'Outfit', 'request': 'grant', 'looktype': 128, 'addon': 3}), expected=True)
interaction_case('an outfit grant may keep its source line instead',
                 lambda i, c, m: c.append({'owner': 'Outfit', 'request': 'grant', 'value_source_line': 9}), expected=True)
interaction_case('an outfit grant is not empty', lambda i, c, m: c.append({'owner': 'Outfit', 'request': 'grant'}))
interaction_case('an outfit addon is in range', lambda i, c, m: c.append({'owner': 'Outfit', 'request': 'grant', 'looktype': 128, 'addon': 4}))
interaction_case('a mount grant names a mount',
                 lambda i, c, m: c.append({'owner': 'Mount', 'request': 'grant', 'mount': 42}), expected=True)
interaction_case('a mount grant is not empty', lambda i, c, m: c.append({'owner': 'Mount', 'request': 'grant'}))
interaction_case('an experience grant names an amount',
                 lambda i, c, m: c.append({'owner': 'Experience', 'request': 'grant', 'amount': 100}), expected=True)
interaction_case('an experience grant is not empty', lambda i, c, m: c.append({'owner': 'Experience', 'request': 'grant'}))
interaction_case('a map mark keeps its source line',
                 lambda i, c, m: c.append({'owner': 'Presentation', 'effect': 'map_mark', 'authoritative': False, 'source_line': 4}),
                 expected=True)
interaction_case('a map mark needs its source line',
                 lambda i, c, m: c.append({'owner': 'Presentation', 'effect': 'map_mark', 'authoritative': False}))
interaction_case('a map mark keeps no label', lambda i, c, m: c.append({'owner': 'Presentation', 'effect': 'map_mark',
                                                                        'authoritative': False, 'source_line': 4, 'label': 'Shop'}))
interaction_case('a message keeps no text',
                 lambda i, c, m: c.append({'owner': 'Presentation', 'effect': 'message', 'authoritative': False, 'source_line': 4,
                                           'text': 'Hello.'}))
interaction_case('a message keeps its source line',
                 lambda i, c, m: c.append({'owner': 'Presentation', 'effect': 'message', 'authoritative': False}))
interaction_case('edge is closed', lambda i, c, m: i['source'].update(edge='ON_WHISPER'))
interaction_case('child owner is closed', lambda i, c, m: c[2].update(owner='Script'))
interaction_case('no committed text in presentation', lambda i, c, m: c[2].update(text='The seal breaks.'))
interaction_case('presentation is not authoritative', lambda i, c, m: c[2].update(authoritative=True))

# D37: relocation children (the scope runtime owns them; VSL-MOVE-01/proposal §3)
interaction_case('relocation to the previous tile accepted',
                 lambda i, c, m: (i['rules'][0]['otherwise'][0].update(target={'kind': 'previous_position'}),
                                  i['anchors'].pop(1)) and None, expected=True)
interaction_case('relocation is in scope only', lambda i, c, m: i['rules'][0]['otherwise'][0].update(scope='cross_scope'))
interaction_case('relocation target kind is closed',
                 lambda i, c, m: i['rules'][0]['otherwise'][0].update(target={'kind': 'scope_handoff'}))
interaction_case('a computed relocation target has no anchor',
                 lambda i, c, m: i['rules'][0]['otherwise'][1].update(to_anchor='p2'))
interaction_case('movement is blocked', lambda i, c, m: i['rules'][0]['otherwise'][1].update(status='ready'))
interaction_case('blocked for the known reason', lambda i, c, m: i['rules'][0]['otherwise'][1].update(reason='later'))
interaction_case('a relocation child is not also blocked',
                 lambda i, c, m: i['rules'][0]['otherwise'][0].update(status='blocked'))

# D38: world-object overlay operations (TRANSFORM/CREATE/REMOVE/RETAG; revert_after_ms is authored only)
def literal(i, **fields):
    """Replace the fixture's TRANSFORM op's evidence-only value with a literal-resolved one."""
    op = i['rules'][0]['branch'][1]['then'][0]
    op.pop('value_source_line')
    op.update(fields)


interaction_case('a transform with a literal from/to accepted',
                 lambda i, c, m: literal(i, **{'from': ref('Item', 'item/2772'), 'to': ref('Item', 'item/2773')},
                                        revert_after_ms=5000),
                 expected=True)
interaction_case('a transform needs a resolved from/to or evidence line',
                 lambda i, c, m: i['rules'][0]['branch'][1]['then'][0].pop('value_source_line'))
interaction_case('a transform has one or the other, not both',
                 lambda i, c, m: i['rules'][0]['branch'][1]['then'][0].update(
                     **{'from': ref('Item', 'item/2772'), 'to': ref('Item', 'item/2773')}))
interaction_case('a create names its def or keeps its source line',
                 lambda i, c, m: literal(i, operation='CREATE', anchor='p1', **{'def': ref('Item', 'item/2793')}),
                 expected=True)
interaction_case('a create is not empty',
                 lambda i, c, m: (i['rules'][0]['branch'][1]['then'][0].update(operation='CREATE'),
                                  i['rules'][0]['branch'][1]['then'][0].pop('value_source_line')) and None)
interaction_case('a remove keeps its source line',
                 lambda i, c, m: i['rules'][0]['branch'][1]['then'][0].update(operation='REMOVE'), expected=True)
interaction_case('a remove names its def or keeps its source line',
                 lambda i, c, m: literal(i, operation='REMOVE', **{'def': ref('Item', 'item/2793')}), expected=True)
interaction_case('a retag needs no action-id field',
                 lambda i, c, m: i['rules'][0]['branch'][1]['then'][0].update(
                     operation='RETAG', **{'from': ref('Item', 'item/2772')}))
interaction_case('a retag needs its source line',
                 lambda i, c, m: (i['rules'][0]['branch'][1]['then'][0].update(operation='RETAG'),
                                  i['rules'][0]['branch'][1]['then'][0].pop('value_source_line')) and None)
interaction_case('a retag by itself accepted',
                 lambda i, c, m: i['rules'][0]['branch'][1]['then'][0].update(operation='RETAG'), expected=True)
interaction_case('overlay operation kind is closed', lambda i, c, m: i['rules'][0]['branch'][1]['then'][0].update(operation='ROTATE'))
interaction_case('revert_after_ms is a positive duration',
                 lambda i, c, m: i['rules'][0]['branch'][1]['then'][0].update(revert_after_ms=0))
interaction_case('an overlay anchor is used',
                 lambda i, c, m: literal(i, operation='CREATE', anchor='p9', **{'def': ref('Item', 'item/2793')}))
interaction_case('an overlay anchor exists',
                 lambda i, c, m: literal(i, operation='CREATE', anchor='p1', **{'def': ref('Item', 'item/2793')}),
                 expected=True)
interaction_case('world object needs its source line', lambda i, c, m: i['rules'][0]['branch'][1]['then'][1].pop('source_line'))
interaction_case('a world object child is not also typed',
                 lambda i, c, m: i['rules'][0]['branch'][1]['then'][1].update(operation='TRANSFORM'))
interaction_case('world object blocked for the known reason',
                 lambda i, c, m: i['rules'][0]['branch'][1]['then'][1].update(reason='later'))
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

def run_converter(lua_body, callback='onUse'):
    """Run a small, hand-written (never real-source) Lua snippet through the real converter
    (`ots_interactions.Script`) and return (flat children, the script's anchors). Regression coverage
    for the converter's own classification logic, distinct from the schema-fixture cases above."""
    text = '\n'.join([f'function test:{callback}(player, item, fromPosition, target, toPosition)']
                     + lua_body + ['end'])
    with tempfile.TemporaryDirectory() as tmp:
        (Path(tmp) / 'test.lua').write_text(text)
        script = oi.Script('canary', tmp, 'test.lua', {}, 'test')
        script.declared = {}
        script.bind(1, callback)
        nodes = oi.lua_blocks.parse(script.lines, oi.lua_blocks.function_body(script.lines, 1))
        rules = script.convert(nodes)
        oi.strip_internal(rules)  # exactly what interactions() does before a rule tree is ever committed
    return list(oi.walk(rules)), script.anchors


def converter_case(name, lua_body, check, callback='onUse', expected=True):
    try:
        children, anchors = run_converter(lua_body, callback)
        ok, detail = check(children, anchors)
    except Exception as exc:  # a converter bug surfaces here as a failed case, not a crashed test run
        ok, detail = False, f'{type(exc).__name__}: {exc}'
    results.append({'name': name, 'expected_valid': expected, 'passed': ok == expected, 'first_error': None if ok else detail})


# Round 1, Finding 1 (P2): a revert call (`decay`/`revertItem`/`addEvent(Position.revertItem, ...)`)
# must never become its own WorldObject child.
converter_case(
    'a same-receiver bare decay attaches with no revert_after_ms, never its own child',
    ['item:transform(2773)', 'item:decay()'],
    lambda c, a: (len(c) == 1 and c[0]['owner'] == 'WorldObject' and c[0]['operation'] == 'TRANSFORM'
                 and 'revert_after_ms' not in c[0], c))
converter_case(
    'a revert with no preceding operation stays blocked, never a standalone TRANSFORM',
    ['item:decay()'],
    lambda c, a: (len(c) == 1 and c[0]['owner'] == 'WorldObject' and c[0].get('status') == 'blocked'
                 and 'operation' not in c[0], c))
converter_case(
    'a revert in a different branch from its target does not merge across branches',
    ['if item.itemid == 2772 then', 'item:transform(2773)', 'end',
     'if item.itemid == 9999 then', 'item:decay()', 'end'],
    lambda c, a: (len([x for x in c if x.get('owner') == 'WorldObject' and x.get('status') == 'blocked']) == 1
                 and len([x for x in c if x.get('operation') == 'TRANSFORM']) == 1, c))

# Round 2, Finding 1 (P2): only a bare `teleportTo(fromPosition)` is a previous-tile relocation; an
# offset or lookup that merely mentions `fromPosition` stays a computed, blocked target.
converter_case(
    'a bare teleportTo(fromPosition) is a previous-tile relocation',
    ['creature:teleportTo(fromPosition)'],
    lambda c, a: (c == [{'owner': 'Movement', 'request': 'relocate', 'scope': 'in_scope',
                         'target': {'kind': 'previous_position'}}], c), callback='onStepIn')
converter_case(
    'a trailing non-positional argument does not disqualify the previous tile',
    ['creature:teleportTo(fromPosition, true)'],
    lambda c, a: (c[0].get('target', {}).get('kind') == 'previous_position', c), callback='onStepIn')
converter_case(
    'the exact Codex example: an offset that merely references fromPosition stays blocked',
    ['creature:teleportTo(Position(fromPosition.x + 1, fromPosition.y, fromPosition.z))'],
    lambda c, a: (len(c) == 1 and c[0].get('status') == 'blocked' and 'target' not in c[0], c), callback='onStepIn')
converter_case(
    'a field access on fromPosition (not the bare variable) stays blocked',
    ['creature:teleportTo(fromPosition.x)'],
    lambda c, a: (len(c) == 1 and c[0].get('status') == 'blocked', c), callback='onStepIn')

# Round 2, Finding 2 (P2): a revert attaches only when its own receiver or literal position provably
# names the same target as the candidate operation; never by list order alone.
converter_case(
    'a revert on a different receiver does not attach to an unrelated preceding operation',
    ['wall1:transform(2773)', 'wall2:decay()'],
    lambda c, a: (len([x for x in c if x.get('operation') == 'TRANSFORM']) == 1
                 and len([x for x in c if x.get('status') == 'blocked']) == 1, c))
converter_case(
    'the exact Codex example: addEvent(Position.revertItem, ...) with no provable same target stays blocked',
    ['item:transform(2773)', 'addEvent(Position.revertItem, 5000, item:getPosition(), 2772)'],
    lambda c, a: (len([x for x in c if x.get('operation') == 'TRANSFORM' and 'revert_after_ms' in x]) == 0
                 and len([x for x in c if x.get('status') == 'blocked']) == 1, c))
converter_case(
    'addEvent(Position.revertItem, ...) with a literal position matching the prior anchor attaches',
    ['Game.createItem(2793, Position(100, 200, 7))',
     'addEvent(Position.revertItem, 5000, Position(100, 200, 7), 2772)'],
    lambda c, a: (len(c) == 1 and c[0]['operation'] == 'CREATE' and c[0].get('revert_after_ms') == 5000, c))
converter_case(
    'addEvent(Position.revertItem, ...) with a different literal position does not attach',
    ['Game.createItem(2793, Position(100, 200, 7))',
     'addEvent(Position.revertItem, 5000, Position(1, 1, 7), 2772)'],
    lambda c, a: (len([x for x in c if x.get('operation') == 'CREATE' and 'revert_after_ms' in x]) == 0
                 and len([x for x in c if x.get('status') == 'blocked']) == 1, c))

# Round 2, Finding 3 (P2): `def` only when the first argument is a complete literal integer.
converter_case(
    'the exact Codex example: createItem(2793 + offset, ...) keeps its source line, no def',
    ['Game.createItem(2793 + offset)'],
    lambda c, a: (len(c) == 1 and 'def' not in c[0] and c[0].get('value_source_line') is not None, c))

# Round 2, Finding 4 (P2): CREATE decides its placement by argument structure (the engine signature
# `createItem(itemId, count/subtype, position)`), never a substring/name heuristic.
converter_case(
    'the exact Codex example: createItem(id, count, destination) stays blocked, not typed without an anchor',
    ['Game.createItem(2793, 1, destination)'],
    lambda c, a: (len(c) == 1 and c[0].get('status') == 'blocked' and 'operation' not in c[0], c))
converter_case(
    'createItem(id, count, literal position) binds the anchor, count is not mistaken for a placement',
    ['Game.createItem(2793, 1, Position(1, 2, 7))'],
    lambda c, a: (len(c) == 1 and c[0].get('operation') == 'CREATE' and c[0].get('anchor') == 'p1', c))
converter_case(
    'createItem(id, count) with no placement argument at all is the implicit target',
    ['Game.createItem(2793, 1)'],
    lambda c, a: (len(c) == 1 and c[0].get('operation') == 'CREATE' and 'anchor' not in c[0], c))
converter_case(
    'a literal position on createItem becomes a bound anchor',
    ['Game.createItem(2793, Position(100, 200, 7))'],
    lambda c, a: (len(c) == 1 and c[0] == {'owner': 'WorldObject', 'operation': 'CREATE', 'anchor': 'p1',
                                           'def': {'family': 'Item', 'key': 'canary:item/2793',
                                                   'revision': oi.REVISION}}
                 and a == [{'key': 'p1', 'source_position': {'x': 100, 'y': 200, 'z': 7}}], (c, a)))
converter_case(
    'a computed position on createItem stays blocked, never an invented anchor',
    ['Game.createItem(2793, toPosition)'],
    lambda c, a: (len(c) == 1 and c[0].get('status') == 'blocked' and 'anchor' not in c[0] and not a, c))
converter_case(
    'createItem with no position is unchanged (implicitly the interaction target)',
    ['Game.createItem(2793)'],
    lambda c, a: (len(c) == 1 and c[0].get('operation') == 'CREATE' and 'anchor' not in c[0], c))

# a reward-container constructor (`self.created_items`) is never also a WorldObject CREATE (round 1,
# Finding 3), re-verified with the internal `_identity` bookkeeping now in play.
converter_case(
    'a bare reward constructor produces no WorldObject child by itself',
    ['local reward = Game.createItem(2793)'],
    lambda c, a: (c == [], c))
converter_case(
    'a reward constructor filled into a hand-out container stays Item, not WorldObject',
    ['local backpack = player:addItem(2000, 1)', 'local reward = Game.createItem(2793)', 'backpack:addItemEx(reward)'],
    lambda c, a: (len(c) == 1 and c[0]['owner'] == 'Item' and c[0]['request'] == 'hand_out'
                 and c[0].get('contents') == [{'item': {'family': 'Item', 'key': 'canary:item/2793',
                                                        'revision': oi.REVISION}, 'count': 1}], c))
converter_case(
    'createItem with a literal position is still a world CREATE even though it is assigned to a local',
    ['local wall = Game.createItem(2793, Position(1, 2, 7))'],
    lambda c, a: (len(c) == 1 and c[0].get('operation') == 'CREATE' and c[0].get('anchor') == 'p1'
                 and '_identity' not in c[0], c))

# Round 3, Finding 1 (P2, affects committed D37 data): teleportTo's own target argument -- not the
# whole statement -- must fully match a literal Position(x,y,z) to bind an anchor.
converter_case(
    'the exact Codex example: teleportTo(toPosition or Position(1,2,7)) stays blocked, not typed as (1,2,7)',
    ['creature:teleportTo(toPosition or Position(1, 2, 7))'],
    lambda c, a: (len(c) == 1 and c[0].get('status') == 'blocked' and 'target' not in c[0], c), callback='onStepIn')
converter_case(
    'a bare teleportTo(Position(x,y,z)) is still a relocation to a named anchor',
    ['creature:teleportTo(Position(100, 200, 7))'],
    lambda c, a: (c == [{'owner': 'Movement', 'request': 'relocate', 'scope': 'in_scope',
                         'target': {'kind': 'anchor', 'anchor': 'p1'}}]
                 and a == [{'key': 'p1', 'source_position': {'x': 100, 'y': 200, 'z': 7}}], (c, a)), callback='onStepIn')
converter_case(
    'a trailing argument after a literal position does not disqualify the anchor',
    ['creature:teleportTo(Position(100, 200, 7), true)'],
    lambda c, a: (c[0].get('target', {}).get('kind') == 'anchor', c), callback='onStepIn')

# Round 3, Finding 2 (P2): revert association searches every preceding candidate for exactly one match,
# not only the immediately preceding operation; more than one equally plausible candidate stays blocked.
converter_case(
    'the exact Codex example: wall1:decay() attaches to wall1, not the nearer wall2',
    ['wall1:transform(2773)', 'wall2:transform(2773)', 'wall1:decay()'],
    lambda c, a: (len([x for x in c if x.get('operation') == 'TRANSFORM']) == 2
                 and len([x for x in c if x.get('status') == 'blocked']) == 0, c))
converter_case(
    'two equally plausible candidates for the same receiver stay blocked, not the nearest one',
    ['wall1:transform(2773)', 'wall1:transform(2774)', 'wall1:decay()'],
    lambda c, a: (len([x for x in c if x.get('operation') == 'TRANSFORM']) == 2
                 and len([x for x in c if x.get('status') == 'blocked']) == 1, c))

# Round 3, Finding 3 (P2): a revert's own position argument (split from the rest, per its call's own
# convention) must itself fully match a literal position; a look-alike buried in a larger expression
# elsewhere in the argument list must not match.
converter_case(
    "the exact Codex example: toPosition + Position(1,2,7) is not itself a literal position argument",
    ['item:transform(2773)', 'addEvent(Position.revertItem, 5000, toPosition + Position(1, 2, 7), 2772)'],
    lambda c, a: (len([x for x in c if x.get('operation') == 'TRANSFORM' and 'revert_after_ms' in x]) == 0
                 and len([x for x in c if x.get('status') == 'blocked']) == 1, c))

failed = [r for r in results if not r['passed']]
if '--verbose' in sys.argv:
    for r in results:
        print(f"{r['name']:<48} {r['first_error']}")
print(json.dumps({'cases': len(results), 'passed': len(results) - len(failed), 'failed': failed}, indent=2))
sys.exit(1 if failed else 0)
