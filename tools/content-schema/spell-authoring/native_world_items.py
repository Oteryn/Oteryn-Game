"""Source-qualified data for Food and rune item operations (S27 B.4/D.2/D.6.1).

No source Lua is executed or embedded in a candidate. Each recognized complete
cast body must match its bounded source template, and its census blob must match
the supplied text. Source disagreements stay in evidence(), outside runtime data.
"""
import hashlib
from pathlib import Path
import re

ITEM_REVISION = 'spell-p2-r21'  # Qualified Item ImportBatch/provider identity; Spell/Effect stay r20.
PINS = {'canary': '99902524e052f37574194466c2949c576e4ab269',
        'crystal': 'ff7ede593c69d4c658b382c97443e8155926924a'}
REPOS = {'canary': 'opentibiabr/canary', 'crystal': 'zimbadev/crystalserver'}
PATHS = {'food': ('instant', 'data/scripts/spells/support/food.lua'),
         'chameleon rune': ('rune', 'data/scripts/runes/chameleon.lua'),
         'desintegrate rune': ('rune', 'data/scripts/runes/desintegrate_rune.lua'),
         'destroy field rune': ('rune', 'data/scripts/runes/destroy_field_rune.lua'),
         'magic wall rune': ('rune', 'data/scripts/runes/magic_wall.lua'),
         'wild growth rune': ('rune', 'data/scripts/runes/wild_growth.lua')}
ALIASES = {'disintegrate rune': 'desintegrate rune'}
# ItemID_t observations at the exact two pins, not allocated native identities.
ITEM_IDS = {'ITEM_MAGICWALL': 2128, 'ITEM_MAGICWALL_SAFE': 10181,
            'ITEM_WILDGROWTH': 2130, 'ITEM_WILDGROWTH_SAFE': 10182}
ITEM_ENUM_SHA256 = {'canary': 'b66a1389bfecfe11d93338208f7e2e3f8efc14fb0941d6d12dc0d9e62ed4abac',
                   'crystal': '8ca63a90b2d34da65ee94ca0e3134ed05bd021bd20ccfad0ed6210ed7ae116e9'}
CASTS = {
    'food': 'if math.random(0, 1) == 1 then creature:addItem(foods[math.random(#foods)]) end creature:addItem(foods[math.random(#foods)]) creature:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN) return true',
    'chameleon rune': 'local position, item = variant:getPosition() if position.x == CONTAINER_POSITION then local container = creature:getContainerById(position.y - 64) if container then item = container:getItem(position.z) else item = creature:getSlotItem(position.y) end else item = Tile(position):getTopDownItem() end if not item or item.itemid == 0 or not isMovable(item.uid) then creature:sendCancelMessage(RETURNVALUE_NOTPOSSIBLE) creature:getPosition():sendMagicEffect(CONST_ME_POFF) return false end condition:setOutfit({ lookTypeEx = item.itemid }) creature:addCondition(condition) creature:getPosition():sendMagicEffect(CONST_ME_MAGIC_RED) return true',
    'desintegrate rune': 'local position = variant:getPosition() local tile = Tile(position) if tile then local items = tile:getItems() if items then for i, item in ipairs(items) do if item:getType():isMovable() and item:getUniqueId() > 65535 and item:getActionId() == 0 and not table.contains(corpseIds, item:getId()) then item:remove() end if i == removalLimit then break end end end end creature:sendCancelMessage(RETURNVALUE_NOTPOSSIBLE) position:sendMagicEffect(CONST_ME_POFF) return true',
    'destroy field rune': 'local inPz = creature:getTile():hasFlag(TILESTATE_PROTECTIONZONE) if inPz then creature:sendCancelMessage(RETURNVALUE_NOTPOSSIBLE) creature:getPosition():sendMagicEffect(CONST_ME_POFF) return false end local position = Variant.getPosition(variant) local tile = Tile(position) local field = tile and tile:getItemByType(ITEM_TYPE_MAGICFIELD) if field and table.contains(fields, field:getId()) then field:remove() position:sendMagicEffect(CONST_ME_POFF) return true end creature:sendCancelMessage(RETURNVALUE_NOTPOSSIBLE) creature:getPosition():sendMagicEffect(CONST_ME_POFF) return false',
}


class NativeItemUnresolved(ValueError):
    """Recognized source no longer proves the complete admitted parameter shape."""


def _name(name):
    normalized = str(name).lower().strip()
    return ALIASES.get(normalized, normalized)


def _plain(text):
    return re.sub(r'\s+', ' ', re.sub(r'--[^\n]*', '', text)).strip()


def _body(text, function):
    found = re.findall(r'function\s+' + re.escape(function) + r'\s*\([^)]*\)(.*?)\nend\b', text, re.S)
    if len(found) != 1:
        raise NativeItemUnresolved(f'expected one complete {function} body')
    return _plain(found[0])


def _integer(text, pattern, label):
    found = re.findall(pattern, text)
    if len(found) != 1 or not str(found[0]).isdigit() or int(found[0]) <= 0:
        raise NativeItemUnresolved(f'{label} is not one positive literal')
    return int(found[0])


def _ids(text, variable):
    found = re.findall(r'local\s+' + variable + r'\s*=\s*\{([^}]*)\}', text, re.S)
    if len(found) != 1:
        raise NativeItemUnresolved(f'{variable}: expected one literal item pool')
    contents = re.sub(r'--[^\n]*', '', found[0])
    values = [v.strip() for v in contents.split(',') if v.strip()]
    if not values or any(not v.isdigit() or int(v) <= 0 for v in values):
        raise NativeItemUnresolved(f'{variable}: non-positive or symbolic item id')
    ids = list(map(int, values))
    if len(set(ids)) != len(ids):
        raise NativeItemUnresolved(f'{variable}: duplicate ids change selection semantics')
    return ids


def _ref(item):
    return {'family': 'Item', 'key': f'candidate:item/{item}', 'revision': ITEM_REVISION}


def _qualified(name, spell_type, records, texts):
    name = _name(name)
    if name not in PATHS or PATHS[name][0] != spell_type:
        return name, {}
    selected = {}
    for source, record in records.items():
        if source not in PINS:
            raise NativeItemUnresolved(f'unsupported item source {source}')
        path = record.get('file')
        if path != PATHS[name][1] or record.get('spell_type') != spell_type or _name(record.get('name')) != name:
            raise NativeItemUnresolved(f'{source}: item record identity/path mismatch')
        if record.get('revision', PINS[source]) != PINS[source]:
            raise NativeItemUnresolved(f'{source}: source revision mismatch')
        try:
            text = texts[source, path]
        except KeyError as exc:
            raise NativeItemUnresolved(f'{source}: full source text absent') from exc
        raw = text.encode('utf-8')
        blob = hashlib.sha1(f'blob {len(raw)}\0'.encode() + raw).hexdigest()
        if blob != record.get('blob'):
            raise NativeItemUnresolved(f'{source}: full source text does not match census blob')
        if name in ('magic wall rune', 'wild growth rune') and record.get('source_root') is not None:
            header = (Path(record['source_root']) / 'src/utils/utils_definitions.hpp').read_bytes()
            if hashlib.sha256(header).hexdigest() != ITEM_ENUM_SHA256[source]:
                raise NativeItemUnresolved(f'{source}: barrier ItemID_t header differs from its pin')
            for symbol, expected in ITEM_IDS.items():
                if not re.search(r'\b' + symbol + r'\s*=\s*' + str(expected) + r'\s*,', header.decode('utf-8')):
                    raise NativeItemUnresolved(f'{source}: missing pinned barrier item identity {symbol}')
        selected[source] = text
    if not selected:
        raise NativeItemUnresolved('no qualified source for recognized item behavior')
    return name, selected


def _parse(name, text):
    variable = 'spell' if name == 'food' else 'rune'
    if _body(text, variable + '.onCastSpell') != CASTS[name]:
        raise NativeItemUnresolved(f'{name}: cast behavior differs from the complete accepted template')
    if name == 'food':
        ids = _ids(text, 'foods')
        if len(ids) != 7:
            raise NativeItemUnresolved('Food: accepted seven-item pool required')
        return {'key': 'random_item_grant', 'parameters': {
            'pool': list(map(_ref, ids)), 'guaranteed': 1, 'extra': 1, 'extra_chance_percent': 50,
            'selection': 'uniform_independent', 'overflow': 'drop_on_caster_tile',
            'draw_order': 'extra_chance_then_optional_then_guaranteed',
            'effect_asset_binding': 'canary.appearance:effect/magic_green', 'always_succeeds': True}}
    if name == 'chameleon rune':
        if len(re.findall(r'Condition\(CONDITION_OUTFIT\)', text)) != 1:
            raise NativeItemUnresolved('Chameleon requires its outfit condition')
        duration = _integer(text, r'condition:setTicks\(\s*(\d+)\s*\)', 'Chameleon duration')
        return {'key': 'tile_item_operation', 'parameters': {
            'operation': 'mimic_item', 'source': ['tile_top_item', 'container_slot', 'equipment_slot'],
            'require_movable': True, 'reject_creature_target': True, 'duration_ms': duration,
            'success_effect_asset_binding': 'canary.appearance:effect/magic_red',
            'failure_effect_asset_binding': 'canary.appearance:effect/poff', 'failure_message': 'not_possible'}}
    if name == 'desintegrate rune':
        ids = _ids(text, 'corpseIds')
        limit = _integer(text, r'local\s+removalLimit\s*=\s*(\d+)\b', 'Disintegrate cap')
        return {'key': 'tile_item_operation', 'parameters': {
            'operation': 'disintegrate', 'source': ['tile_items'], 'require_movable': True,
            'max_items': limit, 'max_items_counts': 'visited', 'exclude_items': list(map(_ref, ids)),
            'exclude_script_tagged': True, 'exclude_action_tagged': True, 'aggressive': False,
            'allow_in_pz': True, 'empty_tile_succeeds': True, 'missing_tile_succeeds': True,
            'success_effect_asset_binding': 'canary.appearance:effect/poff', 'send_cancel_on_success': False}}
    return {'key': 'tile_item_operation', 'parameters': {
        'operation': 'remove_field', 'source': ['first_magic_field'], 'field_items': list(map(_ref, _ids(text, 'fields'))),
        'allow_in_pz': False, 'success_effect_asset_binding': 'canary.appearance:effect/poff',
        'failure_effect_asset_binding': 'canary.appearance:effect/poff', 'failure_message': 'not_possible',
        'failure_effect_position': 'caster'}}


def build(name, spell_type, records, texts):
    name, qualified = _qualified(name, spell_type, records, texts)
    if not qualified or name not in CASTS:
        return None
    parsed = {source: _parse(name, text) for source, text in qualified.items()}
    return parsed.get('canary', next(iter(parsed.values())))


def _barrier(name, source, text):
    if _body(text, 'rune.onCastSpell') != 'return combat:execute(creature, variant)':
        raise NativeItemUnresolved(f'{name}: unexpected barrier cast')
    wall = name == 'magic wall rune'
    variable = 'magicWall' if wall else 'wildGrowth'
    function = 'onCreateMagicWall' if wall else 'onCreateWildGrowth'
    symbol = 'ITEM_MAGICWALL' if wall else 'ITEM_WILDGROWTH'
    prefix = ('local tile = Tile(position) if not tile then return false end '
              'if tile:hasFlag(TILESTATE_FLOORCHANGE) then return false end '
              'if tile:getTopCreature() and not tile:getTopCreature():isPlayer() then return false end ')
    selection = (f'local {variable} if Game.getWorldType() == WORLD_TYPE_NO_PVP then {variable} = {symbol}_SAFE else {variable} = {symbol} end '
                 if source == 'canary' else f'local {variable} = Game.getWorldType() == WORLDTYPE_OPTIONAL and {symbol}_SAFE or {symbol} ')
    callback = _body(text, function)
    # Escape all fixed script text, leaving only literal duration captures variable.
    fixed = f'local item = Game.createItem({variable}, 1, position) if item then item:setDuration('
    pattern = re.escape(prefix + selection + fixed) + r'(?P<minimum>\d+)(?:,\s*(?P<maximum>\d+))?' + re.escape(') item:setAttribute(ITEM_ATTRIBUTE_DESCRIPTION, string.format("Casted by: %s", creature:getName())) end')
    match = re.fullmatch(pattern, callback)
    if not match or not re.search(r'combat:setCallback\(CALLBACK_PARAM_TARGETTILE,\s*"' + function + r'"\)', text):
        raise NativeItemUnresolved(f'{name}: barrier callback differs from the complete accepted template')
    if not re.search(r'combat:setParameter\(COMBAT_PARAM_DISTANCEEFFECT,\s*CONST_ANI_ENERGY\)', text):
        raise NativeItemUnresolved(f'{name}: missing energy projectile')
    minimum = int(match['minimum'])
    maximum = int(match['maximum'] or minimum)
    if minimum <= 0 or maximum < minimum:
        raise NativeItemUnresolved('invalid barrier duration range')
    return {'operation': 'create_item', 'created_item': _ref(ITEM_IDS[symbol]),
            'pvp_safe_item': _ref(ITEM_IDS[symbol + '_SAFE']),
            'duration_range_ms': {'minimum': minimum * 1000, 'maximum': maximum * 1000},
            'duration_selection': 'uniform_integer_seconds', 'safe_world_type': 'optional_pvp',
            'refuse_on': ['floor_change_tile', 'creature_on_tile'],
            'description_template': 'Casted by: {caster_name}',
            'presentation': {'projectile_asset_binding': 'canary.appearance:missile/energy'}}


def barrier_effect(name, records, texts):
    name, qualified = _qualified(name, 'rune', records, texts)
    if not qualified or name not in ('magic wall rune', 'wild growth rune'):
        return None
    parsed = {source: _barrier(name, source, text) for source, text in qualified.items()}
    result = parsed.get('canary', next(iter(parsed.values())))
    if name == 'wild growth rune':
        # S27 D.6.1 explicitly applies the F1056364 duration range over engine 30s.
        result['duration_range_ms'] = {'minimum': 30000, 'maximum': 60000}
    return result


def evidence(name, spell_type, records, texts):
    name, qualified = _qualified(name, spell_type, records, texts)
    if not qualified:
        return []
    observed = {source: (_parse(name, text) if name in CASTS else _barrier(name, source, text))
                for source, text in qualified.items()}
    rows = [{'source': source, 'revision': PINS[source], 'source_file': records[source]['file'],
             'source_blob': records[source]['blob'], 'url': f'https://github.com/{REPOS[source]}/blob/{PINS[source]}/{records[source]["file"]}',
             'extracted_parameters': parameters} for source, parameters in observed.items()]
    if len(observed) > 1 and observed['canary'] != observed['crystal']:
        rows.append({'policy': 'S21', 'selected': 'canary', 'disagreement': 'source parameter payloads differ; both source observations retained above'})
    if name == 'desintegrate rune':
        rows.append({'policy': 'S27 D.2.1.7', 'decision': 'Suppress the erroneous source cancel message on successful disintegration; preserve success and poff, including an empty tile. Script/action tags replace source UID threshold/action-id protection.'})
    if name in ('magic wall rune', 'wild growth rune'):
        rows.append({'policy': 'S27 D.6.1', 'item_enum_sha256_by_source': {s: ITEM_ENUM_SHA256[s] for s in qualified},
                     'item_enum_source': 'src/utils/utils_definitions.hpp ItemID_t',
                     'decision': 'Use per-instance item duration and caster description; preserve normal and optional-PvP item variants. Common blocking refuses creature occupied tiles.'})
    if name == 'wild growth rune':
        rows.append({'policy': 'S27 D.6.1 / F1056364', 'decision': 'Wiki duration 30000..60000ms overrides source fixed30000ms; uniform integer-second choice follows the pinned item:setDuration(min,max) engine implementation.',
                     'open_question': 'Q13 preserves uncertainty about official duration sampling; Q12 preserves invisible-creature cast behavior.'})
    return rows


def _object(properties):
    return {'type': 'object', 'additionalProperties': False, 'properties': properties, 'required': list(properties)}


def _const(value):
    return {'const': value}


def _item_ref():
    return _object({'family': _const('Item'), 'key': {'type': 'string', 'pattern': r'^candidate:item/[1-9][0-9]*$'}, 'revision': _const(ITEM_REVISION)})


def schemas():
    """Strict parameter schemas keyed by native behavior, not entire executions."""
    item_list = {'type': 'array', 'minItems': 1, 'uniqueItems': True, 'items': _item_ref()}
    food = _object({'pool': {**item_list, 'minItems': 7, 'maxItems': 7}, 'guaranteed': _const(1), 'extra': _const(1),
                    'extra_chance_percent': _const(50), 'selection': _const('uniform_independent'),
                    'overflow': _const('drop_on_caster_tile'), 'effect_asset_binding': _const('canary.appearance:effect/magic_green'),
                    'draw_order': _const('extra_chance_then_optional_then_guaranteed'),
                    'always_succeeds': _const(True)})
    variants = []
    for name in ('chameleon rune', 'desintegrate rune', 'destroy field rune'):
        # Use the operational fields, but keep positive source-extracted values
        # variable (duration/cap/pools); no additional field can be ignored.
        fixture = {'chameleon rune': {'operation': 'mimic_item', 'source': ['tile_top_item', 'container_slot', 'equipment_slot'], 'require_movable': True, 'reject_creature_target': True, 'duration_ms': 200000, 'success_effect_asset_binding': 'canary.appearance:effect/magic_red', 'failure_effect_asset_binding': 'canary.appearance:effect/poff', 'failure_message': 'not_possible'},
                   'desintegrate rune': {'operation': 'disintegrate', 'source': ['tile_items'], 'require_movable': True, 'max_items': 500, 'max_items_counts': 'visited', 'exclude_items': [], 'exclude_script_tagged': True, 'exclude_action_tagged': True, 'aggressive': False, 'allow_in_pz': True, 'empty_tile_succeeds': True, 'missing_tile_succeeds': True, 'success_effect_asset_binding': 'canary.appearance:effect/poff', 'send_cancel_on_success': False},
                   'destroy field rune': {'operation': 'remove_field', 'source': ['first_magic_field'], 'field_items': [], 'allow_in_pz': False, 'success_effect_asset_binding': 'canary.appearance:effect/poff', 'failure_effect_asset_binding': 'canary.appearance:effect/poff', 'failure_message': 'not_possible', 'failure_effect_position': 'caster'}}[name]
        properties = {key: (item_list if key in ('exclude_items', 'field_items') else {'type': 'integer', 'minimum': 1, 'maximum': 4294967295} if key in ('max_items', 'duration_ms') else _const(value)) for key, value in fixture.items()}
        variants.append(_object(properties))
    return {'random_item_grant': food, 'tile_item_operation': {'oneOf': variants}}


def schema_defs():
    """Additional typed create_item Effect fields consumed by the shared schema."""
    return {'pvp_safe_item': _item_ref(),
            'duration_range_ms': _object({'minimum': {'type': 'integer', 'minimum': 1000, 'maximum': 4294967000, 'multipleOf': 1000},
                                         'maximum': {'type': 'integer', 'minimum': 1000, 'maximum': 4294967000, 'multipleOf': 1000}}),
            'duration_selection': _const('uniform_integer_seconds'), 'safe_world_type': _const('optional_pvp'),
            'refuse_on': _const(['floor_change_tile', 'creature_on_tile']),
            'description_template': _const('Casted by: {caster_name}')}
