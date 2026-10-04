#!/usr/bin/env python3
"""Six standard canonical bundles under accepted S5; source donor numeric parity is false.

Original source helpers remain immutable in r28/r50; no runtime paths are changed.
"""
import argparse
import copy
import gzip
import json
from pathlib import Path

import import_source_player_bundles as base
import source_monk_formula_library as source_library
import project_player_control_candidates as reader_adapter
import validate_spell

HERE = Path(__file__).resolve().parent
OUTPUT = Path('docs/reference/spells/r53-source-closure')
REVISION = 'canonical-player-r53'
POLICY_PATH = 'docs/architecture/OTERYN_SPELL_AUTHORING_SCHEMA_V1.md'
PR_SHA = 'b8597f8579bb09eed600f3a2f1238be392fe3f59'
MAIN_SHA = 'eaa401001d5a91cc1cb62385223b6ad4d23111ac'
RUNTIME_FILES = {
    'formula.rs': 'c7ba1beb18d5d9a2d2c317b916e243ca82cd61f89ea2103f3fc9217da2407e3e',
    'authoring.rs': '29167b4f72f9b723b8578b00c06a9f13a01e5153ec64b7f051b079721b599c1f',
    'owned_cast_facts.rs': '7b4f6c6b4a3380e709430cec5a5ae222d1dfd3b86843ed29e76fd292f41c1c36',
}
sha = source_library.sha
canonical = source_library.canonical


def normalize(tree):
    if set(tree) == {'fn', 'args'}:
        if tree != {'fn': 'flat_damage_healing', 'args': [{'var': 'level'}]}:
            raise ValueError('normalization covers only the exact source flat helper')
        return {'fn': 'level_base_damage_healing', 'args': [{'var': 'level'}]}
    if set(tree) in ({'const'}, {'var'}):
        return copy.deepcopy(tree)
    if set(tree) == {'op', 'args'} and tree['op'] in {'add', 'sub', 'mul', 'div'} and len(tree['args']) == 2:
        return {'op': tree['op'], 'args': [normalize(arg) for arg in tree['args']]}
    raise ValueError('unrepresented formula operation')


def consumer_proof(runtime_cache):
    texts = {}
    for filename, expected in RUNTIME_FILES.items():
        data = (Path(runtime_cache) / filename).read_bytes()
        if sha(data) != expected:
            raise ValueError('pinned consumer file SHA changed: ' + filename)
        texts[filename] = data.decode()
    if ('LevelBaseDamageHealing(Box<Expression>)' not in texts['formula.rs']
            or '(Some("level_base_damage_healing"), Ok([level]))' not in texts['authoring.rs']
            or '"builder" => HarmonyRole::Builder' not in texts['authoring.rs']):
        raise ValueError('canonical helper/builder consumer capability absent')
    return {'pr_number': 1534, 'pinned_pr_sha': PR_SHA, 'files': RUNTIME_FILES,
            'main_sha': MAIN_SHA, 'main_evidence_method': 'independent_scoped_GitHubContentsAPI_enum44_105_read',
            'main_scoped_result': 'Input8_Expression_LevelBaseDamageHealing', 'main_whole_file_sha_claimed': False,
            'canonical_function_supported': True, 'builder_flag_supported': True}


def build(repo, source_root, runtime_cache):
    repo = Path(repo)
    policy = (repo / POLICY_PATH).read_bytes()
    s5_rows = [line for line in policy.decode().splitlines() if line.startswith('| S5 |')]
    if len(s5_rows) != 1 or "Canary's `calculateFlatDamageHealing` is a defect and is not used." not in s5_rows[0] or '`level_base_damage_healing`' not in s5_rows[0]:
        raise ValueError('accepted S5 canonical normalization decision changed')
    s16_rows = [line for line in policy.decode().splitlines() if line.startswith('| S16 |')]
    if len(s16_rows) != 1 or '`learning_required` is false for every spell' not in s16_rows[0] or 'else by the Canary 15.30 `needLearn`' not in s16_rows[0]:
        raise ValueError('accepted S16 automatic learning/wheel unlock decision changed')
    runtime = consumer_proof(runtime_cache)
    source_path = repo / 'imports/spells/r50/source-programs/source-monk-formula-library.json.gz'
    library = json.loads(gzip.decompress(source_path.read_bytes()))
    source_library.validate_library(library, repo)
    captures = source_library.rows(repo, source_library.CAPTURE_PATH, source_library.CAPTURE_SHA)
    defaults, default_proofs = base.default_fields(Path(source_root) / 'canary', source_library.PIN)
    source_engine = base.source_file(Path(source_root) / 'canary', source_library.PIN, 'src/creatures/combat/spells.cpp')
    source_floor_check = base.source_cpp_function(source_engine, 'InstantSpell::canThrowSpell')
    if b'fromPos.z != toPos.z' not in source_floor_check:
        raise ValueError('source instant floor check changed')
    packet, records = {}, []
    for source_formula in library['formula_definitions']:
        registration = source_formula['registration_key']
        row_id = sha(registration.encode())[:16]
        row_path = 'canary-main-current/' + row_id + '/'
        old_path = repo / 'imports/spells/r28/player-source-bundles' / row_path
        header = json.loads((old_path / 'source-header.json').read_text())
        old_receipt = json.loads((old_path / 'receipt.json').read_text())
        raw = captures[registration]['source_callback_facts']
        key = 'candidate:spell/source/canary-main-current/' + row_id
        def identity(suffix=''):
            return {'key': key + suffix, 'revision': REVISION}
        def reference(family, suffix):
            return {'family': family, **identity(suffix)}
        spell = base.fill_header(header['spell'], defaults, identity(), raw['registrar'])
        display_flags = spell['requirements'].pop('vocation_display_flags', None)
        spell['targeting']['allow_on_self'] = defaults['allowOnSelf']
        spell['targeting']['check_floor'] = True
        source_learning_required = spell['requirements']['learning_required']
        spell['requirements']['learning_required'] = False
        # S16 explicitly maps current Canary needLearn to a Wheel revelation unlock.
        spell['requirements']['wheel_unlock'] = bool(raw['registrar'].get('needLearn', False))
        spell['execution'] = {'ability': reference('Ability', '/ability')}
        captured_formula = raw['combats'][0]['callbacks'][0]['formula']
        magnitude = [{'op': 'abs', 'args': [normalize(captured_formula[bound])]} for bound in ('minimum', 'maximum')]
        formula = {'identity': identity('/formula'), 'kind': 'player_expression', 'inputs': 'skill',
                   'minimum': {'op': 'min', 'args': copy.deepcopy(magnitude)},
                   'maximum': {'op': 'max', 'args': copy.deepcopy(magnitude)}}
        params = raw['combats'][0]['parameters']
        ability = {'identity': identity('/ability'), 'kind': 'spell',
                   'needs_target': spell['targeting']['needs_target'], 'needs_direction': spell['targeting']['needs_direction'],
                   'range_tiles': spell['targeting'].get('range_tiles', 0), 'effects': [reference('Effect', '/effect')]}
        if raw['combats'][0].get('areas'):
            area = raw['combats'][0]['areas'][0]['north']
            ability['area'] = {'matrix': {'north': [''.join({0: '.', 1: 'x', 2: 'c', 3: 'C'}[cell] for cell in row) for row in area]}}
        effect = {'identity': identity('/effect'), 'operation': 'damage', 'damage_type': 'physical',
                  'formula': reference('Formula', '/formula'), 'mitigated_by': ['armor'],
                  'presentation': {'impact_asset_binding': 'canary.appearance:effect/' + params['COMBAT_PARAM_EFFECT'].removeprefix('CONST_ME_').lower()}}
        dependencies = {'abilities': [ability], 'effects': [effect], 'formulas': [formula]}
        catalog = {'definitions': []}
        reader_adapter.validate_reader_shape(repo, {'spell': spell}, dependencies)
        validation_errors = validate_spell.validate({'spell': spell}, dependencies, catalog)
        if validation_errors:
            raise ValueError('canonical bundle schema/semantic validation: ' + ' | '.join(validation_errors))
        receipt = copy.deepcopy(old_receipt)
        receipt.update(status='CANDIDATE_SCHEMA_VALID', blockers=[], native_execution_qualified=False,
                       dependencies={name: len(values) for name, values in dependencies.items()},
                       item_owner_bindings_required=[], schema_and_semantic_validation_errors=[],
                       conversion_notes=['Accepted S5 explicitly normalizes source flat_damage_healing to canonical level_base_damage_healing.',
                                         'Source headers remain exact evidence; S16 normalizes learning_required to false and current Canary needLearn to wheel_unlock.',
                                         'Base powers, arithmetic order, Builder role, area and presentation retained.',
                                         'Canonical normalization used; exact Canary numeric equivalence FALSE; originals remain in r28/r50.'],
                       remaining_mechanics=[{'source_field': 'source.COMBAT_PARAM_USECHARGES', 'reason': 'Exact true declaration retained in proof; live weapon charge/provider execution remains unqualified.'},
                           {'source_field': 'source.native_input_provider_and_combat', 'reason': 'Canonical formula/reader support qualify bundle data only. Live equipment/default fallback, distribution/secondary split, armor metadata, assets and admission remain unqualified.'},
                           {'source_field': 'source.Canary_flat_damage_healing', 'reason': 'Intentional accepted S5 normalization; canonical level curve differs numerically from the source helper.'}])
        validate_spell.Draft202012Validator(base.receipt_schema(), registry=validate_spell.REGISTRY).validate(receipt)
        for name, value in {'source-header.json': header, 'receipt.json': receipt, 'spell.json': {'spell': spell},
                            'dependencies.json': dependencies, 'catalog.json': catalog}.items():
            packet[row_path + name] = value
        records.append({'registration_key': registration, 'candidate_key': key, 'candidate_revision': REVISION,
                        'status': 'CANDIDATE_SCHEMA_VALID', 'canonical_normalization_used': True,
                        'source_numeric_equivalence': False, 'native_execution_qualified': False,
                        'input_provider_equivalence': False, 'source_need_weapon_default': defaults['needWeapon'],
                        'source_learning_required': source_learning_required,
                        'canonical_learning_required': False,
                        'canonical_wheel_unlock': spell['requirements']['wheel_unlock'],
                        'source_capture_fact_sha256': source_formula['capture_fact_sha256'],
                        'source_vocation_display_flags_retained': display_flags,
                        'actual_reader_shape_valid': True,
                        'source_formula_library_key': source_formula['key'], 'source_sha256': source_formula['source_sha256'],
                        'target_parts': {'Spell': 1, 'Ability': 1, 'Effect': 1, 'Formula': 1},
                        'source_combat_parameters': params, 'source_harmony_role': raw['registrar']['monkSpellType'],
                        'consumer_capabilities': {'builder_flag_decoded': True, 'standard_ability_effect_formula_reader': True,
                            'source_flat_helper_used': False, 'canonical_formula_decoded': True,
                            'source_no_weapon_defaults_equal': False, 'useCharges_provider_equal': False,
                            'mitigated_by_metadata_consumed': False}, 'current_validation_errors': validation_errors})
    proof = {'schema': 'OTERYN_R53_CANONICAL_MONK_PROJECTION/v1', 'records': records, 'record_count': 6,
             'standard_bundle_count': 6, 'candidate_schema_valid_count': 6,
             'canonical_normalization_used': True, 'source_numeric_equivalence': False, 'input_provider_equivalence': False, 'executable_spell_count': 0,
             'runtime_activation': False, 'native_execution_qualified': False, 'canonical_selection_changed': False,
             'native_identity_allocation': False, 'runtime_paths_modified': False, 'target_shared_schema_modified': False,
             'source_revision': source_library.PIN, 'source_library_gzip_sha256': sha(source_path.read_bytes()),
             'consumer_proof': runtime,
             'source_capture_path': source_library.CAPTURE_PATH, 'source_capture_sha256': source_library.CAPTURE_SHA,
             'policy_proof': {'path': POLICY_PATH, 'sha256': sha(policy), 'section': '5. Decisions', 'decision': 'S5', 'exact_row': s5_rows[0],
                              'additional_decision': 'S16', 'additional_exact_row': s16_rows[0]},
             'engine_default_proofs': default_proofs, 'producer_sha256': sha(Path(__file__).read_bytes())}
    proof['reader_proof'] = {'path': 'apps/game-server/src/spell/executable_catalog.rs',
                            'sha256': sha((repo / 'apps/game-server/src/spell/executable_catalog.rs').read_bytes()),
                            'adapter_sha256': sha(Path(reader_adapter.__file__).read_bytes()),
                            'deny_unknown_fields_check': True, 'required_targeting_fields_check': True}
    proof['source_floor_check_proof'] = {'source_revision': source_library.PIN,
                                       'path': 'src/creatures/combat/spells.cpp',
                                       'sha256': sha(source_engine), 'function': 'InstantSpell::canThrowSpell',
                                       'function_sha256': sha(source_floor_check), 'same_floor_required': True}
    packet['projection-qualification.json'] = proof
    packet['receipt.schema.json'] = base.receipt_schema()
    packet['import-summary.json'] = {'schema': 'OTERYN_SOURCE_PLAYER_BUNDLE_IMPORT/v1', 'revision': REVISION,
        'records': 6, 'source_populations': {'canary-main-current': 6}, 'status_counts': {'CANDIDATE_SCHEMA_VALID': 6},
        'native_descriptor_count': 0, 'all_receipts_schema_valid': True, 'full_source_mechanics_1_to_1_complete': False,
        'runtime_activation': False, 'native_execution_qualified': False, 'input_provider_equivalence': False,
        'canonical_normalization_used': True, 'source_numeric_equivalence': False, 'canonical_selection_changed': False,
        'native_identity_allocation': False, 'records_index': [{**{key: row[key] for key in ['registration_key', 'candidate_key', 'candidate_revision', 'status']}, 'blockers': []} for row in records]}
    return packet


def write(repo, source_root, runtime_cache):
    repo = Path(repo)
    packet = build(repo, source_root, runtime_cache)
    out = repo / OUTPUT
    for relative, value in packet.items():
        path = out / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(value, sort_keys=True, ensure_ascii=False, indent=2) + '\n')
    manifest = {'schema': 'OTERYN_CANONICAL_PLAYER_PROJECTION_PACKAGE/v1',
                'files': {str(path.relative_to(out)): sha(path.read_bytes()) for path in sorted(out.rglob('*')) if path.is_file() and path.name != 'package-manifest.json'}}
    (out / 'package-manifest.json').write_text(json.dumps(manifest, sort_keys=True, indent=2) + '\n')
    return manifest


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--repo', default='.')
    parser.add_argument('--source-root', default='/workspace/spell-sources')
    parser.add_argument('--runtime-cache', default='/workspace/spell-source-closure/runtime1534-formula-read')
    args = parser.parse_args()
    print(json.dumps(write(args.repo, args.source_root, args.runtime_cache), sort_keys=True))
