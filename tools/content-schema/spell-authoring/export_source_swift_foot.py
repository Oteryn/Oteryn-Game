"""Export exact current Canary Swift Foot as typed source control flow; never activate."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import tempfile

import import_source_player_bundles as base

ROOT = base.HERE.parents[2]
PIN = '04b83b512114bfd888000d6e1433ed8ecaec7c5b'
SNAPSHOT = 'canary-main-current'
SOURCE = 'data/scripts/spells/support/swift_foot.lua'
SOURCE_SHA = '0e6938bb8a910e55221f42ce9b34a0fb19526562adea16cd90696137844abf16'
REVISION = 'source-player-r32'
VARS = ['spellDuration', 'combat', 'condition', 'spell', 'creature', 'var', 'summons', 'i', 'summon', 'summon_t', 'deltaSpeed', 'FamiliarSpeed', 'FamiliarHaste', 'grade', 'exhaust', 'disable', 'exhaustAttackGroup', 'damageDebuff']
CONSTANTS = ['COMBAT_PARAM_EFFECT', 'CONST_ME_MAGIC_GREEN', 'COMBAT_PARAM_AGGRESSIVE', 'CONDITION_HASTE', 'CONDITION_PARAM_TICKS', 'CONDITION_PARAM_SPEED', 'WHEEL_GRADE_NONE', 'WHEEL_GRADE_REGULAR', 'CONDITION_EXHAUST_COMBAT', 'CONDITION_PACIFIED', 'CONDITION_SPELLGROUPCOOLDOWN', 'CONDITION_PARAM_SUBID', 'CONDITION_ATTRIBUTES', 'CONDITION_PARAM_BUFF_DAMAGEDEALT', 'SOUND_EFFECT_TYPE_SPELL_SWIFT_FOOT']
FUNCTIONS = ['Combat', 'Condition', 'Spell', 'type', 'math.max']
METHODS = ['setParameter', 'setFormula', 'addCondition', 'getSummons', 'getType', 'familiar', 'getBaseSpeed', 'execute', 'upgradeSpellsWOD', 'name', 'words', 'group', 'vocation', 'castSound', 'id', 'cooldown', 'groupCooldown', 'level', 'mana', 'isSelfTarget', 'isAggressive', 'isPremium', 'register']


def closed(props):
    return {'type': 'object', 'additionalProperties': False, 'properties': props, 'required': list(props)}


def schema():
    expr = {'$ref': '#/$defs/expression'};stmt = {'$ref': '#/$defs/statement'}
    arr = lambda value: {'type': 'array', 'items': value}
    const = lambda value: {'const': value}
    span = closed({'start_line': {'type': 'integer', 'minimum': 1}, 'end_line': {'type': 'integer', 'minimum': 1}, 'sha256': {'type': 'string', 'pattern': '^[0-9a-f]{64}$'}})
    expressions = [closed({'kind': const('literal'), 'value': {'type': ['string', 'number', 'boolean']}}),
        closed({'kind': const('variable'), 'name': {'enum': VARS}}), closed({'kind': const('constant'), 'name': {'enum': CONSTANTS}}),
        closed({'kind': const('binary'), 'operator': {'enum': ['+', '-', '*', '==', '>', 'and']}, 'left': expr, 'right': expr}),
        closed({'kind': const('length'), 'operand': expr}), closed({'kind': const('index'), 'container': expr, 'index': expr}),
        closed({'kind': const('function_call'), 'function': {'enum': FUNCTIONS}, 'arguments': arr(expr)}),
        closed({'kind': const('method_call'), 'receiver': expr, 'method': {'enum': METHODS}, 'arguments': arr(expr)})]
    statements = [closed({'kind': const('local_assignment'), 'name': {'enum': VARS}, 'value': expr, 'source_span': span}),
        closed({'kind': const('call'), 'expression': {'oneOf': expressions[-2:]}, 'source_span': span}),
        closed({'kind': const('if'), 'condition': expr, 'then': arr(stmt), 'elseif': arr(closed({'condition': expr, 'then': arr(stmt), 'source_span': span})), 'source_span': span}),
        closed({'kind': const('numeric_for'), 'variable': const('i'), 'start': expr, 'end': expr, 'body': arr(stmt), 'source_span': span}),
        closed({'kind': const('return'), 'value': expr, 'source_span': span})]
    grade = closed({'grade': {'enum': ['NONE', 'REGULAR', 'GREATER']}, 'action': {'enum': ['apply_conditions', 'no_op']},
        'conditions': arr(closed({'type': {'enum': ['CONDITION_EXHAUST_COMBAT', 'CONDITION_PACIFIED', 'CONDITION_SPELLGROUPCOOLDOWN', 'CONDITION_ATTRIBUTES']},
            'parameters': arr(closed({'parameter': {'enum': ['CONDITION_PARAM_TICKS', 'CONDITION_PARAM_SUBID', 'CONDITION_PARAM_BUFF_DAMAGEDEALT']}, 'value': {'type': 'integer'}}))})),
        'existing_modifier_operation': {'const': 'no_explicit_removal'}, 'source_control_flow': {'enum': ['explicit_if', 'explicit_elseif', 'implicit_fallthrough']}})
    return {'$schema': 'https://json-schema.org/draft/2020-12/schema', '$defs': {'expression': {'oneOf': expressions}, 'statement': {'oneOf': statements}},
        **closed({'schema': const('OTERYN_SOURCE_SWIFT_FOOT_CONTROL_FLOW/v1'), 'revision': const(REVISION),
            'registration_key': const(SNAPSHOT + '/' + SOURCE + '#1'), 'source_revision': const(PIN),
            'source_path': const(SOURCE), 'source_sha256': const(SOURCE_SHA),
            'program': closed({'initialization': arr(stmt), 'callback': closed({'name': const('spell.onCastSpell'), 'arguments': const(['creature', 'var']), 'body': arr(stmt)}), 'registration': arr(stmt)}),
            'grade_actions': {'type': 'array', 'prefixItems': [{'allOf': [grade, {'const': item}]} for item in grades()], 'items': False, 'minItems': 3, 'maxItems': 3},
            'unsupported': arr(closed({'scope': {'type': 'string'}, 'reason': {'type': 'string', 'minLength': 1}})),
            'status': const('SOURCE_FACTS_TYPED'), 'candidate_created': const(False), 'native_execution_qualified': const(False),
            'execution_equivalence': const(False), 'runtime_admission': const('blocked'), 'runtime_activation': const(False), 'external_sources_used': const(False)})}


def literal(value): return {'kind': 'literal', 'value': value}
def var(name): return {'kind': 'variable', 'name': name}
def constant(name): return {'kind': 'constant', 'name': name}
def binary(op, left, right): return {'kind': 'binary', 'operator': op, 'left': left, 'right': right}
def length(value): return {'kind': 'length', 'operand': value}
def fn(name, *args): return {'kind': 'function_call', 'function': name, 'arguments': list(args)}
def method(receiver, name, *args): return {'kind': 'method_call', 'receiver': var(receiver), 'method': name, 'arguments': list(args)}


def source_program(data):
    if base.sha(data) != SOURCE_SHA:
        raise ValueError('source differs from the exact reviewed Swift Foot pin')
    lines = data.splitlines(keepends=True)
    def span(start, end=None):
        end = start if end is None else end
        return {'start_line': start, 'end_line': end, 'sha256': base.sha(b''.join(lines[start - 1:end]))}
    def assign(line, name, value): return {'kind': 'local_assignment', 'name': name, 'value': value, 'source_span': span(line)}
    def call(line, expression): return {'kind': 'call', 'expression': expression, 'source_span': span(line)}
    def branch(start, end, condition, body, otherwise=()):
        return {'kind': 'if', 'condition': condition, 'then': body, 'elseif': list(otherwise), 'source_span': span(start, end)}
    def param(line, receiver, name, value): return call(line, method(receiver, 'setParameter', constant(name), value))
    init = [assign(1, 'spellDuration', literal(10000)), assign(3, 'combat', fn('Combat')),
        param(4, 'combat', 'COMBAT_PARAM_EFFECT', constant('CONST_ME_MAGIC_GREEN')), param(5, 'combat', 'COMBAT_PARAM_AGGRESSIVE', literal(0)),
        assign(7, 'condition', fn('Condition', constant('CONDITION_HASTE'))), param(8, 'condition', 'CONDITION_PARAM_TICKS', var('spellDuration')),
        call(9, method('condition', 'setFormula', *map(literal, [1.8, 72, 1.8, 72]))), call(10, method('combat', 'addCondition', var('condition'))),
        assign(12, 'spell', fn('Spell', literal('instant')))]
    familiar_body = [assign(21, 'deltaSpeed', fn('math.max', binary('-', method('creature', 'getBaseSpeed'), method('summon', 'getBaseSpeed')), literal(0))),
        assign(22, 'FamiliarSpeed', binary('-', binary('*', binary('+', method('summon', 'getBaseSpeed'), var('deltaSpeed')), literal(0.8)), literal(72))),
        assign(23, 'FamiliarHaste', fn('Condition', constant('CONDITION_HASTE'))), param(24, 'FamiliarHaste', 'CONDITION_PARAM_TICKS', var('spellDuration')),
        param(25, 'FamiliarHaste', 'CONDITION_PARAM_SPEED', var('FamiliarSpeed')), call(26, method('summon', 'addCondition', var('FamiliarHaste')))]
    loop = {'kind': 'numeric_for', 'variable': 'i', 'start': literal(1), 'end': length(var('summons')), 'source_span': span(17, 28),
        'body': [assign(18, 'summon', {'kind': 'index', 'container': var('summons'), 'index': var('i')}), assign(19, 'summon_t', method('summon', 'getType')),
                 branch(20, 27, binary('and', var('summon_t'), method('summon_t', 'familiar')), familiar_body)]}
    none = [assign(34, 'exhaust', fn('Condition', constant('CONDITION_EXHAUST_COMBAT'))), param(35, 'exhaust', 'CONDITION_PARAM_TICKS', var('spellDuration')),
        call(36, method('creature', 'addCondition', var('exhaust'))), assign(37, 'disable', fn('Condition', constant('CONDITION_PACIFIED'))),
        param(38, 'disable', 'CONDITION_PARAM_TICKS', var('spellDuration')), call(39, method('creature', 'addCondition', var('disable'))),
        assign(40, 'exhaustAttackGroup', fn('Condition', constant('CONDITION_SPELLGROUPCOOLDOWN'))), param(41, 'exhaustAttackGroup', 'CONDITION_PARAM_SUBID', literal(1)),
        param(42, 'exhaustAttackGroup', 'CONDITION_PARAM_TICKS', var('spellDuration')), call(43, method('creature', 'addCondition', var('exhaustAttackGroup')))]
    regular = [assign(45, 'damageDebuff', fn('Condition', constant('CONDITION_ATTRIBUTES'))), param(46, 'damageDebuff', 'CONDITION_PARAM_TICKS', var('spellDuration')),
        param(47, 'damageDebuff', 'CONDITION_PARAM_BUFF_DAMAGEDEALT', literal(50)), call(48, method('creature', 'addCondition', var('damageDebuff')))]
    grade_branch = branch(33, 49, binary('==', var('grade'), constant('WHEEL_GRADE_NONE')), none,
        [{'condition': binary('==', var('grade'), constant('WHEEL_GRADE_REGULAR')), 'then': regular, 'source_span': span(44, 48)}])
    body = [assign(15, 'summons', method('creature', 'getSummons')),
        branch(16, 29, binary('and', binary('and', var('summons'), binary('==', fn('type', var('summons')), literal('table'))), binary('>', length(var('summons')), literal(0))), [loop]),
        branch(31, 51, method('combat', 'execute', var('creature'), var('var')),
            [assign(32, 'grade', method('creature', 'upgradeSpellsWOD', literal('Swift Foot'))), grade_branch, {'kind': 'return', 'value': literal(True), 'source_span': span(50)}]),
        {'kind': 'return', 'value': literal(False), 'source_span': span(53)}]
    registry = [(56, 'name', [literal('Swift Foot')]), (57, 'words', [literal('utamo tempo san')]), (58, 'group', [literal('support'), literal('focus')]),
        (59, 'vocation', [literal('paladin;true'), literal('royal paladin;true')]), (60, 'castSound', [constant('SOUND_EFFECT_TYPE_SPELL_SWIFT_FOOT')]),
        (61, 'id', [literal(134)]), (62, 'cooldown', [binary('*', literal(10), literal(1000))]),
        (63, 'groupCooldown', [binary('*', literal(2), literal(1000)), binary('*', literal(10), literal(1000))]),
        (64, 'level', [literal(55)]), (65, 'mana', [literal(400)]), (66, 'isSelfTarget', [literal(True)]),
        (67, 'isAggressive', [literal(False)]), (68, 'isPremium', [literal(True)]), (70, 'register', [])]
    return {'initialization': init, 'callback': {'name': 'spell.onCastSpell', 'arguments': ['creature', 'var'], 'body': body},
            'registration': [call(line, method('spell', name, *args)) for line, name, args in registry]}


def grades():
    def condition(name, values): return {'type': name, 'parameters': [{'parameter': key, 'value': value} for key, value in values]}
    ticks = ('CONDITION_PARAM_TICKS', 10000)
    return [{'grade': 'NONE', 'action': 'apply_conditions', 'existing_modifier_operation': 'no_explicit_removal', 'source_control_flow': 'explicit_if', 'conditions': [
        condition('CONDITION_EXHAUST_COMBAT', [ticks]), condition('CONDITION_PACIFIED', [ticks]), condition('CONDITION_SPELLGROUPCOOLDOWN', [('CONDITION_PARAM_SUBID', 1), ticks])]},
        {'grade': 'REGULAR', 'action': 'apply_conditions', 'existing_modifier_operation': 'no_explicit_removal', 'source_control_flow': 'explicit_elseif', 'conditions': [condition('CONDITION_ATTRIBUTES', [ticks, ('CONDITION_PARAM_BUFF_DAMAGEDEALT', 50)])]},
        {'grade': 'GREATER', 'action': 'no_op', 'existing_modifier_operation': 'no_explicit_removal', 'source_control_flow': 'implicit_fallthrough', 'conditions': []}]


def validate(fact, data):
    base.validate_spell.Draft202012Validator(schema()).validate(fact)
    if fact['program'] != source_program(data) or fact['grade_actions'] != grades():
        raise ValueError('typed operations/grades/order disagree with exact source')
    return True


def generate(out, source_root, r28):
    inventory = json.loads((r28 / 'package-manifest.json').read_text())['files']
    callback = r28 / 'source-callback-facts.jsonl.gz'
    if inventory.get(callback.name) != base.sha(callback.read_bytes()):
        raise ValueError('source capture hash mismatch')
    rows = [json.loads(line) for line in gzip.decompress(callback.read_bytes()).splitlines()]
    selected = [r for r in rows if r['registration_key'].startswith(SNAPSHOT + '/') and r['source_callback_facts']['name'] == 'Swift Foot']
    if len(selected) != 1: raise ValueError('exactly one current Canary Swift Foot required')
    record = selected[0]
    if record['source_revision'] != PIN or record['source_sha256'] != SOURCE_SHA or record['source_callback_facts']['file'] != SOURCE:
        raise ValueError('source identity mismatch')
    data = base.source_file(source_root / 'canary', PIN, SOURCE)
    header = r28 / SNAPSHOT / base.sha(record['registration_key'].encode())[:16] / 'source-header.json'
    if inventory.get(header.relative_to(r28).as_posix()) != base.sha(header.read_bytes()): raise ValueError('header hash mismatch')
    source_header = json.loads(header.read_text())
    base.validate_spell.Draft202012Validator({'$ref': 'urn:oteryn:spell-authoring:candidate:1#/$defs/sourceProjection'}, registry=base.validate_spell.REGISTRY).validate(source_header['spell'])
    fact = {'schema': 'OTERYN_SOURCE_SWIFT_FOOT_CONTROL_FLOW/v1', 'revision': REVISION, 'registration_key': record['registration_key'],
        'source_revision': PIN, 'source_path': SOURCE, 'source_sha256': SOURCE_SHA, 'program': source_program(data), 'grade_actions': grades(),
        'unsupported': [{'scope': 'grade dispatch and condition execution', 'reason': 'Frozen companion native_behavior cannot encode grade NONE exhaustion/pacification/group cooldown or GREATER preservation. Typed source facts are data, not an executable native descriptor.'},
            {'scope': 'engine services', 'reason': 'Condition merging, grade provider, movement input, expiry scheduling and familiar eligibility need owning-engine qualification; no Lua or runtime execution equivalence claimed.'}],
        'status': 'SOURCE_FACTS_TYPED', 'candidate_created': False, 'native_execution_qualified': False, 'execution_equivalence': False,
        'runtime_admission': 'blocked', 'runtime_activation': False, 'external_sources_used': False}
    validate(fact, data)
    for name, obj in [('swift-foot-source.schema.json', schema()), ('swift-foot-source.json', fact), ('source-header.json', source_header), ('source-callback-facts.json', record)]: base.write(out / name, obj)
    blob = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
    if blob != record['source_callback_facts']['blob']: raise ValueError('Git blob/source bytes disagree')
    reference = {'schema': 'OTERYN_EXTERNAL_SOURCE_REFERENCE/v1', 'repository': 'https://github.com/opentibiabr/canary',
        'revision': PIN, 'path': SOURCE, 'git_blob': blob, 'sha256': SOURCE_SHA, 'bytes': len(data),
        'raw_source_distributed': False, 'local_pinned_git_bytes_verified': True}
    reference_schema = {'$schema': 'https://json-schema.org/draft/2020-12/schema', **closed({key: {'const': value} for key, value in reference.items()})}
    base.validate_spell.Draft202012Validator(reference_schema).validate(reference)
    base.write(out / 'source-reference.schema.json', reference_schema);base.write(out / 'source-reference.json', reference)
    summary = {'schema': 'OTERYN_SOURCE_SWIFT_FOOT_PACKAGE/v1', 'revision': REVISION, 'records': 1, 'status_counts': {'SOURCE_FACTS_TYPED': 1}, 'registration_key': record['registration_key'],
        'source_revision': PIN, 'source_sha256': SOURCE_SHA, 'runtime_activation': False, 'native_execution_qualified': False, 'candidate_created': False, 'external_sources_used': False,
        'input_proofs': {'r28/package-manifest.json': base.sha((r28 / 'package-manifest.json').read_bytes()), 'r28/source-callback-facts.jsonl.gz': base.sha(callback.read_bytes()),
                         'r28/' + header.relative_to(r28).as_posix(): base.sha(header.read_bytes())}, 'producer_proofs': {p.name: base.sha(p.read_bytes()) for p in [Path(__file__), Path(base.__file__), base.HERE / 'spell.schema.json']}}
    base.write(out / 'import-summary.json', summary)
    base.write(out / 'package-manifest.json', {'schema': 'OTERYN_SOURCE_PLAYER_PACKAGE/v1', 'files': {p.name: base.sha(p.read_bytes()) for p in sorted(out.iterdir()) if p.is_file()}})
    return summary


def run(out, source_root, r28):
    if out.exists(): raise ValueError('output must be new; existing packets are immutable')
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.r32-swift-foot-', dir=out.parent) as tmp:
        stage = Path(tmp) / 'packet';stage.mkdir()
        summary = generate(stage, source_root, r28);stage.rename(out)
    return summary


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, default=Path('/workspace/spell-sources'))
    parser.add_argument('--r28', type=Path, default=ROOT / 'docs/reference/spells/r28-source-closure/player-source-bundles')
    args = parser.parse_args();print(json.dumps(run(args.out, args.source_root, args.r28), sort_keys=True))
