"""Current source-only r30 Canary speed operators with changed provider inputs retained."""
import argparse
import copy
import gzip
import json
from pathlib import Path
import tempfile

import import_source_player_bundles as base
import native_companions

ROOT = base.HERE.parents[2]
PIN = '04b83b512114bfd888000d6e1433ed8ecaec7c5b'
OLD_PIN = native_companions.PINS['canary']
SNAPSHOT = 'canary-main-current'
REVISION = 'source-player-r30'
NAMES = ('haste', 'strong haste', 'charge')
AUDIT = Path('/workspace/spell-source-closure/condition-speed-equivalence-r30.json')
CHANGED = {'Condition::createCondition', 'Condition::createDamageCondition', 'Player::onEndCondition',
           'ConditionAttr_t', 'PLAYER_MAX_STAFF_SPEED'}


def source_slice(data, item):
    """Verify a precise immutable slice, including source indentation/overloads."""
    lines = data.splitlines(keepends=True)
    offset = sum(map(len, lines[:item['start_line'] - 1]))
    for start in range(max(0, offset - 2), offset + len(lines[item['start_line'] - 1]) + 1):
        value = data[start:start + item['bytes']]
        if base.sha(value) == item['sha256']:
            if 'literal' in item and value != item['literal'].encode():
                raise ValueError('literal slice disagrees with actual immutable source')
            return value
    raise ValueError('source slice SHA/line/length mismatch')


def qualify_helpers(repo, audit):
    if audit['old_revision'] != OLD_PIN or audit['current_revision'] != PIN:
        raise ValueError('helper audit immutable pin mismatch')
    files = {}
    for row in audit['provider_files']:
        for label, pin in [('old', OLD_PIN), ('current', PIN)]:
            data = base.source_file(repo, pin, row['path'])
            if base.sha(data) != row[label + '_sha256']:
                raise ValueError('helper provider full-file SHA mismatch')
            files[row['path'], label] = data
    providers = copy.deepcopy(audit['provider_files'])
    for path in sorted({row['path'] for row in audit['records']}):
        if (path, 'old') not in files:
            old = base.source_file(repo, OLD_PIN, path);current = base.source_file(repo, PIN, path)
            files[path, 'old'], files[path, 'current'] = old, current
            providers.append({'path': path, 'old_sha256': base.sha(old), 'current_sha256': base.sha(current), 'whole_file_equal': old == current})
    records = []
    found = set()
    for row in audit['records']:
        found.add(row['symbol'])
        if row['status'] != 'byte_identical' and row['symbol'] not in CHANGED:
            raise ValueError('unreviewed relevant helper change')
        slices = {label: [source_slice(files[row['path'], label], item) for item in row[label]] for label in ('old', 'current')}
        if row['status'] == 'byte_identical' and slices['old'] != slices['current']:
            raise ValueError('helper equality assertion disagrees with immutable source')
        for item in row['old'] + row['current']:
            if item.get('signature', '').startswith(('if ', 'if(', 'while ', 'for ')):
                raise ValueError('called control block cannot qualify a function definition')
        records.append({'path': row['path'], 'symbol': row['symbol'], 'status': row['status'],
                        **{label: [{k: v for k, v in item.items() if k not in ('literal', 'signature')} for item in row[label]]
                           for label in ('old', 'current')}})
    required = {'ConditionSpeed::startCondition', 'ConditionSpeed::addCondition', 'ConditionSpeed::getFormulaValues',
                'ConditionSpeed class', 'Condition base class storage', 'Creature::addCondition',
                'Creature::removeCondition', 'Creature::getCondition', 'Game::changeSpeed'}
    # The exact class-storage record name is supplied by the read-only extractor.
    required.remove('Condition base class storage')
    if not required <= found or not any('Condition' in s and 'storage' in s for s in found):
        raise ValueError('missing core speed operator/storage closure')
    review = audit['changed_provider_branch_review']
    factory = next(r for r in review if r['symbol'] == 'Condition::createCondition HASTE/PARALYZE branch')
    old, current = [source_slice(files[factory['path'], label], factory[label]) for label in ('old', 'current')]
    if old != current or b'case CONDITION_HASTE:' not in current or b'case CONDITION_PARALYZE:' not in current:
        raise ValueError('source HASTE/PARALYZE factory branch changed')
    damage = next(r for r in audit['records'] if r['symbol'] == 'Condition::createDamageCondition')
    damage_body = source_slice(files[damage['path'], 'current'], damage['current'][0])
    if b'CONDITION_HASTE' in damage_body or b'CONDITION_PARALYZE' in damage_body or b'return nullptr;' not in damage_body:
        raise ValueError('damage-only factory prefilter needs qualification')
    expiry = next(r for r in review if r['symbol'] == 'Player::onEndCondition')
    for part in ('prefix', 'suffix'):
        pieces = [source_slice(files[expiry['path'], label], expiry[label + '_unchanged_context'][part]) for label in ('old', 'current')]
        if pieces[0] != pieces[1]:
            raise ValueError('HASTE expiry context changed')
    expiry_row = next(r for r in audit['records'] if r['symbol'] == 'Player::onEndCondition')
    for label in ('old', 'current'):
        value = source_slice(files[expiry_row['path'], label], expiry_row[label][0])
        if b'type == CONDITION_INFIGHT && !conditionFight' not in value:
            raise ValueError('changed expiry branch guard not established')
    staff = next(r for r in audit['records'] if r['symbol'] == 'PLAYER_MAX_STAFF_SPEED')
    old, current = [source_slice(files[staff['path'], label], staff[label][0]).decode() for label in ('old', 'current')]
    if 'PLAYER_MAX_STAFF_SPEED = 1500;' not in old or 'PLAYER_MAX_STAFF_SPEED = 65535;' not in current:
        raise ValueError('source staff speed provider change not established')
    return records, providers, {'factory_speed_branch_sha256': factory['current']['sha256'],
        'expiry_changed_branch_guard': 'type == CONDITION_INFIGHT && !conditionFight',
        'expiry_qualification_scope': 'HASTE/PARALYZE only; no global INFIGHT or whole-function equivalence'}


def inputs_schema():
    def closed(props):
        return {'type': 'object', 'additionalProperties': False, 'properties': props, 'required': list(props)}
    return {'$schema': 'https://json-schema.org/draft/2020-12/schema', **closed({
        'schema': {'const': 'OTERYN_SOURCE_SPEED_INPUT_PROVIDER/v1'}, 'old_revision': {'const': OLD_PIN},
        'source_revision': {'const': PIN}, 'input_basis': {'const': 'current_engine_getBaseSpeed'},
        'staff_flag': {'const': 'SetMaxSpeed'}, 'old_staff_cap': {'const': 1500}, 'current_staff_cap': {'const': 65535},
        'regular_cap': {'const': 65535}, 'input_provider_equivalence': {'const': False},
        'operator_reuse_is_parametric': {'const': True}, 'runtime_admission': {'const': 'blocked'},
        'runtime_activation': {'const': False}, 'external_sources_used': {'const': False}})}


def generate(out, source_root, r28, audit_path):
    audit = json.loads(audit_path.read_text())
    helpers, providers, branch = qualify_helpers(source_root / 'canary', audit)
    input_fact = {'schema': 'OTERYN_SOURCE_SPEED_INPUT_PROVIDER/v1', 'old_revision': OLD_PIN, 'source_revision': PIN,
                  'input_basis': 'current_engine_getBaseSpeed', 'staff_flag': 'SetMaxSpeed',
                  'old_staff_cap': 1500, 'current_staff_cap': 65535, 'regular_cap': 65535,
                  'input_provider_equivalence': False, 'operator_reuse_is_parametric': True,
                  'runtime_admission': 'blocked', 'runtime_activation': False, 'external_sources_used': False}
    base.validate_spell.Draft202012Validator(inputs_schema()).validate(input_fact)
    base.write(out / 'source-speed-inputs.schema.json', inputs_schema())
    base.write(out / 'source-speed-inputs.json', input_fact)
    base.write(out / 'helper-qualification-proof.json', {'schema': 'OTERYN_SOURCE_SPEED_HELPER_QUALIFICATION/v1',
        'old_revision': OLD_PIN, 'source_revision': PIN, 'audit_sha256': base.sha(audit_path.read_bytes()),
        'source_functions_and_storage': helpers, 'provider_files': providers, 'changed_branch_scope': branch,
        'global_input_provider_equivalent': False, 'runtime_activation': False, 'external_sources_used': False})
    manifest = json.loads((r28 / 'package-manifest.json').read_text())['files']
    callback = r28 / 'source-callback-facts.jsonl.gz'
    if manifest.get(callback.name) != base.sha(callback.read_bytes()):
        raise ValueError('r28 callback source input hash mismatch')
    selected = [r for r in (json.loads(line) for line in gzip.decompress(callback.read_bytes()).splitlines())
                if r['registration_key'].split('/')[0] == SNAPSHOT and r['source_callback_facts']['name'].casefold() in NAMES]
    if len(selected) != 3 or len({r['source_callback_facts']['name'].casefold() for r in selected}) != 3:
        raise ValueError('three current Canary source identities required')
    payload = b''.join((json.dumps(r, sort_keys=True, separators=(',', ':')) + '\n').encode() for r in sorted(selected, key=lambda r: r['registration_key']))
    with (out / callback.name).open('wb') as file, gzip.GzipFile(fileobj=file, mode='wb', filename='', mtime=0) as archive:
        archive.write(payload)
    schema = base.receipt_schema()
    base.write(out / 'receipt.schema.json', schema)
    validator = base.validate_spell.Draft202012Validator(schema, registry=base.validate_spell.REGISTRY)
    defaults, default_proofs = base.default_fields(source_root / 'canary', PIN)
    index = []
    for record in sorted(selected, key=lambda r: r['registration_key']):
        raw = record['source_callback_facts'];name = raw['name'].casefold()
        spec = native_companions.SPECS[name]['sources']['canary']
        if record['source_revision'] != PIN or raw['source'] != 'canary' or raw['file'] != spec['path'] or raw['blob'] != spec['blob']:
            raise ValueError('source companion identity mismatch')
        current = base.source_file(source_root / 'canary', PIN, raw['file'])
        if current != base.source_file(source_root / 'canary', OLD_PIN, raw['file']) or base.sha(current) != spec['sha256'] or base.sha(current) != record['source_sha256']:
            raise ValueError('complete current cast differs from template')
        row_id = base.sha(record['registration_key'].encode())[:16]
        header = r28 / SNAPSHOT / row_id / 'source-header.json'
        if manifest.get(header.relative_to(r28).as_posix()) != base.sha(header.read_bytes()):
            raise ValueError('source header input hash mismatch')
        source_header = json.loads(header.read_text())['spell']
        key = 'candidate:spell/source/' + SNAPSHOT + '/' + row_id
        spell = base.fill_header(source_header, defaults, {'key': key, 'revision': REVISION}, raw['registrar'])
        spell['execution'] = {'native_behavior': copy.deepcopy(native_companions._recipe(name, 'canary'))}
        deps = {'abilities': [], 'effects': [], 'formulas': []};catalog = {'definitions': []}
        errors = base.validate_spell.validate({'spell': spell}, deps, catalog)
        if errors:
            raise ValueError('r30 strict source candidate validation: ' + ' | '.join(errors))
        folder = out / SNAPSHOT / row_id;folder.mkdir(parents=True)
        for filename, value in [('source-header.json', {'spell': source_header}), ('spell.json', {'spell': spell}), ('dependencies.json', deps), ('catalog.json', catalog)]:
            base.write(folder / filename, value)
        receipt = {'schema': 'OTERYN_SOURCE_PLAYER_BUNDLE_RECEIPT/v1', 'registration_key': record['registration_key'],
            'logical_key': ['instant', name], 'source_revision': PIN, 'source_sha256': record['source_sha256'],
            'candidate_key': key, 'source_header': source_header, 'engine_default_proofs': default_proofs,
            'mechanics_reference': {'source_capture': 'r28/player-source-bundles/source-callback-facts.jsonl.gz',
                'source_capture_sha256': base.sha(callback.read_bytes()), 'imported_facts': callback.name,
                'registration_key': record['registration_key'], 'source_callback_scope': 'exact current Canary cast facts'},
            'status': 'CANDIDATE_SCHEMA_VALID', 'blockers': [], 'runtime_activation': False, 'external_sources_used': False,
            'native_execution_qualified': False, 'item_owner_bindings_required': [], 'dependencies': {k: 0 for k in deps},
            'schema_and_semantic_validation_errors': [],
            'remaining_mechanics': [{'source_field': 'base_speed.current_input_provider',
                'reason': 'Current PLAYER_MAX_STAFF_SPEED=65535 replaces old1500; dynamic current_engine_getBaseSpeed consumed, input_provider_equivalence=false; strict source-speed-inputs.json preserves both caps. Runtime admission blocked.'},
                {'source_field': 'companion.transitive_engine_services', 'reason': 'Qualified speed/condition operators retain current source; transitive character, movement and scheduling providers remain unqualified.'}],
            'conversion_notes': ['Exact current Canary speed operators; source-player-r30; no Crystal precedence or Wiki values',
                json.dumps({'selected_donor': 'canary', 'current_source_revision': PIN, 'qualified_template_revision': OLD_PIN,
                    'full_cast_sha256': base.sha(current), 'full_cast_bytes_equal': True,
                    'helper_qualification_proof': 'helper-qualification-proof.json', 'source_speed_inputs': 'source-speed-inputs.json',
                    'input_provider_equivalence': False, 'operator_reuse_is_parametric': True}, sort_keys=True)]}
        validator.validate(receipt);base.write(folder / 'receipt.json', receipt)
        index.append({'registration_key': record['registration_key'], 'candidate_key': key, 'candidate_revision': REVISION,
                      'status': 'CANDIDATE_SCHEMA_VALID', 'blockers': []})
    summary = {'schema': 'OTERYN_SOURCE_PLAYER_BUNDLE_IMPORT/v1', 'records': 3, 'source_populations': {SNAPSHOT: 3},
        'status_counts': {'CANDIDATE_SCHEMA_VALID': 3}, 'native_descriptor_count': 3, 'all_receipts_schema_valid': True,
        'runtime_activation': False, 'external_sources_used': False, 'full_source_mechanics_1_to_1_complete': False,
        'source_callback_payload_sha256': base.sha(payload), 'revision': REVISION, 'records_index': index,
        'input_proofs': {'r28/package-manifest.json': base.sha((r28 / 'package-manifest.json').read_bytes()),
                         audit_path.name: base.sha(audit_path.read_bytes())},
        'converter_proofs': {p.name: base.sha(p.read_bytes()) for p in (Path(__file__), Path(base.__file__), Path(native_companions.__file__), base.HERE / 'spell.schema.json', base.HERE / 'spell-dependencies.schema.json')}}
    base.write(out / 'import-summary.json', summary)
    base.write(out / 'package-manifest.json', {'schema': 'OTERYN_SOURCE_PLAYER_PACKAGE/v1',
        'files': {p.relative_to(out).as_posix(): base.sha(p.read_bytes()) for p in sorted(out.rglob('*')) if p.is_file()}})
    return summary


def run(out, source_root, r28, audit_path):
    if out.exists():
        raise ValueError('output must be new; existing source packages are immutable')
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.r30-source-companions-', dir=out.parent) as temp:
        stage = Path(temp) / 'package';stage.mkdir()
        summary = generate(stage, source_root, r28, audit_path)
        stage.rename(out)
    return summary


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, default=Path('/workspace/spell-sources'))
    parser.add_argument('--r28', type=Path, default=ROOT / 'docs/reference/spells/r28-source-closure/player-source-bundles')
    parser.add_argument('--audit', type=Path, default=AUDIT)
    args = parser.parse_args()
    print(json.dumps(run(args.out, args.source_root, args.r28, args.audit), sort_keys=True))
