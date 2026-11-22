"""Carry already admitted quest offering and chest definitions into the native Item provider."""
import copy
import difflib
import hashlib
import json
import sys
import xml.etree.ElementTree as ET

import timed_item_profiles as timed
import writable_item_profiles as writing
from native_profile_composition import encode_native_profile

SOURCE = 'imports/canary/items-xml/items.xml'
SOURCE_SHA256 = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'
# Crystal201189f:data-global/startup/tables/chest.lua:799-817. These exact
# ordinary one-Item claims already have canonical materialization, equipment and
# physical facts. They need no new Item owner or charged-instance policy.
DEMON_HELMET_REWARDS = {
    'oteryn:reward-claim.quest.u6_4.demon_helmet.rewards.steel_boots': 'oteryn:item.tibia.i3554',
    'oteryn:reward-claim.quest.u6_4.demon_helmet.rewards.demon_helmet': 'oteryn:item.tibia.i3387',
    'oteryn:reward-claim.quest.u6_4.demon_helmet.rewards.demon_shield': 'oteryn:item.tibia.i3420',
}
# These T..Z source chests carry already-admitted Item facts into the native
# provider. No quest progress or access puzzle is inferred here.
QUESTPC4_REWARD_QUESTS = {
    'canary:quest/the_ape_city_quest', 'canary:quest/the_explorer_society_quest',
    'canary:quest/the_hidden_city_of_beregar_quest',
    'canary:quest/the_inquisition_quest', 'canary:quest/the_pits_of_inferno_quest',
    'canary:quest/the_thieves_guild_quest', 'canary:quest/the_white_raven_monastery_quest',
    'canary:quest/threatened_dreams_quest', 'canary:quest/throwing_star_quest',
    'canary:quest/time_ring_quest', 'canary:quest/to_blind_the_enemy_quest',
    'canary:quest/to_outfox_a_fox_quest', 'canary:quest/tomes_of_knowledge_quest',
    'canary:quest/triangle_tower_quest', 'canary:quest/troll_cave_quest',
    'canary:quest/vampire_hunter_quest', 'canary:quest/vampire_shield_quest',
    'canary:quest/waterfall_quest', 'canary:quest/wedding_ring_quest',
    'canary:quest/what_a_foolish_quest',
}

CHEST_PROFILES = 'tools/content-schema/native-gameplay/quest-chest-item-profiles.json'
CHEST_IDS = {1777, 2436, 2438, 2473, 2474}


def admit_map_carriers(root, document):
    """Retain the exact source physics for restored map chest entries."""
    overlay = json.loads((root / CHEST_PROFILES).read_bytes())
    rows = overlay['records']
    keys = [r['production_definition']['production_key'] for r in rows]
    if (overlay['schema'] != 'OTERYN_NATIVE_QUEST_CHEST_ITEM_PROFILES/v1'
            or overlay['source_revision'] != '99902524e052f37574194466c2949c576e4ab269'
            or len(keys) != len(CHEST_IDS)
            or set(keys) != {f'oteryn:item.tibia.i{i}' for i in CHEST_IDS}):
        raise ValueError('QUEST_CHEST_ITEM_PROFILE_SET')
    existing = {row['production_definition']['production_key']: row
                for row in document['records']}
    for row in rows:
        key = row['production_definition']['production_key']
        if key in existing:
            if existing[key] != row:
                raise ValueError('QUEST_CHEST_ITEM_PROFILE_CONFLICT')
        else:
            document['records'].append(row)


def complete_native_stack(root, record, definition, binding):
    """Fill only unqualified stack facts; established native facts keep authority."""
    identity = definition['identity']
    expected = {'family': identity['family'], 'production_key': identity['key'],
                'revision_ref': identity['revision']}
    if record['production_definition'] != expected or record['production_binding'] != binding:
        raise ValueError('QUEST_CALLBACK_NATIVE_IDENTITY')
    admission = record['admission']
    if (not admission['materializable']
            or 'CharacterInventory' not in admission['legal_destinations']
            or admission['stack_class'] != definition['stack_class']):
        raise ValueError('QUEST_CALLBACK_NATIVE_ITEM_NOT_ADMITTED')
    semantics = record['semantics']
    physical = semantics['physical']
    weight = physical.get('value', {}).get('weight', {})
    if (physical.get('state') != 'KNOWN' or weight.get('state') != 'KNOWN'
            or type(weight.get('value')) is not int or not 0 <= weight['value'] <= 0xffffffff
            or any(physical['value'].get(field) != {'state': 'KNOWN', 'value': True}
                   for field in ('movable', 'pickupable'))):
        raise ValueError('QUEST_CALLBACK_NATIVE_PHYSICAL_INCOMPLETE')
    if admission['stack_class'] == 'StackCapable':
        observed, accepted = semantics['stack'], definition['semantics']['stack']
        if observed.get('state') == 'UNKNOWN' and accepted.get('state') == 'KNOWN':
            semantics['stack'] = copy.deepcopy(accepted)
        elif observed.get('state') == accepted.get('state') == 'KNOWN':
            for field in ('stackable', 'stack_max'):
                if (observed['value'][field].get('state') == 'UNKNOWN'
                        and accepted['value'][field].get('state') == 'KNOWN'):
                    observed['value'][field] = copy.deepcopy(accepted['value'][field])
        stack = semantics['stack']
        maximum = stack.get('value', {}).get('stack_max', {})
        if (stack.get('state') != 'KNOWN'
                or stack['value'].get('stackable') != {'state': 'KNOWN', 'value': True}
                or maximum.get('state') != 'KNOWN' or type(maximum.get('value')) is not int
                or not 1 <= maximum['value'] <= 100):
            raise ValueError('QUEST_CALLBACK_NATIVE_STACK_INCOMPLETE')
    expected_container = definition['semantics']['container']
    if expected_container.get('state') == 'KNOWN':
        expected_capacity = expected_container['value']['capacity']
        if expected_capacity.get('state') == 'KNOWN':
            container = semantics['container']
            if (container.get('state') != 'KNOWN'
                    or container['value']['capacity'] != expected_capacity):
                raise ValueError('QUEST_CALLBACK_NATIVE_CONTAINER_INCOMPLETE')
    # The merged chest cohort owns these accepted rounded weights; retain the
    # raw Trader capture, and reconcile only its exact reviewed leaf pairs.
    from hireling_item_profiles import MERGED_MAIN_NATIVE_WEIGHTS
    decision = MERGED_MAIN_NATIVE_WEIGHTS.get(identity['key'])
    if decision is not None:
        accepted, donor, _ = decision
        canonical_weight = definition['semantics']['physical']['value']['weight']
        observed_weight = semantics['physical']['value']['weight']
        if (canonical_weight != {'state': 'KNOWN', 'value': accepted}
                or observed_weight not in ({'state': 'KNOWN', 'value': accepted},
                                           {'state': 'KNOWN', 'value': donor})):
            raise ValueError('QUEST_CALLBACK_SHARED_WEIGHT_DRIFT')
        semantics['physical']['value']['weight'] = copy.deepcopy(canonical_weight)
    # Other native cohorts may already carry this exact chest child. Retain
    # their known facts, but do not lose the canonical food/potion, protection
    # or trade policy merely because the physical Item was admitted first.
    for group, fact in grant_semantics(root, definition).items():
        # I05/TIMED owns these ability groups, including deliberately UNKNOWN
        # inactive forms. Filling them from canonical chest facts would add the
        # inactive bonus to the active form's bonus when the ring is worn.
        if group == 'skill_modifiers' and identity['key'] in {
                f'oteryn:item.tibia.i{number}' for number in timed.IDS}:
            continue
        if (fact.get('state') == 'KNOWN'
                and semantics.get(group, {'state': 'UNKNOWN'}).get('state')
                in ('UNKNOWN', 'NOT_APPLICABLE')):
            semantics[group] = copy.deepcopy(fact)


def grant_semantics(root, definition):
    """Carry source text facts without inventing an unqualified player-write policy."""
    semantics = copy.deepcopy(definition['semantics'])
    # MP01: the native owner completes the equip level and vocations of 91 weapons.
    import weapon_equip_requirements
    semantics = weapon_equip_requirements.complete_semantics(root, definition['identity']['key'], semantics)
    # Canary04b83b5:data/items/items.xml:16088-16091. The admitted poem
    # scroll has an unknown canonical weight, but this exact donor explicitly
    # supplies 120 centi-oz. Preserve the canonical record and known native facts.
    if (definition['identity']['key'] == 'oteryn:item.tibia.i5952'
            and semantics['physical'].get('state') == 'KNOWN'
            and semantics['physical']['value'].get('weight') == {'state': 'UNKNOWN'}):
        raw = (root / SOURCE).read_bytes()
        if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
            raise ValueError('QUEST_CALLBACK_SOURCE_DIGEST')
        witnesses = ET.fromstring(raw).findall("item[@id='5952']/attribute[@key='weight']")
        if len(witnesses) != 1 or witnesses[0].get('value') != '120':
            raise ValueError('QUEST_CALLBACK_POEM_SCROLL_WEIGHT_DRIFT')
        semantics['physical']['value']['weight'] = {'state': 'KNOWN', 'value': 120}
    text = semantics['readable_writeable']
    if (text.get('state') != 'KNOWN'
            or text['value']['writeable'] != writing.known(True)):
        return semantics
    maximum = text['value']['max_text_length']
    if (maximum.get('state') == 'KNOWN' and type(maximum.get('value')) is int
            and 1 <= maximum['value'] <= writing.MAX_CHARACTERS):
        return semantics
    # A source-writable sheet with no qualified limit is still a valid authored
    # quest-text carrier (PR2770 Q13). The existing WRITE owner classifies it as
    # READ_ONLY_OR_ZERO_LIMIT. Reuse that pinned qualification: retain the
    # canonical source facts and grant admission, but assert no player WRITE
    # authority and invent neither a text limit nor a write-once target.
    expected = writing.outputs(root)[writing.PATH]
    if (root / writing.PATH).read_bytes() != expected:
        raise ValueError('QUEST_CALLBACK_WRITABLE_PROFILE_DRIFT')
    profile = next((row for row in json.loads(expected)['records']
                    if row['item'] == definition['identity']), None)
    if (profile is None or profile['qualification'] != 'READ_ONLY_OR_ZERO_LIMIT'
            or profile['max_characters'] != 0 or profile['writable'] is not True
            or profile['readable'] is not True or profile['write_once_to'] is not None
            or maximum != {'state': 'UNKNOWN'}
            or text['value']['readable'] != writing.known(True)
            or text['value']['write_once_target'] != {'state': 'UNKNOWN'}):
        raise ValueError('QUEST_CALLBACK_WRITABLE_GRANT_UNQUALIFIED')
    writing.owner_replace(text['value'], 'writeable', {'state': 'UNKNOWN'},
                          [writing.known(True)])
    return semantics


def requested_keys(root, extra_keys=()):
    keys, _, _ = _requested_inputs(root, extra_keys)
    return keys


def _requested_inputs(root, extra_keys=()):
    # Retain both the selected startup source cohort and authored callback custody.
    sys.path.insert(0, str(root / 'tools/content-schema/world-authoring'))
                if not supported_claim(definition):
                    raise ValueError('SELECTED_QUEST_CHEST_REWARD_NOT_READY')
                # Charge, container and text recipes enter through their existing
                # native projections below, keeping the complete reward closure.
                if definition.get('readiness') == 'ready' or numbered_key_claim(definition) is not None:
