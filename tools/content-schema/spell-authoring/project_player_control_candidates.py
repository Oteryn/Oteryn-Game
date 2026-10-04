"""R52: real target bundles and concrete existing-reader gaps for 21 controls.

This is a target-data projection, not another source IR. One player Nature's
Embrace bundle fits the current reader; source refusal presentation is explicitly
unrepresented. Other wrappers require owned reader/provider changes, not fake
placeholder Effects. No runtime admission or complete-source parity is claimed.
"""
import argparse
import copy
import gzip
import hashlib
import json
from pathlib import Path
import re
import tempfile
import convert_spells

import import_source_player_bundles as base
import source_cast_programs
import validate_spell

HERE = Path(__file__).resolve().parent
BASE = Path('docs/reference/spells/r28-source-closure/player-source-bundles')
OUTPUT = Path('docs/reference/spells/r52-source-closure')
PROGRAM_PATH = Path('docs/reference/spells/r49-source-closure/source-cast-programs.jsonl.gz')
REGISTRATION = "canary-main-current/data/scripts/spells/healing/nature's_embrace.lua#1"
SOURCE_SHA = '5864935eca52d88b28480c58cd9574d6c95a4090cd0bc66fb11a68ce15b95164'
SOURCE_REVISION = '04b83b512114bfd888000d6e1433ed8ecaec7c5b'
CANDIDATE_REVISION = 'player-control-r52'
ROW_ID = hashlib.sha256(REGISTRATION.encode()).hexdigest()[:16]
CANDIDATE_KEY = 'candidate:spell/source/canary-main-current/' + ROW_ID
RUNTIME_FILES = ['authoring.rs', 'executable_catalog.rs', 'plan.rs', 'target.rs', 'mod.rs',
                 'native_actor_states.rs', 'native_house_movement.rs', 'locate.rs', 'part_b_tests.rs']


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def ref(family, suffix):
    return {'family': family, 'key': CANDIDATE_KEY + suffix, 'revision': CANDIDATE_REVISION}


def identity(suffix=''):
    return {'key': CANDIDATE_KEY + suffix, 'revision': CANDIDATE_REVISION}


def reader_fields(repo, struct):
    text = (Path(repo) / 'apps/game-server/src/spell/executable_catalog.rs').read_text()
    match = re.search(r'pub\(crate\) struct ' + re.escape(struct) + r'\s*\{([^}]+)\}', text)
    if not match: raise ValueError('reader struct not found: ' + struct)
    return set(re.findall(r'pub\(crate\)\s+(\w+)\s*:', match.group(1)))


def validate_reader_shape(repo, bundle, deps):
    checks = [('AuthoredSpell', bundle['spell']), ('Requirements', bundle['spell']['requirements']),
              ('Costs', bundle['spell']['costs']), ('Targeting', bundle['spell']['targeting']),
              ('AuthoredExecution', bundle['spell']['execution'])]
    for family, struct in [('abilities', 'AbilityProfile'), ('effects', 'EffectProfile'), ('formulas', 'FormulaProfile')]:
        checks.extend((struct, row) for row in deps[family])
    for struct, value in checks:
        unknown = set(value) - reader_fields(repo, struct)
        if unknown: raise ValueError('actual reader deny_unknown_fields: ' + struct + ': ' + ','.join(sorted(unknown)))
    required = {'aggressive', 'self_target', 'needs_target', 'needs_direction', 'block_walls',
                'allow_on_self', 'check_floor', 'parameter'}
    if not required <= bundle['spell']['targeting'].keys(): raise ValueError('missing required reader targeting fields')
    return True


def qualify(source, fact):
    if (sha(source) != SOURCE_SHA or fact['registration_key'] != REGISTRATION
            or fact['source_revision'] != SOURCE_REVISION or fact['source_sha256'] != SOURCE_SHA):
        raise ValueError('exact current player Nature source identity mismatch')
    raw = fact['source_callback_facts']
    if raw['name'] != "Nature's Embrace" or raw['spell_type'] != 'instant' or len(raw['combats']) != 1:
        raise ValueError('player Nature single Combat identity mismatch')
    combat = raw['combats'][0]
    if combat['parameters'] != {'COMBAT_PARAM_TYPE': 'COMBAT_HEALING', 'COMBAT_PARAM_EFFECT': 'CONST_ME_MAGIC_BLUE',
                                'COMBAT_PARAM_AGGRESSIVE': 0, 'COMBAT_PARAM_DISPEL': 'CONDITION_PARALYZE'}:
        raise ValueError('unmapped current Nature Combat fields')
    if set(combat) != {'parameters', 'callbacks'} or len(combat['callbacks']) != 1:
        raise ValueError('additional unrepresented Combat behavior')
    callback = combat['callbacks'][0]
    if callback['function'] != 'onGetFormulaValues' or callback['kind'] != 'CALLBACK_PARAM_LEVELMAGICVALUE':
        raise ValueError('unexpected Nature formula callback')
    def bound(coefficient):
        return {'op': 'add', 'args': [{'op': 'div', 'args': [{'var': 'level'}, {'const': '2.5'}]},
                                     {'op': 'mul', 'args': [{'var': 'magic_level'}, {'const': coefficient}]}]}
    if callback['formula'] != {'status': 'resolved', 'inputs': ['level', 'magic_level'], 'functions': [],
                                'minimum': bound('20'), 'maximum': bound('28')}:
        raise ValueError('source formula changed or gained unsupported helper')
    return base.source_formula(callback)


def build_candidate(repo, source_root):
    repo, source_root = Path(repo), Path(source_root)
    facts = {row['registration_key']: row for row in map(json.loads, gzip.decompress((repo / BASE / 'source-callback-facts.jsonl.gz').read_bytes()).splitlines())}
    fact = facts[REGISTRATION]
    path = REGISTRATION.split('/', 1)[1].rsplit('#', 1)[0]
    source = base.source_file(source_root / 'canary', SOURCE_REVISION, path)
    formula = qualify(source, fact)
    row = repo / BASE / 'canary-main-current' / ROW_ID
    header = json.loads((row / 'source-header.json').read_text())
    old_receipt = json.loads((row / 'receipt.json').read_text())
    if old_receipt['status'] != 'BLOCKED' or header['spell'] != old_receipt['source_header']:
        raise ValueError('requires exact historical blocked source header')
    defaults, default_proofs = base.default_fields(source_root / 'canary', SOURCE_REVISION)
    spell = base.fill_header(header['spell'], defaults, identity(), fact['source_callback_facts']['registrar'])
    # Display-only donor flags are kept verbatim in source-header/provenance. They
    # are not operational requirements and the actual Rust reader rejects them.
    display_flags = spell['requirements'].pop('vocation_display_flags', None)
    spell['targeting']['allowed_targets'] = 'not_self'
    spell['targeting']['allow_on_self'] = defaults['allowOnSelf']
    engine = base.source_file(source_root / 'canary', SOURCE_REVISION, 'src/creatures/combat/spells.cpp')
    throw = base.source_cpp_function(engine, 'InstantSpell::canThrowSpell')
    if b'fromPos.z != toPos.z' not in throw: raise ValueError('source floor check changed')
    spell['targeting']['check_floor'] = True
    spell['execution'] = {'ability': ref('Ability', '/ability')}
    deps = {'abilities': [{'identity': identity('/ability'), 'kind': 'spell', 'needs_target': True,
                          'needs_direction': False, 'range_tiles': 0,
                          'effects': [ref('Effect', '/heal'), ref('Effect', '/dispel')]}],
            'effects': [{'identity': identity('/heal'), 'operation': 'heal', 'damage_type': 'healing',
                         'formula': ref('Formula', '/formula'),
                         'presentation': {'impact_asset_binding': 'canary.appearance:effect/magic_blue'}},
                        {'identity': identity('/dispel'), 'operation': 'remove_condition', 'removed_condition': 'paralyze'}],
            'formulas': [{'identity': identity('/formula'), **formula}]}
    bundle, catalog = {'spell': spell}, {'definitions': []}
    errors = validate_spell.validate(bundle, deps, catalog)
    if errors: raise ValueError('actual target schema validation: ' + ' | '.join(errors))
    validate_reader_shape(repo, bundle, deps)
    limitations = [
        {'field': 'targeting.source_refusal_presentation', 'source': {'message': "You can't cast this spell to yourself.", 'effect': 'poff'},
         'current_reader': 'TargetNotAllowed with generic rejection; no custom refusal-presentation field',
         'required_endpoint': 'spell::cast TargetIllegal rejection presentation before acceptance'},
        {'field': 'targeting.implicit_source_range', 'source': -1,
         'current_reader': 'range_tiles omitted; viewport/spatial provider equivalence requires owner qualification',
         'required_endpoint': 'operational CastFacts range/line-of-sight resolver'},
        {'field': 'presentation.asset_execution', 'source': 'magic_blue + source cast sound retained',
         'current_reader': 'bindings retained; client asset availability/playback not qualified by data mapping',
         'required_endpoint': 'existing presentation owner/provider'}]
    proof = {'schema': 'OTERYN_PLAYER_CONTROL_TARGET_PROJECTION/v1', 'registration_key': REGISTRATION,
             'candidate_key': CANDIDATE_KEY, 'candidate_revision': CANDIDATE_REVISION,
             'status': 'READER_SHAPE_CANDIDATE_WITH_EXPLICIT_SOURCE_DIFFERENCES',
             'target_schema_and_semantic_validation_errors': [], 'existing_reader_structural_shape_checked': True,
             'existing_reader_executed_on_this_bundle': False, 'source_full_mechanics_1_to_1_complete': False,
             'source_wrapper_qualification_scope': 'player_caster_heal_formula_and_not_self_target_rule',
             'represented_gameplay': ['player_not_self_guard', 'level_300_mana_400_source_header',
                                      'source_level_div_2_5_magic_20_28_formula', 'healing', 'paralysis_dispel'],
             'source_differences_and_provider_limits': limitations, 'source_vocation_display_flags_retained': display_flags,
             'source_sha256': SOURCE_SHA, 'source_revision': SOURCE_REVISION,
             'historical_source_header_sha256': sha((row / 'source-header.json').read_bytes()),
             'historical_receipt_sha256': sha((row / 'receipt.json').read_bytes()),
             'source_fact_sha256': sha(canonical(fact)), 'engine_default_proofs': default_proofs,
             'source_floor_check_function_sha256': sha(throw),
             'reader_file_proofs': {name: sha((repo / 'apps/game-server/src/spell' / name).read_bytes()) for name in RUNTIME_FILES},
             'schema_proofs': {name: sha((HERE / name).read_bytes()) for name in ['spell.schema.json', 'spell-dependencies.schema.json']},
             'producer_sha256': sha(Path(__file__).read_bytes()),
             'runtime_activation': False, 'native_execution_qualified': False, 'canonical_selection_changed': False,
             'native_identity_allocation': False, 'original_r28_receipt_status': 'BLOCKED'}
    receipt = copy.deepcopy(old_receipt)
    receipt.update(status='CANDIDATE_SCHEMA_VALID', blockers=[],
                   dependencies={family: len(values) for family, values in deps.items()},
                   schema_and_semantic_validation_errors=[], native_execution_qualified=False,
                   item_owner_bindings_required=[],
                   remaining_mechanics=[{'source_field': value['field'],
                                         'reason': value['current_reader'] + '; owner: ' + value['required_endpoint']}
                                        for value in limitations],
                   conversion_notes=['Real existing-reader target data: player Nature heal/dispel/formula with not_self rule.',
                                     'Full source gameplay/UI/provider parity is not claimed: exact failure text/POFF and viewport providers remain explicit gaps.',
                                     'Display-only vocation flags remain verbatim in source-header/proof; they are excluded from operational reader Requirements.',
                                     'Canonical selection, runtime activation and native admission remain unchanged.'])
    validate_spell.Draft202012Validator(base.receipt_schema(), registry=validate_spell.REGISTRY).validate(receipt)
    return {'spell.json': bundle, 'dependencies.json': deps, 'catalog.json': catalog,
            'source-header.json': header, 'receipt.json': receipt, 'projection-receipt.json': proof}


def gap_details(key):
    filename = key.rsplit('/', 1)[1].rsplit('#', 1)[0]
    donor = key.split('-')[0]
    if key == REGISTRATION:
        return ['targeting.allowed_targets=not_self; heal/dispel/formula existing reader'], ['custom source failure message and POFF; spatial provider viewport'], ['cast.rs::TargetIllegal rejection presentation', 'target.rs::AllowedTargets::NotSelf'], ['targeting.refusal_presentation']
    if filename == 'find_person.lua':
        return ['locate_message native endpoint exists; spoken/player-name parameter exists'], ['source 5/101/275 differs schema and actual locate.rs hardcoded 5/101/251'], ['native_house_movement.rs::Position::phrase', 'locate.rs::locate'], ['native_behavior.locate_message.parameters.bands_tiles configurable + consumer dispatch']
    if filename == 'cancel_magic_shield.lua':
        return ['remove_condition magic_shield effect admitted; successful cast supported C.5'], ['source removeCondition happens before Combat; plan separates effects/side_effects and does not expose pre-combat phase'], ['authoring.rs::spell_effect', 'plan.rs::plan', 'actor_conditions.rs'], ['Ability.pre_combat_caster_condition_operations with condition-id semantics']
    if filename in ('blood_rage.lua', 'protector.lua'):
        return ['stance_toggle native owner exists'], ['source conditional ATTRIBUTES remove(COMBAT,subid) before Combat; existing stance percentages/vocation policy differ current donor'], ['native_actor_states.rs::plan_stance', 'actor_execution.rs'], ['Ability.pre_combat_caster_condition_operations with exact condition_id/sub_id', 'source attribute percent parameters in accepted condition data']
    if filename == 'expose_weakness.lua':
        return ['condition attributes and damage_received modifiers exist'], ['incoming Variant override with creature object; target callback player refusal/summon no-op; dynamic Wheel drain-body grade'], ['authoring.rs::ability_effects', 'target.rs', 'native_combat.rs'], ['Ability.target_variant_override', 'Effect.target_callback_guard_and_dynamic_condition_parameters']
    if filename in ('intense_healing_rune.lua', 'ultimate_healing_rune.lua'):
        if donor == 'canary':
            missing = ['source Monster target refusal cannot be represented by self_only/self_or_own_summons/not_self; refusal message+POFF']
            if filename == 'ultimate_healing_rune.lua': missing.append('Player conversion and explicit exalted monk refusal chronology/message')
            return ['heal, paralyze dispel, caster/top selector and rune Item carrier reader exist'], missing, ['target.rs::CastTarget/AllowedTargets', 'authoring.rs::spell_from_bundle', 'cast.rs::rejection'], ['CastTarget.creature_kind + targeting.allowed_targets=not_monster', 'targeting.refusal_presentation and source vocation refusal ordering']
        return ['heal/dispel and self_only target rule exist'], ['Leiden direct addHealth special branch before other-monster rejection; Creature(number) or attacked target routing; self-only refusal and exact Variant route'], ['target.rs', 'plan.rs::effect_plan', 'native_combat.rs'], ['conditional target-name direct-health action + ordered target guards', 'Ability.target_variant_override']
    if filename in ('devastating_knockout.lua', 'greater_tiger_clash.lua', 'tiger_clash.lua'):
        return ['equipment_attack / Monk Harmony native owners exist'], ['current donor Harmony callback helper/conversion and spender branch must be mapped into exact accepted equipment params; no generic source callback executor'], ['native_actor_states.rs', 'native_combat.rs::plan'], ['native_behavior.equipment_attack current donor branch parameters/operation mapping']
    if filename == 'sweeping_takedown.lua':
        return ['equipment_attack and occurrence-local cache native owner exist'], ['two Combat execution chronology plus source cache cleanup must match occurrence-local native provider; current donor coefficient/chain facts not qualified by old native descriptor'], ['native_actor_states.rs', 'native_combat.rs', 'spell_execution'], ['native_behavior.equipment_attack ordered dual-Combat + cache cleanup contract']
    if filename in ('forked_glacier.lua', 'forked_thorns.lua'):
        return ['Ability.chain supports max_targets/range_tiles/fork; formula helper mapping exists'], ['current chain callback base ' + ('seven' if filename == 'forked_glacier.lua' else 'six') + ' plus dynamic Wheel additional target; impact numeric asset ' + ('324' if filename == 'forked_glacier.lua' else '325') + ' requires qualified owner binding'], ['authoring.rs::chain_spec', 'native_combat.rs', 'chain.rs'], ['Ability.chain dynamic Wheel extra-target provider reference', 'qualified current formula/presentation bindings']
    if donor == 'crystal' and filename in ('heal_friend.lua', "nature's_embrace.lua"):
        return ['ordinary primary healing Combat + party snapshot owners exist'], ['Shared Conservation nearest valid same-floor party member excluding primary/caster, strict tie; secondary runs after primary even on false; separate scaled floor formula'], ['native_combat.rs', 'party.rs', 'plan.rs::party_plans'], ['ordered secondary-single-target heal after primary with independent result', 'native accepted shared_conservation selection/formula data']
    raise ValueError('unclassified exact control variant ' + key)


def build_chain_candidates(repo, source_root):
    repo, source_root = Path(repo), Path(source_root)
    source_revision = source_cast_programs.source_syntax.PINS['crystal']
    facts = {row['registration_key']: row for row in map(json.loads, gzip.decompress((repo / BASE / 'source-callback-facts.jsonl.gz').read_bytes()).splitlines())}
    chain_path = HERE / 'chain-behaviours.json'; chains = json.loads(chain_path.read_text())['spells']
    models = []
    enum_path = base.canary_batch.EFFECT_CONSTANTS
    enum_bytes = base.source_file(source_root / 'crystal', source_revision, enum_path)
    with tempfile.TemporaryDirectory(prefix='r52-current-enum-') as temporary:
        root = Path(temporary); enum_file = root / enum_path; enum_file.parent.mkdir(parents=True); enum_file.write_bytes(enum_bytes)
        converter = base.canary_batch.Converter(root, {}, {}, {}, {})
        for filename, name, total in [('forked_glacier.lua', 'Forked Glacier', 7), ('forked_thorns.lua', 'Forked Thorns', 6)]:
            key = 'crystal-summer-current/data/scripts/spells/attack/' + filename + '#1'
            fact = facts[key]; source = base.source_file(source_root / 'crystal', source_revision, fact['source_callback_facts']['file'])
            if sha(source) != fact['source_sha256']: raise ValueError('current chain file identity mismatch')
            raw = fact['source_callback_facts']; combat = raw['combats'][0]
            if len(raw['combats']) != 1 or raw['name'] != name or len(combat['callbacks']) != 2:
                raise ValueError('chain base requires exact one Combat + formula/chain callbacks')
            formula_callback = next(callback for callback in combat['callbacks'] if callback['kind'] == 'CALLBACK_PARAM_LEVELMAGICVALUE')
            chain_callback = next(callback for callback in combat['callbacks'] if callback['kind'] == 'CALLBACK_PARAM_CHAINVALUE')
            if chain_callback['function'] != 'getChainValue' or formula_callback['formula']['status'] != 'resolved':
                raise ValueError('chain formula/callback identity differs')
            expected_type = 'COMBAT_ICEDAMAGE' if total == 7 else 'COMBAT_EARTHDAMAGE'
            if combat['parameters']['COMBAT_PARAM_TYPE'] != expected_type or set(combat['parameters']) != {'COMBAT_PARAM_TYPE', 'COMBAT_PARAM_EFFECT', 'COMBAT_PARAM_DISTANCEEFFECT', 'COMBAT_PARAM_CHAIN_EFFECT'}:
                raise ValueError('unmapped chain Combat parameter')
            row_id = sha(key.encode())[:16]; candidate_key = 'candidate:spell/source/crystal-summer-current/' + row_id
            def ident(suffix=''): return {'key': candidate_key + suffix, 'revision': CANDIDATE_REVISION}
            def reference(family, suffix): return {'family': family, **ident(suffix)}
            row = repo / BASE / 'crystal-summer-current' / row_id
            header = json.loads((row / 'source-header.json').read_text()); old_receipt = json.loads((row / 'receipt.json').read_text())
            defaults, default_proofs = base.default_fields(source_root / 'crystal', source_revision)
            spell = base.fill_header(header['spell'], defaults, ident(), raw['registrar'])
            display_flags = spell['requirements'].pop('vocation_display_flags', None)
            spell['targeting']['allow_on_self'] = defaults['allowOnSelf']; spell['targeting']['check_floor'] = True
            spell['execution'] = {'ability': reference('Ability', '/ability')}
            accepted = chains[name.casefold()]
            chain = {field: copy.deepcopy(accepted[field]) for field in convert_spells.CHAIN_FIELDS if field in accepted}
            if chain != {'max_targets': total - 1, 'range_tiles': 4, 'backtracking': False, 'shape': 'fork', 'initial_range_tiles': 7}:
                raise ValueError('accepted canonical base chain binding changed')
            def visual(value, kind):
                raw_value = '@' + value if isinstance(value, str) else value
                return converter.visual(raw_value, kind)[0].replace('canary.appearance:', 'crystal.appearance:')
            params = combat['parameters']; chain['chain_asset_binding'] = visual(params['COMBAT_PARAM_CHAIN_EFFECT'], 'effect')
            notes = set(); bounds = [convert_spells.convert_expr(formula_callback['formula'][field], notes) for field in ['minimum', 'maximum']]
            magnitudes = [{'op': 'abs', 'args': [bound]} for bound in bounds]
            formula = {'identity': ident('/formula'), 'kind': 'player_expression', 'inputs': 'level_magic',
                       'minimum': {'op': 'min', 'args': copy.deepcopy(magnitudes)}, 'maximum': {'op': 'max', 'args': copy.deepcopy(magnitudes)}}
            deps = {'abilities': [{'identity': ident('/ability'), 'kind': 'spell', 'needs_target': False,
                                  'needs_direction': False, 'range_tiles': 7, 'chain': chain, 'effects': [reference('Effect', '/damage')]}],
                    'effects': [{'identity': ident('/damage'), 'operation': 'damage',
                                 'damage_type': 'ice' if total == 7 else 'earth', 'formula': reference('Formula', '/formula'),
                                 'presentation': {'impact_asset_binding': visual(params['COMBAT_PARAM_EFFECT'], 'effect'),
                                                  'projectile_asset_binding': visual(params['COMBAT_PARAM_DISTANCEEFFECT'], 'missile')}}],
                    'formulas': [formula]}
            bundle = {'spell': spell}; catalog = {'definitions': []}
            errors = validate_spell.validate(bundle, deps, catalog)
            if errors: raise ValueError('canonical base chain target validation: ' + ' | '.join(errors))
            validate_reader_shape(repo, bundle, deps)
            limits = [{'source_field': 'source.WheelAdditionalTarget', 'reason': 'Accepted S6 separates base Ability and Wheel ProjectV2AugmentBinding; dynamic source augment retained in R49 and awaits existing Wheel owner binding.'},
                      {'source_field': 'source.chain.range_and_shape', 'reason': 'Current Crystal jump5/implicit sequential is explicitly superseded by accepted S23 canonical fork/jump4/initial7; original source facts retained.'},
                      {'source_field': 'source.formula.base_damage_healing', 'reason': 'Accepted S5 maps donor helper to existing world level_base_damage_healing; donor formula AST retained in R49.'},
                      {'source_field': 'source.presentation_and_execution', 'reason': 'Current source effect/projectile/chain/sound bindings retained; owner asset playback and provider/native execution remain unqualified.'}]
            receipt = copy.deepcopy(old_receipt); receipt.update(status='CANDIDATE_SCHEMA_VALID', blockers=[],
                dependencies={family: len(values) for family, values in deps.items()}, schema_and_semantic_validation_errors=[],
                native_execution_qualified=False, item_owner_bindings_required=[], remaining_mechanics=limits,
                conversion_notes=['Real canonical base Ability.chain reuses accepted S23 binding and S5 formula normalization.',
                                  'S6 Wheel target augment is separate from base Spell; no source unlock gate removed.',
                                  'All current donor source facts remain in immutable R49; canonical changes are explicit.'])
            validate_spell.Draft202012Validator(base.receipt_schema(), registry=validate_spell.REGISTRY).validate(receipt)
            proof = {'schema': 'OTERYN_PLAYER_CONTROL_TARGET_PROJECTION/v1', 'registration_key': key, 'candidate_key': candidate_key,
                     'candidate_revision': CANDIDATE_REVISION, 'status': 'CANONICAL_BASE_READER_SHAPE_CANDIDATE',
                     'source_sha256': sha(source), 'source_revision': source_revision,
                     'accepted_normalization': {'chain_policy': 'S23/S3/S21', 'formula_policy': 'S5', 'augment_policy': 'S6',
                                                'canonical_chain_binding_sha256': sha(chain_path.read_bytes()),
                                                'source_chain_total_targets': total, 'source_chain_jump': 5,
                                                'source_chain_backtracking': False, 'canonical_chain': chain},
                     'separate_augment_dependency': {'binding_type': 'ProjectV2AugmentBinding', 'target': reference('Ability', '/ability'),
                                                    'source_getter': 'getWheelSpellAdditionalTarget', 'spell_name': name, 'binding_complete': False},
                     'source_vocation_display_flags_retained': display_flags, 'source_enum_sha256': sha(enum_bytes),
                     'target_schema_and_semantic_validation_errors': [], 'existing_reader_structural_shape_checked': True,
                     'existing_reader_executed_on_this_bundle': False, 'source_full_mechanics_1_to_1_complete': False,
                     'engine_default_proofs': default_proofs, 'runtime_activation': False, 'native_execution_qualified': False,
                     'canonical_selection_changed': False, 'native_identity_allocation': False, 'producer_sha256': sha(Path(__file__).read_bytes())}
            models.append((key, row_id, {'spell.json': bundle, 'dependencies.json': deps, 'catalog.json': catalog,
                                        'source-header.json': header, 'receipt.json': receipt, 'projection-receipt.json': proof}))
    return models


def build_existing_native_bindings(repo):
    """Keep exact already-qualified native identity/header/dependencies; alias source outside."""
    repo = Path(repo); path = HERE / 'samples/native-spell-profiles.json'; document = json.loads(path.read_text())
    if document['revision'] != 'spell-p2-r20' or len(document['profiles']) != 67: raise ValueError('existing native profile set changed')
    wanted = {'Blood Rage': ['canary-main-current/data/scripts/spells/support/blood_rage.lua#1'],
              'Protector': ['canary-main-current/data/scripts/spells/support/protector.lua#1'],
              'Find Person': [donor + '/data/scripts/spells/support/find_person.lua#1' for donor in ['canary-main-current', 'crystal-summer-current']]}
    results = []
    for name, registrations in wanted.items():
        profile = next(row for row in document['profiles'] if row['name'] == name and row['carrier'] == 'instant')
        bundle, deps = {'spell': copy.deepcopy(profile['spell'])}, copy.deepcopy(profile['dependencies'])
        catalog = {'definitions': []}; errors = validate_spell.validate(bundle, deps, catalog)
        if errors: raise ValueError('existing qualified native schema: ' + ' | '.join(errors))
        validate_reader_shape(repo, bundle, deps)
        if bundle['spell'] != profile['spell'] or deps != profile['dependencies']: raise ValueError('native equality binding altered')
        bindings = []
        for key in registrations:
            row_id = sha(key.encode())[:16]; population = key.split('/')[0]
            original = json.loads((repo / BASE / population / row_id / 'source-header.json').read_text())
            differences = []
            for field in ['requirements', 'costs', 'cooldown_ms', 'groups', 'targeting']:
                if original['spell'].get(field) != bundle['spell'].get(field):
                    differences.append({'field': field, 'source': original['spell'].get(field), 'existing_reader_template': bundle['spell'].get(field)})
            source_receipt = json.loads((repo / BASE / population / row_id / 'receipt.json').read_text())
            bindings.append({'registration_key': key, 'source_header': original,
                             'source_sha256': source_receipt['source_sha256'], 'source_revision': source_receipt['source_revision'],
                             'source_program_reference': {'path': PROGRAM_PATH.as_posix(), 'registration_key': key}, 'source_to_existing_template_differences': differences})
        proof = {'schema': 'OTERYN_PLAYER_CONTROL_EXISTING_NATIVE_BINDING/v1', 'registrations': bindings,
                 'target_identity': profile['spell']['identity'], 'binding_scope': 'proposed_source_alias_to_existing_reader_qualified_template_not_metadata_acceptance',
                 'existing_native_profile_sha256': sha(canonical(profile)), 'existing_native_profile_file_sha256': sha(path.read_bytes()),
                 'native_reader_requires_complete_profile_equality': True, 'complete_profile_equality_verified': True,
                 'existing_reader_executed_on_this_bundle': False,
                 'accepted_behavior_rule': 'Existing S27 C.4 stance or B.3 P7 locate behavior; donor source remains unchanged',
                 'template_header_policy_qualification': 'NOT_ESTABLISHED_FOR_CURRENT_DONOR',
                 'template_binding_requires_header_policy_review': True,
                 'current_source_receipts_promoted': False,
                 'template_values_are_not_claimed_current_source_correct': True,
                 'source_full_mechanics_1_to_1_complete': False, 'runtime_activation': False,
                 'native_execution_qualified': False, 'canonical_selection_changed': False, 'native_identity_allocation': False}
        results.append((name.casefold().replace(' ', '_'), {'spell.json': bundle, 'dependencies.json': deps,
                                                        'catalog.json': catalog, 'projection-receipt.json': proof}))
    return results


def build_ultimate_caster_requirements(repo, source_root):
    """Actual header fields ready for a later full UHR bundle, never a full Spell."""
    key = 'canary-main-current/data/scripts/runes/ultimate_healing_rune.lua#1'
    expected = '7b3ef8a4f3ef7214ec6b0024b5151c02af102ae70400d671dd12414fa728728b'
    source = base.source_file(Path(source_root) / 'canary', SOURCE_REVISION, key.split('/', 1)[1].split('#')[0])
    if sha(source) != expected or b'vocation == "exalted monk"' not in source:
        raise ValueError('current UHR source caster guard changed')
    defaults, proofs = base.default_fields(Path(source_root) / 'canary', SOURCE_REVISION)
    source_allowed = [v for v in defaults['unrestricted_vocations'] if v != 'exalted_monk']
    runtime = (Path(repo) / 'apps/game-server/src/spell/mod.rs').read_text()
    match = re.search(r'impl Vocation \{(.*?)\n\}', runtime, re.S)
    if not match: raise ValueError('current Vocation implementation absent')
    supported = set(re.findall(r'"([a-z_]+)"\s*=>\s*Self::', match.group(1)))
    reader_allowed = [v for v in source_allowed if v in supported]
    if 'monk' not in reader_allowed or 'exalted_monk' in reader_allowed:
        raise ValueError('derived UHR guard loses ordinary Monk or accepts Exalted Monk')
    return {'schema': 'OTERYN_PLAYER_CONTROL_PARTIAL_TARGET_HEADER/v1', 'registration_key': key,
            'source_sha256': expected, 'source_derived_requirements': {'vocations': source_allowed},
            'existing_reader_domain_requirements': {'vocations': reader_allowed},
            'source_vocations_outside_current_reader_domain': sorted(set(source_allowed) - supported),
            'engine_default_proofs': proofs, 'source_caster_domain': 'Player',
            'current_reader_caster_domain': 'CasterState with recognized Vocation',
            'scope': 'real requirement fields only; target monster guard/full Spell remain unresolved',
            'full_spell_emitted': False, 'runtime_activation': False,
            'remaining_mechanics': ['target Monster refusal/message/POFF', 'source vocation refusal timing/message',
                                    'source None vocation has no current runtime Vocation representation']}


def build_matrix(repo):
    programs = [json.loads(line) for line in gzip.decompress((Path(repo) / PROGRAM_PATH).read_bytes()).splitlines()]
    if len(programs) != 21 or {row['registration_key'] for row in programs} != set(source_cast_programs.EXPECTED_KEYS):
        raise ValueError('requires exact R49 other_cast cohort')
    rows = []
    for program in programs:
        supported, missing, endpoints, schema_fields = gap_details(program['registration_key'])
        rows.append({'registration_key': program['registration_key'], 'source_revision': program['source_revision'],
                     'source_sha256': program['source_sha256'], 'existing_reader_supported_parts': supported,
                     'specific_unrepresented_behavior': missing, 'existing_runtime_endpoints': endpoints,
                     'mapping_stage': ('TARGET_SCHEMA_VALID_BASE_CANDIDATE' if program['registration_key'] == REGISTRATION or program['registration_key'].endswith(('forked_glacier.lua#1', 'forked_thorns.lua#1')) else 'EXISTING_NATIVE_TEMPLATE_BINDING_REQUIRES_METADATA_POLICY' if program['registration_key'].endswith('find_person.lua#1') or (program['registration_key'].startswith('canary') and program['registration_key'].endswith(('blood_rage.lua#1', 'protector.lua#1'))) else 'REQUIRES_SPECIFIC_DATA_OR_READER_CAPABILITY'),
                     'proposed_owned_data_fields': schema_fields,
                     'target_bundle_emitted': program['registration_key'] == REGISTRATION or program['registration_key'].endswith(('forked_glacier.lua#1', 'forked_thorns.lua#1', 'find_person.lua#1')) or (program['registration_key'].startswith('canary') and program['registration_key'].endswith(('blood_rage.lua#1', 'protector.lua#1'))),
                     'full_source_equivalent': False, 'original_r28_receipt_status': 'BLOCKED'})
    return rows


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('--repo', default='.'); parser.add_argument('--source-root', default='/workspace/spell-sources')
    args = parser.parse_args(); repo = Path(args.repo).resolve(); values = build_candidate(repo, args.source_root)
    out = repo / OUTPUT; row = out / 'player-control-candidates' / 'canary-main-current' / ROW_ID; row.mkdir(parents=True, exist_ok=True)
    for name, value in values.items(): write_json(row / name, value)
    for key, row_id, artifacts in build_chain_candidates(repo, args.source_root):
        target = out / 'player-control-candidates' / 'crystal-summer-current' / row_id; target.mkdir(parents=True, exist_ok=True)
        for name, value in artifacts.items(): write_json(target / name, value)
    for alias, artifacts in build_existing_native_bindings(repo):
        target = out / 'existing-native-canonical-bindings' / alias; target.mkdir(parents=True, exist_ok=True)
        for name, value in artifacts.items(): write_json(target / name, value)
    write_json(out / 'ultimate-healing-caster-requirements.json', build_ultimate_caster_requirements(repo, args.source_root))
    matrix = {'schema': 'OTERYN_PLAYER_CONTROL_REAL_MAPPING_PROPOSAL/v1', 'records': 21,
              'target_bundles_emitted': 3, 'native_template_bundle_copies': 3, 'proposed_native_template_source_bindings': 4, 'source_registration_bindings_emitted': 7, 'complete_source_equivalent_spells': 0, 'rows': build_matrix(repo),
              'runtime_activation': False, 'canonical_selection_changed': False, 'native_execution_qualified': False,
              'input_programs_sha256': sha((repo / PROGRAM_PATH).read_bytes()),
              'candidate_relative_path': row.relative_to(out).as_posix()}
    write_json(out / 'player-control-mapping-proposal.json', matrix)
    receipt = {'schema': 'OTERYN_PLAYER_CONTROL_REAL_MAPPING_PROPOSAL/v1/receipt', 'records': 21,
               'reader_shape_candidates': 3, 'native_template_profiles': 3, 'proposed_native_template_source_bindings': 4, 'source_registration_bindings': 7, 'source_equivalent_full_spell_candidates': 0,
               'runtime_activation': False, 'native_execution_qualified': False, 'canonical_selection_changed': False,
               'native_identity_allocation': False, 'source_population_review_status_unchanged': True,
               'artifacts': {path.relative_to(out).as_posix(): sha(path.read_bytes()) for path in sorted(out.rglob('*')) if path.is_file() and path.name != 'player-control-mapping-receipt.json'}}
    write_json(out / 'player-control-mapping-receipt.json', receipt)
    print(json.dumps({'records': 21, 'reader_shape_candidates': 3, 'native_template_profiles': 3, 'proposed_native_template_source_bindings': 4, 'source_registration_bindings': 7, 'source_equivalent_full_spell_candidates': 0}))


if __name__ == '__main__': main()
