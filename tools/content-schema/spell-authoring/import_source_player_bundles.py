"""Import current source variants through the existing Combat authoring converter.

Uses local verified captures, never Wiki, selected catalog or formula corrections.
P4 and unsupported formula/guard semantics keep their header + mechanics reference.
A structurally valid candidate bundle is not a native activation/semantic parity claim.
"""
import argparse
import copy
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import os
import tempfile
from types import SimpleNamespace
import xml.etree.ElementTree as ET

HERE = Path(__file__).resolve().parent
_dependency_path = sys.path[:]
try:
    sys.path.insert(0, str(HERE.parent / 'monster-authoring'))
    import canary_batch
    import spell_scripts
finally:
    sys.path[:] = _dependency_path
del _dependency_path
import validate_spell
import native_house_movement
import native_world_items
import source_formula_evidence

SOURCES = ('canary-main-current', 'crystal-summer-current')
REVISION = 'source-player-r28'


class SourceBlocked(ValueError):
    pass


def sha(data):
    return hashlib.sha256(data).hexdigest()


def expr_source(expr):
    """Preserve exact source arithmetic order: notably level / 5 stays level / 5."""
    if set(expr) == {'const'}:
        return copy.deepcopy(expr)
    if set(expr) == {'var'} and expr['var'] in validate_spell.INPUT_SETS['skill'] | validate_spell.INPUT_SETS['level_magic']:
        return copy.deepcopy(expr)
    if expr.get('op') in {'add', 'sub', 'mul', 'div', 'neg', 'floor', 'ceil', 'sqrt', 'abs', 'min', 'max'} and set(expr) == {'op', 'args'}:
        return {'op': expr['op'], 'args': [expr_source(a) for a in expr['args']]}
    raise SourceBlocked('source expression function/input needs typed equivalent: ' + json.dumps(expr, sort_keys=True))


def source_formula(callback):
    source = callback.get('formula', {})
    if source.get('status') != 'resolved':
        raise SourceBlocked('callback expression capture unresolved: ' + source.get('error', 'missing formula'))
    bounds = [expr_source(source[k]) for k in ('minimum', 'maximum')]
    # Source callbacks return signed health deltas; authoring Effects own the sign.
    # Preserve both expression trees beneath abs/min/max, rather than changing a
    # coefficient, level contribution, library function or expression ordering.
    magnitude = [{'op': 'abs', 'args': [b]} for b in bounds]
    return {'kind': 'player_expression',
            'inputs': 'skill' if callback['kind'] == 'CALLBACK_PARAM_SKILLVALUE' else 'level_magic',
            'minimum': {'op': 'min', 'args': copy.deepcopy(magnitude)},
            'maximum': {'op': 'max', 'args': copy.deepcopy(magnitude)}}


def source_file(repo, revision, path):
    return subprocess.check_output(['git', '-C', str(repo), 'show', revision + ':' + path])


def qualify_player_combat_rules(repo, revision):
    """Bind bounded C++ semantics; no source Lua or C++ is executed."""
    path = 'src/creatures/combat/combat.cpp'
    combat = source_file(repo, revision, path)
    plain = re.sub(r'\s+', ' ', combat.decode())
    required = [
        'if (formulaType == COMBAT_FORMULA_DAMAGE) { damage.primary.value = normal_random( static_cast<int32_t>(mina), static_cast<int32_t>(maxa) ); } else if (creature)',
        'if (params.valueCallback) { params.valueCallback->getMinMaxValues(player, damage, params.useCharges); }',
        'if (params.targetCasterOrTopMost) { if (caster && caster->getTile() == tile) { if (creature != caster)',
        'else if (creature != topCreature)',
    ]
    if any(fragment not in plain for fragment in required):
        raise SourceBlocked('bounded source C++ combat precedence/selector anchors differ')
    game_path = 'src/game/game.cpp'
    game = source_file(repo, revision, game_path)
    if not re.search(r'bool Game::combatChangeHealth\([^\{]+\{.*?if \(damage.primary.value > 0\)', game.decode(), re.S):
        raise SourceBlocked('signed source health-delta branch not established')
    return {'explicit_damage_precedes_callback': True, 'caster_or_top_creature': True,
            'signed_health_delta': True,
            'proofs': [{'path': p, 'sha256': sha(b)} for p, b in ((path, combat), (game_path, game))]}


def default_fields(repo, revision):
    path = 'src/creatures/combat/spells.hpp'
    data = source_file(repo, revision, path)
    text = data.decode()
    fields = {}
    for name in ('level', 'mana', 'soul', 'cooldown', 'premium', 'learnable', 'selfTarget', 'needTarget',
                 'needDirection', 'casterTargetOrDirection', 'checkLineOfSight', 'pzLocked', 'needWeapon',
                 'aggressive', 'allowOnSelf', 'blockingSolid', 'blockingCreature', 'magLevel', 'hasParam', 'hasPlayerNameParam'):
        values = set(re.findall(r'\b(?:bool|uint32_t)\s+' + name + r'\s*=\s*(true|false|[0-9]+)\s*;', text))
        if len(values) != 1:
            raise SourceBlocked('ambiguous/missing pinned default initializer: ' + name)
        value = values.pop()
        fields[name] = value == 'true' if value in ('true', 'false') else int(value)
    xml_path = 'data/XML/vocations.xml'
    voc_data = source_file(repo, revision, xml_path)
    vocations = sorted(v.attrib['name'].casefold().replace(' ', '_') for v in ET.fromstring(voc_data).findall('vocation'))
    engine_path = 'src/creatures/combat/spells.cpp'
    engine = source_file(repo, revision, engine_path)
    if 'vocSpellMap.empty()' not in engine.decode():
        raise SourceBlocked('unrestricted empty vocation map behavior not established')
    fields['unrestricted_vocations'] = vocations
    return fields, [{'path': p, 'sha256': sha(b)} for p, b in ((path, data), (xml_path, voc_data), (engine_path, engine))]


def fill_header(projection, defaults, identity, registrar):
    spell = copy.deepcopy(projection)
    if 'source_carrier_symbol' in spell:
        raise SourceBlocked('disabled symbolic fixture has no executable carrier')
    spell['identity'] = identity
    req = spell.setdefault('requirements', {})
    for key, engine in (('level', 'level'), ('premium', 'premium'), ('learning_required', 'learnable')):
        req.setdefault(key, defaults[engine])
    req.setdefault('vocations', defaults['unrestricted_vocations'])
    spell.setdefault('costs', {}).setdefault('mana', defaults['mana'])
    spell['costs'].setdefault('soul', defaults['soul'])
    spell.setdefault('cooldown_ms', defaults['cooldown'])
    for key, engine in (('pz_locks_caster', 'pzLocked'), ('needs_weapon', 'needWeapon')):
        spell.setdefault(key, defaults[engine])
    target = spell.setdefault('targeting', {})
    for key, engine in (('aggressive', 'aggressive'), ('self_target', 'selfTarget'), ('needs_target', 'needTarget'),
                        ('needs_direction', 'needDirection'), ('target_or_direction', 'casterTargetOrDirection'),
                        ('block_walls', 'checkLineOfSight')):
        target.setdefault(key, defaults[engine])
    target.setdefault('parameter', ('player_name' if defaults['hasPlayerNameParam'] else 'text') if defaults['hasParam'] else 'none')
    if 'groups' not in spell:
        raise SourceBlocked('source has no authored cooldown group; none cannot be invented as attack/support')
    if spell['carrier'] == 'rune':
        rune = spell.setdefault('rune', {})
        if not all(k in registrar for k in ('runeId', 'charges', 'allowFarUse')):
            raise SourceBlocked('rune Item/charge/action admission requires source-qualified missing rule')
        rune['item'] = {'family': 'Item', 'key': 'candidate:item/source/' + identity['key'].split('/')[-2] + '/' + str(registrar['runeId']), 'revision': REVISION}
        rune.setdefault('magic_level', defaults['magLevel'])
        rune.setdefault('blocking', {'solid': defaults['blockingSolid'], 'creature': defaults['blockingCreature']})
    return spell


def requalify(value, key_prefix):
    if isinstance(value, list):
        return [requalify(v, key_prefix) for v in value]
    if not isinstance(value, dict):
        return value
    result = {k: requalify(v, key_prefix) for k, v in value.items()}
    if 'key' in result and 'revision' in result:
        result['revision'] = REVISION
        if result['key'].startswith('canary:item/'):
            result['key'] = key_prefix + '/item/' + result['key'].rsplit('/', 1)[1]
    return result


def convert_combat(converter, info, raw, spell, prefix):
    if raw.get('cast', {}).get('tier') not in ('plain_combat', 'random_combat'):
        raise SourceBlocked('player cast contains unrepresented custom guard/branch semantics: ' + json.dumps(raw.get('cast', {}), sort_keys=True))
    if info.get('tier') not in ('P1', 'P2', 'P3'):
        raise SourceBlocked('custom/P4 cast needs source-qualified native semantics; no placeholder effects')
    variants = list(dict.fromkeys(info.get('variants', [])))
    if len(variants) != 1:
        raise SourceBlocked('multi-combat selection/sequence needs exact source probability/order representation')
    index = variants[0]
    combat = copy.deepcopy(info['combats'][str(index)])
    engine_rules = getattr(converter, 'source_player_rules', {})
    direct_formula = combat.get('formula')
    if direct_formula:
        if not engine_rules.get('explicit_damage_precedes_callback') or not engine_rules.get('signed_health_delta'):
            raise SourceBlocked('combat:setFormula requires source-specific exact formula precedence representation')
        if len(direct_formula) != 5 or direct_formula[0] != '@COMBAT_FORMULA_DAMAGE' or not all(type(v) is int for v in direct_formula[1:]):
            raise SourceBlocked('explicit formula requires bounded integer COMBAT_FORMULA_DAMAGE semantics')
        if not ((direct_formula[1] < 0 and direct_formula[3] < 0) or (direct_formula[1] > 0 and direct_formula[3] > 0)):
            raise SourceBlocked('explicit signed formula crosses or touches zero; no sign-preserving Effect mapping')
        combat['formula'] = None
    selector = combat.get('params', {}).get('COMBAT_PARAM_TARGETCASTERORTOPMOST')
    if selector is not None:
        if not engine_rules.get('caster_or_top_creature') or selector != 1:
            raise SourceBlocked('source TARGETCASTERORTOPMOST selector lacks exact true-rule qualification')
        combat['param_calls'] = [call for call in combat['param_calls'] if call[0] != 'COMBAT_PARAM_TARGETCASTERORTOPMOST']
    if any(k in combat.get('callbacks', {}) for k in ('CALLBACK_PARAM_CHAINVALUE', 'CALLBACK_PARAM_CHAINPICKER')):
        raise SourceBlocked('source chain callback requires typed source chain value/picker semantics')
    captured = raw['combats'][index]
    combat, scalar_proofs = source_formula_evidence.scalar_free_combat_adapter(
        combat, captured, converter.source_snapshot, converter.source_revision)
    deps = {'abilities': [], 'effects': [], 'formulas': []}
    notes = []
    if scalar_proofs:
        # A misspelled Lua global may numerically resolve to parameter zero (TYPE).
        # Filter by the converter's pinned owning enums, not just the spelling.
        combat['param_calls'] = [call for call in combat.get('param_calls', [])
                                 if 'COMBAT_PARAM_TYPE' not in converter.engine_params([call], [])]
        notes.append('Source player health delta remains default zero; no invented direct-health formula')
        notes.append(json.dumps({'zero_health_delta_source_default': True, 'engine_rule_proofs': scalar_proofs}, sort_keys=True))
    converter.pending_definitions = set()
    key = prefix + '/ability'
    target = spell['targeting']
    converter.combat_ability(key, combat,
                            {'needs_target': target['needs_target'] or target['target_or_direction'],
                             'needs_direction': target['needs_direction']}, target.get('range_tiles', 0),
                            deps, lambda asset: asset, notes)
    if scalar_proofs and not any(effect.get('presentation') for effect in deps['effects']):
        visual = {}
        for field, presentation_key, kind, none in (
                ('COMBAT_PARAM_EFFECT', 'impact_asset_binding', 'effect', 'CONST_ME_NONE'),
                ('COMBAT_PARAM_DISTANCEEFFECT', 'projectile_asset_binding', 'missile', 'CONST_ANI_NONE')):
            value = combat['params'].get(field)
            if value not in (None, 0, none):
                visual[presentation_key] = converter.visual('@' + value if isinstance(value, str) else value, kind)[0]
        if visual:
            effect_key = key + '/effect-source-presentation'
            deps['effects'].append({'identity': {'key': effect_key, 'revision': REVISION},
                                   'operation': 'presentation_only', 'presentation': visual})
            deps['abilities'][-1]['effects'].append({'family': 'Effect', 'key': effect_key, 'revision': REVISION})
    if scalar_proofs and captured.get('parameters', {}).get('COMBAT_PARAM_TYPE') != 'COMBAT_NONE':
        deps['abilities'][-1]['zero_damage_health_path'] = True
    if selector is not None:
        deps['abilities'][-1]['target_selection'] = 'caster_or_top_creature'
        notes.append('Source COMBAT_PARAM_TARGETCASTERORTOPMOST=true preserved as caster_or_top_creature selector')
    captured = raw['combats'][index]
    callbacks = [c for c in captured.get('callbacks', []) if 'formula' in c]
    if direct_formula:
        callbacks = [{'kind': 'CALLBACK_PARAM_LEVELMAGICVALUE', 'formula': {'status': 'resolved',
                      'minimum': {'const': str(direct_formula[1])}, 'maximum': {'const': str(direct_formula[3])}}}]
        for effect in deps['effects']:
            if effect.get('formula', {}).get('key') == canary_batch.CASTER_MAGNITUDE:
                effect['operation'] = 'damage' if direct_formula[1] < 0 else 'heal'
        notes.append('COMBAT_FORMULA_DAMAGE mina/maxa signed source deltas override valueCallback; minb/maxb unused')
        notes.append(json.dumps({'source_setFormula': direct_formula, 'engine_rule_proofs': engine_rules['proofs']}, sort_keys=True))
    placeholder = canary_batch.CASTER_MAGNITUDE
    for effect in deps['effects']:
        if effect.get('formula', {}).get('key') != placeholder:
            continue
        if len(callbacks) != 1:
            raise SourceBlocked('damage/heal Combat lacks exactly one captured player formula')
        formula_key = key + '/formula-player'
        if not any(f['identity']['key'] == formula_key for f in deps['formulas']):
            if getattr(converter, 'crystal_base_helper', None) and not direct_formula:
                try:
                    normalized = source_formula_evidence.source_formula(callbacks[0], converter.crystal_base_helper, getattr(converter, 'source_shielding_qualified', False))
                except ValueError as error:
                    raise SourceBlocked('source expression function/input needs typed equivalent: ' + str(error)) from error
                notes.append(json.dumps({'source_formula_helper_proof': converter.crystal_base_helper_proof, 'source_shielding_helper_proof': getattr(converter, 'source_shielding_proof', None)}, sort_keys=True))
            else:
                normalized = source_formula(callbacks[0])
            deps['formulas'].append({'identity': {'key': formula_key, 'revision': REVISION}, **normalized})
        effect['formula'] = {'family': 'Formula', 'key': formula_key, 'revision': REVISION}
    deps['formulas'] = [f for f in deps['formulas'] if f['identity']['key'] != placeholder]
    gaps = []
    if selector is not None:
        gaps.append({'source_field': 'target_selection.variant_route', 'reason': 'Caster/top selector applies only to position/tile variants; source creature-ID variant must pass through its exact creature target without selector override'})
        notes.append(json.dumps({'target_selector_engine_rule_proofs': engine_rules['proofs']}, sort_keys=True))
    for field in ('COMBAT_PARAM_CASTSOUND', 'COMBAT_PARAM_IMPACTSOUND', 'COMBAT_PARAM_USECHARGES', 'COMBAT_PARAM_AGGRESSIVE'):
        if field in combat['params']:
            gaps.append({'source_field': field, 'reason': 'Existing Combat converter retains source receipt but has no independent exact Effect owner binding for this field'})
    return {'ability': {'family': 'Ability', 'key': key, 'revision': REVISION}}, requalify(deps, prefix), notes, gaps



def convert_conjure(converter, raw, spell, snapshot):
    if spell['carrier'] != 'instant' or raw['cast']['tier'] != 'conjure':
        raise SourceBlocked('conjure requires exact instant source conjure cast')
    source = raw['cast']['conjure']
    if not isinstance(source.get('result_item_id'), int) or source['result_item_id'] <= 0:
        raise SourceBlocked('conjure source result Item unresolved')
    def item(number):
        return {'family': 'Item', 'key': 'candidate:item/source/' + snapshot + '/' + str(number), 'revision': REVISION}
    body = {'result': item(source['result_item_id']), 'count': source['count']}
    if source.get('reagent_item_id', 0):
        body['reagent'] = item(source['reagent_item_id'])
    if source.get('effect'):
        body['effect_asset_binding'] = converter.visual('@' + source['effect'], 'effect')[0]
    gaps = [] if 'effect' in source else [{'source_field': 'conjure.default_effect',
                                         'reason': 'Source conjureItem default visual effect requires separately qualified owning helper; explicit result/count/reagent retained'}]
    return {'conjure': body}, {'abilities': [], 'effects': [], 'formulas': []}, [], gaps

def source_cpp_function(data, symbol):
    """Exact signature/body slice with comments and quoted braces ignored."""
    text = data.decode()
    # Mask lexical trivia at equal length; keep real braces and line boundaries.
    masked = re.sub(r'''//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*' '''.rstrip(),
                    lambda m: ''.join('\n' if c == '\n' else ' ' for c in m.group()), text, flags=re.S)
    matches = list(re.finditer(r'(?m)^[^\n{};]*\b' + re.escape(symbol) + r'\s*\([^;{}]*\)[^;{}]*\{', masked))
    if len(matches) != 1:
        raise SourceBlocked('C++ function identity not unique: ' + symbol)
    match = matches[0]
    depth, end = 1, match.end()
    while depth and end < len(masked):
        depth += (masked[end] == '{') - (masked[end] == '}')
        end += 1
    if depth:
        raise SourceBlocked('unterminated C++ function: ' + symbol)
    return text[match.start():end].encode()


def source_house_descriptor(name, raw, repo, revision, source, current):
    """Current Aleta closure: full stable helpers + exact relevant C++ methods."""
    module = native_house_movement
    old = module.REVISIONS[source]
    proofs = []
    full_files = ['src/map/house/house.cpp', 'src/map/map_definitions.hpp',
                  'src/lua/functions/map/house_functions.cpp', 'src/lua/functions/map/tile_functions.cpp',
                  'src/lua/functions/map/position_functions.cpp']
    scoped = {
        'src/lua/functions/creatures/player/player_functions.cpp': [
            'PlayerFunctions::luaPlayerCreate', 'PlayerFunctions::luaPlayerSetEditHouse',
            'PlayerFunctions::luaPlayerSendHouseWindow'],
        'src/creatures/players/player.cpp': ['Player::sendHouseWindow', 'Player::getEditHouse', 'Player::setEditHouse'],
        'src/game/game.cpp': ['Game::playerUpdateHouseWindow', 'Game::getPlayerByName'],
    }
    for path in full_files:
        now, before = source_file(repo, revision, path), source_file(repo, old, path)
        if now != before:
            raise SourceBlocked('Aleta whole source helper changed: ' + path)
        proofs.append({'path': path, 'scope': 'full_file', 'sha256': sha(now), 'current_bytes_equal_template_bytes': True})
    for path, symbols in scoped.items():
        now, before = source_file(repo, revision, path), source_file(repo, old, path)
        for symbol in symbols:
            current_body, old_body = source_cpp_function(now, symbol), source_cpp_function(before, symbol)
            if current_body != old_body:
                raise SourceBlocked('Aleta relevant source function changed: ' + symbol)
            proofs.append({'path': path, 'scope': 'function', 'symbol': symbol,
                           'current_file_sha256': sha(now), 'template_file_sha256': sha(before),
                           'function_sha256': sha(current_body), 'current_bytes_equal_template_bytes': True})
    proof = {'module': module.__name__, 'module_sha256': sha(Path(module.__file__).read_bytes()),
             'qualified_template_revision': old, 'current_source_revision': revision,
             'full_cast_sha256': sha(current), 'current_bytes_equal_template_bytes': True,
             'source_helper_proofs': proofs, 'external_value_overrides_used': False,
             'helper_scope': 'house API domain, Lua bridge and editor session methods; transitive common engine services remain unqualified'}
    descriptor = copy.deepcopy(module.BEHAVIOURS[name])
    return {'native_behavior': descriptor}, {'abilities': [], 'effects': [], 'formulas': []}, [
        'Exact source-only native descriptor; current donor bytes equal qualified template bytes',
        json.dumps(proof, sort_keys=True)], [{'source_field': 'house.transitive_common_engine_services',
        'reason': 'Source-exact house API operations represented; player/permission/network/teleport provider execution not qualified'}]


def source_native_descriptor(raw, repo, revision, snapshot, scratch_parent):
    """Reuse only exact source-body templates without external value overrides."""
    name = raw['name'].casefold()
    source = snapshot.split('-')[0]
    world_names = {'food', 'chameleon rune', 'destroy field rune'}
    movement_names = {'creature illusion', 'levitate', 'magic rope', 'house door list', 'house guest list', 'house subowner list', 'house kick'}
    if name not in world_names | movement_names:
        return None
    module = native_world_items if name in world_names else native_house_movement
    pins = module.PINS if name in world_names else module.REVISIONS
    path = raw['file']
    current = source_file(repo, revision, path)
    qualified = source_file(repo, pins[source], path)
    if current != qualified:
        raise SourceBlocked('native descriptor cast bytes differ from the exact source-qualified template: ' + name)
    if name.startswith('house '):
        return source_house_descriptor(name, raw, repo, revision, source, current)
    helper_proofs = []
    with tempfile.TemporaryDirectory(prefix='.native-source-reference-', dir=scratch_parent) as temporary:
        reference_root = Path(temporary)
        if name in movement_names:
            for helper in module.SUPPORT_FILES[module.BEHAVIOURS[name]['key']]:
                data = source_file(repo, revision, helper)
                expected = module.SUPPORT_DIGESTS[source, helper]
                if sha(data) != expected:
                    raise SourceBlocked('native descriptor source helper closure changed: ' + helper)
                target = reference_root / helper
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
                helper_proofs.append({'path': helper, 'sha256': expected})
        record = {**raw, 'revision': pins[source], 'source_root': str(reference_root)}
        try:
            descriptor = module.build(name, raw['spell_type'], {source: record}, {(source, path): current.decode()})
        except ValueError as error:
            raise SourceBlocked('exact-source native descriptor qualification failed: ' + str(error)) from error
    if descriptor is None:
        raise SourceBlocked('existing exact-source native descriptor did not qualify')
    def items(value):
        if isinstance(value, list):
            return [items(v) for v in value]
        if not isinstance(value, dict):
            return value
        result = {k: items(v) for k, v in value.items()}
        if result.get('family') == 'Item':
            match = re.fullmatch(r'candidate:item/([0-9]+)', result['key'])
            if not match:
                raise SourceBlocked('native descriptor Item requires explicit numeric source identity')
            result['key'] = 'candidate:item/source/' + snapshot + '/' + match[1]
            result['revision'] = REVISION
        return result
    proof = {'module': module.__name__, 'module_sha256': sha(Path(module.__file__).read_bytes()),
             'qualified_template_revision': pins[source], 'current_source_revision': revision,
             'full_cast_sha256': sha(current), 'current_bytes_equal_template_bytes': True,
             'source_helper_proofs': helper_proofs, 'external_value_overrides_used': False}
    return {'native_behavior': items(descriptor)}, {'abilities': [], 'effects': [], 'formulas': []}, [
        'Exact source-only native descriptor; current donor bytes equal qualified template bytes',
        json.dumps(proof, sort_keys=True)], []


def item_references(value):
    found = {}
    def walk(node):
        if isinstance(node, list):
            for v in node:
                walk(v)
        elif isinstance(node, dict):
            if node.get('family') == 'Item' and set(node) == {'family', 'key', 'revision'}:
                found[node['key'], node['revision']] = node
            for v in node.values():
                walk(v)
    walk(value)
    return [found[k] for k in sorted(found)]


def receipt_schema():
    string = {'type': 'string', 'minLength': 1}
    digest = {'type': 'string', 'pattern': '^[0-9a-f]{64}$'}
    properties = {
        'schema': {'const': 'OTERYN_SOURCE_PLAYER_BUNDLE_RECEIPT/v1'},
        'registration_key': string, 'logical_key': {'type': 'array', 'items': string, 'minItems': 2, 'maxItems': 2},
        'source_revision': {'type': 'string', 'pattern': '^[0-9a-f]{40}$'}, 'source_sha256': digest,
        'candidate_key': string,
        'source_header': {'$ref': 'urn:oteryn:spell-authoring:candidate:1#/$defs/sourceProjection'},
        'mechanics_reference': {'type': 'object', 'additionalProperties': False,
                               'properties': {'source_capture': string, 'source_capture_sha256': digest,
                                              'imported_facts': {'const': 'source-callback-facts.jsonl.gz'},
                                              'registration_key': string, 'source_callback_scope': string},
                               'required': ['source_capture', 'source_capture_sha256', 'imported_facts', 'registration_key', 'source_callback_scope']},
        'engine_default_proofs': {'type': 'array', 'items': {'type': 'object', 'additionalProperties': False,
                                                        'properties': {'path': string, 'sha256': digest}, 'required': ['path', 'sha256']}, 'minItems': 1},
        'runtime_activation': {'const': False}, 'external_sources_used': {'const': False},
        'status': {'enum': ['BLOCKED', 'CANDIDATE_SCHEMA_VALID']},
        'blockers': {'type': 'array', 'items': string},
        'remaining_mechanics': {'type': 'array', 'items': {'type': 'object', 'additionalProperties': False,
                                                          'properties': {'source_field': string, 'reason': string}, 'required': ['source_field', 'reason']}},
        'dependencies': {'type': 'object', 'additionalProperties': False,
                         'properties': {k: {'type': 'integer', 'minimum': 0} for k in ('abilities', 'effects', 'formulas')},
                         'required': ['abilities', 'effects', 'formulas']},
        'schema_and_semantic_validation_errors': {'type': 'array', 'maxItems': 0},
        'conversion_notes': {'type': 'array', 'items': {'type': 'string'}},
        'native_execution_qualified': {'const': False},
        'item_owner_bindings_required': {'type': 'array', 'items': {'$ref': 'urn:oteryn:spell-authoring:candidate:1#/$defs/ItemRef'}}}
    return {'$schema': 'https://json-schema.org/draft/2020-12/schema', '$id': 'urn:oteryn:source-player-bundle-receipt:1',
            'type': 'object', 'additionalProperties': False, 'properties': properties,
            'required': [k for k in properties if k not in ('dependencies', 'schema_and_semantic_validation_errors',
                                                          'conversion_notes', 'native_execution_qualified', 'item_owner_bindings_required')],
            'allOf': [{'if': {'properties': {'status': {'const': 'CANDIDATE_SCHEMA_VALID'}}},
                       'then': {'required': ['dependencies', 'schema_and_semantic_validation_errors', 'conversion_notes', 'native_execution_qualified', 'item_owner_bindings_required'],
                                'properties': {'blockers': {'maxItems': 0}}},
                       'else': {'properties': {'blockers': {'minItems': 1}}}}]}


def generate(args):
    projections_path = args.player_inputs / 'player-source-authoring-projections.jsonl.gz'
    facts_path = args.player_inputs / 'player-source-registrars.jsonl.gz'
    projections = {r['registration_key']: r for r in map(json.loads, gzip.decompress(projections_path.read_bytes()).splitlines())}
    facts = [r for r in map(json.loads, gzip.decompress(facts_path.read_bytes()).splitlines()) if r['snapshot'] in SOURCES]
    captures = json.loads(args.combat_captures.read_text())
    combat_index = {}
    for row in captures:
        key = (row['source'], row['kind'], row['name'], row['provenance']['path'])
        if key in combat_index:
            raise ValueError('ambiguous cached named Combat registration: ' + repr(key))
        combat_index[key] = row
    schema = receipt_schema()
    validate_spell.Draft202012Validator.check_schema(schema)
    receipt_validator = validate_spell.Draft202012Validator(schema, registry=validate_spell.REGISTRY)
    write(args.out / 'receipt.schema.json', schema)
    all_results = []
    callback_facts = []
    for snapshot in SOURCES:
        source = snapshot.split('-')[0]
        source_rows = [r for r in facts if r['snapshot'] == snapshot]
        revision = source_rows[0]['source_revision']
        repo = args.source_root / source
        defaults, default_proofs = default_fields(repo, revision)
        snapshot_root = args.snapshot_root / snapshot
        converter = canary_batch.Converter(snapshot_root, {}, {}, {}, {})
        converter.spell_scripts = SimpleNamespace(enums=spell_scripts.engine_enums(snapshot_root))
        converter.source_player_rules = qualify_player_combat_rules(repo, revision)
        converter.source_snapshot = snapshot
        converter.source_revision = revision
        converter.crystal_base_helper = None
        if source == 'crystal':
            converter.crystal_base_helper, converter.crystal_base_helper_proof = source_formula_evidence.crystal_helper(snapshot, revision)
        raw_path = args.capture_root / (snapshot + '-registered-spells.json')
        raw_bytes = raw_path.read_bytes()
        raw_hash = sha(raw_bytes)
        raw_rows = {r['registration_key']: r for r in json.loads(raw_bytes)}
        for fact in source_rows:
            reg_key = fact['registration_key']
            raw_row = raw_rows[reg_key]
            if raw_row['sha256'] != fact['source_sha256'] or raw_row['record']['registrar'] != fact['registrar']:
                raise ValueError('source registrar/callback capture mismatch: ' + reg_key)
            data = source_file(repo, revision, fact['source_file'])
            if sha(data) != fact['source_sha256']:
                raise ValueError('source file digest mismatch: ' + reg_key)
            callback_facts.append({'registration_key': reg_key, 'source_revision': revision,
                                   'source_sha256': fact['source_sha256'], 'source_callback_facts': raw_row['record']})
            row_id = sha(reg_key.encode())[:16]
            prefix = 'candidate:spell/source/' + snapshot + '/' + row_id
            out = args.out / snapshot / row_id
            out.mkdir(parents=True, exist_ok=True)
            result = {'schema': 'OTERYN_SOURCE_PLAYER_BUNDLE_RECEIPT/v1', 'registration_key': reg_key, 'logical_key': fact['logical_key'],
                      'source_revision': revision, 'source_sha256': fact['source_sha256'],
                      'candidate_key': prefix, 'source_header': projections[reg_key]['spell'],
                      'mechanics_reference': {'source_capture': raw_path.name, 'source_capture_sha256': raw_hash,
                                              'imported_facts': 'source-callback-facts.jsonl.gz',
                                              'registration_key': reg_key, 'source_callback_scope': 'cached exact source callback expressions and cast tier'},
                      'engine_default_proofs': default_proofs, 'runtime_activation': False,
                      'external_sources_used': False, 'status': 'BLOCKED', 'blockers': [], 'remaining_mechanics': []}
            write(out / 'source-header.json', {'spell': result['source_header']})
            try:
                raw = raw_row['record']
                converter.source_shielding_qualified, converter.source_shielding_proof = source_formula_evidence.shielding_evidence(snapshot, revision, data.decode())
                key = (source, fact['logical_key'][0], fact['logical_key'][1], fact['source_file'])
                capture = combat_index.get(key)
                spell = fill_header(result['source_header'], defaults, {'key': prefix, 'revision': REVISION}, fact['registrar'])
                native = source_native_descriptor(raw, repo, revision, snapshot, args.out.parent)
                if native:
                    execution, deps, notes, gaps = native
                elif raw['cast']['tier'] == 'conjure':
                    execution, deps, notes, gaps = convert_conjure(converter, raw, spell, snapshot)
                else:
                    if not capture:
                        raise SourceBlocked('no qualified cached Combat conversion for this exact registration')
                    if capture['provenance']['revision'] != revision or capture['provenance']['sha256'] != fact['source_sha256']:
                        raise ValueError('cached Combat source identity mismatch: ' + reg_key)
                    execution, deps, notes, gaps = convert_combat(converter, capture['conversion'], raw, spell, prefix)
                if spell['targeting'].get('cast_at_position') and spell['targeting']['needs_target']:
                    gaps.append({'source_field': 'targeting.position_route_shadowed', 'reason': 'Owning InstantSpell engine prioritizes needTarget creature-ID route over needPosition clicked tile; both source flags retained independently, no crosshair execution claim'})
                spell['execution'] = execution
                catalog = {'definitions': item_references({'spell': spell, 'dependencies': deps})}
                errors = validate_spell.validate({'spell': spell}, deps, catalog)
                if errors:
                    raise SourceBlocked('candidate schema/semantic validation: ' + ' | '.join(errors))
                write(out / 'spell.json', {'spell': spell})
                write(out / 'dependencies.json', deps)
                write(out / 'catalog.json', catalog)
                result.update(status='CANDIDATE_SCHEMA_VALID', dependencies={k: len(v) for k, v in deps.items()},
                              schema_and_semantic_validation_errors=[], conversion_notes=notes,
                              remaining_mechanics=gaps, native_execution_qualified=False,
                              item_owner_bindings_required=catalog['definitions'])
            except (SourceBlocked, canary_batch.SpellUnresolved) as error:
                result['blockers'].append(str(error))
                result['remaining_mechanics'].append({'source_field': 'source.cast_or_dependency', 'reason': str(error)})
            receipt_validator.validate(result)
            write(out / 'receipt.json', result)
            all_results.append(result)
    payload = b''.join((json.dumps(r, sort_keys=True, separators=(',', ':')) + '\n').encode() for r in sorted(callback_facts, key=lambda r: r['registration_key']))
    with (args.out / 'source-callback-facts.jsonl.gz').open('wb') as output:
        with gzip.GzipFile(fileobj=output, mode='wb', filename='', mtime=0) as compressed:
            compressed.write(payload)
    from collections import Counter
    summary = {'schema': 'OTERYN_SOURCE_PLAYER_BUNDLE_IMPORT/v1', 'records': len(all_results),
               'source_populations': dict(Counter(r['registration_key'].split('/')[0] for r in all_results)),
               'status_counts': dict(Counter(r['status'] for r in all_results)),
               'external_sources_used': False, 'runtime_activation': False,
               'full_source_mechanics_1_to_1_complete': False,
               'converter': 'existing canary_batch.Converter.combat_ability + exact source expression adapter',
               'input_proofs': {str(p.name): sha(p.read_bytes()) for p in (projections_path, facts_path, args.combat_captures)},
               'converter_proofs': {str(p.name): sha(p.read_bytes()) for p in (Path(__file__), Path(canary_batch.__file__), Path(spell_scripts.__file__), Path(validate_spell.__file__), Path(native_house_movement.__file__), Path(native_world_items.__file__), Path(source_formula_evidence.__file__), HERE / 'spell.schema.json', HERE / 'spell-dependencies.schema.json')},
               'all_receipts_schema_valid': True,
               'records_index': [{'registration_key': r['registration_key'], 'candidate_key': r['candidate_key'],
                                  'status': r['status'], 'blockers': r['blockers']} for r in all_results]}
    summary['blocker_reason_counts'] = dict(sorted(Counter(
        b.split(':', 1)[0] for r in all_results for b in r['blockers']).items()))
    summary['remaining_mechanics_counts'] = dict(sorted(Counter(
        g['source_field'] for r in all_results for g in r['remaining_mechanics']).items()))
    summary['native_descriptor_count'] = sum(
        any(note.startswith('Exact source-only native descriptor') for note in r.get('conversion_notes', []))
        for r in all_results)
    summary['source_callback_payload_sha256'] = sha(payload)
    write(args.out / 'import-summary.json', summary)
    files = {str(p.relative_to(args.out)): sha(p.read_bytes()) for p in sorted(args.out.rglob('*')) if p.is_file()}
    write(args.out / 'package-manifest.json', {'schema': 'OTERYN_SOURCE_PLAYER_PACKAGE/v1', 'files': files})
    return summary



def check_owned_package(path):
    """Refuse replacement of unrelated files; old first-pass layout is recognized."""
    if not path.exists():
        return
    if not path.is_dir() or path.is_symlink():
        raise ValueError('output is not an owned regular package directory')
    summary_path = path / 'import-summary.json'
    if not summary_path.exists() or json.loads(summary_path.read_text()).get('schema') != 'OTERYN_SOURCE_PLAYER_BUNDLE_IMPORT/v1':
        raise ValueError('existing output is not a recognized source player package')
    allowed = {'import-summary.json', 'package-manifest.json', 'source-callback-facts.jsonl.gz', 'receipt.schema.json'}
    for file in path.rglob('*'):
        if file.is_symlink():
            raise ValueError('symlink in existing source package')
        if file.is_file():
            relative = file.relative_to(path).as_posix()
            if relative not in allowed and not re.fullmatch(r'(?:canary-main-current|crystal-summer-current)/[0-9a-f]{16}/(?:source-header|receipt|spell|dependencies|catalog)\.json', relative):
                raise ValueError('unexpected existing package file: ' + relative)


def run(args):
    """Build in a fresh directory; swap only after complete successful validation."""
    check_owned_package(args.out)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.player-source-stage-', dir=args.out.parent) as temporary:
        stage = Path(temporary) / 'package'
        stage.mkdir()
        candidate = SimpleNamespace(**vars(args))
        candidate.out = stage
        summary = generate(candidate)
        backup = Path(temporary) / 'previous'
        if args.out.exists():
            os.rename(args.out, backup)
        try:
            os.rename(stage, args.out)
        except Exception:
            if backup.exists():
                os.rename(backup, args.out)
            raise
        return summary

def write(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--player-inputs', type=Path, default=HERE.parents[2] / 'docs/reference/spells/r28-source-closure')
    parser.add_argument('--source-root', type=Path, default=Path('/workspace/spell-sources'))
    parser.add_argument('--snapshot-root', type=Path, default=Path('/workspace/spells-r22-source-audit/upstream-local-only'))
    parser.add_argument('--capture-root', type=Path, default=Path('/workspace/spells-r22-source-audit'))
    parser.add_argument('--combat-captures', type=Path, default=Path('/workspace/spell-source-closure/generated-monsters-r28-complete/all-registered-spells.json'))
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps({k: v for k, v in run(args).items() if k != 'records_index'}, sort_keys=True))
