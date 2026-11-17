"""Replay only definition-file witnesses after rebuilding the Quest tree.

A full donor capture must precede this command. Every existing capture digest is
verified, and changing any crosswalk association requires another full capture.
"""
import argparse
import copy
import hashlib
import gzip
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile

from donor_sources.validate_archive import verify_archive
from quest_component_authoring import write as write_component

HERE = Path(__file__).resolve().parent
DIRECTORY = 'tools/content-schema/quest-authoring/samples/donor-source'
CROSSWALK = 'components248/crosswalk.json'


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def amend(coverage, previous, expected, raw):
    left, right = copy.deepcopy(previous), copy.deepcopy(expected)
    left.pop('input_sha256')
    right.pop('input_sha256')
    if left != right:
        raise ValueError('crosswalk associations changed; full donor replay required')
    if coverage['component_batch']['crosswalk'] != expected['summary']:
        raise ValueError('crosswalk summary changed; full donor replay required')
    result = copy.deepcopy(coverage)
    rows = [row for row in result['outputs'] if row['path'] == CROSSWALK]
    if len(rows) != 1:
        raise ValueError('crosswalk capture descriptor must occur exactly once')
    rows[0]['sha256'] = digest(raw)
    return result



def require_same(previous, expected, *witness_paths):
    left, right = copy.deepcopy(previous), copy.deepcopy(expected)
    for path in witness_paths:
        a, b = left, right
        for key in path[:-1]:
            a, b = a[key], b[key]
        a.pop(path[-1])
        b.pop(path[-1])
    if left != right:
        raise ValueError('capture semantics changed; full donor replay required')


def read_packet(path):
    raw = path.read_bytes()
    return json.loads(gzip.decompress(raw) if path.suffix == '.gz' else raw)


def outputs(root):
    directory = root / DIRECTORY
    coverage = json.loads((directory / 'coverage.json').read_bytes())
    for row in coverage['outputs']:
        if digest((directory / row['path']).read_bytes()) != row['sha256']:
            raise ValueError('stale capture before witness replay: ' + row['path'])
    declaration = json.loads((directory / 'corpus-input.json').read_bytes())
    archive = directory / declaration['path']
    verify_archive(archive, declaration['sha256'])
    with tempfile.TemporaryDirectory(prefix='quest-crosswalk-witness-') as temporary:
        scratch = Path(temporary)
        with tarfile.open(archive, 'r:gz') as stream:
            stream.extractall(scratch / 'corpus', filter='data')
        manifest = scratch / 'corpus/corpus-manifest.json'
        if digest(manifest.read_bytes()) != declaration['manifest_sha256']:
            raise ValueError('portable source manifest differs')
        target = scratch / 'crosswalk.json'
        subprocess.run([
            sys.executable, str(HERE / 'donor_sources/components/joins/component_crosswalk.py'),
            '--repo-root', str(root), '--assignment', str(directory / 'components248/assignments/all.json'),
            '--corpus-manifest', str(manifest), '--out', str(target),
        ], check=True)
        raw = target.read_bytes()
        report = amend(coverage, json.loads((directory / CROSSWALK).read_bytes()),
                       json.loads(raw), raw)
        subprocess.run([
            sys.executable, str(HERE / 'donor_sources/npc_capture.py'),
            '--corpus-manifest', str(manifest), '--repo-root', str(root),
            '--out', str(scratch / 'npc.json.gz'), '--summary-out', str(scratch / 'npc-index.json'),
        ], check=True)
        subprocess.run([
            sys.executable, str(HERE / 'donor_sources/npc_dependencies.py'),
            '--corpus-manifest', str(manifest), '--capture', str(scratch / 'npc.json.gz'),
            '--out', str(scratch / 'npc-dependencies.json'),
        ], check=True)
        dialogue = scratch / 'dialogue.json'
        subprocess.run([
            sys.executable, str(HERE / 'donor_sources/components/dialogue/dialogue_supplement.py'),
            '--corpus-manifest', str(manifest), '--ast-root', str(directory / 'lua-ast'),
            '--repo-root', str(root), '--out', str(dialogue),
        ], check=True)
        write_component(scratch / 'components248/dialogue.json.gz', json.loads(dialogue.read_bytes()))
        allowed = {
            'npc.json.gz': (('quest_inputs',),),
            'components248/dialogue.json.gz': (('quest_inputs',),),
            'npc-index.json': (('capture_artifact', 'sha256'),),
            'npc-dependencies.json': (('input_provenance', 'npc_capture_sha256'),),
        }
        result = {directory / CROSSWALK: raw}
        for name, paths in allowed.items():
            require_same(read_packet(directory / name), read_packet(scratch / name), *paths)
            result[directory / name] = (scratch / name).read_bytes()
        component_summary = json.loads((directory / 'components248/summary.json').read_bytes())
        for row in component_summary['outputs']:
            path = directory / 'components248' / row['path']
            if path in result:
                row['sha256'] = digest(result[path])
            elif digest(path.read_bytes()) != row['sha256']:
                raise ValueError('unreplayed component capture changed: ' + row['path'])
        write_component(scratch / 'components248/summary.json', component_summary)
        result[directory / 'components248/summary.json'] = (scratch / 'components248/summary.json').read_bytes()
        for row in report['outputs']:
            if directory / row['path'] in result:
                row['sha256'] = digest(result[directory / row['path']])
    result[directory / 'coverage.json'] = (json.dumps(report, sort_keys=True, indent=2) + '\n').encode()
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for path, raw in outputs(HERE.parents[2]).items():
        if args.check:
            if path.read_bytes() != raw:
                raise ValueError('stale donor definition witness: ' + str(path))
        else:
            path.write_bytes(raw)
    print('Donor definition witnesses replayed; Source associations and NPC captures unchanged')


if __name__ == '__main__':
    main()
