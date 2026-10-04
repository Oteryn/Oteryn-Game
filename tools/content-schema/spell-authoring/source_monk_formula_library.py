#!/usr/bin/env python3
"""Six source Formula definitions with a closed reference to the archived tier helper.

The evaluator qualifies source arithmetic within a bounded domain for research/tests.
It neither produces live player inputs nor implements a Game runtime/provider.
"""
import argparse
import bisect
import copy
import gzip
import hashlib
import json
import math
from pathlib import Path
import subprocess
import os

from jsonschema import Draft202012Validator

PIN = '04b83b512114bfd888000d6e1433ed8ecaec7c5b'
HELPER_KEY = 'source:canary/Player.calculateFlatDamageHealing/' + PIN
MAX_LEVEL = 1000000
HERE = Path(__file__).resolve().parent
SCHEMA_PATH = HERE / 'source-monk-formula-library.schema.json'
OUTPUT = Path('docs/reference/spells/r50-source-closure')
CAPTURE_PATH = 'imports/spells/r28/player-source-bundles/source-callback-facts.jsonl.gz'
CAPTURE_SHA = '77c90d487f673b66f59c61a3831048870861ae88e5f34a8fcd46dca32a64ccd8'
PROGRAM_PATH = 'imports/spells/r28/source-formula-evidence.jsonl.gz'
PROGRAM_FILE_SHA = '90473d1ad1c120e6c947b50a48de6e34c55aaec4140094f424d4d070cd65f2c6'
PROGRAM_SHA = '187ae29a5fc8cb94fa98792128c8909a18fccbeac8bcfcd3794f66481c140af8'
SPECS = {
    'double_jab': (50, '9e728c37472fc6f332b5ad077b721c6285ff02dbf05f2e8653bf9e7c12fd597e', '54a21ce7f71cff06694c997458fab91e1a69e86e337513171d7e5a7af1ef5bc4'),
    'flurry_of_blows': (65, '79bf99ebd02d533160ea9909ae8b6a5e3d801189c5b8e5f6306575509b83cbac', '815400e7c5ad61d22099ebcd6fa5a0eb8d3d70816280a2ad4cd13ca761ac8cc1'),
    'forceful_uppercut': (130, '38531e5357ae9164a94f2ae77bca10eaceff35dfe119d61903d95246f6bc914c', '40d6004d3127f6f56e2e5fed92dec8684a7d67c2425a1fae363f81bb1944230d'),
    'greater_flurry_of_blows': (100, 'f590c25e12aea6006e5028c6e60009d96378d3d94bddda59fbdf61c077f2762d', 'dad9685ebe3ac0b87a95e469c6ee47a4fb181712c6b260be8a854257594c7b22'),
    'mystic_repulse': (72, 'c7d196c15087a2e5ae0b6a3c6745f962e6ba6a40e39097848b3498d96ccc032f', '72b435dda34490de6a245673e3240ac8dd9c3ae6df2dc8e5bd800f1659193bd8'),
    'swift_jab': (12, 'cc8b0a61b517b2f507d089da68144a8f330df5d089260888a220476790d6f7cc', 'bdfb9a746eba1f0d213f315fadd3bb5823a943dab676da0d2718f3f17ae68f01'),
}
PROVIDER_SHAS = {
    'src/creatures/players/player.cpp': '7c572074fd7625f6f3e89626ae4dda6b7acc9296896d7c9521db9becfe14aa4c',
    'src/lua/functions/creatures/player/player_functions.cpp': 'd9860d99eb8893b80d7b6d77ac2fad433733c360ce143c1d723d78a0d7de2aac',
    'src/creatures/combat/combat.cpp': 'c57a9cdf3d68f7d82c5cb3220a4a603e76c3992d5517a654bb01814c68f80889',
    'src/creatures/combat/spells.cpp': '18f44b128c5cdc1f40283e8f5527ca0d53045fcd00e3edaef7c2a778fd58a035',
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def source(repo, path):
    return subprocess.check_output(['git', '-C', str(repo), 'show', PIN + ':' + path], env=dict(os.environ, GIT_NO_LAZY_FETCH='1'))


def registration(slug):
    return 'canary-main-current/data/scripts/spells/attack/' + slug + '.lua#1'


def rows(repo, path, digest):
    data = (Path(repo) / path).read_bytes()
    if sha(data) != digest:
        raise ValueError('archived source input SHA mismatch: ' + path)
    values = [json.loads(line) for line in gzip.decompress(data).splitlines()]
    mapping = {row['registration_key']: row for row in values}
    if len(mapping) != len(values):
        raise ValueError('archived source registrations not unique')
    return mapping


def load_program(repo):
    entry = rows(repo, PROGRAM_PATH, PROGRAM_FILE_SHA)[registration('double_jab')]
    program = entry['tier_program']
    if sha(canonical(program)) != PROGRAM_SHA:
        raise ValueError('archived tier program SHA mismatch')
    return program


def link_expression(tree):
    if set(tree) == {'fn', 'args'} and tree['fn'] == 'flat_damage_healing' and tree['args'] == [{'var': 'level'}]:
        return {'helper_ref': HELPER_KEY, 'args': [{'var': 'level'}]}
    if set(tree) == {'const'} or set(tree) == {'var'}:
        return copy.deepcopy(tree)
    if set(tree) == {'op', 'args'} and tree['op'] in {'add', 'sub', 'mul', 'div'} and len(tree['args']) == 2:
        return {'op': tree['op'], 'args': [link_expression(arg) for arg in tree['args']]}
    raise ValueError('unknown source formula helper/input operation')


def evaluate_expression(tree, environment, helper=None):
    """IEEE binary64 operation order; helper calls are closed to one linked source."""
    if set(tree) == {'const'}:
        return float(tree['const'])
    if set(tree) == {'var'}:
        return environment[tree['var']]
    if set(tree) == {'helper_ref', 'args'} and tree['helper_ref'] == HELPER_KEY and tree['args'] == [{'var': 'level'}] and helper is not None:
        return float(helper.evaluate(environment['level']))
    op = tree.get('op')
    args = [evaluate_expression(arg, environment, helper) for arg in tree.get('args', [])]
    if op == 'ceil' and len(args) == 1:
        return float(math.ceil(args[0]))
    if op == 'min' and len(args) == 2:
        return min(args)
    if len(args) != 2:
        raise ValueError('unsupported source evaluation operation/arity')
    if op == 'add':
        return args[0] + args[1]
    if op == 'sub':
        return args[0] - args[1]
    if op == 'mul':
        return args[0] * args[1]
    if op == 'div':
        return args[0] / args[1]
    raise ValueError('unsupported source evaluation operation')


class SourceTierEvaluator:
    """Bounded numeric qualification utility derived from the archived program.

    Tier states are computed in the source update order once; no approximating curve.
    Only finalizers depend on the requested level. No live providers/runtime admission.
    """
    def __init__(self, program):
        if sha(canonical(program)) != PROGRAM_SHA:
            raise ValueError('numeric qualifier requires exact archived tier program')
        self.program = program
        self.types = {}
        state = {}
        for assignment in program['initializers']:
            self._assign(assignment, state)
        self.bounds, self.states = [0], [dict(state)]
        while state['threshold'] <= MAX_LEVEL:
            lower = state['threshold']
            for assignment in program['while']['updates']:
                self._assign(assignment, state)
            if state['threshold'] <= lower:
                raise ValueError('tier threshold overflow/unbounded progression')
            self.bounds.append(lower)
            self.states.append(dict(state))

    def _assign(self, assignment, environment):
        target = assignment['target']
        scalar = assignment.get('scalar_type', self.types.get(target))
        value = evaluate_expression(assignment['value'], environment)
        if scalar == 'uint32_t':
            if not math.isfinite(value) or not 0 <= value <= 4294967295:
                raise ValueError('numeric qualifier refuses out-of-range uint32 conversion')
            value = int(value)
        elif scalar != 'double':
            raise ValueError('unknown source program scalar type')
        self.types[target] = scalar
        environment[target] = value

    def evaluate(self, level):
        if isinstance(level, bool) or not isinstance(level, int) or not 0 <= level <= MAX_LEVEL:
            raise ValueError('source helper numeric domain is integer level 0..1000000')
        state = dict(self.states[bisect.bisect_right(self.bounds, level) - 1], level=level)
        for assignment in self.program['finalizers']:
            self._assign(assignment, state)
        result = evaluate_expression(self.program['return'], state)
        if result != int(result) or not 0 <= result <= 65535:
            raise ValueError('source helper uint16 return domain mismatch')
        return int(result)


def evaluate_formula(record, environment, helper):
    if set(environment) != {'level', 'attack_skill', 'attack_value'}:
        raise ValueError('formula source inputs must be explicit level/skill/attack')
    helper.evaluate(environment['level'])
    for name in ('attack_skill', 'attack_value'):
        value = environment[name]
        if isinstance(value, bool) or not isinstance(value, int) or not -2147483648 <= value <= 2147483647:
            raise ValueError('source callback input numeric domain is int32')
    return tuple(evaluate_expression(record['expressions'][bound], environment, helper) for bound in ('minimum', 'maximum'))


def validate_library(library, repo):
    """Schema plus exact archived linkage; reject duplicate and altered source Formulae."""
    Draft202012Validator(json.loads(SCHEMA_PATH.read_text())).validate(library)
    expected = {registration(slug): (slug, power, source_sha, fact_sha) for slug, (power, source_sha, fact_sha) in SPECS.items()}
    records = library['formula_definitions']
    if {row['registration_key'] for row in records} != set(expected):
        raise ValueError('source formula registration population differs')
    captures = rows(repo, CAPTURE_PATH, CAPTURE_SHA)
    for row in records:
        slug, power, source_sha, fact_sha = expected[row['registration_key']]
        captured = captures[row['registration_key']]
        formulas = captured['source_callback_facts']['combats'][0]['callbacks'][0]['formula']
        if (row['base_power'] != power or row['source_sha256'] != source_sha
                or row['capture_fact_sha256'] != fact_sha
                or row['source_path'] != captured['source_callback_facts']['file']
                or row['key'] != 'source:formula/canary/' + slug + '/' + PIN
                or row['expressions'] != {bound: link_expression(formulas[bound]) for bound in ('minimum', 'maximum')}):
            raise ValueError('source Formula linkage/expression differs')
    ref = library['helpers'][0]['program_reference']
    if ref['gzip_sha256'] != PROGRAM_FILE_SHA or ref['program_sha256'] != PROGRAM_SHA:
        raise ValueError('helper archived SHA linkage differs')
    evidence = rows(repo, PROGRAM_PATH, PROGRAM_FILE_SHA)[registration('double_jab')]
    if library['helpers'][0]['source_proof'] != evidence['source_proofs'][1]:
        raise ValueError('helper source proof differs')
    expected_proofs = [{'path': path, 'revision': PIN, 'sha256': digest} for path, digest in PROVIDER_SHAS.items()]
    if library['input_provider']['source_proofs'] != expected_proofs:
        raise ValueError('source provider identities differ')
    load_program(repo)


def build(repo, source_root):
    repo, donor = Path(repo), Path(source_root) / 'canary'
    captures = rows(repo, CAPTURE_PATH, CAPTURE_SHA)
    evidence = rows(repo, PROGRAM_PATH, PROGRAM_FILE_SHA)
    program = load_program(repo)
    provider_proofs = []
    for path, digest in PROVIDER_SHAS.items():
        if sha(source(donor, path)) != digest:
            raise ValueError('source input provider bytes changed: ' + path)
        provider_proofs.append({'path': path, 'revision': PIN, 'sha256': digest})
    records = []
    for slug, (power, source_sha, capture_sha) in SPECS.items():
        key = registration(slug)
        fact = captures[key]
        raw = fact['source_callback_facts']
        if fact['source_revision'] != PIN or fact['source_sha256'] != source_sha or sha(canonical(fact)) != capture_sha or sha(source(donor, raw['file'])) != source_sha:
            raise ValueError('exact Monk source/capture identity mismatch: ' + key)
        if evidence[key]['tier_program'] != program or raw['cast'] != {'executed_combats': [1], 'tier': 'plain_combat'}:
            raise ValueError('Monk callback/helper closure changed')
        formula = raw['combats'][0]['callbacks'][0]['formula']
        records.append({'registration_key': key, 'key': 'source:formula/canary/' + slug + '/' + PIN,
                        'status': 'SOURCE_FORMULA_DATA_COMPLETE_SPELL_RUNTIME_UNQUALIFIED',
                        'source_path': raw['file'], 'source_sha256': source_sha, 'source_revision': PIN,
                        'capture_fact_sha256': capture_sha, 'base_power': power,
                        'callback_kind': 'CALLBACK_PARAM_SKILLVALUE', 'unused_callback_inputs': ['attack_factor'],
                        'inputs': ['level', 'attack_skill', 'attack_value'], 'helper_refs': [HELPER_KEY],
                        'expressions': {bound: link_expression(formula[bound]) for bound in ('minimum', 'maximum')},
                        'pair_semantics': 'raw_Lua_callback_return_pair_no_magnitude_projection',
                        'formula_data_complete': True, 'complete_spell_candidate': False,
                        'native_execution_qualified': False, 'monk_spell_type': 'MonkSpell_Builder'})
    library = {'schema': 'OTERYN_SOURCE_MONK_FORMULA_LIBRARY/v1', 'source_revision': PIN,
               'formula_count': 6, 'helper_count': 1, 'complete_spell_candidates': 0,
               'source_only': True, 'runtime_activation': False, 'native_execution_qualified': False,
               'canonical_selection_changed': False, 'native_identity_allocation': False,
               'helpers': [{'key': HELPER_KEY, 'input': 'level', 'return_type': 'uint16_t',
                            'numeric_domain': {'minimum_level': 0, 'maximum_level': MAX_LEVEL, 'outside_domain': 'refused'},
                            'program_reference': {'path': PROGRAM_PATH, 'gzip_sha256': PROGRAM_FILE_SHA,
                               'registration_key': registration('double_jab'), 'field': 'tier_program', 'program_sha256': PROGRAM_SHA},
                            'source_function': 'Player::calculateFlatDamageHealing',
                            'source_proof': evidence[registration('double_jab')]['source_proofs'][1]}],
               'input_provider': {'identity': 'source:canary/ValueCallback.getMinMaxValues/' + PIN,
                                  'live_input_production_qualified': False, 'source_proofs': provider_proofs,
                                  'callback_arguments': ['Player_userdata', 'attack_skill', 'attack_value', 'attack_factor'],
                                  'level_provider': 'Player_userdata.calculateFlatDamageHealing.member_level',
                                  'no_weapon_defaults': {'attack_skill': 0, 'attack_value': 7, 'attack_factor': 0},
                                  'weapon_provider': 'weapon.calculateSkillFormula_useCharges',
                                  'downstream_conversion': 'Lua_pair_to_int32_then_normal_random_then_optional_element_round_split',
                                  'helper_binding': 'valid_Player_uint16_result_to_lua_Number_invalid_Player_diagnostic_return1_unqualified',
                                  'builder_lifecycle': 'finishedCast_only_buildHarmony_before_cooldowns_aggression_sounds_cost',
                                  'downstream_execution_qualified': False},
               'formula_definitions': records,
               'remaining_gaps': ['Live weapon/input provider and charge use ownership',
                                  'Lua callback int32 conversion, normal_random and secondary damage splitting',
                                  'Monk Builder Harmony lifecycle and accepted native consumer admission',
                                  'Native assets/identities, canonical selection and server activation']}
    validate_library(library, repo)
    return library


def write(repo, source_root):
    repo = Path(repo)
    library = build(repo, source_root)
    payload = canonical(library) + b'\n'
    zipped = gzip.compress(payload, mtime=0)
    out = repo / OUTPUT
    out.mkdir(parents=True, exist_ok=True)
    (out / 'source-monk-formula-library.json.gz').write_bytes(zipped)
    qualification = {'schema': 'OTERYN_R50_MONK_FORMULA_LIBRARY_QUALIFICATION/v1',
                     'source_only': True, 'source_revision': PIN, 'formula_count': 6, 'helper_count': 1,
                     'full_spell_candidates': 0, 'runtime_activation': False, 'native_execution_qualified': False,
                     'canonical_selection_changed': False, 'native_identity_allocation': False,
                     'input_provider_equivalence': False,
                     'external_sources_used': False, 'network_downloads': 0,
                     'gzip_sha256': sha(zipped), 'payload_sha256': sha(payload), 'schema_sha256': sha(SCHEMA_PATH.read_bytes()),
                     'producer_sha256': sha(Path(__file__).read_bytes()),
                     'input_proofs': {CAPTURE_PATH: CAPTURE_SHA, PROGRAM_PATH: PROGRAM_FILE_SHA},
                     'numeric_qualification_scope': 'source_helper_and_raw_callback_arithmetic_only',
                     'records': [{'registration_key': row['registration_key'], 'source_sha256': row['source_sha256'],
                                  'formula_data_complete': True, 'spell_status': 'BLOCKED'} for row in library['formula_definitions']]}
    (out / 'source-monk-formula-library-qualification.json').write_text(json.dumps(qualification, sort_keys=True, indent=2) + '\n')
    return qualification


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--repo', default='.')
    parser.add_argument('--source-root', default='/workspace/spell-sources')
    args = parser.parse_args()
    print(json.dumps(write(args.repo, args.source_root), sort_keys=True))
