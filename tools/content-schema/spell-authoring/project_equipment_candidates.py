#!/usr/bin/env python3
"""R56: exact 25-row equipment/Monk audit and typed partial target proposals.

No native-profile aliasing, fake full candidates or runtime/shared-index mutation.
"""
import argparse
import copy
import json
from pathlib import Path

import import_source_player_bundles as base
import project_player_control_candidates as reader
import source_monk_formula_library as source_library
import validate_spell
import native_actor_states

OUTPUT = Path('docs/reference/spells/r56-source-closure')
PROFILES_PATH = 'tools/content-schema/spell-authoring/samples/native-spell-profiles.json'
WORKLIST_PATH = 'docs/reference/spells/blocked-173-worklist.json'
POLICY_PATH = 'docs/architecture/OTERYN_SPELL_AUTHORING_SCHEMA_V1.md'
PINS = {'canary': '04b83b512114bfd888000d6e1433ed8ecaec7c5b', 'crystal': '00ce02a57ca5a12e48f32a3476e37471167e4c3f'}
REVISION = 'source-equipment-r56'
sha = source_library.sha
canonical = source_library.canonical


def differences(left, right, prefix=''):
    if isinstance(left, dict) and isinstance(right, dict):
        result = []
        for key in sorted(set(left) | set(right)):
            path = prefix + '/' + key
            if key not in left or key not in right:
                result.append({'path': path, 'source_present': key in left, 'canonical_present': key in right,
                               'source_value': left.get(key), 'canonical_value': right.get(key)})
            else:
                result.extend(differences(left[key], right[key], path))
        return result
    if left == right:
        return []
    return [{'path': prefix, 'source_present': True, 'canonical_present': True,
             'source_value': left, 'canonical_value': right}]


def source_parameter_proposal(name, donor, text):
    """Only source-supported values for EXISTING native parameter fields, not a new DSL."""
    if name == 'focus serenity':
        if donor == 'canary' and 'setSerene(true, 7 * 1000)' not in text:
            raise ValueError('source Serene duration drift')
        if donor == 'crystal' and 'setSereneCooldown(7000)' not in text:
            raise ValueError('source Serene duration drift')
        return {'serene_ms': 7000, 'fill_harmony': True, 'harmony_max': 5}
    if name == 'focus harmony':
        if ('fillHarmony()' if donor == 'canary' else 'setHarmony(5)') not in text:
            raise ValueError('source Harmony fill drift')
        return {'fill_harmony': True, 'harmony_max': 5, 'serene_ms': None}
    if name.startswith('virtue of ') and donor == 'crystal':
        # Direct setVirtue is not the accepted toggle operation. Preserve this
        # precise disagreement as a source-correct parameter PATCH, never admit it.
        if 'setVirtue(VIRTUE_' not in text:
            raise ValueError('source virtue assignment drift')
        return {'toggle_same_stance_off': False}
    return {}


def focus_proof(source_root):
    root = Path(source_root) / 'canary'
    proofs = []
    helpers = {
        'src/creatures/players/player.cpp': ['Player::fillHarmony', 'Player::buildHarmony',
            'Player::healFromHarmony', 'Player::setSerene', 'Player::clearCooldowns'],
        'src/creatures/combat/combat.cpp': ['Combat::harmonyHeal'],
        'src/lua/functions/creatures/player/player_functions.cpp': ['PlayerFunctions::luaPlayerFillHarmony',
            'PlayerFunctions::luaPlayerSetSerene', 'PlayerFunctions::luaPlayerClearSpellCooldowns'],
    }
    for path, symbols in helpers.items():
        old = base.source_file(root, native_actor_states.PINS['canary'], path)
        current = base.source_file(root, PINS['canary'], path)
        for symbol in symbols:
            before = base.source_cpp_function(old, symbol)
            after = base.source_cpp_function(current, symbol)
            if before != after:
                raise ValueError('Focus scoped helper drift: ' + symbol)
            proofs.append({'path': path, 'symbol': symbol, 'scoped_sha256': sha(after),
                'old_revision': native_actor_states.PINS['canary'], 'current_revision': PINS['canary'],
                'exact_scoped_bytes_equal': True, 'full_file_equivalence': False})
    return proofs


def build(repo, source_root):
    repo = Path(repo)
    profiles_bytes = (repo / PROFILES_PATH).read_bytes()
    profiles = json.loads(profiles_bytes)['profiles']
    if len(profiles) != 67:
        raise ValueError('closed native profile population changed')
    captures = source_library.rows(repo, source_library.CAPTURE_PATH, source_library.CAPTURE_SHA)
    work = json.loads((repo / WORKLIST_PATH).read_text())
    lane = [row for row in work['records'] if row['lane'] == 'monk_equipment']
    if len(lane) != 25:
        raise ValueError('allocated lane must contain exactly 25 original registrations')
    rows, partials, bindings, headers, candidates = [], [], [], {}, {}
    scoped_focus = focus_proof(source_root)
    defaults = {donor: base.default_fields(Path(source_root) / donor, pin) for donor, pin in PINS.items()}
    policy = (repo / POLICY_PATH).read_bytes()
    s16 = next(line for line in policy.decode().splitlines() if line.startswith('| S16 |'))
    if '`learning_required` is false for every spell' not in s16:
        raise ValueError('S16 policy changed')
    for item in lane:
        reg = item['registration_key']
        snapshot = reg.split('/')[0]
        donor = snapshot.split('-')[0]
        row_id = sha(reg.encode())[:16]
        archived = repo / 'imports/spells/r28/player-source-bundles' / snapshot / row_id
        header_bytes = (archived / 'source-header.json').read_bytes()
        header = json.loads(header_bytes)
        receipt = json.loads((archived / 'receipt.json').read_text())
        fact = captures[reg]
        raw = fact['source_callback_facts']
        source = base.source_file(Path(source_root) / donor, PINS[donor], raw['file'])
        if (receipt['status'] != 'BLOCKED' or receipt['source_header'] != header['spell']
                or sha(source) != fact['source_sha256'] or fact['source_revision'] != PINS[donor]):
            raise ValueError('exact archived header/source identity mismatch')
        headers[snapshot + '/' + row_id + '/source-header.json'] = header_bytes
        name = raw['name'].casefold()
        matches = [profile for profile in profiles if profile['name'].casefold() == name and profile['carrier'] == raw['spell_type']]
        if len(matches) > 1:
            raise ValueError('native names are not unique')
        row = {'registration_key': reg, 'name': raw['name'], 'source_revision': PINS[donor],
               'source_path': raw['file'], 'source_sha256': fact['source_sha256'],
               'source_header_path': snapshot + '/' + row_id + '/source-header.json', 'source_header_sha256': sha(header_bytes),
               'capture_fact_sha256': sha(canonical(fact)), 'candidate_key': receipt['candidate_key'], 'candidate_revision': REVISION, 'patterns': item['patterns'],
               'source_receipt_status': 'BLOCKED', 'full_candidate': False, 'runtime_activation': False, 'native_execution_qualified': False}
        if matches:
            profile = matches[0]
            profile_digest = sha(canonical(profile))
            spell = base.fill_header(header['spell'], defaults[donor][0], {'key': receipt['candidate_key'], 'revision': REVISION}, raw['registrar'])
            source_harmony_cost = spell.pop('harmony_cost', None)
            display_flags = spell['requirements'].pop('vocation_display_flags', None)
            spell['requirements']['learning_required'] = False
            if raw['registrar'].get('needLearn'):
                spell['requirements']['wheel_unlock'] = True
            spell['targeting']['allow_on_self'] = defaults[donor][0]['allowOnSelf']
            spell['targeting']['check_floor'] = True
            spell['execution'] = copy.deepcopy(profile['spell']['execution'])
            deps = copy.deepcopy(profile['dependencies'])
            reader.validate_reader_shape(repo, {'spell': spell}, deps)
            errors = validate_spell.validate({'spell': spell}, deps, {'definitions': []})
            source_params = source_parameter_proposal(name, donor, source.decode())
            proposed_params = profile['spell']['execution']['native_behavior']['parameters']
            param_diff = differences(source_params, {key: proposed_params.get(key) for key in source_params})
            row.update(status='SOURCE_SCHEMA_VALID_RUNTIME_CAPABILITY_BLOCKED' if not errors else 'BLOCKED',
                       native_key=profile['spell']['execution']['native_behavior']['key'], schema_errors=errors,
                       proposed_source_parameter_patch=source_params, source_parameter_differences=param_diff,
                       canonical_descriptor_source_numeric_equivalence=False,
                       exact_native_profile_match=spell == profile['spell'] and deps == profile['dependencies'],
                       source_to_canonical_differences=differences(spell, profile['spell']),
                       source_costs_preserved=spell['costs'] == {**header['spell']['costs'], 'soul': spell['costs']['soul']},
                       canonical_costs_equal=spell['costs'] == profile['spell']['costs'],
                       reader_blocker='Full source-shaped Spell differs from closed native profile; copying canonical identity/costs/parameters does not establish source admission.',
                       source_vocation_display_flags_retained=display_flags)
            complete_focus = donor == 'canary' and name in ('focus harmony', 'focus serenity')
            if complete_focus:
                expected = native_actor_states.SOURCE_SPECS[name]['canary']['sha256']
                if sha(source) != expected or spell['execution']['native_behavior'] != native_actor_states.MODELS[name]:
                    raise ValueError('Focus exact cast/native accepted descriptor drift')
                if errors:
                    raise ValueError('Complete Focus schema invalid: ' + repr(errors))
                row.update(status='CANDIDATE_SCHEMA_VALID', full_candidate=True,
                    native_data_model_complete=True, source_alias_to_existing_native_profile=False,
                    required_operations_unrepresented=[], reader_acceptance_qualified=False,
                    runtime_capability_blocked=True, canonical_normalization_used=True,
                    source_numeric_equivalence=False, scoped_helper_proofs=scoped_focus)
                projected_receipt = copy.deepcopy(receipt)
                projected_receipt.update(status='CANDIDATE_SCHEMA_VALID', blockers=[],
                    native_execution_qualified=False, dependencies={key: len(value) for key, value in deps.items()},
                    item_owner_bindings_required=[], schema_and_semantic_validation_errors=[],
                    conversion_notes=['Accepted S5/S27 monk_focus canonical parameters with exact current Canary cast and scoped stable helper closure.',
                        'Source header and costs retained; accepted S16 learning normalization. Strict native reader admission remains blocked.'],
                    remaining_mechanics=[{'source_field': 'source.native_provider_and_admission',
                        'reason': 'Native/input provider execution, runtime strict-profile admission and assets remain unqualified.'},
                        {'source_field': 'source.Canary_flat_damage_healing',
                         'reason': 'Explicit accepted S5/S27 canonical normalization differs from source numeric helper.'}])
                validate_spell.Draft202012Validator(base.receipt_schema(), registry=validate_spell.REGISTRY).validate(projected_receipt)
                prefix = snapshot + '/' + row_id + '/'
                for filename, value in {'spell.json': {'spell': spell}, 'dependencies.json': deps,
                        'catalog.json': {'definitions': []}, 'receipt.json': projected_receipt}.items():
                    candidates[prefix + filename] = value
            row['source_harmony_cost_retained'] = source_harmony_cost
            row['required_operations_unrepresented'] = [] if complete_focus else [
                'Source-correct complete native branches/parameter mapping requires closure; proposed binding alone is insufficient']
            if not complete_focus:
                partials.append({'registration_key': reg, 'status': row['status'], 'target_data_complete': False,
                             'full_candidate': False, 'partial_target_bundle': {'spell': spell}, 'dependencies': deps,
                             'catalog': {'definitions': []}, 'native_parameters_source_equivalent': False,
                             'source_correct_parameter_patch': source_params, 'source_harmony_cost_retained': source_harmony_cost})
            bindings.append({'registration_key': reg, 'status': 'PROPOSED_NATIVE_BINDING',
                             'canonical_profile_identity': profile['spell']['identity'], 'canonical_profile_name': profile['name'],
                             'canonical_profile_sha256': profile_digest, 'profile_catalog_path': PROFILES_PATH,
                             'profile_catalog_sha256': sha(profiles_bytes), 'source_binding_qualified': False,
                             'source_receipt_promoted': False, 'runtime_activation': False})
        else:
            row.update(status='BLOCKED', native_key=None, source_to_canonical_differences=[],
                       required_capabilities=['Elemental-bond equipment Combat selection without collapsing physical/energy/earth routes',
                          'Monk Harmony gain/spend and source helper inputs/conversions',
                          'Source area/chain/target legality and source-correct native parameter contract'],
                       reader_blocker='No same-name native profile; fixed physical damage would omit source equipment/Harmony mechanics.')
        rows.append(row)
    audit = {'schema': 'OTERYN_R56_EQUIPMENT_LANE_AUDIT/v1', 'records': rows, 'record_count': 25,
             'full_candidate_count': len(candidates) // 4, 'partial_target_count': len(partials), 'proposed_native_binding_count': len(bindings),
             'remaining_original_receipts_blocked': 25 - len(candidates) // 4, 'runtime_activation': False,
             'native_execution_qualified': False, 'input_provider_equivalence': False, 'canonical_selection_changed': False,
             'native_identity_allocation': False}
    proof = {'schema': 'OTERYN_R56_EQUIPMENT_QUALIFICATION/v1', **{key: audit[key] for key in audit if key != 'records'},
             'source_pins': PINS, 'capture_path': source_library.CAPTURE_PATH, 'capture_sha256': source_library.CAPTURE_SHA,
             'native_catalog_path': PROFILES_PATH, 'native_catalog_sha256': sha(profiles_bytes),
             'native_guard_path': 'apps/game-server/src/spell/native.rs',
             'native_guard_sha256': sha((repo / 'apps/game-server/src/spell/native.rs').read_bytes()),
             'reader_path': 'apps/game-server/src/spell/executable_catalog.rs',
             'reader_sha256': sha((repo / 'apps/game-server/src/spell/executable_catalog.rs').read_bytes()),
             'policy_proofs': [{'path': POLICY_PATH, 'sha256': sha(policy), 'exact_rows': [line for line in policy.decode().splitlines() if any(line.startswith('| ' + decision + ' |') for decision in ('S5', 'S16', 'S27'))]}],
             'scoped_focus_helper_proofs': scoped_focus,
             'accepted_native_model_proof': {'path': 'docs/architecture/OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md',
                'sha256': sha((repo / 'docs/architecture/OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md').read_bytes()),
                'section': 'A.2 monk_harmony_virtue', 'acceptance_decision': 'S27',
                'focus_model_sha256': {name: sha(canonical(native_actor_states.MODELS[name])) for name in ('focus harmony', 'focus serenity')},
                'source_numeric_equivalence': False, 'open_questions_remain_owner_qualified': False},
             'producer_sha256': sha(Path(__file__).read_bytes())}
    return {**candidates, 'lane-audit.json': audit, 'partial-data.json': {'partial_targets': partials, 'proposed_native_bindings': bindings},
            'projection-proof.json': proof, 'receipt.schema.json': base.receipt_schema(),
            'import-summary.json': {'records': 25, 'full_candidates': len(candidates) // 4, 'partial_target_count': len(partials),
                'proposed_native_binding_count': len(bindings), 'blocked_count': 25 - len(candidates) // 4,
                'records_index': [{key: row[key] for key in ('registration_key', 'candidate_key', 'candidate_revision', 'status')} for row in rows]}}, headers


def write(repo, source_root):
    repo = Path(repo)
    values, headers = build(repo, source_root)
    out = repo / OUTPUT
    out.mkdir(parents=True, exist_ok=True)
    for relative, value in values.items():
        (out / relative).parent.mkdir(parents=True, exist_ok=True)
        (out / relative).write_text(json.dumps(value, sort_keys=True, ensure_ascii=False, indent=2) + '\n')
    for relative, data in headers.items():
        path = out / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    manifest = {'files': {str(path.relative_to(out)): sha(path.read_bytes()) for path in sorted(out.rglob('*')) if path.is_file() and path.name != 'package-manifest.json'}}
    (out / 'package-manifest.json').write_text(json.dumps(manifest, sort_keys=True, indent=2) + '\n')
    return manifest


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--repo', default='.')
    parser.add_argument('--source-root', default='/workspace/spell-sources')
    args = parser.parse_args()
    print(json.dumps(write(args.repo, args.source_root), sort_keys=True))
