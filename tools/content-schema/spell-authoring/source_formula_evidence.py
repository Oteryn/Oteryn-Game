"""Source-qualified expression and scalar-free Combat evidence, never Wiki/runtime activation."""
import argparse
import ast
from collections import Counter
import copy
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess

HERE = Path(__file__).resolve().parent
REPOS = {'canary-main-current': Path('/workspace/spell-sources/canary'),
         'crystal-summer-current': Path('/workspace/spell-sources/crystal')}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def source(snapshot, revision, path):
    data = subprocess.check_output(['git', '-C', str(REPOS[snapshot]), 'show', revision + ':' + path])
    return data, {'path': path, 'revision': revision, 'sha256': sha(data)}


def expression(text, environment=None):
    environment = environment or {}
    tree = ast.parse(text, mode='eval').body
    def convert(node):
        if isinstance(node, ast.Constant) and isinstance(node.value, (int, float)) and not isinstance(node.value, bool):
            return {'const': ast.get_source_segment(text, node)}
        if isinstance(node, ast.Name):
            return copy.deepcopy(environment.get(node.id, {'var': node.id}))
        if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.USub):
            return {'op': 'neg', 'args': [convert(node.operand)]}
        if isinstance(node, ast.BinOp) and type(node.op) in (ast.Add, ast.Sub, ast.Mult, ast.Div):
            return {'op': {ast.Add: 'add', ast.Sub: 'sub', ast.Mult: 'mul', ast.Div: 'div'}[type(node.op)],
                    'args': [convert(node.left), convert(node.right)]}
        if isinstance(node, ast.Call) and not node.keywords:
            name = node.func.id if isinstance(node.func, ast.Name) else node.func.attr if isinstance(node.func, ast.Attribute) and isinstance(node.func.value, ast.Name) and node.func.value.id == 'math' else None
            if name in ('floor', 'ceil', 'sqrt', 'abs', 'min', 'max'):
                return {'op': name, 'args': [convert(a) for a in node.args]}
        raise ValueError('unsupported source expression syntax: ' + type(node).__name__)
    return convert(tree)


def crystal_helper(snapshot, revision):
    data, proof = source(snapshot, revision, 'data/scripts/lib/register_spells.lua')
    text = data.decode()
    matches = list(re.finditer(r'^function calculateBaseDamageHealing\(level\)\n(.*?)^end\s*$', text, re.M | re.S))
    if len(matches) != 1:
        raise ValueError('ambiguous source base helper')
    match = matches[0]
    body = match.group(1)
    environment = {}
    result = None
    for line in body.splitlines():
        line = line.split('--')[0].strip()
        if not line:
            continue
        assignment = re.fullmatch(r'local (\w+) = (.+)', line)
        if assignment:
            environment[assignment[1]] = expression(assignment[2], environment)
        elif line.startswith('return '):
            result = expression(line[7:], environment)
        else:
            raise ValueError('non-arithmetic helper statement')
    if result is None:
        raise ValueError('helper has no source return')
    proof.update(function='calculateBaseDamageHealing', line_start=text[:match.start()].count('\n') + 1,
                 body_sha256=sha(body.encode()))
    return result, proof


def substitute_level(tree, value):
    if tree == {'var': 'level'}:
        return copy.deepcopy(value)
    if isinstance(tree, dict):
        return {k: substitute_level(v, value) for k, v in tree.items()}
    if isinstance(tree, list):
        return [substitute_level(v, value) for v in tree]
    return tree


def adapt_expression(tree, base_helper, shielding=False):
    if set(tree) == {'const'}:
        return copy.deepcopy(tree)
    if set(tree) == {'var'}:
        if tree['var'] == 'skill:SKILL_SHIELD':
            if not shielding:
                raise ValueError('shield skill lacks source method evidence')
            return {'var': 'shielding_skill'}
        if tree['var'] in {'level', 'magic_level', 'attack_skill', 'attack_value', 'attack_factor', 'base_power', 'shielding_skill', 'shield_defense'}:
            return copy.deepcopy(tree)
        raise ValueError('unknown formula input: ' + tree['var'])
    if set(tree) == {'fn', 'args'} and tree['fn'] == 'base_damage_healing' and len(tree['args']) == 1 and base_helper is not None:
        return substitute_level(base_helper, adapt_expression(tree['args'][0], base_helper, shielding))
    if set(tree) == {'op', 'args'} and tree['op'] in {'add', 'sub', 'mul', 'div', 'neg', 'floor', 'ceil', 'sqrt', 'abs', 'min', 'max'}:
        return {'op': tree['op'], 'args': [adapt_expression(a, base_helper, shielding) for a in tree['args']]}
    raise ValueError('source helper/input remains unsupported: ' + json.dumps(tree, sort_keys=True))


def source_formula(callback, helper, shielding=False):
    original = callback['formula']
    if original.get('status') != 'resolved':
        raise ValueError('source callback not resolved')
    bounds = [adapt_expression(original[k], helper, shielding) for k in ('minimum', 'maximum')]
    magnitude = [{'op': 'abs', 'args': [b]} for b in bounds]
    return {'kind': 'player_expression', 'inputs': 'skill' if callback['kind'] == 'CALLBACK_PARAM_SKILLVALUE' else 'level_magic',
            'minimum': {'op': 'min', 'args': copy.deepcopy(magnitude)}, 'maximum': {'op': 'max', 'args': copy.deepcopy(magnitude)}}


def shielding_evidence(snapshot, revision, script):
    if re.search(r'getEffectiveSkillLevel\(SKILL_SHIELD\)', script):
        return True, None
    if 'calculateKnightHealing(' not in script:
        return False, None
    data, proof = source(snapshot, revision, 'data/scripts/lib/register_spells.lua')
    text = data.decode()
    match = re.search(r'^function calculateKnightHealing\([^\n]*\)\n(.*?)^end\s*$', text, re.M | re.S)
    if not match or 'player:getEffectiveSkillLevel(SKILL_SHIELD)' not in match[1]:
        raise ValueError('source Knight helper shielding input not qualified')
    proof.update(function='calculateKnightHealing', line_start=text[:match.start()].count('\n') + 1,
                 body_sha256=sha(match[1].encode()))
    return True, proof


def flat_helper(snapshot, revision):
    """Extract the actual C++ tier recurrence; no substitute curve or Lua callback execution."""
    data, proof = source(snapshot, revision, 'src/creatures/players/player.cpp')
    text = data.decode()
    signature = 'uint16_t Player::calculateFlatDamageHealing() const {'
    start = text.index(signature)
    end = text.index('\n}', start) + 2
    body = text[start + len(signature):end - 1]
    clean = re.sub(r'//[^\n]*', '', body)
    before, loop, after = re.fullmatch(r'(.*?)while \(level >= threshold\) \{(.*?)\}(.*)', clean, re.S).groups()
    def assignments(part):
        result = []
        for statement in part.split(';'):
            statement = statement.strip()
            if not statement:
                continue
            declare = re.fullmatch(r'(double|uint32_t) (\w+) = (.+)', statement, re.S)
            increment = re.fullmatch(r'\+\+(\w+)', statement)
            update = re.fullmatch(r'(\w+) (\+=|=) (.+)', statement, re.S)
            if declare:
                typ, target, value = declare.groups()
                node = expression(value.replace('std::ceil', 'ceil'))
                result.append({'target': target, 'value': node, 'scalar_type': typ})
            elif increment:
                target = increment[1]
                result.append({'target': target, 'value': {'op': 'add', 'args': [{'var': target}, {'const': '1'}]}})
            elif update:
                target, operator, value = update.groups()
                node = expression(value)
                if operator == '+=':
                    node = {'op': 'add', 'args': [{'var': target}, node]}
                result.append({'target': target, 'value': node})
            else:
                raise ValueError('unsupported source recurrence statement')
        return result
    return_match = re.search(r'return std::min<uint32_t>\(computed, std::numeric_limits<uint16_t>::max\(\)\);', after)
    if not return_match:
        raise ValueError('source clamp/return type changed')
    program = {'input': 'level', 'initializers': assignments(before),
               'while': {'comparison': 'gte', 'left': {'var': 'level'}, 'right': {'var': 'threshold'}, 'updates': assignments(loop)},
               'finalizers': assignments(after[:return_match.start()]),
               'return': {'op': 'min', 'args': [{'var': 'computed'}, {'const': '65535'}]}, 'return_type': 'uint16_t'}
    proof.update(function='Player::calculateFlatDamageHealing', line_start=text[:start].count('\n') + 1,
                 body_sha256=sha(body.encode()))
    return program, proof


def scalar_free_combat_adapter(combat, raw_combat, snapshot, revision):
    """Remove only an invented player direct-health effect, keeping item/condition payloads."""
    value_kinds = {'CALLBACK_PARAM_LEVELMAGICVALUE', 'CALLBACK_PARAM_SKILLVALUE'}
    callbacks = raw_combat.get('callbacks', [])
    if any(c.get('kind') in value_kinds for c in callbacks) or raw_combat.get('set_formula') or combat.get('formula'):
        return copy.deepcopy(combat), []
    params = combat.get('params', {})
    if 'COMBAT_PARAM_TYPE' not in params or not ('COMBAT_PARAM_CREATEITEM' in params or combat.get('conditions')):
        return copy.deepcopy(combat), []
    proofs = []
    texts = {}
    for path in ['src/creatures/creatures_definitions.hpp', 'src/creatures/combat/combat.hpp',
                 'src/creatures/combat/combat.cpp', 'src/creatures/creature.hpp',
                 'src/creatures/players/player.hpp', 'src/creatures/players/player.cpp']:
        data, proof = source(snapshot, revision, path)
        proofs.append(proof)
        texts[path] = data.decode()
    if not re.search(r'struct CombatDamage\s*\{.*?int32_t value = 0;', texts['src/creatures/creatures_definitions.hpp'], re.S):
        raise ValueError('source default health value not established')
    if 'formulaType_t formulaType = COMBAT_FORMULA_UNDEFINED;' not in texts['src/creatures/combat/combat.hpp']:
        raise ValueError('source default formula type not established')
    if not re.search(r'virtual bool getCombatValues\(int32_t &, int32_t &\)\s*\{\s*return false;', texts['src/creatures/creature.hpp']):
        raise ValueError('source Creature health provider default changed')
    if 'getCombatValues' in texts['src/creatures/players/player.hpp'] or 'Player::getCombatValues' in texts['src/creatures/players/player.cpp']:
        raise ValueError('Player health-provider override needs qualification')
    cpp = texts['src/creatures/combat/combat.cpp']
    start = cpp.index('CombatDamage Combat::getCombatDamage(')
    end = cpp.index('\n}\n', start)
    method = cpp[start:end]
    for token in ('damage.primary.type = params.combatType;', 'if (params.valueCallback)',
                  'formulaType == COMBAT_FORMULA_DAMAGE', 'formulaType == COMBAT_FORMULA_LEVELMAGIC',
                  'formulaType == COMBAT_FORMULA_SKILL', 'return damage;'):
        if token not in method:
            raise ValueError('source scalar-health branch needs requalification')
    result = copy.deepcopy(combat)
    result['params'].pop('COMBAT_PARAM_TYPE')
    if 'param_calls' in result:
        result['param_calls'] = [call for call in result['param_calls'] if call[0] != 'COMBAT_PARAM_TYPE']
    return result, proofs


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode() + b'\n'


def export(bundles, out):
    from jsonschema import Draft202012Validator
    import validate_spell
    population_path = out / 'source-formula-population.json'
    population_bytes = population_path.read_bytes()
    population = json.loads(population_bytes)
    selected = {r['registration_key']: r for r in population['records']}
    if len(selected) != population['record_count'] or len(selected) != len(population['records']):
        raise ValueError('source formula cohort identities not conserved')
    callbacks_path = bundles / 'source-callback-facts.jsonl.gz'
    callbacks_bytes = callbacks_path.read_bytes()
    schema_path = HERE / 'source-formula-evidence.schema.json'
    record_schema = json.loads(schema_path.read_text())
    validator = Draft202012Validator(record_schema)
    Draft202012Validator({'$ref': '#/$defs/population', '$defs': record_schema['$defs']}).validate(population)
    records = []
    counts = Counter()
    for line in gzip.decompress(callbacks_bytes).decode().splitlines():
        raw = json.loads(line)
        key = raw['registration_key']
        if key not in selected:
            continue
        if raw['source_revision'] != selected[key]['source_revision'] or raw['source_sha256'] != selected[key]['source_sha256']:
            raise ValueError('selected source formula identity changed')
        snapshot = key.split('/')[0]
        captured = raw['source_callback_facts']
        revision = raw['source_revision']
        data, script_proof = source(snapshot, revision, captured['file'])
        if sha(data) != raw['source_sha256']:
            raise ValueError('source callback input hash mismatch')
        formulas = []
        proofs = [script_proof]
        program = None
        helper = None
        if snapshot == 'crystal-summer-current':
            helper, proof = crystal_helper(snapshot, revision)
            proofs.append(proof)
        reasons = []
        shielding, shield_proof = shielding_evidence(snapshot, revision, data.decode())
        if shield_proof:
            proofs.append(shield_proof)
        roles = []
        for index, combat in enumerate(captured['combats']):
            for callback in combat.get('callbacks', []):
                if 'formula' not in callback:
                    continue
                try:
                    formula = source_formula(callback, helper, shielding)
                    errors = validate_spell.structural('spell-dependencies.schema.json', {'abilities': [], 'effects': [], 'formulas': [{'identity': {'key': 'candidate:source/formula', 'revision': 'source-r28'}, **formula}]})
                    if errors:
                        raise ValueError('existing formula schema rejected source tree')
                    formulas.append({'combat_index': index, 'callback_kind': callback['kind'], 'formula': formula})
                except ValueError as error:
                    reasons.append(str(error))
                    if 'flat_damage_healing' in json.dumps(callback['formula']):
                        program, proof = flat_helper(snapshot, revision)
                        if proof not in proofs:
                            proofs.append(proof)
            if not any(c.get('kind') in {'CALLBACK_PARAM_LEVELMAGICVALUE', 'CALLBACK_PARAM_SKILLVALUE'} for c in combat.get('callbacks', [])) and not combat.get('set_formula'):
                params = combat.get('parameters', {})
                item = params.get('COMBAT_PARAM_CREATEITEM')
                if item or combat.get('conditions'):
                    _, role_proofs = scalar_free_combat_adapter({'params': params, 'conditions': combat.get('conditions', [])}, combat, snapshot, revision)
                    for proof in role_proofs:
                        if proof not in proofs:
                            proofs.append(proof)
                    roles.append({'combat_index': index, 'kind': 'field_creation' if item else 'condition_only',
                                  'direct_health_delta': 'zero_default', 'created_item_symbol': item or '',
                                  'condition_constants': sorted(set(re.findall(r'Condition\((CONDITION_[A-Z0-9_]+)', data.decode())))})
        status = 'existing_formula_schema_ready' if formulas and not reasons else 'source_tier_program_preserved_runtime_unimplemented' if program else 'scalar_free_combat_adapter_ready' if roles and not reasons else 'unsupported'
        record = {'schema': 'OTERYN_SOURCE_FORMULA_EVIDENCE/v1', 'registration_key': key, 'runtime_activation': False,
                  'status': status, 'source_proofs': proofs, 'source_formulas': formulas,
                  'scalar_free_combats': roles, 'unsupported_reasons': reasons,
                  'tier_program': program, 'shielding_effective_skill_evidence': shielding}
        validator.validate(record)
        records.append(record)
        counts[status] += 1
    if len(records) != len(selected):
        raise ValueError('selected source formula population not conserved')
    out.mkdir(parents=True, exist_ok=True)
    artifact = out / 'source-formula-evidence.jsonl.gz'
    payload = b''.join(canonical(r) for r in sorted(records, key=lambda r: r['registration_key']))
    with artifact.open('wb') as raw, gzip.GzipFile(fileobj=raw, mode='wb', filename='', mtime=0) as compressed:
        compressed.write(payload)
    receipt = {'schema': 'OTERYN_SOURCE_FORMULA_EVIDENCE_RECEIPT/v1', 'records': len(records), 'status_counts': dict(counts),
               'artifact': artifact.name, 'gzip_path': str(artifact), 'gzip_sha256': sha(artifact.read_bytes()), 'payload_sha256': sha(payload),
               'record_schema': 'tools/content-schema/spell-authoring/source-formula-evidence.schema.json', 'record_schema_sha256': sha(schema_path.read_bytes()),
               'schema_path': str(schema_path.relative_to(HERE.parents[2])), 'schema_sha256': sha(schema_path.read_bytes()),
               'record_count': len(records), 'source_revisions': sorted({proof['revision'] for record in records for proof in record['source_proofs']}),
               'selection_population_path': str(population_path), 'selection_population_sha256': sha(population_bytes),
               'selection_population_schema_ref': '#/$defs/population',
               'input_callback_facts_sha256': sha(callbacks_bytes),
               'external_sources_used': False, 'runtime_activation': False,
               'limitations': ['C++ tier program is source evidence, not implemented as a runtime Formula.', 'Scalar-free Combat adapter must retain condition/item effects and source type metadata.', 'No callback guards, timing, item ownership or gameplay activation is inferred.']}
    (out / 'source-formula-evidence-receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    return receipt


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--bundles', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(export(args.bundles, args.out)['status_counts'], sort_keys=True))
