"""Physical quest custody with an exact recipe omission or a source-duration deadline.

These are explicit Quest counterparts, not native inventory decay. The
original XML and canonical food/temporal facts remain unchanged. The returned
native profile admits custody only: FOOD, transform and temporal use stay
unqualified until their native inventory owner is independently admitted.
"""
import copy
import hashlib
import json
from dataclasses import dataclass

import authored_quest_item_custody as custody
import grizzly_shop_profiles as grizzly
import quest_reward_item_semantics as reward


@dataclass(frozen=True)
class ExpiryContract:
    duration_seconds: int
    decay_to: int
    clock_track: str
    producer: str
    consumers: tuple[str, ...]
    refresh_producers: tuple[str, ...]


@dataclass(frozen=True)
class OmittedExpiryContract:
    duration_seconds: int
    decay_to: int
    quest: str
    producer: str
    consumers: tuple[str, ...]
    recipe_pointer: str
    adaptation: str
    mode: str = 'EXPLICIT_RECIPE_OMISSION'


OMISSION_QUEST = 'oteryn:quest.the_travelling_trader_quest'
OMISSION_ITEM = 'oteryn:item.tibia.i169'
OMISSION_PATH = 'content/quests/definitions/quests-00200-00299.json'
OMISSION_ADAPTATION = ('Remove weekday availability, cheese decay, vase fragility and the exact price dialogue; '
                       'keep all six delivery objects and Rashid recognition as one once-only route.')


def omitted_expiry_contract(root, callbacks, item_key, expiry):
    """The canonical selected adaptation, never a callback flag, authorizes omission."""
    if item_key != OMISSION_ITEM or not any(c.get('quest') == OMISSION_QUEST for c in callbacks):
        return None
    document = json.loads((root / OMISSION_PATH).read_bytes())
    definition = document['records'][45]['definition']
    authored = definition.get('oteryn_recipe') or {}
    recipe = authored.get('payload', {}).get('recipe', {})
    if (definition.get('identity', {}).get('key') != OMISSION_QUEST
            or authored.get('profile') != 'chosen_source_completion_v1'
            or (recipe.get('adaptations') or [None])[0] != OMISSION_ADAPTATION
            or expiry != (1800, 3120)
            or not any(item.get('item', {}).get('key') == item_key
                       for stage in recipe.get('stages', [])
                       for item in stage.get('target_items', []))):
        raise ValueError('AUTHORED_TIMED_CANONICAL_OMISSION')
    producers, consumers = [], []
    for callback in callbacks:
        give = callback.get('give') or callback.get('effects', {}).get('item_reward')
        nested = give and any(child.get('item') == item_key
                              for field in ('contents', 'backpack_outputs')
                              for child in give.get(field, []))
        inputs = [item for field in ('take', 'present', 'offerings')
                  for item in callback.get(field, []) if item.get('item') == item_key]
        tool = callback.get('item_on_target') or {}
        if nested or tool.get('item') == item_key:
            raise ValueError('AUTHORED_TIMED_OMISSION_UNBOUND_ITEM')
        if give and give.get('item') == item_key:
            if give.get('count') != 1 or give.get('contents') or give.get('backpack_outputs'):
                raise ValueError('AUTHORED_TIMED_OMISSION_UNBOUND_ITEM')
            producers.append(callback)
        if inputs:
            if inputs != [{'item': item_key, 'count': 1}] or callback.get('take') != inputs:
                raise ValueError('AUTHORED_TIMED_OMISSION_UNBOUND_ITEM')
            consumers.append(callback)
    if len(producers) != 1 or len(consumers) != 1:
        raise ValueError('AUTHORED_TIMED_OMISSION_CLOSED_ROUTE')
    producer, consumer = producers[0], consumers[0]
    if (producer.get('quest') != OMISSION_QUEST or consumer.get('quest') != OMISSION_QUEST
            or producer.get('npc') != 'oteryn:npc.miraia' or consumer.get('npc') != 'oteryn:npc.rashid'
            or producer.get('state_ops') or consumer.get('state_ops')):
        raise ValueError('AUTHORED_TIMED_OMISSION_CLOSED_ROUTE')
    advances = [storage for storage in producer.get('storage', [])
                if not storage.get('preserve') and not storage.get('read_only')
                and storage.get('set', 0) > storage.get('maximum', 0)]
    if len(advances) != 1 or not any(
            storage.get('track') == advances[0]['track']
            and storage.get('minimum') == advances[0]['set']
            and storage.get('maximum') == advances[0]['set']
            and storage.get('set', 0) > advances[0]['set']
            and not storage.get('preserve') and not storage.get('read_only')
            for storage in consumer.get('storage', [])):
        raise ValueError('AUTHORED_TIMED_OMISSION_CLOSED_ROUTE')
    return OmittedExpiryContract(expiry[0], expiry[1], OMISSION_QUEST, producer['key'],
                                 (consumer['key'],), OMISSION_PATH + '#/records/45/definition/'
                                 'oteryn_recipe/payload/recipe/adaptations/0', OMISSION_ADAPTATION)


def source_expiry(xml_raw, item_id):
    """Read unchanged pinned XML; partial or equipment timer shapes fail closed."""
    if hashlib.sha256(xml_raw).hexdigest() != custody.XML_SHA256:
        raise ValueError('AUTHORED_TIMED_XML_PIN')
    nodes = grizzly.source.xml_index(xml_raw)
    node = nodes.get(item_id)
    if node is None:
        raise ValueError('AUTHORED_TIMED_SOURCE_ITEM')
    attributes = grizzly.source.attrs(node)
    if not {'duration', 'decayto', 'stopduration'} & attributes.keys():
        return None
    if ({'charges', 'stopduration', 'transformequipto', 'transformdeequipto'} & attributes.keys()
            or not {'duration', 'decayto'} <= attributes.keys()):
        raise ValueError('AUTHORED_TIMED_UNSUPPORTED_OWNER')
    try:
        seconds, target = int(attributes['duration']), int(attributes['decayto'])
    except (TypeError, ValueError) as error:
        raise ValueError('AUTHORED_TIMED_SOURCE_DURATION') from error
    if not 1 <= seconds <= 366 * 24 * 60 * 60 or not 1 <= target <= 65535 or target not in nodes:
        raise ValueError('AUTHORED_TIMED_SOURCE_DURATION')
    return seconds, target


def expiry_contract(callbacks, item_key, expiry):
    """Require the same SQL-clock stamp and strict deadline for every hand-in."""
    if not expiry or len(callbacks) > 1024:
        raise ValueError('AUTHORED_TIMED_CALLBACK_BOUNDS')
    seconds, target = expiry
    producers = []
    consumers = []
    for callback in callbacks:
        give = callback.get('give') or callback.get('effects', {}).get('item_reward')
        if give and (give.get('item') == item_key
                     or any(child.get('item') == item_key for field in ('contents', 'backpack_outputs')
                            for child in give.get(field, []))):
            if (give.get('item') != item_key or give.get('count') != 1
                    or give.get('contents') or give.get('backpack_outputs')):
                raise ValueError('AUTHORED_TIMED_UNBOUND_OUTPUT')
            producers.append(callback)
        evidence = [item for field in ('take', 'present', 'offerings')
                    for item in callback.get(field, []) if item.get('item') == item_key]
        tool = callback.get('item_on_target')
        if tool and tool.get('item') == item_key:
            evidence.append(tool)
        if evidence:
            if len(evidence) != 1 or evidence[0].get('count') != 1:
                raise ValueError('AUTHORED_TIMED_UNBOUND_INPUT')
            consumers.append(callback)
    initial = [callback for callback in producers if any(
        not storage.get('preserve') and not storage.get('read_only')
        and storage.get('set', 0) > storage.get('maximum', 0)
        for storage in callback.get('storage', []))]
    if len(initial) != 1 or len(producers) > 2:
        raise ValueError('AUTHORED_TIMED_SOURCE_OCCURRENCE')
    producer = initial[0]
    refreshes = [callback for callback in producers if callback is not producer]
    handins = [callback for callback in consumers if callback not in refreshes]
    if not handins or producer in consumers:
        raise ValueError('AUTHORED_TIMED_SOURCE_OCCURRENCE')
    stamps = [operation for operation in producer.get('state_ops', [])
              if operation.get('effect') == {'kind': 'set_now'}]
    if len(stamps) != 1:
        raise ValueError('AUTHORED_TIMED_START_CLOCK')
    clock = stamps[0].get('track')
    if (not isinstance(clock, str) or not clock.startswith('oteryn:quest-progress/')
            or len(clock) > 128 or stamps[0].get('from') != {'kind': 'any'}
            or any(operation.get('track') == clock and operation != stamps[0]
                   for operation in producer.get('state_ops', []))):
        raise ValueError('AUTHORED_TIMED_START_CLOCK')
    advances = [storage for storage in producer.get('storage', [])
                if not storage.get('preserve') and not storage.get('read_only')
                and storage.get('set', 0) > storage.get('maximum', 0)]
    if len(advances) != 1:
        raise ValueError('AUTHORED_TIMED_ONCE_PRODUCER')
    phase = advances[0]
    for callback in refreshes:
        guard = {'track': clock, 'from': {'kind': 'elapsed_at_least', 'seconds': seconds},
                 'effect': {'kind': 'set_now'}}
        fee = callback.get('fee') or {}
        if (callback.get('quest') != producer.get('quest')
                or callback.get('take') != [{'item': item_key, 'count': 1}]
                or type(fee.get('gold')) is not int or not 1 <= fee['gold'] <= 1_000_000_000_000
                or type(fee.get('include_bank')) is not bool
                or callback.get('storage') != [{'track': phase['track'],
                    'minimum': phase['set'], 'maximum': phase['set'],
                    'set': phase['set'], 'preserve': True}]
                or callback.get('state_ops') != [guard]):
            raise ValueError('AUTHORED_TIMED_UNGUARDED_REFRESH')
    for callback in callbacks:
        if (any(storage.get('track') == clock for storage in callback.get('storage', []))
                or (callback is not producer and callback not in consumers
                    and callback not in refreshes
                    and any(operation.get('track') == clock
                            for operation in callback.get('state_ops', [])))):
            raise ValueError('AUTHORED_TIMED_FOREIGN_CLOCK_WRITER')
    for callback in handins:
        guard = {'track': clock, 'from': {'kind': 'elapsed_less_than', 'seconds': seconds},
                 'effect': {'kind': 'add', 'value': 0}}
        if (callback.get('quest') != producer.get('quest')
                or sum(operation == guard for operation in callback.get('state_ops', [])) != 1
                or any(operation.get('track') == clock and operation != guard
                       for operation in callback.get('state_ops', []))
                or not any(storage.get('track') == phase['track']
                           and storage.get('minimum') == phase['set']
                           and storage.get('maximum') == phase['set']
                           and storage.get('set', 0) > phase['set']
                           and not storage.get('preserve') and not storage.get('read_only')
                           for storage in callback.get('storage', []))):
            raise ValueError('AUTHORED_TIMED_UNGUARDED_HANDIN')
    return ExpiryContract(seconds, target, clock, producer['key'],
                          tuple(sorted(callback['key'] for callback in handins)),
                          tuple(sorted(callback['key'] for callback in refreshes)))


def qualify(root, definition, xml_raw, callbacks, *, _inputs=None):
    """Qualify custody only after the canonical omission or full deadline contract exists."""
    from authored_quest_item_custody import _source
    source = grizzly.source
    bindings = _source(_inputs, 'index', lambda: source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes()))
    matches = [(external, binding) for external, binding in bindings.items()
               if binding['target'] == definition['identity'] and binding['disposition'] == 'EXACT']
    if len(matches) != 1:
        raise ValueError('AUTHORED_TIMED_EXACT_BINDING')
    item_id = int(matches[0][0])
    expiry = source_expiry(xml_raw, item_id)
    contract = omitted_expiry_contract(root, callbacks, definition['identity']['key'], expiry)
    if contract is None:
        contract = expiry_contract(callbacks, definition['identity']['key'], expiry)
    holds = _source(_inputs, 'admissions', lambda: reward.load_admissions(root)).get(
        definition['identity']['key'], ({},))[0].get('holds', [])
    if holds:
        raise ValueError('AUTHORED_TIMED_ACCEPTED_HOLD:' + ','.join(sorted(holds)))
    client = _source(_inputs, 'client', lambda: (root / reward.CLIENT).read_bytes())
    if hashlib.sha256(client).hexdigest() != reward.CLIENT_SHA or len(client) != reward.CLIENT_BYTES:
        raise ValueError('AUTHORED_TIMED_CLIENT_PIN')
    appearance = _source(_inputs, 'objects', lambda: source.engine_items.load_appearance_objects(client)).get(item_id)
    if appearance is None or any(appearance['flags'].get('flags.' + name, False)
                                 for name in ('liquidcontainer', 'liquidpool')):
        raise ValueError('AUTHORED_TIMED_STATEFUL_APPEARANCE')
    policies = _source(_inputs, 'policies', lambda: {row['server_item_id']: row
        for row in source.policies(client, xml_raw, _inputs['ids'] if _inputs is not None else {item_id})})
    physical = policies.get(item_id)
    if physical is None:
        raise ValueError('AUTHORED_TIMED_PHYSICAL_SOURCE')
    nodes = _source(_inputs, 'nodes', lambda: source.xml_index(xml_raw))
    row = source.profile(item_id, nodes[item_id], appearance,
                         physical, bindings, None)
    if (not row['admission']['materializable']
            or row['admission']['legal_destinations'] != ['CharacterInventory', 'Ground']
            or row['admission']['stack_class'] != 'NonStackable'):
        raise ValueError('AUTHORED_TIMED_NOT_PORTABLE')
    for group in ('physical', 'stack', 'container'):
        if grizzly.known_conflict(definition['semantics'][group], row['semantics'][group]):
            raise ValueError('AUTHORED_TIMED_CANONICAL_' + group.upper())
    if definition['stack_class'] not in ('Unknown', row['admission']['stack_class']):
        raise ValueError('AUTHORED_TIMED_STACK_CONFLICT')
    for group in row['semantics']:
        row['semantics'][group] = (grizzly.accepted_first(definition['semantics'][group], row['semantics'][group])
                                  if group in ('physical', 'stack', 'container')
                                  else copy.deepcopy(definition['semantics'][group]))
    # Custody is not FOOD or inventory-decay readiness. Canonical facts remain
    # intact; UNKNOWN native use groups deliberately prevent that promotion.
    for group in ('consumption', 'temporal', 'use_transform'):
        row['semantics'][group] = {'state': 'UNKNOWN'}
    qualified = source.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': [row]},
                                     bindings, {item_id})['records'][0]
    return qualified, contract
