"""File and base-schema guards for new inactive source import sets."""
import json
from pathlib import Path
import shutil

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

from import_source_spell_package import bundle_member_path, digest, encoded, require


def read_base(root, expected_sha):
    folder = root / 'imports/spells/r28'
    body = bundle_member_path(folder, 'import-manifest.json').read_bytes()
    require(digest(body) == expected_sha, 'BASE_MANIFEST_PIN_MISMATCH')
    manifest = json.loads(body)
    require(manifest['admission_status'] == 'source_only_not_active' and manifest['runtime_activation'] is False
            and manifest['native_identity_allocation'] is False, 'BASE_NOT_SOURCE_ONLY')
    for entry in manifest['artifacts'] + manifest['schemaRefs']:
        require(digest(bundle_member_path(folder, entry['path']).read_bytes()) == entry['sha256'],
                'BASE_ARTIFACT_PIN_MISMATCH:' + entry['path'])
    schemas = {s['path']: (folder / s['path']).read_bytes() for s in manifest['schemaRefs']}
    registry = Registry().with_resources((s['uri'], Resource.from_contents(json.loads(schemas[s['path']]))) for s in manifest['schemaRefs'])
    validators = {name: Draft202012Validator(json.loads(data), registry=registry) for name, data in schemas.items()}
    return folder, body, manifest, schemas, validators


def read_package(source):
    body = bundle_member_path(source, 'package-manifest.json').read_bytes()
    manifest = json.loads(body)
    names = set(manifest['files']) | {'package-manifest.json'}
    require(names == {p.relative_to(source).as_posix() for p in source.rglob('*') if p.is_file()}, 'SOURCE_PACKAGE_MEMBERSHIP_MISMATCH')
    data = {}
    for name in sorted(names):
        require(name.endswith('.json') or name == 'source-callback-facts.jsonl.gz', 'ORIGINAL_ASSET_REFUSED')
        data[name] = bundle_member_path(source, name).read_bytes()
        require(name == 'package-manifest.json' or digest(data[name]) == manifest['files'][name], 'SOURCE_PACKAGE_PIN_MISMATCH:' + name)
    return data


def write_source_only_set(root, destination, relative_destination, data, manifest):
    require(destination.resolve() == (root / relative_destination).resolve(), 'IMPORT_DESTINATION_REFUSED')
    require(not destination.exists(), 'IMPORT_SET_ALREADY_EXISTS')
    require(manifest['admission_status'] == 'source_only_not_active' and all(manifest[k] is False for k in (
        'runtime_activation', 'native_identity_allocation', 'canonical_selection_changed', 'native_execution_qualified', 'input_provider_equivalence')),
        'ACTIVE_OR_EQUIVALENT_CLAIM_REFUSED')
    require(len(manifest['artifacts']) == len(data) and {e['path'] for e in manifest['artifacts']} == set(data), 'OUTPUT_MEMBERSHIP_MISMATCH')
    for entry in manifest['artifacts']:
        name = entry['path']
        require(not Path(name).is_absolute() and '..' not in Path(name).parts and digest(data[name]) == entry['sha256']
                and len(data[name]) == entry['bytes'], 'OUTPUT_PIN_MISMATCH:' + name)
    outputs = dict(data, **{'import-manifest.json': encoded(manifest)})
    destination.mkdir(parents=True, exist_ok=False)
    try:
        for name, body in outputs.items():
            path = destination / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(body)
        (destination / 'SHA256SUMS').write_text(''.join(f'{digest(body)}  {name}\n' for name, body in sorted(outputs.items())))
    except BaseException:
        shutil.rmtree(destination)
        raise
    return {'manifest_sha256': digest(outputs['import-manifest.json']), 'files': len(outputs) + 1, 'runtime_activation': False}
