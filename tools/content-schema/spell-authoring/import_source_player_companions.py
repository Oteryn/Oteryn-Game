"""Source-only r29 Crystal companion candidates; frozen r28 is read-only input."""
import argparse
import copy
import gzip
import json
from pathlib import Path
import tempfile

import import_source_player_bundles as base
import native_companions

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SNAPSHOT = 'crystal-summer-current'
SOURCE = 'crystal'
REVISION = 'source-player-r29'
PIN = '00ce02a57ca5a12e48f32a3476e37471167e4c3f'
OLD_PIN = native_companions.PINS[SOURCE]
NAMES = ('haste', 'strong haste', 'charge', 'swift foot')
FULL_HELPERS = ('src/creatures/combat/condition.cpp', 'src/creatures/combat/condition.hpp',
                'src/creatures/creature.cpp', 'src/creatures/creature.hpp',
                'src/lua/functions/creatures/combat/condition_functions.cpp',
                'src/lua/functions/creatures/creature_functions.cpp',
                'src/lua/functions/creatures/monster/monster_type_functions.cpp')
FUNCTION_HELPERS = {'src/game/game.cpp': ('Game::changeSpeed',),
                    'src/lua/functions/creatures/player/player_functions.cpp': ('PlayerFunctions::luaPlayerUpgradeSpellWOD',),
                    'src/creatures/players/wheel/player_wheel.cpp': ('PlayerWheel::getSpellUpgrade',)}


def qualify(name, raw, repo):
    if name not in NAMES or raw['source'] != SOURCE or raw['spell_type'] != 'instant':
        raise base.SourceBlocked('unsupported companion source identity')
    spec = native_companions.SPECS[name]['sources'][SOURCE]
    if raw['name'].casefold() != name or raw['file'] != spec['path'] or raw['blob'] != spec['blob']:
        raise base.SourceBlocked('companion path/blob/name identity differs from template')
    current = base.source_file(repo, PIN, spec['path'])
    before = base.source_file(repo, OLD_PIN, spec['path'])
    if current != before or base.sha(current) != spec['sha256']:
        raise base.SourceBlocked('companion full source cast differs from qualified template')
    proofs = [{'path': spec['path'], 'scope': 'full_file', 'sha256': base.sha(current),
               'current_bytes_equal_template_bytes': True}]
    for path in FULL_HELPERS:
        current, before = base.source_file(repo, PIN, path), base.source_file(repo, OLD_PIN, path)
        if current != before:
            raise base.SourceBlocked('companion whole helper changed: ' + path)
        proofs.append({'path': path, 'scope': 'full_file', 'sha256': base.sha(current),
                       'current_bytes_equal_template_bytes': True})
    scoped = FUNCTION_HELPERS if name == 'swift foot' else {'src/game/game.cpp': ('Game::changeSpeed',)}
    for path, symbols in scoped.items():
        current, before = base.source_file(repo, PIN, path), base.source_file(repo, OLD_PIN, path)
        for symbol in symbols:
            now, old = base.source_cpp_function(current, symbol), base.source_cpp_function(before, symbol)
            if now != old:
                raise base.SourceBlocked('companion relevant helper changed: ' + symbol)
            proofs.append({'path': path, 'scope': 'function', 'symbol': symbol, 'function_sha256': base.sha(now),
                           'current_file_sha256': base.sha(current), 'template_file_sha256': base.sha(before),
                           'current_bytes_equal_template_bytes': True})
    return copy.deepcopy(native_companions._recipe(name, SOURCE)), proofs


def grade_actions_schema():
    def closed(properties):
        return {'type': 'object', 'additionalProperties': False,
                'properties': properties, 'required': list(properties)}
    write_action = lambda grade, percent: closed({'grade': {'const': grade}, 'action': {'const': 'apply_modifier'},
                                                'percent': {'const': percent}, 'duration_ms': {'const': 10000}})
    no_op = closed({'grade': {'const': 'greater'}, 'action': {'const': 'no_op'},
                    'preserve_existing_modifier': {'const': True}, 'execution_equivalence': {'const': False},
                    'runtime_admission': {'const': 'blocked'}})
    record = closed({'schema': {'const': 'OTERYN_SOURCE_COMPANION_GRADE_ACTIONS/v1'},
                     'registration_key': {'type': 'string', 'minLength': 1},
                     'source_revision': {'const': PIN}, 'source_sha256': {'type': 'string', 'pattern': '^[0-9a-f]{64}$'},
                     'actions': {'type': 'array', 'prefixItems': [write_action('none', 70), write_action('regular', 50), no_op],
                                 'minItems': 3, 'maxItems': 3, 'items': False},
                     'descriptor_placeholder': closed({'field': {'const': '/execution/native_behavior/parameters/damage_dealt_percent/greater'},
                                                      'value': {'const': 100},
                                                      'classification': {'const': 'legacy_model_placeholder_not_source_write'}}),
                     'execution_equivalence': {'const': False}, 'runtime_admission': {'const': 'blocked'},
                     'external_sources_used': {'const': False}, 'runtime_activation': {'const': False}})
    return {'$schema': 'https://json-schema.org/draft/2020-12/schema', **record}


def generate(out, source_root, r28):
    manifest = json.loads((r28 / 'package-manifest.json').read_text())['files']
    callback_path = r28 / 'source-callback-facts.jsonl.gz'
    if manifest.get(callback_path.name) != base.sha(callback_path.read_bytes()):
        raise ValueError('r28 callback input digest mismatch')
    records = [json.loads(line) for line in gzip.decompress((r28 / 'source-callback-facts.jsonl.gz').read_bytes()).splitlines()]
    selected = [r for r in records if r['registration_key'].split('/')[0] == SNAPSHOT and r['source_callback_facts']['name'].casefold() in NAMES]
    if len(selected) != len(NAMES) or len({r['source_callback_facts']['name'].casefold() for r in selected}) != len(NAMES):
        raise ValueError('source companion population must contain exactly four unique current Crystal registrations')
    payload = b''.join((json.dumps(r, sort_keys=True, separators=(',', ':')) + '\n').encode() for r in sorted(selected, key=lambda r: r['registration_key']))
    with (out / 'source-callback-facts.jsonl.gz').open('wb') as file:
        with gzip.GzipFile(fileobj=file, mode='wb', filename='', mtime=0) as archive:
            archive.write(payload)
    base.write(out / 'source-grade-actions.schema.json', grade_actions_schema())
    schema = base.receipt_schema()
    base.write(out / 'receipt.schema.json', schema)
    receipt_validator = base.validate_spell.Draft202012Validator(schema, registry=base.validate_spell.REGISTRY)
    defaults, default_proofs = base.default_fields(source_root / SOURCE, PIN)
    index = []
    for record in sorted(selected, key=lambda r: r['registration_key']):
        if record['source_revision'] != PIN:
            raise ValueError('current source revision mismatch')
        raw = record['source_callback_facts']
        name = raw['name'].casefold()
        descriptor, proofs = qualify(name, raw, source_root / SOURCE)
        if base.sha(base.source_file(source_root / SOURCE, PIN, raw['file'])) != record['source_sha256']:
            raise ValueError('source callback metadata hash mismatch')
        row_id = base.sha(record['registration_key'].encode())[:16]
        old = r28 / SNAPSHOT / row_id
        header_path = old / 'source-header.json'
        if manifest.get(header_path.relative_to(r28).as_posix()) != base.sha(header_path.read_bytes()):
            raise ValueError('r28 source header input digest mismatch')
        source_header = json.loads(header_path.read_text())['spell']
        candidate_key = 'candidate:spell/source/' + SNAPSHOT + '/' + row_id
        spell = base.fill_header(source_header, defaults, {'key': candidate_key, 'revision': REVISION}, raw['registrar'])
        spell['execution'] = {'native_behavior': descriptor}
        deps = {'abilities': [], 'effects': [], 'formulas': []}
        catalog = {'definitions': []}
        errors = base.validate_spell.validate({'spell': spell}, deps, catalog)
        if errors:
            raise ValueError('source companion strict candidate validation: ' + ' | '.join(errors))
        target = out / SNAPSHOT / row_id
        target.mkdir(parents=True)
        base.write(target / 'source-header.json', {'spell': source_header})
        base.write(target / 'spell.json', {'spell': spell})
        base.write(target / 'dependencies.json', deps)
        base.write(target / 'catalog.json', catalog)
        proof = {'source_snapshot': SNAPSHOT, 'source_revision': PIN, 'qualified_template_revision': OLD_PIN,
                 'selected_donor': SOURCE, 'canary_precedence_applied': False, 'external_value_overrides_used': False,
                 'module': 'native_companions', 'module_sha256': base.sha(Path(native_companions.__file__).read_bytes()),
                 'helpers': proofs, 'runtime_activation': False,
                 'qualification_scope': 'exact cast, speed/condition replacement domain helpers and optional wheel-grade getter; transitive common engine services unqualified'}
        receipt = {'schema': 'OTERYN_SOURCE_PLAYER_BUNDLE_RECEIPT/v1', 'registration_key': record['registration_key'],
                   'logical_key': ['instant', name], 'source_revision': PIN, 'source_sha256': record['source_sha256'],
                   'candidate_key': candidate_key, 'source_header': source_header,
                   'mechanics_reference': {'source_capture': 'r28/player-source-bundles/source-callback-facts.jsonl.gz',
                       'source_capture_sha256': base.sha((r28 / 'source-callback-facts.jsonl.gz').read_bytes()),
                       'imported_facts': 'source-callback-facts.jsonl.gz',
                       'registration_key': record['registration_key'], 'source_callback_scope': 'exact current Crystal cast facts'},
                   'engine_default_proofs': default_proofs, 'runtime_activation': False, 'external_sources_used': False,
                   'status': 'CANDIDATE_SCHEMA_VALID', 'blockers': [],
                   'remaining_mechanics': [{'source_field': 'companion.transitive_engine_services',
                       'reason': 'Typed speed/familiar/grade behavior retains source facts; live scheduler, character and movement providers remain unqualified'}],
                   'dependencies': {k: 0 for k in deps}, 'schema_and_semantic_validation_errors': [],
                   'conversion_notes': ['Exact source-only current Crystal companion descriptor; no Canary selection', json.dumps(proof, sort_keys=True)],
                   'native_execution_qualified': False, 'item_owner_bindings_required': []}
        if name == 'swift foot':
            grade_fact = {'schema': 'OTERYN_SOURCE_COMPANION_GRADE_ACTIONS/v1',
                          'registration_key': record['registration_key'], 'source_revision': PIN,
                          'source_sha256': record['source_sha256'],
                          'actions': [{'grade': 'none', 'action': 'apply_modifier', 'percent': 70, 'duration_ms': 10000},
                                      {'grade': 'regular', 'action': 'apply_modifier', 'percent': 50, 'duration_ms': 10000},
                                      {'grade': 'greater', 'action': 'no_op', 'preserve_existing_modifier': True,
                                       'execution_equivalence': False, 'runtime_admission': 'blocked'}],
                          'descriptor_placeholder': {'field': '/execution/native_behavior/parameters/damage_dealt_percent/greater',
                                                     'value': 100, 'classification': 'legacy_model_placeholder_not_source_write'},
                          'execution_equivalence': False, 'runtime_admission': 'blocked',
                          'external_sources_used': False, 'runtime_activation': False}
            base.validate_spell.Draft202012Validator(grade_actions_schema()).validate(grade_fact)
            base.write(target / 'source-grade-actions.json', grade_fact)
            receipt['remaining_mechanics'].append({'source_field': 'swift_foot.greater_grade_no_op',
                'reason': 'Source GREATER action=no_op preserves any existing modifier; native greater=100 is a legacy_model_placeholder_not_source_write. execution_equivalence=false; runtime_admission=blocked for this branch. Strict source-grade-actions.json retains the exact source actions.'})
            receipt['conversion_notes'].append('source-grade-actions.json: GREATER creates/removes no attribute condition; prior REGULAR 50 percent slot may persist. Native descriptor 100 is not a source write.')
        receipt_validator.validate(receipt)
        base.write(target / 'receipt.json', receipt)
        index.append({'registration_key': record['registration_key'], 'candidate_key': candidate_key,
                      'candidate_revision': REVISION, 'status': 'CANDIDATE_SCHEMA_VALID', 'blockers': []})
    summary = {'schema': 'OTERYN_SOURCE_PLAYER_BUNDLE_IMPORT/v1', 'records': len(index),
               'source_populations': {SNAPSHOT: len(index)}, 'status_counts': {'CANDIDATE_SCHEMA_VALID': len(index)},
               'external_sources_used': False, 'runtime_activation': False, 'full_source_mechanics_1_to_1_complete': False,
               'all_receipts_schema_valid': True, 'native_descriptor_count': len(index), 'records_index': index,
               'revision': REVISION, 'source_revision': PIN, 'source_callback_payload_sha256': base.sha(payload),
               'input_proofs': {'r28/package-manifest.json': base.sha((r28 / 'package-manifest.json').read_bytes()),
                                'r28/source-callback-facts.jsonl.gz': base.sha((r28 / 'source-callback-facts.jsonl.gz').read_bytes())},
               'converter_proofs': {p.name: base.sha(p.read_bytes()) for p in (
                   Path(__file__), Path(base.__file__), Path(native_companions.__file__), HERE / 'spell.schema.json', HERE / 'spell-dependencies.schema.json')}}
    base.write(out / 'import-summary.json', summary)
    base.write(out / 'package-manifest.json', {'schema': 'OTERYN_SOURCE_PLAYER_PACKAGE/v1',
        'files': {p.relative_to(out).as_posix(): base.sha(p.read_bytes()) for p in sorted(out.rglob('*')) if p.is_file()}})
    return summary


def run(out, source_root, r28):
    if out.exists():
        raise ValueError('r29 output must be a new directory; existing source packages are immutable')
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.r29-player-companions-', dir=out.parent) as temp:
        stage = Path(temp) / 'package'
        stage.mkdir()
        summary = generate(stage, source_root, r28)
        stage.rename(out)
    return summary


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, default=Path('/workspace/spell-sources'))
    parser.add_argument('--r28', type=Path, default=ROOT / 'docs/reference/spells/r28-source-closure/player-source-bundles')
    args = parser.parse_args()
    print(json.dumps(run(args.out, args.source_root, args.r28), sort_keys=True))
