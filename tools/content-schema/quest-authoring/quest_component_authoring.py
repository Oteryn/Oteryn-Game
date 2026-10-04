"""Reproduce all 248 Source component definitions and their honest Quest crosswalk."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile

import jsonschema
from donor_sources.validate_archive import verify_archive

DIRECTORY = 'tools/content-schema/quest-authoring/samples/donor-source/components248'
OUTPUTS = ('assignments/all.json', 'assignments/boss.json', 'assignments/events.json',
           'assignments/other.json', 'boss.json.gz', 'events.json.gz', 'other.json.gz',
           'crosswalk.json', 'dialogue.json.gz', 'summary.json')


def read(path):
    raw = path.read_bytes()
    return json.loads(gzip.decompress(raw) if path.suffix == '.gz' else raw)


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    raw = (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + '\n').encode()
    path.write_bytes(gzip.compress(raw, mtime=0) if path.suffix == '.gz' else raw)


def partition(records):
    result = {name: [] for name in ('boss', 'events', 'other')}
    for row in records:
        tags = row['source_only_mechanisms']
        name = ('boss' if 'boss_lever_shared_constructor' in tags else
                'events' if any(tag.endswith('_event') for tag in tags) else 'other')
        result[name].append(row)
    ids = [r['source_component_id'] for rows in result.values() for r in rows]
    if len(ids) != len(set(ids)) or len(ids) != len(records):
        raise ValueError('component assignments overlap or omit records')
    return result


def assemble(root, manifest, ast_root, output):
    here = Path(__file__).resolve().parent
    modules = here / 'donor_sources/components'
    records = read(here / 'samples/donor-source/inventory/unjoined-components.json')['records']
    lanes = partition(records)
    write(output / 'assignments/all.json', {'records': records})
    for name, rows in lanes.items():
        write(output / 'assignments' / (name + '.json'), {'records': rows})
        temporary = output / (name + '.json')
        subprocess.run([sys.executable, str(modules / name / 'builder.py'),
                        '--assignment', str(output / 'assignments' / (name + '.json')),
                        '--corpus-manifest', str(manifest), '--ast-root', str(ast_root),
                        '--out', str(temporary)], check=True)
        packet = read(temporary)
        jsonschema.Draft202012Validator(read(modules / name / 'schema.json')).validate(packet)
        expected = {r['source_component_id']: r['provenance'] for r in rows}
        observed = {r['source_component_id']: r['provenance'] for r in packet['records']}
        if len(observed) != len(packet['records']) or set(observed) != set(expected):
            raise ValueError('Source component membership differs: ' + name)
        for key, source in expected.items():
            if any(observed[key].get(field) != value for field, value in source.items()):
                raise ValueError('Source component identity differs: ' + key)
        write(output / (name + '.json.gz'), packet)
        temporary.unlink()
    subprocess.run([sys.executable, str(modules / 'joins/component_crosswalk.py'),
                    '--repo-root', str(root), '--assignment', str(output / 'assignments/all.json'),
                    '--corpus-manifest', str(manifest), '--out', str(output / 'crosswalk.json')], check=True)
    crosswalk = read(output / 'crosswalk.json')
    jsonschema.Draft202012Validator(read(modules / 'joins/component-crosswalk.schema.json')).validate(crosswalk)
    subprocess.run([sys.executable, str(modules / 'dialogue/dialogue_supplement.py'),
                    '--repo-root', str(root), '--corpus-manifest', str(manifest),
                    '--ast-root', str(ast_root), '--out', str(output / 'dialogue.json')], check=True)
    dialogue = read(output / 'dialogue.json')
    jsonschema.Draft202012Validator(read(modules / 'dialogue/dialogue_supplement.schema.json')).validate(dialogue)
    write(output / 'dialogue.json.gz', dialogue)
    (output / 'dialogue.json').unlink()
    summary = {'schema': 'OTERYN_SOURCE_COMPONENT_BATCH_COVERAGE/v1',
               'scope': 'SOURCE_DEFINITIONS_NOT_NATIVE_OR_COMPLETE_QUEST_CERTIFICATION',
               'component_count': len(records), 'definition_count': sum(len(read(output / (name + '.json.gz'))['records']) for name in lanes),
               'lane_summaries': {name: read(output / (name + '.json.gz'))['summary'] for name in lanes},
               'crosswalk': crosswalk['summary'], 'dialogue': dialogue['summary'],
               'quest_semantic_completeness': 'NOT_ESTABLISHED', 'native_admission': False,
               'external_wiki_read': False,
               'outputs': [{'path': name, 'sha256': hashlib.sha256((output / name).read_bytes()).hexdigest()} for name in OUTPUTS[:-1]]}
    write(output / 'summary.json', summary)
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument('--corpus-manifest', type=Path)
    parser.add_argument('--ast-root', type=Path)
    parser.add_argument('--out', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    root = args.root.resolve()
    inputs = root / 'tools/content-schema/quest-authoring/samples/donor-source'
    destination = args.out or root / DIRECTORY
    with tempfile.TemporaryDirectory(prefix='quest-components-') as temporary:
        scratch = Path(temporary)
        manifest = args.corpus_manifest
        if manifest is None:
            declaration = read(inputs / 'corpus-input.json')
            archive = inputs / declaration['path']
            verify_archive(archive, declaration['sha256'])
            with tarfile.open(archive, 'r:gz') as stream:
                stream.extractall(scratch / 'corpus', filter='data')
            manifest = scratch / 'corpus/corpus-manifest.json'
        summary = assemble(root, manifest, args.ast_root or inputs / 'lua-ast', scratch / 'outputs')
        for name in OUTPUTS:
            target = destination / name
            data = (scratch / 'outputs' / name).read_bytes()
            if args.check:
                if not target.is_file() or target.read_bytes() != data:
                    raise ValueError('stale Source component output: ' + name)
            else:
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
        print(json.dumps({'components': summary['component_count'], 'definitions': summary['definition_count'],
                          'native_admission': False}, sort_keys=True))


if __name__ == '__main__':
    main()
