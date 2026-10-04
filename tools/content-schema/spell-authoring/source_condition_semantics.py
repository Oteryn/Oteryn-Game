"""Source Condition identity/lifetime semantics, separate from qualified application."""
import argparse
from collections import Counter
import gzip
import json
from pathlib import Path
import re
import subprocess
import source_condition_templates as templates

HERE = Path(__file__).resolve().parent
PATHS = ('src/creatures/creatures_definitions.hpp', 'src/lua/functions/creatures/combat/condition_functions.cpp', 'src/creatures/combat/condition.cpp')


def source_rules(source):
    revision = templates.PINS[source]
    texts, proofs = {}, []
    for path in PATHS:
        data = subprocess.check_output(['git', '-C', '/workspace/spell-sources/' + source, 'show', revision + ':' + path])
        texts[path] = data.decode()
        proofs.append({'source': source, 'revision': revision, 'path': path, 'sha256': templates.sha(data)})
    binding, engine = texts[PATHS[1]], texts[PATHS[2]]
    for fragment in ('L, 3, CONDITIONID_COMBAT', 'L, 4, 0', 'L, 5, false'):
        if fragment not in binding: raise ValueError('constructor default proof changed')
    for pattern in (r'executeCondition\([^{}]*\)\s*\{\s*if \(ticks == -1\)\s*\{\s*return true;',
                    r'isPersistent\(\) const\s*\{.*?if \(ticks == -1\)\s*\{\s*return false;',
                    r'isRemovableOnDeath\(\) const\s*\{.*?if \(ticks == -1\)\s*\{\s*return false;'):
        if not re.search(pattern, engine, re.S): raise ValueError('permanent condition semantics proof changed')
    enums = {name: templates.spell_scripts.enum_values(texts[PATHS[0]], name)
             for name in ('ConditionType_t', 'ConditionParam_t', 'ConditionId_t')}
    return enums, proofs


def enum_value(namespace, symbol, enums):
    if symbol not in enums[namespace]: raise ValueError('unregistered source enum: ' + symbol)
    return {'namespace': namespace, 'symbol': symbol, 'numeric_value': enums[namespace][symbol]}


def semantics(record, enums):
    constructor_args = record['constructor_arguments']
    if len(constructor_args) > 3: raise ValueError('explicit persistence constructor requires separate semantics')
    id_symbol = constructor_args[1]['value'] if len(constructor_args) > 1 and constructor_args[1]['kind'] == 'source_symbol' else 'CONDITIONID_COMBAT'
    if len(constructor_args) > 1 and constructor_args[1]['kind'] != 'source_symbol': raise ValueError('non-symbol constructor ID')
    params = []
    ticks = {'kind': 'scalar', 'value': 0}
    subid = constructor_args[2] if len(constructor_args) > 2 else {'kind': 'scalar', 'value': 0}
    prefix_seen = False
    for call in record['calls']:
        args = call['arguments']
        if call['method'] == 'setParameter' and len(args) == 2 and args[0]['kind'] == 'source_symbol':
            parameter = args[0]['value']
            value = args[1]
        elif call['method'] == 'setTicks' and len(args) == 1:
            parameter, value = 'CONDITION_PARAM_TICKS', args[0]
        else: continue
        params.append({'parameter': enum_value('ConditionParam_t', parameter, enums),
                       'value': value, 'declaration_prefix': call['declaration_prefix'],
                       'source_call_ref': call['source_call_ref']})
        if call['declaration_prefix']:
            if parameter == 'CONDITION_PARAM_TICKS': ticks, prefix_seen = value, True
            if parameter == 'CONDITION_PARAM_SUBID': subid = value
    indefinite = ticks.get('kind') == 'scalar' and ticks.get('value') == -1
    fixed = ticks.get('kind') == 'scalar' and isinstance(ticks.get('value'), (int, float)) and not isinstance(ticks['value'], bool) and ticks['value'] > 0
    lifetime = {'mode': 'indefinite_no_countdown' if indefinite else 'positive_source_ticks' if fixed else 'unresolved_or_initial_zero',
                'source_ticks': ticks, 'source_prefix_tick_assignment': prefix_seen,
                'serialization_persistence': 'false_under_default_constructor' if indefinite else 'not_derived',
                'removable_on_death': 'false_under_default_constructor' if indefinite else 'not_derived'}
    return {'schema': 'OTERYN_SOURCE_CONDITION_SEMANTICS/v1',
            'source_identity': record['source_identity'], 'source': record['source'], 'revision': record['revision'],
            'path': record['path'], 'source_sha256': record['source_sha256'],
            'condition_type': enum_value('ConditionType_t', record['source_type_constant'], enums),
            'constructor_default_condition_id': enum_value('ConditionId_t', 'CONDITIONID_COMBAT', enums),
            'constructor_condition_id': enum_value('ConditionId_t', id_symbol, enums),
            'constructor_sub_id': constructor_args[2] if len(constructor_args) > 2 else {'kind': 'scalar', 'value': 0},
            'constructor_default_sub_id': 0, 'constructor_default_persistence': False,
            'constructor_arguments': record['constructor_arguments'], 'constructor_call_ref': record['constructor_call_ref'],
            'prefix_sub_id': subid, 'lifetime': lifetime, 'parameter_assignments': params,
            'application_binding_qualified': False, 'runtime_activation': False,
            'limits': ['Prefix semantics do not qualify later or conditional mutations.',
                       'Indefinite countdown and persistence are separate engine properties.',
                       'Symbol values remain unresolved; enum membership does not imply a Lua global binding.']}


def generate(packet, out):
    from jsonschema import Draft202012Validator
    input_receipt_path = packet.with_name('source-condition-templates-receipt.json')
    input_receipt = json.loads(input_receipt_path.read_text())
    if templates.sha(packet.read_bytes()) != input_receipt['gzip_sha256']: raise ValueError('r31 packet digest mismatch')
    records = [json.loads(line) for line in gzip.decompress(packet.read_bytes()).splitlines()]
    rules = {source: source_rules(source) for source in templates.PINS}
    schema_path = HERE / 'source-condition-semantics.schema.json'
    validator = Draft202012Validator(json.loads(schema_path.read_text()))
    output = []
    checked = set()
    for record in records:
        if record['revision'] != templates.PINS[record['source']]: raise ValueError('source revision mismatch')
        identity = (record['source'], record['path'])
        if identity not in checked:
            data = subprocess.check_output(['git', '-C', '/workspace/spell-sources/' + record['source'], 'show', record['revision'] + ':' + record['path']])
            if templates.sha(data) != record['source_sha256']: raise ValueError('source Lua digest mismatch')
            checked.add(identity)
        row = semantics(record, rules[record['source']][0])
        validator.validate(row)
        output.append(row)
    out.mkdir(parents=True, exist_ok=True)
    artifact = out / 'source-condition-semantics.jsonl.gz'
    payload = b''.join((json.dumps(r, sort_keys=True, separators=(',', ':'))+'\n').encode() for r in sorted(output,key=lambda r:r['source_identity']))
    with artifact.open('wb') as raw, gzip.GzipFile(filename='', fileobj=raw, mode='wb', mtime=0) as stream: stream.write(payload)
    proof = {'schema':'OTERYN_SOURCE_CONDITION_SEMANTICS_RECEIPT/v1', 'record_count':len(output),
             'lifetime_counts':dict(Counter(r['lifetime']['mode'] for r in output)),
             'parameter_assignment_count':sum(len(r['parameter_assignments']) for r in output),
             'gzip_path':str(artifact),'gzip_sha256':templates.sha(artifact.read_bytes()),'payload_sha256':templates.sha(payload),
             'schema_path':str(schema_path.relative_to(HERE.parents[2])),'schema_sha256':templates.sha(schema_path.read_bytes()),
             'input_packet_path':str(packet),'input_packet_sha256':templates.sha(packet.read_bytes()),
             'input_receipt_path':str(input_receipt_path),'input_receipt_sha256':templates.sha(input_receipt_path.read_bytes()),
             'inventory_path':input_receipt['inventory_path'],'inventory_sha256':input_receipt['inventory_sha256'],
             'exporter_sha256':templates.sha(Path(__file__).read_bytes()),
             'argument_mapper_sha256':templates.sha(Path(templates.__file__).read_bytes()),
             'source_rules_provenance':[p for source in sorted(rules) for p in rules[source][1]],
             'source_revisions':sorted(templates.PINS.values()),'runtime_activation':False,'external_sources_used':False}
    (out/'source-condition-semantics-receipt.json').write_text(json.dumps(proof,indent=2)+'\n')
    return proof


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('--packet',type=Path,required=True)
    parser.add_argument('--out',type=Path,required=True)
    args=parser.parse_args()
    print(json.dumps(generate(args.packet,args.out)['lifetime_counts'],sort_keys=True))
