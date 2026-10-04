"""Bounded source-only Condition templates from retained calls; no actor MODELS or Lua VM."""
import argparse
import ast
from collections import Counter
from fractions import Fraction
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from types import SimpleNamespace

HERE = Path(__file__).resolve().parent
_dependency_path = sys.path[:]
try:
    sys.path.insert(0, str(HERE.parent / 'monster-authoring'))
    import canary_batch as cb
    import spell_scripts
finally:
    sys.path[:] = _dependency_path
del _dependency_path
import validate_spell


def sha(data):
    return hashlib.sha256(data).hexdigest()


PINS = {'canary': '04b83b512114bfd888000d6e1433ed8ecaec7c5b',
        'crystal': '00ce02a57ca5a12e48f32a3476e37471167e4c3f'}


def pinned_engine_enums(source, source_inputs):
    revision = PINS[source]
    path = spell_scripts.ENGINE_DEFINITIONS
    data = subprocess.check_output(['git', '-C', '/workspace/spell-sources/' + source,
                                    'show', revision + ':' + path])
    if (source_inputs / source / path).read_bytes() != data:
        raise ValueError('staged engine enum header differs from pinned Git blob: ' + source)
    enums = {enum: spell_scripts.enum_values(data.decode('utf-8'), enum) for enum in spell_scripts.ENUMS}
    return enums, {'source': source, 'revision': revision, 'path': path, 'sha256': sha(data)}


def scalar(argument):
    if argument.get('kind') in ('number_literal', 'literal') and isinstance(argument.get('value'), (int, float, bool)):
        return argument['value']
    if argument.get('kind') != 'expression_tokens' or len(argument.get('tokens', [])) > 32:
        raise ValueError('not bounded literal arithmetic')
    tree = ast.parse(' '.join(argument['tokens']), mode='eval').body
    def value(node):
        if isinstance(node, ast.Constant) and isinstance(node.value, (int, float)) and not isinstance(node.value, bool):
            return Fraction(str(node.value))
        if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.USub):
            return -value(node.operand)
        if isinstance(node, ast.BinOp):
            a, b = value(node.left), value(node.right)
            if isinstance(node.op, ast.Add): return a + b
            if isinstance(node.op, ast.Sub): return a - b
            if isinstance(node.op, ast.Mult): return a * b
            if isinstance(node.op, ast.Div): return a / b
        raise ValueError('dynamic or unsupported condition argument')
    result = value(tree)
    return int(result) if result.denominator == 1 else float(result)


def argument_fact(argument):
    if argument.get('kind') == 'table_expression':
        fields = {}
        for entry in argument.get('entries', []):
            tokens = entry.get('tokens', [])
            if len(tokens) != 3 or tokens[1] != '=' or tokens[0] not in {'lookType', 'lookTypeEx', 'lookHead', 'lookBody', 'lookLegs', 'lookFeet', 'lookAddons', 'lookMount'}:
                break
            try:
                value = scalar({'kind': 'expression_tokens', 'tokens': tokens[2:]})
                if isinstance(value, bool) or not isinstance(value, int) or value < 0 or tokens[0] in fields:
                    break
                fields[tokens[0]] = value
            except (ValueError, SyntaxError):
                break
        else:
            if fields:
                return {'kind': 'source_outfit_literals', 'fields': fields}
    try:
        return {'kind': 'scalar', 'value': scalar(argument)}
    except (ValueError, SyntaxError, ZeroDivisionError):
        if argument.get('kind') == 'symbol':
            return {'kind': 'source_symbol', 'value': argument['name']}
        return {'kind': 'unresolved_source_argument', 'source_argument_sha256': sha(json.dumps(argument, sort_keys=True).encode())}


def map_template(kind, call_rows, enums, key):
    calls, gaps = [], []
    for row in call_rows:
        method = row['method']
        arguments = row['arguments']
        if method == 'setParameter' and len(arguments) == 2 and arguments[0]['kind'] == 'source_symbol' and arguments[1]['kind'] == 'scalar':
            parameter = arguments[0]['value']
            if parameter not in enums['ConditionParam_t']:
                gaps.append('source parameter is not registered: ' + parameter)
            else:
                calls.append(('setParameter', [parameter, arguments[1]['value']]))
        elif method == 'setTicks' and len(arguments) == 1 and arguments[0]['kind'] == 'scalar':
            calls.append(('setParameter', ['CONDITION_PARAM_TICKS', arguments[0]['value']]))
        elif method == 'setFormula' and len(arguments) == 4 and all(a['kind'] == 'scalar' for a in arguments):
            calls.append(('setFormula', [a['value'] for a in arguments]))
        else:
            gaps.append('source declaration method/argument lacks condition payload mapping: ' + method)
    if gaps:
        return None, [], gaps
    deps = {'formulas': []}
    converter = cb.Converter.__new__(cb.Converter)
    converter.spell_scripts = SimpleNamespace(enums=enums)
    notes = []
    try:
        body = converter.script_condition({'type': kind, 'calls': calls}, deps, key + '/speed', notes)
        if body is None:
            return None, [], notes or ['source Condition does not provide a declarative payload']
        effect = {'identity': {'key': key, 'revision': 'source-condition-r31'}, **body}
        errors = validate_spell.structural('spell-dependencies.schema.json', {'abilities': [], 'effects': [effect], 'formulas': deps['formulas']})
        if errors:
            return None, deps['formulas'], ['existing Effect schema rejects source payload: ' + errors[0]]
        return effect, deps['formulas'], notes
    except (cb.SpellUnresolved, ValueError, TypeError) as error:
        return None, [], [str(error)]


def generate(inventory_path, source_inputs, out):
    from jsonschema import Draft202012Validator
    inventory_bytes = inventory_path.read_bytes()
    inventory = json.loads(gzip.decompress(inventory_bytes))
    schema_path = HERE / 'source-condition-template.schema.json'
    schema = json.loads(schema_path.read_text())
    validator = Draft202012Validator(schema, registry=validate_spell.REGISTRY)
    pinned = {source: pinned_engine_enums(source, source_inputs) for source in PINS}
    enums = {source: value[0] for source, value in pinned.items()}
    records = []
    outcomes = Counter()
    for item in inventory['files']:
        if '/spells/' not in item['path'] or '/monster/' in item['path'] or not any(c['call_identity'] == 'Condition' for c in item['calls']):
            continue
        data = subprocess.check_output(['git', '-C', '/workspace/spell-sources/' + item['source'], 'show', item['revision'] + ':' + item['path']])
        if item['revision'] != PINS[item['source']]:
            raise ValueError('retained mechanics source revision differs from exact pin')
        if sha(data) != item['sha256']:
            raise ValueError('retained mechanics source digest mismatch')
        text = data.decode()
        lines = text.splitlines()
        cutoff = min([i + 1 for i, line in enumerate(lines) if re.match(r'\s*(?:(?:local )?function\b|if\b|for\b|while\b|repeat\b)', line)] + [len(lines) + 1])
        combat_variables = {m[1] for line in lines if (m := re.match(r'\s*local (\w+) = Combat\(', line))}
        declarations = []
        for call in item['calls']:
            if call['call_identity'] != 'Condition': continue
            match = re.match(r'\s*local (\w+) = Condition\(', lines[call['line'] - 1])
            if not match or not call['arguments'] or call['arguments'][0].get('kind') != 'symbol': continue
            variable = match[1]
            attached = any(c['call_identity'].endswith(':addCondition') and (c.get('receiver_expression') or {}).get('name') in combat_variables and any(a.get('name') == variable for a in c['arguments']) for c in item['calls'])
            if not attached:
                declarations.append((call, variable))
        for constructor, variable in declarations:
            # Any later assignment ends this lexical receiver binding, even if its
            # constructor was excluded because it attaches to Combat.
            next_same = min([i + 1 for i, line in enumerate(lines)
                             if i + 1 > constructor['line'] and re.match(r'[ \t]*(?:local[ \t]+)?' + re.escape(variable) + r'[ \t]*=(?!=)', line)]
                            + [len(lines) + 1])
            selected = [c for c in item['calls'] if c['call_identity'].startswith(variable + ':') and constructor['line'] < c['line'] < next_same]
            literal_refs = []
            for call in selected:
                for argument in call['arguments']:
                    if argument.get('kind') != 'symbol': continue
                    symbol = argument['name']
                    matches = list(re.finditer(r'^[ \t]*local ' + re.escape(symbol) + r'\s*=\s*(-?[0-9]+(?:\.[0-9]+)?)\s*(?:--[^\n]*)?$', text, re.M))
                    assignments = re.findall(r'\b' + re.escape(symbol) + r'\s*=(?!=)', text)
                    if len(matches) == 1 and len(assignments) == 1:
                        value = float(matches[0][1]) if '.' in matches[0][1] else int(matches[0][1])
                        entry = {'symbol': symbol, 'value': value, 'line': text[:matches[0].start()].count('\n') + 1}
                        if entry not in literal_refs: literal_refs.append(entry)
            rows = [{'source_call_ref': {'inventory_call_index': item['calls'].index(c), 'line': c['line'], 'source_order': c['source_order']}, 'line': c['line'], 'method': c['call_identity'].split(':')[-1], 'arguments': [argument_fact(a) for a in c['arguments']], 'declaration_prefix': c['line'] < cutoff} for c in selected]
            prefix = [c for c in rows if c['declaration_prefix']]
            kind = constructor['arguments'][0]['name']
            key = 'candidate:source/condition/' + sha((item['source'] + '/' + item['path'] + ':' + str(constructor['line'])).encode())[:20]
            if len(constructor['arguments']) != 1:
                effect, formulas, gaps = None, [], ['additional Condition constructor arguments require explicit binding mapping']
            elif constructor['line'] < cutoff:
                effect, formulas, gaps = map_template(kind, prefix, enums[item['source']], key)
            else:
                effect, formulas, gaps = None, [], ['condition declaration inside/after source control scope; binding unqualified']
            appearance_unresolved = kind == 'CONDITION_OUTFIT'
            if appearance_unresolved:
                gaps.append('appearance payload and dynamic setOutfit application remain unresolved')
            status = 'existing_effect_schema_valid_source_only' if effect else 'source_condition_payload_gap'
            record = {'schema': 'OTERYN_SOURCE_CONDITION_TEMPLATE/v1', 'source_identity': item['source'] + '/' + item['path'] + ':' + str(constructor['line']),
                      'source': item['source'], 'revision': item['revision'], 'path': item['path'], 'source_sha256': item['sha256'],
                      'declaration_line': constructor['line'], 'source_variable': variable, 'source_type_constant': kind,
                      'constructor_call_ref': {'inventory_call_index': item['calls'].index(constructor), 'line': constructor['line'], 'source_order': constructor['source_order']},
                      'constructor_arguments': [argument_fact(a) for a in constructor['arguments']],
                      'appearance_binding_unresolved': appearance_unresolved, 'template_complete': False,
                      'calls': rows, 'source_literal_references': literal_refs, 'status': status, 'typed_effect': effect, 'typed_formulas': formulas, 'mapping_gaps': gaps,
                      'runtime_activation': False, 'application_binding_qualified': False,
                      'limits': ['Only declaration-prefix scalar calls feed Effect mapping.', 'Later/conditional mutation and application hooks are retained as source references, not executed or qualified.']}
            validator.validate(record)
            records.append(record)
            outcomes[status] += 1
    out.mkdir(parents=True, exist_ok=True)
    artifact = out / 'source-condition-templates.jsonl.gz'
    payload = b''.join((json.dumps(r, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode() for r in sorted(records, key=lambda r:r['source_identity']))
    with artifact.open('wb') as raw, gzip.GzipFile(filename='', fileobj=raw, mode='wb', mtime=0) as compressed: compressed.write(payload)
    proof = {'schema': 'OTERYN_SOURCE_CONDITION_TEMPLATE_RECEIPT/v1', 'record_count': len(records), 'status_counts': dict(outcomes),
             'gzip_path': str(artifact), 'gzip_sha256': sha(artifact.read_bytes()), 'payload_sha256': sha(payload),
             'schema_path': str(schema_path.relative_to(HERE.parents[2])), 'schema_sha256': sha(schema_path.read_bytes()),
             'inventory_path': str(inventory_path), 'inventory_sha256': sha(inventory_bytes),
             'engine_enum_provenance': [pinned[source][1] for source in sorted(pinned)],
             'exporter_sha256': sha(Path(__file__).read_bytes()), 'runtime_activation': False,
             'source_revisions': sorted({r['revision'] for r in records}), 'external_sources_used': False}
    (out / 'source-condition-templates-receipt.json').write_text(json.dumps(proof, indent=2) + '\n')
    return proof


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--inventory', type=Path, required=True)
    parser.add_argument('--source-inputs', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(generate(args.inventory, args.source_inputs, args.out)['status_counts'], sort_keys=True))
