#!/usr/bin/env python3
"""Package verified source-only monster JSON, preserving every selected byte."""
import argparse
from collections import Counter
import gzip
import hashlib
import io
import json
from pathlib import Path
import platform
import tarfile
import zlib


def digest(data):
    return hashlib.sha256(data).hexdigest()


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode()


def load(path):
    return json.loads(path.read_bytes())


class HashSink:
    def __init__(self):
        self.hash = hashlib.sha256()
        self.bytes = 0

    def write(self, data):
        self.hash.update(data)
        self.bytes += len(data)
        return len(data)

    def flush(self):
        pass


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input', type=Path, required=True)
    args = parser.parse_args()
    source = args.input.resolve(strict=True)
    output = Path(__file__).resolve().parent
    repository = output.parents[3]
    proof = load(source / 'verification-proof.json')
    summary = load(source / 'summary.json')
    assert proof['status'] == 'PASS' and proof['checks'] > 0 and proof['failures'] == []
    assert summary['runtime_activation'] is False and summary['source_first'] is True
    assert summary['external_reference_inputs'] == 0 and load(source / 'reference-inputs.json') == []
    excluded = ['all-registered-spells.json', 'monster-profiles.json', 'monster-spell-slots.json']
    top = ['summary.json', 'verification-proof.json', 'reference-inputs.json']
    for donor in ('canary', 'crystal'):
        top += [f'{donor}-monster-{kind}.json' for kind in ('files', 'profiles', 'spell-slots')]
        top += [f'{donor}-conversion-errors.json', f'registered-spells/{donor}.json']
    selected = {name: source / name for name in top}
    for path in (source / 'converted-bundles').rglob('*'):
        if path.is_file():
            assert not path.is_symlink() and path.name in {'monster.json', 'dependencies.json', 'catalog.json', 'manifest.json'}
            selected[path.relative_to(source).as_posix()] = path
    actual = {p.relative_to(source).as_posix() for p in source.rglob('*') if p.is_file()}
    assert actual == set(selected) | set(excluded)
    assert all(p.suffix == '.json' and not p.is_symlink() for p in selected.values())
    for name, expected in proof['input_sha256'].items():
        assert digest((source / name).read_bytes()) == expected
    conservation = {'scope': 'Exact source-only inventory/structure conservation; no external or runtime qualification', 'donors': {}}
    for donor in ('canary', 'crystal'):
        profiles = load(source / f'{donor}-monster-profiles.json')
        slots = load(source / f'{donor}-monster-spell-slots.json')
        files = load(source / f'{donor}-monster-files.json')
        registrations = load(source / f'registered-spells/{donor}.json')
        assert all(x['runtime_activation'] is False for x in profiles + slots + registrations)
        ids = {x['candidate_id'] for x in profiles}
        assert len(ids) == len(profiles)
        assert all(x['candidate_id'] in ids for x in slots)
        keys = [(x['candidate_id'], x['group'], x['source_slot_index']) for x in slots]
        assert len(set(keys)) == len(keys)
        for profile in profiles:
            for name, expected in profile.get('bundle_sha256', {}).items():
                assert digest(selected[f"{profile['bundle_path']}/{name}"].read_bytes()) == expected
        conservation['donors'][donor] = {
            'profiles': len(profiles), 'source_monster_files': len(files),
            'spell_slots': len(slots), 'spell_registrations': len(registrations),
            'profile_outcomes': dict(sorted(Counter(x['conversion_status'] for x in profiles).items())),
            'source_file_outcomes': dict(sorted(Counter(x.get('conversion_status', x.get('status', 'MISSING_STATUS')) for x in files).items())),
            'slot_outcomes': dict(sorted(Counter(x['conversion_status'] for x in slots).items())),
            'conversion_errors': len(load(source / f'{donor}-conversion-errors.json')),
        }
    for combined, splits in {
        'monster-profiles.json': ['canary-monster-profiles.json', 'crystal-monster-profiles.json'],
        'monster-spell-slots.json': ['canary-monster-spell-slots.json', 'crystal-monster-spell-slots.json'],
        'all-registered-spells.json': ['registered-spells/canary.json', 'registered-spells/crystal.json'],
    }.items():
        values = load(source / splits[0]) + load(source / splits[1])
        assert encoded(values) == (source / combined).read_bytes()
    conservation['combined_arrays_reconstructed_byte_identically'] = excluded
    conservation['verified_original_checks'] = proof['checks']
    conservation['original_proof_sha256'] = digest((source / 'verification-proof.json').read_bytes())
    code_paths = [
        'docs/reference/spells/r22-audit/monster-import-current/import_all_monster_spells.py',
        'docs/reference/spells/r22-audit/monster-import-current/verify_import.py',
        'docs/reference/spells/r22-audit/monster-import-current/partial_monster_loader.py',
        'tools/content-schema/monster-authoring/canary_batch.py',
        'tools/content-schema/monster-authoring/combat_types.py',
        'tools/content-schema/monster-authoring/spell_scripts.py',
        'tools/content-schema/monster-authoring/spell_census.py',
        'tools/content-schema/monster-authoring/validate_monster.py',
        'tools/content-schema/monster-authoring/monster.schema.json',
        'tools/content-schema/monster-authoring/monster-dependencies.schema.json',
    ]
    inputs = {
        'schema': 'OTERYN_MONSTER_SOURCE_REPRODUCIBLE_INPUTS/v1',
        'sources': summary['sources'],
        'code_sha256': {name: digest((repository / name).read_bytes()) for name in code_paths},
        'external_reference_inputs': load(source / 'reference-inputs.json'),
        'package_producer_sha256': digest(Path(__file__).resolve().read_bytes()),
        'auxiliary_local_reference_inputs': [
            {'source': donor, 'path': name, 'sha256': digest((Path(summary['source_input_root']) / donor / name).read_bytes()),
             'bytes': (Path(summary['source_input_root']) / donor / name).stat().st_size,
             'provenance_scope': 'Exact local reference bytes consumed by converter; no unsupported claim of tracked donor Git identity',
             'distributed': False}
            for donor in ('canary', 'crystal') for name in ('data/items/items.xml', 'data/items/appearances.dat')
        ],
        'raw_source_files_distributed': False,
        'source_file_identity_index': ['canary-monster-files.json', 'crystal-monster-files.json'],
        'registration_identity_index': ['registered-spells/canary.json', 'registered-spells/crystal.json'],
        'conversion_command_from_repository': 'python docs/reference/spells/r22-audit/monster-import-current/import_all_monster_spells.py --source-inputs SOURCE_INPUTS --out NEW_GENERATED_DIRECTORY',
        'verification_command_from_repository': 'python docs/reference/spells/r22-audit/monster-import-current/verify_import.py --input NEW_GENERATED_DIRECTORY',
        'environment_requirements': [
            'Python with repository monster-authoring requirements (including lupa), Git and exact code digests above',
            'Local Git objects at the pinned revisions; importer/verifier currently read /workspace/spell-sources/{canary,crystal}',
            'SOURCE_INPUTS/{canary,crystal} contains the pinned donor snapshot, including monster/script directories and data/items/items.xml plus appearances.dat; acquire separately and retain locally',
            'summary source_input_root and elapsed_seconds are original-run observations, not portable authority or byte-stable conversion outputs',
        ],
        'archive_reproduction': 'python docs/reference/spells/r27-source-closure/monster-source-package.py --input EXACT_VERIFIED_GENERATED_DIRECTORY',
        'reproduction_scope': 'Archive bytes are reproducible from exact retained generated bytes and stated Python/zlib environment. Regeneration semantic/hash parity uses pinned code/source inputs; original elapsed/path metadata is preserved only as historical evidence.',
    }
    metadata = {'monster-source-inputs.json': encoded(inputs), 'monster-conservation.json': encoded(conservation)}
    entries = []
    for name, path in sorted(selected.items()):
        entries.append({'path': name, 'bytes': path.stat().st_size, 'sha256': digest(path.read_bytes()), 'owner': 'Game source-only authoring evidence', 'input_relative_path': name})
    for name, data in sorted(metadata.items()):
        entries.append({'path': name, 'bytes': len(data), 'sha256': digest(data), 'owner': 'Game package derivation evidence'})
    package = {'schema': 'OTERYN_MONSTER_SOURCE_PACKAGE/v1', 'runtime_activation': False,
               'classification': 'Source/schema closure only; every external spell verification remains NOT_VERIFIED',
               'excluded_redundant_combined_arrays': excluded, 'sources': summary['sources'],
               'limits': summary['limits'], 'conservation': conservation, 'entries': sorted(entries, key=lambda x: x['path'])}
    metadata['monster-package-manifest.json'] = encoded(package)
    hashes = {x['path']: x['sha256'] for x in entries}
    hashes['monster-package-manifest.json'] = digest(metadata['monster-package-manifest.json'])
    metadata['SHA256SUMS'] = ''.join(f'{h}  {n}\n' for n, h in sorted(hashes.items())).encode()
    names = sorted(set(selected) | set(metadata))

    def emit(sink):
        with gzip.GzipFile(filename='', mode='wb', fileobj=sink, compresslevel=9, mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode='w|', format=tarfile.PAX_FORMAT) as archive:
                for name in names:
                    data = metadata[name] if name in metadata else selected[name].read_bytes()
                    info = tarfile.TarInfo(name)
                    info.size = len(data)
                    info.mode = 0o644
                    info.mtime = info.uid = info.gid = 0
                    info.uname = info.gname = ''
                    archive.addfile(info, io.BytesIO(data))

    target = output / 'monster-source-package.tar.gz'
    with target.open('wb') as sink:
        emit(sink)
    first = digest(target.read_bytes())
    second = HashSink()
    emit(second)
    assert second.hash.hexdigest() == first and second.bytes == target.stat().st_size
    with tarfile.open(target, 'r:gz') as archive:
        members = archive.getmembers()
        assert [x.name for x in members] == names
        for member in members:
            assert member.isfile() and member.mtime == member.uid == member.gid == 0 and member.mode == 0o644
            data = archive.extractfile(member).read()
            expected = metadata[member.name] if member.name in metadata else selected[member.name].read_bytes()
            assert data == expected
            if member.name in hashes:
                assert digest(data) == hashes[member.name]
    report = {'schema': 'OTERYN_MONSTER_SOURCE_PACKAGE_RECEIPT/v1',
              'owner': 'Game source-only authoring evidence; artifact_composition package files only',
              'runtime_activation': False, 'network_or_publication_performed': False,
              'archive': {'path': target.name, 'sha256': first, 'bytes': target.stat().st_size, 'members': len(names)},
              'original_payload_files': len(selected), 'original_payload_bytes': sum(p.stat().st_size for p in selected.values()),
              'internal_manifest': 'monster-package-manifest.json', 'internal_checksums': 'SHA256SUMS',
              'determinism': {'two_independent_emissions_byte_hash_equal': True, 'gzip_mtime': 0, 'compression': 9, 'tar_format': 'PAX', 'sorted_members': True, 'uid_gid_mtime': 0, 'mode': '0644', 'python': platform.python_version(), 'zlib': zlib.ZLIB_RUNTIME_VERSION},
              'checks': {'all_archive_members_byte_identical': True, 'no_raw_Lua_XML_DAT_or_other_assets': True, 'complete_selected_inventory': True, 'combined_arrays_reconstructed_byte_identically': True, 'original_proof_pass_checks': proof['checks']},
              'conservation': conservation, 'reproducible_inputs': inputs,
              'limits': summary['limits']}
    report_path = output / 'monster-source-package-manifest.json'
    report_path.write_bytes(encoded(report))
    review_files = []
    for source_name, target_name in [('summary.json', 'monster-import-summary.json'), ('verification-proof.json', 'monster-verification-proof.json')]:
        path = output / target_name
        path.write_bytes((source / source_name).read_bytes())
        review_files.append(path)
    owned = [target, report_path, Path(__file__).resolve()] + review_files
    (output / 'monster-source-package.SHA256SUMS').write_text(''.join(f'{digest(x.read_bytes())}  {x.name}\n' for x in owned))
    print(json.dumps({'archive': report['archive'], 'checks': report['checks'], 'deterministic_rebuild': True}))


if __name__ == '__main__':
    main()
