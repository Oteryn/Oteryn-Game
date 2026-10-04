#!/usr/bin/env python3
"""R42: one exact current-Canary Heal Friend source adapter to existing schemas.

No generic custom-cast acceptance, Lua execution, legacy matcher patch or active identity allocation.
"""
import argparse
import copy
import gzip
import hashlib
import json
import os
import pathlib
import subprocess

import import_source_player_bundles as base
import validate_spell

HERE = pathlib.Path(__file__).resolve().parent
REVISION = '04b83b512114bfd888000d6e1433ed8ecaec7c5b'
SOURCE_PATH = 'data/scripts/spells/healing/heal_friend.lua'
SOURCE_SHA = '4aa65b4d4ec49a7e2b1e0487e7afe27a553b789a77648c2d1e9ea8cd9c85bdc6'
REGISTRATION = 'canary-main-current/' + SOURCE_PATH + '#1'
ROW_ID = 'd8cf713dc25c39e6'
CANDIDATE_KEY = 'candidate:spell/source/canary-main-current/' + ROW_ID
CANDIDATE_REVISION = 'source-player-r42'
ORIGINAL = pathlib.Path('docs/reference/spells/r28-source-closure/player-source-bundles')
ORIGINAL_ROW = ORIGINAL / 'canary-main-current' / ROW_ID
OUTPUT = pathlib.Path('docs/reference/spells/r42-source-closure/player-heal-friend-candidate')
BODY = 'creature:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE) return combat:execute(creature, variant)'


def sha(value):
    return hashlib.sha256(value).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def reference(family, suffix):
    return {'family': family, 'key': CANDIDATE_KEY + suffix, 'revision': CANDIDATE_REVISION}


def identity(suffix=''):
    return {'key': CANDIDATE_KEY + suffix, 'revision': CANDIDATE_REVISION}


def qualify(source, fact):
    """Full-file identity plus explicit bounded capture shapes; no permissive fallbacks."""
    if sha(source) != SOURCE_SHA or fact['registration_key'] != REGISTRATION or fact['source_revision'] != REVISION or fact['source_sha256'] != SOURCE_SHA:
        raise ValueError('exact pinned Heal Friend source/capture identity mismatch')
    raw = fact['source_callback_facts']
    if raw['file'] != SOURCE_PATH or raw['source'] != 'canary' or raw['name'] != 'Heal Friend' or raw['spell_type'] != 'instant':
        raise ValueError('unexpected source record identity')
    if raw['cast'] != {'body': BODY, 'executed_combats': [1], 'patterns': ['extra_presentation_only'], 'player_calls': ['getPosition'], 'tier': 'custom'}:
        raise ValueError('cast is outside bounded before-combat presentation adapter')
    if len(raw['combats']) != 1:
        raise ValueError('adapter requires exactly one Combat')
    combat = raw['combats'][0]
    if set(combat) != {'parameters', 'callbacks'} or combat['parameters'] != {
            'COMBAT_PARAM_TYPE': 'COMBAT_HEALING', 'COMBAT_PARAM_EFFECT': 'CONST_ME_MAGIC_GREEN',
            'COMBAT_PARAM_DISPEL': 'CONDITION_PARALYZE', 'COMBAT_PARAM_AGGRESSIVE': False}:
        raise ValueError('unrepresented Combat source fields')
    if len(combat['callbacks']) != 1 or combat['callbacks'][0]['function'] != 'onGetFormulaValues' or combat['callbacks'][0]['kind'] != 'CALLBACK_PARAM_LEVELMAGICVALUE':
        raise ValueError('formula callback identity mismatch')
    formula = combat['callbacks'][0]['formula']
    def bound(coefficient, addition):
        return {'op': 'add', 'args': [{'op': 'add', 'args': [{'op': 'mul', 'args': [{'var': 'level'}, {'const': '0.2'}]},
                 {'op': 'mul', 'args': [{'var': 'magic_level'}, {'const': coefficient}]}]}, {'const': addition}]}
    if formula != {'status': 'resolved', 'inputs': ['level', 'magic_level'], 'functions': [],
                   'minimum': bound('10', '3'), 'maximum': bound('14', '5')}:
        raise ValueError('formula capture differs from exact source arithmetic')
    return base.source_formula(combat['callbacks'][0])


def build(repo, source_root):
    repo = pathlib.Path(repo)
    env = dict(os.environ, GIT_NO_LAZY_FETCH='1')
    source = subprocess.check_output(['git', '-C', str(pathlib.Path(source_root) / 'canary'), 'show', REVISION + ':' + SOURCE_PATH], env=env)
    capture_path = repo / ORIGINAL / 'source-callback-facts.jsonl.gz'
    capture_bytes = capture_path.read_bytes()
    selected = [json.loads(line) for line in gzip.decompress(capture_bytes).splitlines() if json.loads(line)['registration_key'] == REGISTRATION]
    if len(selected) != 1:
        raise ValueError('immutable capture registration not unique')
    fact = selected[0]
    normalized = qualify(source, fact)
    if sha(REGISTRATION.encode())[:16] != ROW_ID:
        raise ValueError('registration-derived candidate key mismatch')
    index_path = repo / ORIGINAL / 'import-summary.json'
    index = json.loads(index_path.read_text())
    matches = [row for row in index['records_index'] if row['registration_key'] == REGISTRATION]
    if index['records'] != 483 or len(matches) != 1 or matches[0]['status'] != 'BLOCKED' or matches[0]['candidate_key'] != CANDIDATE_KEY:
        raise ValueError('overlay requires one blocked existing registration')
    old_receipt_path = repo / ORIGINAL_ROW / 'receipt.json'
    old_receipt = json.loads(old_receipt_path.read_text())
    if old_receipt['status'] != 'BLOCKED' or old_receipt['source_sha256'] != SOURCE_SHA:
        raise ValueError('historical blocked source receipt mismatch')
    header_path = repo / ORIGINAL_ROW / 'source-header.json'
    header = json.loads(header_path.read_text())
    if header['spell'] != old_receipt['source_header']:
        raise ValueError('historical header/receipt mismatch')
    defaults, default_proofs = base.default_fields(pathlib.Path(source_root) / 'canary', REVISION)
    spell = base.fill_header(header['spell'], defaults, identity(), fact['source_callback_facts']['registrar'])
    spell['execution'] = {'ability': reference('Ability', '/ability')}
    deps = {'abilities': [{'identity': identity('/ability'), 'kind': 'spell', 'needs_target': True,
                         'needs_direction': False, 'range_tiles': 0,
                         'effects': [reference('Effect', '/ability/effect'), reference('Effect', '/ability/effect-dispel')]}],
            'effects': [{'identity': identity('/ability/effect'), 'operation': 'heal', 'damage_type': 'healing',
                         'formula': reference('Formula', '/ability/formula-player'),
                         'presentation': {'impact_asset_binding': 'canary.appearance:effect/magic_green',
                                          'caster_effect_asset_binding': 'canary.appearance:effect/magic_blue',
                                          'caster_effect_timing': 'before_combat'}},
                        {'identity': identity('/ability/effect-dispel'), 'operation': 'remove_condition', 'removed_condition': 'paralyze'}],
            'formulas': [{'identity': identity('/ability/formula-player'), **normalized}]}
    catalog = {'definitions': []}
    errors = validate_spell.validate({'spell': spell}, deps, catalog)
    if errors:
        raise ValueError('existing schema/semantic validation: ' + ' | '.join(errors))
    receipt = copy.deepcopy(old_receipt)
    receipt.update(status='CANDIDATE_SCHEMA_VALID', blockers=[], dependencies={k: len(v) for k, v in deps.items()},
                   schema_and_semantic_validation_errors=[], native_execution_qualified=False, item_owner_bindings_required=[],
                   remaining_mechanics=[
                       {'source_field': 'source.COMBAT_PARAM_AGGRESSIVE', 'reason': 'Exact false declaration retained by source proof and Spell targeting; independent Effect combat-legality owner execution remains unqualified.'},
                       {'source_field': 'source.implicit_instant_range', 'reason': 'Source InstantSpell range is -1; existing Ability range_tiles=0 follows authoring no-explicit-range projection, not a qualified viewport/engine-range execution contract.'},
                       {'source_field': 'source.combat_engine_and_assets', 'reason': 'Source wrapper and existing authoring schema qualify data only; targeting/provider legality, source Lua numeric-to-engine conversion, effect assets and runtime execution remain unqualified.'}],
                   conversion_notes=['Bounded current source adapter represents caster MAGIC_BLUE before single healing Combat, impact MAGIC_GREEN and paralysis dispel.',
                                     'Exact source arithmetic uses level * 0.2 plus magicLevel * 10/14 plus 3/5. No Wiki or source-value substitutions.',
                                     'Candidate revision source-player-r42; canonical identity selection and runtime admission unchanged.'])
    validate_spell.Draft202012Validator(base.receipt_schema(), registry=validate_spell.REGISTRY).validate(receipt)
    proof = {'schema': 'OTERYN_SOURCE_HEAL_FRIEND_QUALIFICATION/v1', 'registration_key': REGISTRATION,
             'candidate_key': CANDIDATE_KEY, 'candidate_revision': CANDIDATE_REVISION, 'source_revision': REVISION,
             'source_path': SOURCE_PATH, 'source_sha256': SOURCE_SHA, 'source_git_blob': fact['source_callback_facts']['blob'],
             'status': 'CANDIDATE_SCHEMA_VALID', 'qualification_scope': 'exact_source_wrapper_to_existing_authoring_schema',
             'source_lines': {'combat_parameters': [2, 3, 4, 5], 'formula': [7, 8, 9, 10], 'cast_chronology': [18, 19]},
             'cast_body_sha256': sha(BODY.encode()), 'formula_fact_sha256': sha(canonical(fact['source_callback_facts']['combats'][0]['callbacks'][0]['formula'])),
             'source_capture_gzip_sha256': sha(capture_bytes), 'selected_capture_fact_sha256': sha(canonical(fact)),
             'historical_receipt_sha256': sha(old_receipt_path.read_bytes()), 'historical_header_sha256': sha(header_path.read_bytes()),
             'engine_default_proofs': default_proofs, 'source_statements_represented': ['caster_magic_blue_before_combat', 'single_healing_combat', 'target_magic_green', 'paralysis_dispel', 'literal_level_magic_formula'],
             'mechanics_qualification': 'wrapper_schema_qualified_engine_execution_unqualified',
             'native_identity_allocation': False, 'canonical_selection_changed': False, 'runtime_activation': False,
             'native_execution_qualified': False, 'external_sources_used': False, 'legacy_matcher_modified': False,
             'producer_sha256': sha(pathlib.Path(__file__).read_bytes()),
             'schema_proofs': {name: sha((HERE / name).read_bytes()) for name in ['spell.schema.json', 'spell-dependencies.schema.json']}}
    return {'spell.json': {'spell': spell}, 'dependencies.json': deps, 'catalog.json': catalog,
            'source-header.json': header, 'receipt.json': receipt, 'receipt.schema.json': base.receipt_schema(),
            'source-qualification-proof.json': proof}, fact


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('--repo', default='.'); parser.add_argument('--source-root', default='/workspace/spell-sources'); args = parser.parse_args()
    repo = pathlib.Path(args.repo).resolve(); values, fact = build(repo, args.source_root)
    out = repo / OUTPUT; out.mkdir(parents=True, exist_ok=True)
    row = out / 'canary-main-current' / ROW_ID; row.mkdir(parents=True, exist_ok=True)
    # Remove only this producer's unpublished initial flat-layout files.
    for name in ['spell.json', 'dependencies.json', 'catalog.json', 'source-header.json', 'receipt.json']:
        if (out / name).is_file():
            (out / name).unlink()
    for name, value in values.items():
        write_json((out if name in ['receipt.schema.json', 'source-qualification-proof.json'] else row) / name, value)
    payload = canonical(fact) + b'\n'
    (out / 'source-callback-facts.jsonl.gz').write_bytes(gzip.compress(payload, mtime=0))
    summary = {'schema': 'OTERYN_SOURCE_PLAYER_BUNDLE_IMPORT/v1', 'records': 1,
               'revision': CANDIDATE_REVISION, 'source_populations': {'canary-main-current': 1},
               'status_counts': {'CANDIDATE_SCHEMA_VALID': 1}, 'native_descriptor_count': 0,
               'all_receipts_schema_valid': True, 'full_source_mechanics_1_to_1_complete': False,
               'external_sources_used': False, 'runtime_activation': False,
               'source_callback_payload_sha256': sha(payload),
               'input_proofs': {'r28/package-manifest.json': sha((repo / ORIGINAL / 'package-manifest.json').read_bytes()),
                                'r28/import-summary.json': sha((repo / ORIGINAL / 'import-summary.json').read_bytes())},
               'converter_proofs': {path.name: sha(path.read_bytes()) for path in [pathlib.Path(__file__), pathlib.Path(base.__file__), pathlib.Path(validate_spell.__file__), HERE / 'spell.schema.json', HERE / 'spell-dependencies.schema.json']},
               'records_index': [{'registration_key': REGISTRATION, 'candidate_key': CANDIDATE_KEY,
                                  'candidate_revision': CANDIDATE_REVISION, 'status': 'CANDIDATE_SCHEMA_VALID', 'blockers': []}]}
    write_json(out / 'import-summary.json', summary)
    manifest = {'schema': 'OTERYN_SOURCE_PLAYER_PACKAGE/v1',
                'files': {str(p.relative_to(out)): sha(p.read_bytes()) for p in sorted(out.rglob('*')) if p.is_file() and p.name != 'package-manifest.json'}}
    write_json(out / 'package-manifest.json', manifest)
    print(json.dumps(summary, indent=2))


if __name__ == '__main__':
    main()
