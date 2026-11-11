"""Carry already admitted quest offering and chest definitions into the native Item provider."""
import copy
import difflib
import hashlib
import json

from native_profile_composition import encode_native_profile
import sys
import xml.etree.ElementTree as ET

import timed_item_profiles as timed
import writable_item_profiles as writing

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
    from quest_chest_bindings import panpipe_key_claim
    sys.path.insert(0, str(root / 'tools/content-schema/reward-claim-authoring'))
    import container_native

    callbacks = json.loads((root / 'content/quests/world-callbacks.json').read_bytes())
    keys = {offering['item'] for callback in callbacks['callbacks']
            for offering in callback['offerings']}
    authored_keys = set()
    authored_callbacks = []
    # Executable authored callbacks use the same exact canonical/native Item
    # qualification as source chests. Include every output, never a partial bag.
    for relative in ('authored-world-callbacks.json', 'chosen-reward-callbacks.json'):
        path = root / 'content/quests' / relative
        if not path.is_file():
            continue
        for callback in json.loads(path.read_bytes())['callbacks']:
            authored_callbacks.append(callback)
            reward = callback.get('give') or callback.get('effects', {}).get('item_reward')
            if reward:
                authored_keys.add(reward['item'])
                authored_keys.update(child['item'] for field in ('contents', 'backpack_outputs')
                                     for child in reward.get(field, []))
                keys.add(reward['item'])
                keys.update(child['item'] for field in ('contents', 'backpack_outputs')
                            for child in reward.get(field, []))
            carried = callback.get('effects', {}).get('carried_inputs', [])
            authored_keys.update(item['item'] for item in carried)
            keys.update(item['item'] for item in carried)
            for field in ('take', 'present', 'offerings'):
                keys.update(item['item'] for item in callback.get(field, []))
                authored_keys.update(item['item'] for item in callback.get(field, []))
            tool = callback.get('item_on_target')
            if tool:
                authored_keys.add(tool['item'])
                keys.add(tool['item'])
                if tool.get('replacement'):
                    authored_keys.add(tool['replacement'])
                    keys.add(tool['replacement'])
    # Rotten Blood's fixed Rust callback is not part of the JSON inventory.
    # Crystal 201189f actions_sacrifice.lua:4-29 requires this canonical Item;
    # apply the same pinned source, exact binding and portability checks below.
    keys.add('oteryn:item.tibia.i32594')
    # M07's Rust-authored Lion trials need the same native physical Item owner.
    import lion_trial_item_profiles
    keys.update(f'oteryn:item.tibia.i{i}' for i in lion_trial_item_profiles.SOURCE_RECORDS)
    # Crystal 201189f movements_lions_rock.lua:54-108: the native STEP
    # amphora grants this canonical scroll; it is authored in Rust, not JSON.
    keys.add('oteryn:item.tibia.i21467')
    # Crystal201189f chest.lua:508-513: Fanfare's key3520 uses the already
    # admitted bone key2973. Its key-binding hold is separate from Item admission.
    keys.add('oteryn:item.tibia.i2973')
    # Restored startup chest selectors must carry their exact ready reward into
    # the native provider too. Otherwise a correctly bound chest has nothing to
    # mint (notably Nightmare Teddy). This is the selected cohort, not a general
    # promotion of held source definitions.
    selected_chests = {
        row['claim']
        for path in sorted((root / 'tools/content-schema/world-authoring/samples/quest-chest-bindings').glob('*.json'))
        for row in json.loads(path.read_bytes())['bindings']
    }
    keys.update(extra_keys)
    for callback in callbacks['callbacks']:
        item = callback.get('item_on_target')
        if item:
            keys.add(item['item'])
            if item.get('replacement'):
                keys.add(item['replacement'])
    # These source chests also advance a quest track. Their Item definitions
    # already have canonical admission; retain the same proof in the native provider.
    for path in sorted((root / 'content/interactions/reward_claims').glob('reward-claim*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            if definition['identity']['key'] in selected_chests:
                # Filled-container/text claims already enter through the exact
                # W15b native projection below; selection adds only map binding.
                if definition.get('readiness') == 'ready' or panpipe_key_claim(definition):
                    keys.update(item['item']['key'] for placement in definition['placements']
                                for item in placement['reward']['items'])
                elif (definition.get('readiness') != 'waiting_implementation'
                      or definition.get('data_holds') != []
                      or definition.get('native_contents', {}).get('schema')
                      != 'OTERYN_NATIVE_CONTAINER_REWARD/v1'):
                    raise ValueError('SELECTED_QUEST_CHEST_REWARD_NOT_READY')
                selected_chests.remove(definition['identity']['key'])
            # QUEST-FINISH owner scope: retain the donor record as evidence,
            # but do not admit Dawnport's rewards into the Newhaven start path.
            if definition.get('quest', {}).get('key') == 'canary:quest/dawnport_quest':
                continue
            demon_item = DEMON_HELMET_REWARDS.get(definition['identity']['key'])
            if demon_item is not None:
                placements = definition['placements']
                expected = [{'count': 1, 'item': {'family': 'Item', 'key': demon_item,
                                                'revision': 'definition-r1'}}]
                if (definition.get('readiness') != 'ready' or len(placements) != 1
                        or placements[0]['reward']['items'] != expected):
                    raise ValueError('DEMON_HELMET_QUALIFIED_CLAIM_CHANGED')
                keys.add(demon_item)
            # Crystal201189f chest.lua:556: UID5024 grants the already-canonical
            # silver key3700. KEY owns its native numbered instance; the claim's
            # historical waiting_implementation flag is not an Item hold.
            # Crystal 201189f chest.lua: UID5018 is the Fanfare access key.
            # It grants a numbered key but never a canonical quest transition.
            if definition['identity']['key'] == 'oteryn:reward-claim.quest.key.id3520':
                placements = definition['placements']
                key = 'oteryn:item.tibia.i2973'
                expected = {'items': [{'count': 1, 'item': {
                    'family': 'Item', 'key': key, 'revision': 'definition-r1'}}],
                    'key_binding': 'canary:door-key/3520'}
                if (len(placements) != 1 or placements[0]['reward'] != expected
                        or placements[0].get('quest_transition') is not None
                        or placements[0]['source_binding']['project_position'] !=
                            {'x': 32376, 'y': 31802, 'z': 7}
                        or {'server': 'crystalserver', 'unique_id': 5018} not in
                            placements[0]['source_binding']['legacy_unique_ids']):
                    raise ValueError('FANFARE_ACCESS_KEY_CLAIM_CHANGED')
                keys.add(key)
            if definition['identity']['key'] == 'oteryn:reward-claim.quest.key.id3700':
                placements = definition['placements']
                key = 'oteryn:item.tibia.i2971'
                expected = {'items': [{'count': 1, 'item': {
                    'family': 'Item', 'key': key, 'revision': 'definition-r1'}}],
                    'key_binding': 'canary:door-key/3700'}
                if len(placements) != 1 or placements[0]['reward'] != expected:
                    raise ValueError('PITS_OF_INFERNO_KEY_CLAIM_CHANGED')
                keys.add(key)
            # W15b's complete recipe already qualified the canonical root and
            # every child. Carry that whole closure into the native provider;
            # admitting only the visible chest reward would mint a partial bag.
            for placement in definition.get('native_contents', {}).get('placements', []):
                keys.add(placement['root']['key'])
                keys.update(child['item']['key'] for child in placement['contents'])
            if definition.get('readiness') == 'ready':
                for placement in definition['placements']:
                    # A ready source reward still needs the same exact native
                    # Item admission when it has no quest-state side effect.
                    keys.update(item['item']['key'] for item in placement['reward']['items'])
    # Falcon repeat owner freezes one of these exact seven source rewards.
    keys.update(f'oteryn:item.tibia.i{number}' for number in (9081, 28821, 28823, 9058, 6299, 3052, 3035))
    if selected_chests:
        raise ValueError('SELECTED_QUEST_CHEST_MISSING')
    grizzly_path = root / 'content/quests/killing-in-the-name-of.json'
    if grizzly_path.is_file():
        grizzly = json.loads(grizzly_path.read_bytes())
        # The fixed Holy Icon entitlement uses its existing canonical identity
        # and admission. Shop Items with unqualified facts are a separate input.
        keys.update(f"oteryn:item.tibia.i{task['item']['source_id']}"
                    for task in grizzly['tasks'] if task.get('item'))
    return keys, authored_keys, authored_callbacks


def admit(root, generated, profile_path, manifest_path, order, canonical, extra_keys=()):
    keys, authored_keys, authored_callbacks = _requested_inputs(root, extra_keys)
    import container_native
    import lion_trial_item_profiles
    document = json.loads(generated[profile_path])
    admit_map_carriers(root, document)
    present = {row['production_definition']['production_key']: row for row in document['records']}
    if len(present) != len(document['records']):
        raise ValueError('QUEST_CALLBACK_DUPLICATE_NATIVE_ITEM')
    definitions = {}
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            key = definition['identity']['key']
            if key in keys:
                if key in definitions:
                    raise ValueError('QUEST_CALLBACK_DUPLICATE_ITEM')
                definitions[key] = definition
    if set(definitions) != keys:
        raise ValueError('QUEST_CALLBACK_ITEM_MISSING')
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    import authored_static_item_constructors
    authored = json.loads(generated[authored_static_item_constructors.ADMISSIONS])
    bindings.extend(row['binding'] for row in authored['records'])
    raw = (root / SOURCE).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError('QUEST_CALLBACK_SOURCE_DIGEST')
    source_items = ET.fromstring(raw).findall('item')
    lion_keys = {f'oteryn:item.tibia.i{item}' for item in lion_trial_item_profiles.SOURCE_RECORDS}
    top_order_keys = {key for key in lion_keys if key in present
                      and isinstance(present[key].get('attributes'), dict)
                      and any(field in present[key]['attributes']
                              for field in ('always_on_top', 'always_on_top_order'))}
    lion_records = lion_trial_item_profiles.records(
        root, raw, definitions, top_order_keys=top_order_keys)
    # QuickLoot may already have attached canonical navigation metadata. Keep
    # that optional owner in the expected record without borrowing its value
    # from the native row or changing the shape of newly admitted Lion Items.
    taxonomy_keys = {key for key in lion_records if key in present
                     and isinstance(present[key].get('authoring'), dict)
                     and 'taxonomy' in present[key]['authoring']}
    lion_taxonomy = {}
    if taxonomy_keys:
        taxonomy = json.loads((root / 'content/items/taxonomy/items.json').read_bytes())
        for row in taxonomy['records']:
            key = row['target']['key']
            if key in taxonomy_keys:
                if key in lion_taxonomy or row['target'] != definitions[key]['identity']:
                    raise ValueError('LION_TRIAL_ITEM_TAXONOMY:' + key)
                lion_taxonomy[key] = row['source_taxonomy']
        if lion_taxonomy.keys() != taxonomy_keys:
            raise ValueError('LION_TRIAL_ITEM_TAXONOMY:'
                             + ','.join(sorted(taxonomy_keys - lion_taxonomy.keys())))
    import quests_2_emperor_item_profiles
    source_packet = json.loads((root / quests_2_emperor_item_profiles.PATH).read_bytes())
    import authored_quest_item_custody
    missing_authored = [definitions[key] for key in sorted(authored_keys - present.keys())
                        if key in definitions]
    custody_inputs = (authored_quest_item_custody.qualification_inputs(root, missing_authored, raw)
                      if missing_authored else None)
    for key, definition in sorted(definitions.items()):
        if key in lion_records:
            record = lion_records[key]
            if key in present:
                if key in lion_taxonomy:
                    record['authoring']['taxonomy'] = copy.deepcopy(lion_taxonomy[key])
                if present[key] != record:
                    before = (json.dumps(present[key], ensure_ascii=False,
                                         sort_keys=True, indent=2) + '\n').splitlines(keepends=True)
                    after = (json.dumps(record, ensure_ascii=False,
                                        sort_keys=True, indent=2) + '\n').splitlines(keepends=True)
                    difference = ''.join(difflib.unified_diff(
                        before, after, fromfile='existing/' + key, tofile='qualified/' + key,
                        n=max(len(before), len(after))))
                    raise ValueError('LION_TRIAL_NATIVE_PROFILE_DRIFT:' + key + '\n' + difference)
            else:
                document['records'].append(record)
            continue
        # These exact source-qualified Items retain their canonical identity;
        # this is not a general bypass for an unqualified native record.
        definition = quests_2_emperor_item_profiles.callback_definition(
            source_packet, present.get(key), definition, root)
        import questpc8_nail_custody
        definition = questpc8_nail_custody.callback_definition(root, present.get(key), definition)
        if key in {f'oteryn:item.tibia.i{item}' for item in container_native.CHARGED_QUEST_CLAIMS.values()}:
            qualified = container_native.native_charge_item(definition, present.get(key))
            if qualified is definition:
                raise ValueError(f'QUEST_CALLBACK_ITEM_NOT_ADMITTED: {key}')
            definition = qualified
        if key == 'oteryn:item.tibia.i2874':
            # Only an agreeing, already-admitted native empty-vial profile closes this hold.
            definition = container_native.native_water_vial(definition, present.get(key))
        identity = definition['identity']
        if key in authored_keys and key not in present:
            import authored_quest_timed_custody
            exact = [binding for binding in bindings if binding['target'] == identity]
            if len(exact) != 1 or exact[0]['disposition'] != 'EXACT':
                raise ValueError('AUTHORED_TIMED_EXACT_BINDING')
            expiry = authored_quest_timed_custody.source_expiry(raw, int(exact[0]['external_id']))
            if expiry is not None:
                profile, _ = authored_quest_timed_custody.qualify(root, definition, raw, authored_callbacks, _inputs=custody_inputs)
                document['records'].append(profile)
                continue
        if key in authored_keys and not definition['materializable'] and definition['stack_class'] == 'Unknown':
            import authored_quest_item_custody
            if key in present:
                authored_quest_item_custody.reuse_existing(definition, present[key], bindings)
                continue
            profile = authored_quest_item_custody.qualify(root, definition, raw, _inputs=custody_inputs)
            document['records'].append(profile)
            continue
        matches = [binding for binding in bindings if binding['target'] == identity]
        if (not definition['materializable']
                or definition['stack_class'] not in ('StackCapable', 'NonStackable')
                or len(matches) != 1 or matches[0]['disposition'] != 'EXACT'):
            raise ValueError(f'QUEST_CALLBACK_ITEM_NOT_ADMITTED: {key}')
        physical = definition['semantics']['physical']
        if (physical['state'] != 'KNOWN'
                or any(physical['value'][field] != {'state': 'KNOWN', 'value': True}
                       for field in ('movable', 'pickupable'))):
            raise ValueError('QUEST_CALLBACK_ITEM_NOT_PORTABLE')
        if key == 'oteryn:item.tibia.i2992' and key not in present:
            # Goblin Temple reuses the seasonal owner's existing Snowball.
            # Preserve its complete equipment/weapon facts before this pass.
            import seasonal_item_profiles as seasonal
            packet = json.loads((root / seasonal.PATH).read_bytes())
            if (packet['schema'] != 'OTERYN_SEASONAL_ITEM_PROFILES/v1'
                    or packet['crystal_commit'] != seasonal.PIN
                    or len(packet['records']) != len(seasonal.IDS)
                    or {r['production_definition']['production_key'] for r in packet['records']}
                        != {f'oteryn:item.tibia.i{i}' for i in seasonal.IDS}):
                raise ValueError('GOBLIN_SNOWBALL_SOURCE_DRIFT')
            candidates = [r for r in packet['records'] if r['production_definition']['production_key'] == key]
            if len(candidates) != 1:
                raise ValueError('GOBLIN_SNOWBALL_SOURCE_ITEM')
            present[key] = copy.deepcopy(candidates[0])
            document['records'].append(present[key])
        if key == 'oteryn:item.tibia.i2955' and key not in present:
            # Fanfare already belongs to the retained House provider. Seed its
            # exact source profile before this earlier callback pass can create
            # a poorer canonical-only record that would shadow that owner.
            import house_decoration_actions as house
            packet = json.loads((root / house.PROFILES).read_bytes())
            if (packet['schema'] != 'OTERYN_HOUSE_DECORATION_PROFILES/v1'
                    or packet['crystal_commit'] != house.PIN
                    or packet['canary_commit'] != house.CANARY
                    or packet['catalogue_sha256'] != hashlib.sha256(
                        (root / house.CATALOGUE).read_bytes()).hexdigest()):
                raise ValueError('FANFARE_NATIVE_SOURCE_DRIFT')
            candidates = [row for row in packet['records']
                          if row['production_definition']['production_key'] == key]
            if len(candidates) != 1:
                raise ValueError('FANFARE_NATIVE_SOURCE_ITEM')
            present[key] = copy.deepcopy(candidates[0])
            document['records'].append(present[key])
        if key in present:
            complete_native_stack(root, present[key], definition, matches[0])
            continue
        item_id = int(matches[0]['external_id'])
        source_matches = [item for item in source_items
                          if int(item.get('id', item.get('fromid', '0'))) <= item_id
                          <= int(item.get('id', item.get('toid', '0')))]
        if len(source_matches) != 1:
            raise ValueError('QUEST_CALLBACK_SOURCE_ITEM_MISSING')
        speeds = [attr for attr in source_matches[0].findall('attribute')
                  if attr.get('key', '').lower() == 'speed']
        if len(speeds) > 1:
            raise ValueError('QUEST_CALLBACK_DUPLICATE_SPEED')
        # Canary's Abilities constructor defaults speed to zero when XML omits it.
        speed = int(speeds[0].attrib['value']) if speeds else 0
        document['records'].append({
            'authoring': {'item': identity},
            'semantics': grant_semantics(root, definition),
            'attributes': {'speed_bonus': speed},
            'admission': {'materializable': True, 'stack_class': definition['stack_class'],
                          'legal_destinations': ['CharacterInventory', 'Ground']},
            'production_definition': {'family': identity['family'], 'production_key': key,
                                      'revision_ref': identity['revision']},
            'production_binding': matches[0], 'production_binding_qualification': None,
        })
        complete_native_stack(root, document['records'][-1], definition, matches[0])
    document['records'].sort(key=order)
    raw = encode_native_profile(document)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
