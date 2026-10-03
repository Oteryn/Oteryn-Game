"""One pinned SOURCE callback profile; not NPC runtime or dialogue admission."""
import hashlib
from ots_chests import ref, SOURCES

PROFILE = 'pemaret_marlin_v1'
PINS = {
    'canary': ('data-otservbr-global/npc/pemaret.lua', 'e9f9e9cc4a35ec8157aa1836ee382afb33c8dd4b0747ef7e44170044a0138c74', 55),
    'crystalserver': ('data-global/npc/pemaret.lua', '7860edb917cf36e43f2f675b9a2f9b8bbbc0821051ef20e4a9443cf04649489c', 53),
}
TARGET = 'Storage.Quest.U7_8.MarlinTrophy'
TRACK = 'canary:quest-progress/quest/u7_8/marlin_trophy'
HOLDS = ['conversation_and_topic_predicates_unadmitted', 'checkInteraction_unadmitted',
         'remove_return_and_handout_success_unknown', 'callback_other_branches_untranscribed']


def transcribe(server, path, raw, write):
    pin = PINS.get(server)
    if not pin or path != pin[0]:
        return None
    if hashlib.sha256(raw).hexdigest() != pin[1]:
        raise ValueError('NPC exchange source blob changed')
    if write['target'] != TARGET:
        return None
    start = pin[2]
    if (write['line'], write.get('to'), write['owner'], write['dialogue']) != (
            start + 19, 1, 'npc', {'keywords': ['yes'], 'topics': [1]}) or type(write.get('to')) is not int:
        raise ValueError('NPC exchange write/requester changed')
    lines = raw.splitlines()
    roles = {'callback': 0, 'actor': 1, 'interaction_guard': 4, 'offer_keyword': 8, 'offer_guard': 9,
             'offer_items': 10, 'offer_topic': 12, 'commit_guard': 15, 'consume': 16,
             'hand_out': 18, 'progress': 19, 'topic_reset': 23}
    evidence = [{'role': role, 'line': start + offset,
                 'line_sha256': hashlib.sha256(lines[start + offset - 1]).hexdigest()}
                for role, offset in roles.items()]
    # removeItem executes in the condition. Its result remains UNKNOWN: never
    # treat item-count eligibility as proof that either remove/add succeeded.
    item = lambda ident: ref('Item', f'{server}:item/{ident}')
    return {'profile': PROFILE, 'classification': 'OTS_HYPOTHESIS_ONLY',
            'callback': 'creatureSayCallback', 'callback_complete': False,
            'runtime_readiness': 'UNKNOWN', 'remaining_holds': HOLDS.copy(), 'evidence': evidence,
            'rules': [{'branch': [{'when': {'all': [
                {'unresolved': {'line': start + 4}}, {'unresolved': {'line': start + 15}},
                {'quest_stage': {'progress': TRACK, 'op': '<', 'value': 1}, 'negate': False}]},
                'then': [{'owner': 'Item', 'request': 'consume', 'item': item(901), 'count': 1},
                         {'branch': [{'when': {'unresolved': {'line': start + 16}}, 'then': [
                             {'owner': 'Item', 'request': 'hand_out', 'item': item(902), 'count': 1},
                             {'owner': 'Quest', 'request': 'set_progress', 'progress': TRACK, 'to': 1,
                              'value_source_line': start + 19}]}]}]}]}]}


def schema_update(schema, profiles):
    """Offline schema derivation from reviewed profiles; does not recheck donors."""
    if set(profiles) != set(PINS):
        raise ValueError('missing or extra NPC exchange provider profile')
    profiles = {s: profiles[s] for s in sorted(profiles)}
    defs = schema['$defs']
    for server, profile in profiles.items():
        defs['pemaret_' + server] = {'const': profile}
    defs['source_exchange'] = {'allOf': [
        {'type': 'object', 'properties': {'rules': {'type': 'array', 'items':
            {'$ref': 'oteryn:schema/interaction/v1#/$defs/rule'}}}},
        {'oneOf': [{'$ref': '#/$defs/pemaret_' + s} for s in profiles]}]}
    occurrence = defs['source_occurrence']
    occurrence['properties']['source_exchange'] = {'$ref': '#/$defs/source_exchange'}
    occurrence['allOf'] = [r for r in occurrence['allOf']
                          if r.get('if', {}).get('required') != ['source_exchange']]
    occurrence['allOf'] += [{'if': {'required': ['source_exchange'],
        'properties': {'source': {'const': s}}}, 'then': {'properties':
        {'source_exchange': {'$ref': '#/$defs/pemaret_' + s}}}} for s in profiles]
    for rule, server in zip(occurrence['allOf'][-len(profiles):], profiles):
        start = PINS[server][2]
        rule['then']['properties'].update({
            'target': {'const': TARGET}, 'path': {'const': PINS[server][0]},
            'revision': {'const': SOURCES[server]['revision']}, 'repository': {'const': SOURCES[server]['repository']},
            'line': {'const': start + 19}, 'occurrence': {'const': 1},
            'blob_sha256': {'const': PINS[server][1]},
            'line_sha256': {'const': next(e['line_sha256'] for e in profiles[server]['evidence'] if e['role'] == 'progress')},
            'write': {'const': {'key': 'npc_1', 'owner': 'npc', 'callback': None, 'from': None,
                'to': 1, 'servers': [server], 'requested_by': {
                'npc': ('canary' if server == 'canary' else 'crystal') + ':npc/pemaret',
                'keywords': ['yes'], 'topics': [1]}}}})
    present = {'contains': {'required': ['source_exchange']}}
    transition = defs['transition']
    primary = occurrence['allOf'][-len(profiles)]['then']['properties']['write']['const'].copy()
    primary['servers'] = sorted(PINS)
    transition['allOf'] = [{'if': {'properties': {'source_occurrences': present}},
        'then': {'properties': {'key': {'const': 'npc_1'}, 'script': {'const': 'npc/pemaret.lua'},
            'write': {'const': primary}, 'source_occurrences': {'minItems': 2, 'maxItems': 2,
                'items': {'required': ['source_exchange']}, 'allOf': [
                    {'contains': {'properties': {'source': {'const': s}}}} for s in PINS]}}}}]
    progress = defs['progress']
    existing = [r for r in progress['allOf'] if r.get('then', {}).get('properties', {}).get('key') != {'const': TRACK}]
    progress['allOf'] = existing + [{'if': {'properties': {'transitions': {'contains':
        {'properties': {'source_occurrences': present}}}}}, 'then': {'required': ['auxiliary_of'],
        'properties': {'key': {'const': TRACK}, 'auxiliary_of': {'const': ['canary:quest/marlin_trophy_quest']}}}}]
    return schema
