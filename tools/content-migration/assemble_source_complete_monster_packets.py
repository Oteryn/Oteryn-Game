"""Compose unsealed monster DATA proposals; keep accepted schemas and imports intact."""
import argparse
import importlib
import json
import sys
from pathlib import Path
import gzip
from contextlib import contextmanager
from jsonschema import Draft202012Validator
from referencing import Registry, Resource

ROOT = Path(__file__).resolve().parents[2]
AUTHORING = ROOT / 'tools/content-schema/monster-authoring'


@contextmanager
def authoring_imports():
    previous = list(sys.path)
    sys.path.insert(0, str(AUTHORING))
    try:
        yield
    finally:
        sys.path[:] = previous


with authoring_imports():
    from source_complete_monster_common import (FLAGS, INPUTS, PINS, OPTIONAL_FALSE_FLAGS, PRODUCER_FAMILIES, canonical, encoded,
                                               sha, verify_rows)

SPELL_URI = 'urn:oteryn:monster-slot:source-complete:candidate:2'
RECEIPT_URI = 'urn:oteryn:monster-slot-receipt:source-complete:candidate:2'
MODULES = {63: 'project_monster_inline_complete', 64: 'project_monster_world_controllers',
           65: 'project_monster_combat_controllers', 66: 'project_monster_staged_controllers'}
EXTENSIONS = {63: 'monster-inline-controller.schema.json', 64: 'monster-world-controllers.schema.json',
              65: 'monster-combat-controllers.schema.json', 66: 'monster-staged-controllers.schema.json'}
EVIDENCE_FILES = {63: 'monster-inline-complete.json.gz', 64: 'monster-world-controllers.json.gz',
                  65: 'monster-combat-controllers.json', 66: 'monster-staged-controllers.json.gz'}


def shape(value):
    """Closed shape for provenance/schedule metadata; not a controller interpreter.

    Behavioral parameters are supplied exclusively by reviewed owning extensions.
    Exact source values are additionally sealed and joined to immutable inputs.
    """
    if value is None:
        return {'type': 'null'}
    if isinstance(value, bool):
        return {'type': 'boolean'}
    if isinstance(value, int):
        return {'type': 'integer'}
    if isinstance(value, float):
        return {'type': 'number'}
    if isinstance(value, str):
        return {'type': 'string'}
    if isinstance(value, list):
        return {'type': 'array', 'items': variants(shape(child) for child in value) if value else False}
    if isinstance(value, dict):
        return {'type': 'object', 'additionalProperties': False,
                'properties': {name: shape(child) for name, child in sorted(value.items())},
                'required': sorted(value)}
    raise ValueError('non-JSON proposal metadata')


def variants(values):
    unique = {canonical(value): value for value in values}
    if not unique:
        raise ValueError('empty schema alternatives')
    return next(iter(unique.values())) if len(unique) == 1 else {'anyOf': list(unique.values())}


def complete(row):
    value = dict(row)
    value.update(FLAGS, status='SOURCE_SCHEMA_VALID', full_slot_projection_complete=True,
                 required_operations_unrepresented=[], source_alias_to_existing_native_profile=False)
    return value


def compose(all_rows):
    extensions = {}
    for filename in EXTENSIONS.values():
        body = (AUTHORING / filename).read_bytes()
        document = json.loads(body)
        if 'controller' not in document.get('$defs', {}):
            raise ValueError('owning schema must expose closed controller')
        if document['$id'] in extensions:
            raise ValueError('duplicate owning controller schema')
        extensions[document['$id']] = (filename, body)
    required = ['slot_identity', 'source', 'monster', 'original_slot_sha256', 'source_parameters',
                'controller', 'source_proofs', 'target_schedule', 'status', 'full_slot_projection_complete',
                'source_alias_to_existing_native_profile', 'required_operations_unrepresented', *FLAGS]
    fields = {name for row in all_rows for name in row}
    properties = {name: variants(shape(row[name]) for row in all_rows if name in row)
                  for name in sorted(fields) if name not in ('controller', 'source_parameters')}
    properties['source_parameters'] = {'type': 'object',
        'description': 'Immutable donor declarations, evidence only; importer requires exact source equality.'}
    properties['controller'] = {'anyOf': [{'$ref': uri + '#/$defs/controller'} for uri in extensions]}
    for name, value in FLAGS.items():
        properties[name] = {'const': value}
    for name in OPTIONAL_FALSE_FLAGS:
        if name in properties:
            properties[name] = {'const': False}
    properties.update(status={'const': 'SOURCE_SCHEMA_VALID'}, full_slot_projection_complete={'const': True},
                      source_alias_to_existing_native_profile={'const': False},
                      required_operations_unrepresented={'type': 'array', 'maxItems': 0})
    model = {'$schema': 'https://json-schema.org/draft/2020-12/schema', '$id': SPELL_URI,
             'title': 'Private source-complete MonsterSlot DATA v2; pending contracts and consumers',
             'type': 'object', 'additionalProperties': False, 'properties': properties, 'required': required}
    receipt_properties = {name: properties[name] for name in ['slot_identity', 'original_slot_sha256',
                         'status', 'full_slot_projection_complete', *FLAGS]}
    receipt_properties['target_model_sha256'] = {'type': 'string', 'pattern': '^[a-f0-9]{64}$'}
    receipt = {'$schema': model['$schema'], '$id': RECEIPT_URI, 'type': 'object',
               'additionalProperties': False, 'properties': receipt_properties,
               'required': sorted(receipt_properties)}
    resources = {filename: body for filename, body in extensions.values()}
    resources['source-complete-monster-slot.schema.json'] = encoded(model)
    resources['source-complete-monster-slot-receipt.schema.json'] = encoded(receipt)
    validators(resources)
    return resources


def validators(resources):
    documents = {json.loads(body)['$id']: json.loads(body) for body in resources.values()}
    if len(documents) != len(resources):
        raise ValueError('duplicate composed schema resource identity')
    for schema in documents.values():
        Draft202012Validator.check_schema(schema)
    registry = Registry().with_resources((uri, Resource.from_contents(schema)) for uri, schema in documents.items())
    return {role: Draft202012Validator(documents[uri], registry=registry)
            for role, uri in [('monster_slot', SPELL_URI), ('receipt', RECEIPT_URI)]}


def assemble(revision, rows, snapshots):
    verify_rows(revision, rows)
    rows = sorted((complete(row) for row in rows), key=lambda row: canonical(row['slot_identity']))
    receipts = [{**{name: row[name] for name in ('slot_identity', 'original_slot_sha256', 'status',
                 'full_slot_projection_complete', *FLAGS)}, 'target_model_sha256': sha(canonical(row))}
                for row in rows]
    checks = validators(snapshots)
    for row, receipt in zip(rows, receipts):
        checks['monster_slot'].validate(row)
        checks['receipt'].validate(receipt)
    audit = [{**receipt, 'controller_kind': row['controller']['kind']} for row, receipt in zip(rows, receipts)]
    folder = ROOT / f'docs/reference/spells/r{revision}-monster-closure'
    if (ROOT / f'imports/spells/r{revision}').exists():
        raise ValueError('sealed monster import already exists')
    files = {'models.json': encoded({'schema': 'OTERYN_PRIVATE_MONSTER_SLOT_MODELS/v2', 'records': rows}),
             'slot-receipts.json': encoded({'records': receipts}), 'lane-audit.json': encoded({'records': audit, **FLAGS}),
             'import-summary.json': encoded({'records': len(rows), 'records_index':
                 [{'slot_identity': row['slot_identity'], 'status': row['status']} for row in rows],
                 'status_counts': {'SOURCE_SCHEMA_VALID': len(rows)}, **FLAGS})}
    proof = {'schema': 'OTERYN_PRIVATE_MONSTER_SLOT_PROJECTION_PROOF/v2', **FLAGS,
             'source_pins': PINS, 'input_proofs': INPUTS,
             'producer_sha256': sha((AUTHORING / (MODULES[revision] + '.py')).read_bytes()),
             'producer_schema_family': PRODUCER_FAMILIES[revision],
             'status_normalization': 'Reviewed private DATA candidates use SOURCE_SCHEMA_VALID in the monster index; execution qualification remains false.',
             'assembler_sha256': sha(Path(__file__).read_bytes()),
             'schema_roles': {'monster_slot': SPELL_URI, 'receipt': RECEIPT_URI},
             'schema_resources': [{'path': 'source-schema-snapshot/' + name, 'uri': json.loads(body)['$id'],
                                   'sha256': sha(body)} for name, body in sorted(snapshots.items())],
             'limits': ['Private typed DATA proposal; accepted Monster v1 is unchanged.',
                        'Source bugs and uncertain equivalence are retained; no execution admission is claimed.']}
    files['projection-proof.json'] = encoded(proof)
    files.update({'source-schema-snapshot/' + name: body for name, body in snapshots.items()})
    # Preserve separately produced bounded evidence, excluding old provisional
    # wrappers and schema copies. No original Lua bodies or assets are packaged.
    if folder.exists():
        for path in folder.rglob('*'):
            if not path.is_file():
                continue
            name = path.relative_to(folder).as_posix()
            if name in files or name == 'package-manifest.json' or name.startswith('source-schema-snapshot/'):
                continue
            if name == EVIDENCE_FILES[revision]:
                body = path.read_bytes()
                json.loads(gzip.decompress(body) if name.endswith('.gz') else body)
                files[name] = body
                continue
            if not name.endswith('.json') and name != 'README.md':
                raise ValueError('unexpected producer asset: ' + name)
            files[name] = path.read_bytes()
        existing_proof = folder / 'projection-proof.json'
        if existing_proof.exists() and json.loads(existing_proof.read_bytes()).get('schema') != proof['schema']:
            files['producer-proof.json'] = existing_proof.read_bytes()
    folder.mkdir(parents=True, exist_ok=True)
    for name, body in files.items():
        path = folder / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(body)
    manifest = {'schema': 'OTERYN_PRIVATE_MONSTER_SLOT_PACKAGE/v2',
                'files': {name: sha(body) for name, body in sorted(files.items())}}
    (folder / 'package-manifest.json').write_bytes(encoded(manifest))
    return {'revision': revision, 'records': len(rows), 'manifest_sha': sha(encoded(manifest)),
            'proof_sha': sha(files['projection-proof.json'])}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assemble', action='store_true', required=True)
    parser.parse_args()
    with authoring_imports():
        by_revision = {revision: importlib.import_module(module).build() for revision, module in MODULES.items()}
    for revision, rows in by_revision.items():
        verify_rows(revision, rows)
    snapshots = compose([complete(row) for rows in by_revision.values() for row in rows])
    print(json.dumps([assemble(revision, rows, snapshots) for revision, rows in by_revision.items()], indent=2))


if __name__ == '__main__':
    main()
