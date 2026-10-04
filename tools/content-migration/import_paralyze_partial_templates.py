#!/usr/bin/env python3
"""Import partial Paralyze source templates while retaining both full registrations BLOCKED."""
import argparse
from collections import Counter
import gzip
import json
import os
from pathlib import Path
import subprocess

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

from import_source_spell_package import ROOT, bundle_member_path, digest, require
from source_spell_import_guards import read_base, write_source_only_set

BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
QUALIFIED_PACKET_SHA = 'cef0cc9ca486a10c1ff08f26a36fcf8a9392036c6f7e9a0e469c91e805f1ac83'
QUALIFIED_RECEIPT_SHA = 'b701c1f3d6a57c84cc790638681e4a6fb10313f04457e07a14f6c617f18c6611'
SOURCE_PATH = 'data/scripts/runes/paralyze_rune.lua'
KEYS = {snapshot + '/' + SOURCE_PATH + '#1' for snapshot in ('canary-main-current', 'crystal-summer-current')}
FALSE_FLAGS = ('model_projection_equivalence', 'source_program_fully_represented', 'runtime_activation', 'native_execution_qualified', 'canonical_selection_changed', 'native_identity_allocation')


def verify_row(row, callback, header_body, receipt_body):
    source = row['source_identity']; header = json.loads(header_body)['spell']; receipt = json.loads(receipt_body)
    require(row['registration_key'] in KEYS and source['snapshot'] + '/' + source['path'] + '#1' == row['registration_key']
            and source['sha256'] == callback['source_sha256'] and source['revision'] == callback['source_revision']
            and source['git_blob'] == callback['source_callback_facts']['blob'] and row['source_header'] == header
            and row['source_header_sha256'] == digest(header_body) and row['historical_receipt_sha256'] == digest(receipt_body)
            and receipt['status'] == row['full_spell_status'] == 'BLOCKED', 'PARALYZE_PARTIAL_SOURCE_JOIN_MISMATCH')
    require(row['candidate_count'] == 0 and row['executable_spell_exported'] is False and row['runtime_blocked'] is True
            and all(row[k] is False for k in FALSE_FLAGS), 'PARALYZE_COMPLETE_SPELL_CLAIM_REFUSED')
    dependencies = row['dependency_templates']
    require(all(len(dependencies[k]) == 1 for k in ('abilities', 'effects', 'formulas')), 'PARALYZE_PARTIAL_DEPENDENCY_COHORT_MISMATCH')
    ability, effect, formula = (dependencies[k][0] for k in ('abilities', 'effects', 'formulas'))
    require(effect['operation'] == 'condition' and effect['duration_ms'] == 6000
            and effect['condition'] == {'type': 'paralyze', 'lifetime': 'fixed_duration',
                'speed_formula': {'family': 'Formula', **formula['identity']}}
            and formula['kind'] == 'speed_modifier' and formula['speed'] == {
                'maximum_multiplier': {'denominator': 1, 'numerator': 0}, 'minimum_multiplier': {'denominator': 1, 'numerator': 0},
                'maximum_offset': 40, 'minimum_offset': 40}
            and ability['effects'] == [{'family': 'Effect', **effect['identity']}]
            and ability['zero_damage_health_path'] is True and ability['range_tiles'] == 0,
            'PARALYZE_SOURCE_CONDITION_MISMATCH')
    donor = source['source']
    require(effect['presentation'] == {'caster_effect_asset_binding': donor + '.appearance:effect/magic_green',
            'caster_effect_timing': 'after_success', 'impact_asset_binding': donor + '.appearance:effect/magic_red'},
            'PARALYZE_DONOR_PRESENTATION_MISMATCH')
    require(row['source_cast_program'] == [
            {'arguments': ['creature', 'var'], 'operation': 'execute_combat', 'order': 0, 'receiver': 'combat', 'return_binding': 'execute_result'},
            {'binding': 'execute_result', 'operation': 'return_false_if_execute_false', 'order': 1},
            {'constant': 'CONST_ME_MAGIC_GREEN', 'operation': 'caster_position_magic_effect_if_execute_true', 'order': 2},
            {'operation': 'return_true', 'order': 3}], 'PARALYZE_SOURCE_CAST_PROGRAM_MISMATCH')
    proof = row['cpp_control_proof']
    require(proof['baseline_primary_secondary_values'] == [0, 0] and proof['source_combat_type'] == 'COMBAT_UNDEFINEDDAMAGE'
            and proof['owning_order'] == ['combatBlockHit', 'combatChangeHealth', 'conditional_CombatConditionFunc', 'conditional_CombatDispelFunc']
            and proof['caster_success_scope'] == 'Lua_Combat_execute_boolean_not_health_or_condition_application'
            and proof['number_variant_result'] == 'initialized_true_doCombat_return_not_assigned'
            and all(proof[k] is False for k in ('external_event_and_buff_mutation_qualified', 'final_damage_value_qualified', 'cpp_execution_qualified')),
            'PARALYZE_CPP_RETURN_CONTRACT_MISMATCH')
    speed = proof['speed_normalization']
    require(speed['source_coefficients'] == [-1, 0, -1, 0] and speed['normalized_base_domain_target_speed'] == 40
            and speed['base_speed_type'] == 'uint16_t' and speed['source_formula_variable'] == 'base_speed_minus_40'
            and speed['condition_delta_projection'] == '40_minus_base_speed'
            and all(speed[k] is False for k in ('final_creature_speed_qualified', 'provider_execution_qualified', 'stacking_and_var_speed_provider_qualified')),
            'PARALYZE_SOURCE_SPEED_PROJECTION_MISMATCH')


def prepare_import(root, source=None, repositories=Path('/workspace/spell-sources')):
    source = source or root / 'docs/reference/spells/r45-source-closure'
    base, base_body, baseline, schemas, _ = read_base(root, BASE_SHA)
    proof_name, artifact = 'source-paralyze-partial-receipt.json', 'source-paralyze-partial-templates.jsonl.gz'
    proof_body = bundle_member_path(source, proof_name).read_bytes(); proof = json.loads(proof_body)
    require(digest(proof_body) == QUALIFIED_RECEIPT_SHA, 'PARALYZE_QUALIFIED_RECEIPT_HASH_MISMATCH')
    compressed = bundle_member_path(source, artifact).read_bytes(); payload = gzip.decompress(compressed)
    require(QUALIFIED_PACKET_SHA is not None and digest(compressed) == QUALIFIED_PACKET_SHA == proof['gzip_sha256'] and digest(payload) == proof['payload_sha256'], 'PARALYZE_QUALIFIED_PACKET_HASH_MISMATCH')
    schema_path = 'tools/content-schema/spell-authoring/source-paralyze-partial-templates.schema.json'
    schema_body = (root / schema_path).read_bytes(); schema = json.loads(schema_body)
    require(digest(schema_body) == proof['schema_sha256'], 'PARALYZE_PARTIAL_SCHEMA_PIN_MISMATCH')
    for name, expected in proof['existing_schema_proofs'].items():
        matches = [body for path, body in schemas.items() if Path(path).name == name]
        require(len(matches) == 1 and digest(matches[0]) == expected, 'PARALYZE_ARCHIVED_DEPENDENCY_SCHEMA_MISMATCH')
    registry = Registry().with_resources((ref['uri'], Resource.from_contents(json.loads(schemas[ref['path']]))) for ref in baseline['schemaRefs'])
    validator = Draft202012Validator(schema, registry=registry)
    callback_body = (base / 'player-source-bundles/source-callback-facts.jsonl.gz').read_bytes()
    callbacks = {r['registration_key']: r for r in map(json.loads, gzip.decompress(callback_body).splitlines())}
    rows = [json.loads(line) for line in payload.splitlines()]
    require(len(rows) == len(KEYS) and {r['registration_key'] for r in rows} == KEYS, 'PARALYZE_PARTIAL_SOURCE_POPULATION_MISMATCH')
    pins = {r['source']: r['revision'] for r in baseline['source_pins']['monster_donors']}; verified = {}
    require(proof['source_revisions'] == {snapshot: pins[snapshot.split('-')[0]] for snapshot in ('canary-main-current', 'crystal-summer-current')}, 'PARALYZE_RECEIPT_SOURCE_PINS_MISMATCH')
    for row in rows:
        validator.validate(row); reg = row['registration_key']; folder = base / 'player-source-bundles' / reg.split('/')[0] / digest(reg.encode())[:16]
        verify_row(row, callbacks[reg], (folder / 'source-header.json').read_bytes(), (folder / 'receipt.json').read_bytes())
        require(row['source_capture_gzip_sha256'] == digest(callback_body), 'PARALYZE_FROZEN_CAPTURE_PIN_MISMATCH')
        source_ref = row['source_identity']; donor = source_ref['source']; require(source_ref['revision'] == pins[donor], 'PARALYZE_SOURCE_REVISION_MISMATCH')
        for ref in row['cpp_control_proof']['owning_source_facts']:
            path = ref['path']; require(not Path(path).is_absolute() and '..' not in Path(path).parts, 'PARALYZE_CPP_SOURCE_PATH_REFUSED')
            key = donor, path
            if key not in verified:
                body = subprocess.check_output(['git', '-C', str(repositories / donor), 'show', source_ref['revision'] + ':' + path], env={**os.environ, 'GIT_NO_LAZY_FETCH': '1'})
                verified[key] = digest(body)
            require(verified[key] == ref['file_sha256'], 'PARALYZE_CPP_SOURCE_FILE_HASH_MISMATCH')
    counts = {'source_records': len(rows), 'partial_abilities': sum(len(r['dependency_templates']['abilities']) for r in rows),
              'source_backed_speed_formulas': sum(len(r['dependency_templates']['formulas']) for r in rows),
              'partial_condition_effects': sum(len(r['dependency_templates']['effects']) for r in rows), 'full_spell_status_counts': dict(Counter(r['full_spell_status'] for r in rows)),
              'candidate_count': 0, 'executable_spell_count': 0, 'full_spell_candidates': 0}
    require(counts['source_records'] == proof['records'] and all(counts[k] == proof[k] for k in ('partial_abilities', 'partial_condition_effects', 'source_backed_speed_formulas', 'full_spell_status_counts', 'candidate_count', 'executable_spell_count'))
            and proof['runtime_blocked'] is True and all(proof[k] is False for k in FALSE_FLAGS), 'PARALYZE_PARTIAL_COUNT_OR_QUALIFICATION_MISMATCH')
    local_schema = 'schemas/source-paralyze-partial-templates.schema.json'; schemas[local_schema] = schema_body; source_path = source.relative_to(root).as_posix()
    data = {**schemas, 'base-r28/import-manifest.json': base_body, 'evidence/' + artifact: compressed, 'evidence/' + proof_name: proof_body}
    def origin(path):
        if path.startswith('evidence/'): return source_path + '/' + path.split('/', 1)[1]
        if path == local_schema: return schema_path
        return 'imports/spells/r28/import-manifest.json' if path.startswith('base-r28/') else 'imports/spells/r28/' + path
    manifest = {'schema': 'OTERYN_PARALYZE_PARTIAL_TEMPLATE_IMPORT/v1', 'revision': 45, 'admission_status': 'source_only_not_active',
                **{k: False for k in FALSE_FLAGS}, 'runtime_blocked': True, 'input_provider_equivalence': False, 'full_spell_candidates': 0,
                'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': BASE_SHA, 'snapshot': 'base-r28/import-manifest.json'},
                'source_pins': baseline['source_pins'], 'source_keys': sorted(KEYS), 'counts': counts, 'source_metadata_path': 'evidence/' + artifact,
                'schemaRefs': baseline['schemaRefs'] + [{'uri': local_schema, 'path': local_schema, 'sha256': digest(schema_body)}],
                'artifacts': [{'path': p, 'sourcePath': origin(p), 'sha256': digest(b), 'bytes': len(b), 'role': 'partial_source_template' if p.startswith('evidence/') else 'reference_schema' if p.startswith('schemas/') else 'base_manifest',
                               'schemaRefs': [local_schema] if p == 'evidence/' + artifact else []} for p, b in sorted(data.items())],
                'limits': ['Both full Paralyze Rune registrations remain BLOCKED; no complete Spell candidate or executable preview is exported.',
                           'Condition and Ability templates are partial projections; Lua execute-return semantics conflict with the existing after-success interpretation.',
                           'Source runtime, damage providers, assets, Item bindings and canonical selection remain unqualified and unchanged.']}
    return data, manifest


def write_import(root, destination, data, manifest):
    require(manifest['revision'] == 45 and manifest['full_spell_candidates'] == 0 and manifest['runtime_blocked'] is True
            and all(manifest[k] is False for k in FALSE_FLAGS), 'PARALYZE_COMPLETE_SPELL_CLAIM_REFUSED')
    return write_source_only_set(root, destination, 'imports/spells/r45', data, manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument('--source', type=Path); parser.add_argument('--source-repositories', type=Path, default=Path('/workspace/spell-sources')); args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.source, args.source_repositories); print(json.dumps(write_import(ROOT, ROOT / 'imports/spells/r45', data, manifest), indent=2))


if __name__ == '__main__': main()
