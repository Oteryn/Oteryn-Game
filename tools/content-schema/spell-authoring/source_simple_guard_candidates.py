"""R47 bounded guard/action evidence for fourteen current donor registrations.

This emits no executable Spell candidate. The existing authoring schema cannot
represent the exact wrappers. Ordered lexical events retain branch markers;
source semantic facts describe bounded declarations, never an executable CFG.
"""
import argparse
import copy
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

import export_source_guards
import jsonschema

HERE = Path(__file__).resolve().parent
PINS = {'canary': '04b83b512114bfd888000d6e1433ed8ecaec7c5b',
        'crystal': '00ce02a57ca5a12e48f32a3476e37471167e4c3f'}
BASE = Path('docs/reference/spells/r28-source-closure/player-source-bundles')
OUTPUT = Path('docs/reference/spells/r47-source-closure')
SCHEMA_ID = 'OTERYN_SOURCE_SIMPLE_GUARD_EVIDENCE/v1'
SCHEMA_FILE = 'source-simple-guard-evidence.schema.json'


def spec(donor, path, digest, family, semantics, gaps):
    population = 'canary-main-current' if donor == 'canary' else 'crystal-summer-current'
    return {'registration_key': population + '/' + path + '#1', 'donor': donor,
            'source_file': path, 'source_sha256': digest, 'family': family,
            'semantics': semantics, 'gaps': gaps}


SPECS = []
for name, digest in [('intense', '2640a83fe4f033222d0ee9c2e5b55014f453b8f42fdc1c39ff86506ee03bcb7e'),
                     ('ultimate', '7b3ef8a4f3ef7214ec6b0024b5151c02af102ae70400d671dd12414fa728728b')]:
    SPECS.append(spec('canary', f'data/scripts/runes/{name}_healing_rune.lua', digest,
                     'canary_healing_rune_refusal',
                     {'monster_lookup_number_argument': 1073762188,
                      'extra_getNumber_argument_semantics_qualified': False,
                      'monster_refusal_message': 'Sorry, not possible.', 'refusal_effect': 'poff',
                      'requires_player_conversion': name == 'ultimate',
                      'blocked_vocation_name': 'exalted monk' if name == 'ultimate' else None,
                      'combat_only_after_guards': True},
                     ['pre_combat_monster_refusal_with_message_and_effect',
                      'Lua_Variant_getNumber_extra_argument_not_an_authored_default'] +
                     (['player_conversion_and_exalted_monk_refusal_before_combat'] if name == 'ultimate' else [])))
for donor, digest in [('canary', '5864935eca52d88b28480c58cd9574d6c95a4090cd0bc66fb11a68ce15b95164'),
                      ('crystal', 'b6084a7b93affb998e9acafb224657189881c513e66ab80814d6b4606ee0a02a')]:
    secondary = donor == 'crystal'
    SPECS.append(spec(donor, "data/scripts/spells/healing/nature's_embrace.lua", digest,
                     'nature_self_refusal',
                     {'self_refusal_only_for_player_caster': True,
                      'refusal_message': "You can't cast this spell to yourself.", 'refusal_effect': 'poff',
                      'primary_result_returned': True, 'secondary_helper_called_after_primary': secondary,
                      'secondary_helper_runs_even_when_primary_returns_false': secondary,
                      'secondary_ratio': '0.30' if secondary else None,
                      'secondary_selection': 'nearest_same_floor_living_party_member_excluding_caster_and_primary_strict_distance_tie_keeps_first' if secondary else None,
                      'secondary_selector_has_viewport_check': False,
                      'secondary_variant_argument': 'numeric_creature_id' if secondary else None},
                     ['player_only_self_guard_and_refusal_before_combat'] +
                     (['shared_conservation_second_combat_after_primary_regardless_of_primary_result',
                       'party_member_order_and_30_percent_scaled_floor_formula'] if secondary else [])))
for name, digest in [('blood_rage', 'c8140f7aae498b098b93f4be59a11f06b855763fa5589a8928fd30bd2bce1ebe'),
                     ('protector', 'fbe9842a435a642cdaacd60aadaedbcf81d41a187822a27ed69a08374efdd450')]:
    SPECS.append(spec('canary', f'data/scripts/spells/support/{name}.lua', digest,
                     'attribute_condition_remove_before_combat',
                     {'condition': 'attributes', 'condition_id': 'CONDITIONID_COMBAT',
                      'sub_id': 'AttrSubId_BloodRageProtector', 'remove_only_if_present': True,
                      'remove_before_combat': True, 'removal_result_used': False},
                     ['conditional_caster_remove_with_exact_condition_id_and_subid_before_combat',
                      'current_source_attribute_parameters_and_replacement_semantics']))
for donor in PINS:
    SPECS.append(spec(donor, 'data/scripts/spells/support/cancel_magic_shield.lua',
                     '8a355ea72ef070e5e87698dc95f45c8608adc25f7c8fa88bcaf3e3ccf0ac0000',
                     'mana_shield_remove_before_combat',
                     {'condition': 'mana_shield', 'remove_only_if_present': False,
                      'remove_before_combat': True, 'removal_result_used': False,
                      'combat_presentation_effect': 'magic_blue'},
                     ['unconditional_caster_remove_before_combat_legality_not_post_combat_dispel']))
    SPECS.append(spec(donor, 'data/scripts/spells/support/expose_weakness.lua',
                     'cf1ba67e85769725b82e0a74d34b72b7242e5cdeabe3f6acbd7b1fbcaad5df8f',
                     'current_target_variant_and_callback',
                     {'current_target_overrides_incoming_variant_if_present': True,
                      'variant_constructor_argument': 'creature_object',
                      'variant_constructor_execution_qualified': False,
                      'target_callback_rejects_players': True,
                      'target_callback_returns_true_for_summons_without_condition': True,
                      'condition_duration_ms': 16000, 'damage_received_percent': 105,
                      'drain_body_grade_source': 'upgradeSpellsWOD("Drain_Body_Spells")'},
                     ['current_target_Variant_creature_object_override_before_combat',
                      'targetcreature_callback_player_summon_branches_and_Wheel_grade']))
    SPECS.append(spec(donor, 'data/scripts/spells/support/find_person.lua',
                     '5635f5bc286cd958755d03f8049915c292e0d3f1e725b5091df02cfd0890f1d6',
                     'find_person_source_distance_bands',
                     {'distance_metric': 'chebyshev_xy', 'source_bands_tiles': [5, 101, 275],
                      'existing_descriptor_bands_tiles': [5, 101, 251],
                      'direction_tangents': ['0.4142', '2.4142'],
                      'hide_staff_from_nonstaff': True, 'result_message_class': 'MESSAGE_LOOK',
                      'success_effect': 'magic_blue', 'failure_effect': 'poff'},
                     ['locate_message_schema_const_251_conflicts_with_source_275',
                      'name_resolution_and_engine_spending_closure_not_qualified_by_identical_script']))
for name, digest, coefficient, numeric in [
        ('intense', 'ea6a42b9103d2c69593515c1342e7d429e08c5dbac09df503d6174b84e6a28f2', ['5.4', '40'], True),
        ('ultimate', '3ef49893c221a984ff4688c343b03ebdc29989b9ba5d0b505689e638198ef210', ['12.4', '90'], False)]:
    SPECS.append(spec('crystal', f'data/scripts/runes/{name}_healing_rune.lua', digest,
                     'crystal_healing_rune_leiden_and_self',
                     {'target_selection': 'Creature_variant_number_or_current_target',
                      'reject_missing_or_noncreature_target': True, 'leiden_name_casefolded': 'leiden',
                      'leiden_direct_health_delta_formula': {'level_divisor': '5', 'magic_multiplier': coefficient[0],
                                                           'addition': coefficient[1], 'sign': -1},
                      'leiden_effect': 'magic_blue', 'leiden_success_returns_true': True,
                      'reject_other_monsters': True, 'reject_other_character': True,
                      'self_refusal_message': 'You may only use this rune on yourself.',
                      'combat_variant_argument': 'numeric_target_id' if numeric else 'original_incoming_variant'},
                     ['ordered_Leiden_direct_addHealth_branch_before_monster_and_self_refusal',
                      'source_target_fallback_and_variant_route_not_plain_healing_Combat']))
SPECS.sort(key=lambda value: value['registration_key'])


def sha(value):
    return hashlib.sha256(value).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def object_schema(properties):
    return {'type': 'object', 'additionalProperties': False, 'properties': properties, 'required': list(properties)}


def evidence_schema():
    string = {'type': 'string', 'minLength': 1}
    digest = {'type': 'string', 'pattern': '^[0-9a-f]{64}$'}
    event_schema = json.loads((HERE / 'player-source-guards.schema.json').read_text())['properties']['events']
    return {'$schema': 'https://json-schema.org/draft/2020-12/schema', '$id': 'urn:oteryn:source-simple-guard-evidence:1',
            **object_schema({'schema': {'const': SCHEMA_ID}, 'registration_key': string,
                             'source_revision': {'enum': list(PINS.values())}, 'source_file': string,
                             'source_sha256': digest, 'source_git_blob': {'type': 'string', 'pattern': '^[0-9a-f]{40}$'},
                             'historical_capture_fact_sha256': digest, 'historical_receipt_sha256': digest,
                             'historical_receipt_status': {'const': 'BLOCKED'},
                             'source_semantics': {'oneOf': [{'const': {'family': value['family'], **value['semantics']}}
                                                           for value in unique_semantic_specs()]},
                             'ordered_source_events': event_schema,
                             'source_binding_anchors': {'type': 'array', 'items': object_schema({
                                 'source_line': {'type': 'integer', 'minimum': 1},
                                 'expression': string, 'resolution': {'const': 'source_expression_not_executable'}})},
                             'source_order_scope': {'const': 'whole_file_lexical_not_executable_CFG'},
                             'existing_schema_gaps': {'type': 'array', 'minItems': 1, 'uniqueItems': True, 'items': string},
                             'complete_spell_candidate': {'const': False}, 'runtime_activation': {'const': False},
                             'native_execution_qualified': {'const': False}, 'external_sources_used': {'const': False}})}


def unique_semantic_specs():
    found = {}
    for value in SPECS:
        found[canonical({'family': value['family'], **value['semantics']})] = value
    return list(found.values())


def qualify(raw, fact, value):
    if (sha(raw) != value['source_sha256'] or fact['source_sha256'] != value['source_sha256']
            or fact['registration_key'] != value['registration_key'] or fact['source_revision'] != PINS[value['donor']]):
        raise ValueError('exact current guard source/capture identity mismatch')
    text = raw.decode('utf-8')
    # Full file is pinned; existing capture may contain truncated custom callback bodies.
    # R47 uses a fresh local lexical sweep, not that truncated body as source authority.
    captured = export_source_guards.capture(text)
    if not captured['events'] or not re.search(r'function\s+\w+\.onCastSpell\(', text):
        raise ValueError('missing exact cast source')
    return captured['events']


def build(repo, source_root):
    repo, source_root = Path(repo), Path(source_root)
    facts = [json.loads(line) for line in gzip.decompress((repo / BASE / 'source-callback-facts.jsonl.gz').read_bytes()).splitlines()]
    selected = {fact['registration_key']: fact for fact in facts}
    if len(selected) != len(facts):
        raise ValueError('duplicate source callback facts')
    records = []
    for value in SPECS:
        fact = selected[value['registration_key']]
        raw = subprocess.check_output(['git', '-C', str(source_root / value['donor']), 'show',
                                       PINS[value['donor']] + ':' + value['source_file']],
                                      env=dict(os.environ, GIT_NO_LAZY_FETCH='1'))
        events = qualify(raw, fact, value)
        row_id = sha(value['registration_key'].encode())[:16]
        receipt_path = repo / BASE / value['registration_key'].split('/')[0] / row_id / 'receipt.json'
        receipt = json.loads(receipt_path.read_text())
        if receipt['status'] != 'BLOCKED' or receipt['source_sha256'] != value['source_sha256']:
            raise ValueError('base receipt no longer exact blocked source')
        record = {'schema': SCHEMA_ID, 'registration_key': value['registration_key'],
                  'source_revision': PINS[value['donor']], 'source_file': value['source_file'],
                  'source_sha256': value['source_sha256'],
                  'source_git_blob': hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest(),
                  'historical_capture_fact_sha256': sha(canonical(fact)), 'historical_receipt_sha256': sha(receipt_path.read_bytes()),
                  'historical_receipt_status': 'BLOCKED', 'source_semantics': {'family': value['family'], **copy.deepcopy(value['semantics'])},
                  'ordered_source_events': events,
                  'source_binding_anchors': [{'source_line': line_number, 'expression': line.strip(),
                                              'resolution': 'source_expression_not_executable'}
                                             for line_number, line in enumerate(raw.decode('utf-8').splitlines(), 1)
                                             if re.match(r'^\s*(?:local\s+)?[\w.,\s\[\]]+\s*=(?!=)', line)
                                             and not line.lstrip().startswith('--')],
                  'source_order_scope': 'whole_file_lexical_not_executable_CFG',
                  'existing_schema_gaps': value['gaps'], 'complete_spell_candidate': False,
                  'runtime_activation': False, 'native_execution_qualified': False, 'external_sources_used': False}
        if record['source_git_blob'] != fact['source_callback_facts']['blob']:
            raise ValueError('source git blob mismatch')
        jsonschema.Draft202012Validator(evidence_schema()).validate(record)
        records.append(record)
    return records


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('--repo', default='.'); parser.add_argument('--source-root', default='/workspace/spell-sources')
    args = parser.parse_args(); repo = Path(args.repo).resolve(); records = build(repo, args.source_root)
    out = repo / OUTPUT; out.mkdir(parents=True, exist_ok=True)
    schema = evidence_schema(); (HERE / SCHEMA_FILE).write_text(json.dumps(schema, indent=2) + '\n')
    payload = b''.join(canonical(record) + b'\n' for record in records)
    path = out / 'source-simple-guard-evidence.jsonl.gz'; path.write_bytes(gzip.compress(payload, mtime=0))
    proof = {'schema': SCHEMA_ID + '/proof', 'records': len(records), 'complete_spell_candidates': 0,
             'source_semantic_families': len({value['family'] for value in SPECS}),
             'ordered_lexical_events': sum(len(record['ordered_source_events']) for record in records),
             'source_pins': PINS, 'payload_sha256': sha(payload), 'gzip_sha256': sha(path.read_bytes()),
             'schema_sha256': sha((HERE / SCHEMA_FILE).read_bytes()), 'producer_sha256': sha(Path(__file__).read_bytes()),
             'lexical_capture_producer_sha256': sha(Path(export_source_guards.__file__).read_bytes()),
             'base_capture_sha256': sha((repo / BASE / 'source-callback-facts.jsonl.gz').read_bytes()),
             'runtime_activation': False, 'native_execution_qualified': False, 'canonical_selection_changed': False,
             'limitations': ['Source-only normalized observations with exact source hashes; no executable Spell or admission.',
                             'Ordered lexical events cover entire file and retain unresolved expressions; not Lua AST or CFG.',
                             'Source callback helpers, native providers and runtime are not qualified by these observations.',
                             'Crystal Nature secondary selector has no viewport check despite source comment; tie preserves first party iteration member.']}
    (out / 'source-simple-guard-proof.json').write_text(json.dumps(proof, sort_keys=True, indent=2) + '\n')
    receipt = {'schema': SCHEMA_ID + '/receipt', 'records': len(records),
               'status': 'PARTIAL_SOURCE_EVIDENCE_SPELLS_BLOCKED', 'complete_spell_candidates': 0,
               'registration_keys': [record['registration_key'] for record in records],
               'runtime_activation': False, 'native_execution_qualified': False, 'canonical_selection_changed': False,
               'base_capture_sha256': proof['base_capture_sha256'],
               'schema_path': str((HERE / SCHEMA_FILE).relative_to(repo)),
               'schema_sha256': proof['schema_sha256'],
               'artifacts': {p.name: sha(p.read_bytes()) for p in [path, out / 'source-simple-guard-proof.json']}}
    (out / 'source-simple-guard-receipt.json').write_text(json.dumps(receipt, sort_keys=True, indent=2) + '\n')
    print(json.dumps({key: proof[key] for key in ['records', 'complete_spell_candidates', 'ordered_lexical_events', 'source_semantic_families']}))


if __name__ == '__main__':
    main()
