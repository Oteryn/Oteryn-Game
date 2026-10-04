"""Replay donor-only capture inputs offline; never grant semantic or runtime closure."""
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
from donor_sources.validate_corpus import safe_path
from donor_sources.maps.verify import qualify as qualify_maps
from quest_component_authoring import assemble as assemble_components, OUTPUTS as COMPONENT_OUTPUTS
from quest_donor_refinement_authoring import assemble as assemble_refinements, OUTPUTS as REFINEMENT_OUTPUTS

DIRECTORY = 'tools/content-schema/quest-authoring/samples/donor-source'
OUTPUTS = ('npc.json.gz', 'npc-index.json', 'rewards.json.gz', 'npc-dependencies.json',
           'gap-triage.json', 'inventory/inventory.json', 'inventory/unjoined-components.json',
           'inventory/correction-proposals.json', 'semantic-conditions.json',
           'semantic-conditions-qualification.json', 'semantic-rewards.json',
           'semantic-rewards-qualification.json', 'coverage.json')

OUTPUTS = OUTPUTS[:-1] + tuple('components248/' + name for name in COMPONENT_OUTPUTS) + ('coverage.json',)
OUTPUTS = OUTPUTS[:-1] + tuple('refinements/' + name for name in REFINEMENT_OUTPUTS) + ('coverage.json',)

def read(path):
    raw = path.read_bytes()
    return json.loads(gzip.decompress(raw) if path.suffix == '.gz' else raw)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_ast(directory, corpus):
    manifest = read(directory / 'manifest.json')
    if manifest['schema'] != 'OTERYN_PORTABLE_LUA_AST_ARCHIVES/v1' or manifest['native_semantic_admission'] is not False:
        raise ValueError('AST source archive cannot grant Native admission')
    index_path = directory / safe_path(manifest['index']['path'])
    if sha(index_path) != manifest['index']['sha256']:
        raise ValueError('AST index digest differs')
    raw = gzip.decompress(index_path.read_bytes())
    if hashlib.sha256(raw).hexdigest() != manifest['index']['uncompressed_sha256']:
        raise ValueError('AST decoded index digest differs')
    index = json.loads(raw)
    if index['scope'] != 'SOURCE_STRUCTURE_ONLY_NOT_QUEST_OR_RUNTIME_EQUIVALENCE':
        raise ValueError('AST source scope differs')
    expected = {(r['source'], r['revision'], r['path']): r for r in corpus['files']
                if r['path'].lower().endswith(('.lua', '.lua.dist'))}
    observed = {(r['source'], r['revision'], r['path']): r for r in index['sources']}
    if len(observed) != len(index['sources']) or set(observed) != set(expected):
        raise ValueError('AST donor source membership differs')
    for key, row in observed.items():
        if any(row[field] != expected[key][field] for field in
               ('repository', 'sha256', 'git_blob_sha1', 'byte_count')):
            raise ValueError('AST source witness differs')
    captures = {r['sha256']: r for r in index['captures']}
    if len(captures) != len(index['captures']) or set(captures) != {r['sha256'] for r in expected.values()}:
        raise ValueError('AST unique source membership differs')
    seen = set()
    for archive in manifest['archives']:
        path = directory / safe_path(archive['path'])
        if sha(path) != archive['sha256'] or path.stat().st_size != archive['byte_count']:
            raise ValueError('AST archive digest differs')
        archive_seen = set()
        metadata_seen = set()
        with tarfile.open(path, 'r:gz') as stream:
            for member in stream:
                safe_path(member.name)
                if not member.isfile():
                    raise ValueError('AST archive member is not a file')
                if member.name.startswith('metadata/'):
                    name = member.name.removeprefix('metadata/')
                    if name not in manifest['metadata_sidecar_sha256'] or name in metadata_seen:
                        raise ValueError('unregistered/duplicate AST metadata')
                    if hashlib.sha256(stream.extractfile(member).read()).hexdigest() != manifest['metadata_sidecar_sha256'][name]:
                        raise ValueError('AST metadata sidecar digest differs')
                    metadata_seen.add(name)
                    continue
                if not member.name.startswith('captures/'):
                    raise ValueError('unregistered AST archive member')
                key = Path(member.name).name.removesuffix('.json.gz')
                if key in seen or key not in captures:
                    raise ValueError('duplicate/unregistered AST capture')
                raw = stream.extractfile(member).read()
                if hashlib.sha256(raw).hexdigest() != captures[key]['container_sha256']:
                    raise ValueError('AST capture container digest differs')
                seen.add(key)
                archive_seen.add(key)
        if archive_seen != set(archive['capture_sha256s']) or len(archive_seen) != archive['capture_count']:
            raise ValueError('AST shard membership differs')
        if metadata_seen != set(manifest['metadata_sidecar_sha256']):
            raise ValueError('AST metadata sidecar missing')
    if seen != set(captures) or any(row['status'] != 'PARSED' for row in captures.values()):
        raise ValueError('AST capture missing or failed')
    return index['summary']


def assemble(root, scratch):
    here = Path(__file__).resolve().parent
    inputs = root / DIRECTORY
    declaration = read(inputs / 'corpus-input.json')
    corpus_report = verify_archive(inputs / declaration['path'], declaration['sha256'])
    with tarfile.open(inputs / declaration['path'], 'r:gz') as stream:
        stream.extractall(scratch / 'corpus', filter='data')
    manifest_path = scratch / 'corpus/corpus-manifest.json'
    corpus = read(manifest_path)
    if sha(manifest_path) != declaration['manifest_sha256']:
        raise ValueError('portable source manifest differs')
    subprocess.run([sys.executable, str(here / 'donor_sources/npc_capture.py'),
                    '--corpus-manifest', str(manifest_path), '--repo-root', str(root),
                    '--out', str(scratch / 'npc.json.gz'), '--summary-out',
                    str(scratch / 'npc-index.json')], check=True)
    subprocess.run([sys.executable, str(here / 'donor_sources/reward_capture.py'),
                    '--repo-root', str(root), '--corpus-manifest', str(manifest_path),
                    '--out', str(scratch / 'rewards.json.gz')], check=True)
    npc = read(scratch / 'npc.json.gz')
    rewards = read(scratch / 'rewards.json.gz')
    for name, packet in (('npc_capture', npc), ('reward_capture', rewards)):
        jsonschema.Draft202012Validator(read(here / 'donor_sources' / (name + '.schema.json'))).validate(packet)
    ast = verify_ast(inputs / 'lua-ast', corpus)
    maps = qualify_maps(here / 'donor_sources/maps/qualification.json', structural_only=True)
    map_summary = read(here / 'donor_sources/maps/summary.json')
    subprocess.run([sys.executable, str(here / 'donor_sources/npc_dependencies.py'),
                    '--corpus-manifest', str(manifest_path), '--capture',
                    str(scratch / 'npc.json.gz'), '--out',
                    str(scratch / 'npc-dependencies.json')], check=True)
    dependencies = read(scratch / 'npc-dependencies.json')
    jsonschema.Draft202012Validator(read(here / 'donor_sources/npc_dependencies.schema.json')).validate(dependencies)
    # Materialize exact baseline Source paths from the verified CAS for legacy triage.
    roots = {}
    for row in corpus['files']:
        if row['source'] not in ('canary', 'crystalserver'):
            continue
        source_root = scratch / 'source-paths' / row['source']
        roots[row['source']] = source_root
        target = source_root / safe_path(row['path'])
        target.parent.mkdir(parents=True, exist_ok=True)
        target.symlink_to(manifest_path.parent / safe_path(row['cache_path']))
    subprocess.run([sys.executable, str(here / 'ots_gap_triage.py'),
                    '--canary', str(roots['canary']), '--crystal', str(roots['crystalserver']),
                    '--out', str(scratch / 'gap-triage.json')], check=True)
    triage = read(scratch / 'gap-triage.json')
    subprocess.run([sys.executable, str(here / 'donor_sources/donor_inventory.py'),
                    '--root', str(root), '--corpus-manifest', str(manifest_path),
                    '--output', str(scratch / 'inventory')], check=True)
    for name in ('inventory', 'unjoined-components', 'correction-proposals'):
        jsonschema.Draft202012Validator(read(here / 'donor_sources' / (name + '.schema.json'))).validate(read(scratch / 'inventory' / (name + '.json')))
    inventory = read(scratch / 'inventory/inventory.json')
    source_root = scratch / 'source-paths'
    subprocess.run([sys.executable, str(here / 'donor_semantic_conditions.py'),
                    '--authoring', str(here), '--corpus', str(manifest_path),
                    '--ast-root', str(inputs / 'lua-ast'), '--source-root', str(source_root),
                    '--output', str(scratch / 'semantic-conditions.json')], check=True)
    subprocess.run([sys.executable, str(here / 'qualify_conditions.py'),
                    '--authoring', str(here), '--corpus', str(manifest_path),
                    '--source-root', str(source_root), '--packet',
                    str(scratch / 'semantic-conditions.json'), '--output',
                    str(scratch / 'semantic-conditions-qualification.json')], check=True)
    subprocess.run([sys.executable, str(here / 'quest_donor_reward_authoring.py'),
                    '--authoring', str(here), '--corpus-manifest', str(manifest_path),
                    '--source-root', str(source_root), '--out',
                    str(scratch / 'semantic-rewards.json')], check=True)
    subprocess.run([sys.executable, str(here / 'quest_donor_reward_qualify.py'),
                    '--authoring', str(here), '--corpus-manifest', str(manifest_path),
                    '--packet', str(scratch / 'semantic-rewards.json'), '--out',
                    str(scratch / 'semantic-rewards-qualification.json')], check=True)
    semantic_conditions = read(scratch / 'semantic-conditions.json')
    semantic_rewards = read(scratch / 'semantic-rewards.json')
    components = assemble_components(root, manifest_path, inputs / 'lua-ast', scratch / 'components248')
    refinements = assemble_refinements(root, manifest_path, inputs / 'lua-ast', scratch / 'refinements')
    report = {'schema': 'OTERYN_DONOR_FIRST_CAPTURE_COVERAGE/v1',
              'scope': 'SOURCE_CAPTURE_AND_STRUCTURE_NOT_SEMANTIC_QUEST_MIGRATION',
              'source_manifest_sha256': declaration['manifest_sha256'],
              'raw_source': corpus_report['summary'], 'lua_structure': ast,
              'npc': npc['summary'], 'reward_components': rewards['summary'],
              'npc_dependencies': dependencies['summary'],
              'map_structure': {'verification': maps, 'capture_summary': map_summary},
              'normalized_quest_gaps': triage['counts'],
              'donor_quest_inventory': inventory['summary'],
              'semantic_condition_supplement': semantic_conditions['counts'],
              'semantic_reward_supplement': semantic_rewards['summary'],
              'component_batch': {'component_count': components['component_count'], 'definition_count': components['definition_count'],
                                  'crosswalk': components['crosswalk'], 'dialogue': components['dialogue']},
              'quest_refinements': refinements,
              'outputs': [{'path': name, 'sha256': sha(scratch / name)} for name in OUTPUTS[:-1]],
              'source_quest_semantic_1_to_1': 'NOT_ESTABLISHED',
              'map_dependency_closure': 'NOT_ESTABLISHED',
              'native_admission': 'NOT_ASSESSED',
              'external_quest_research_allowed': False}
    (scratch / 'coverage.json').write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='quest-donor-source-') as temporary:
        scratch = Path(temporary)
        report = assemble(args.root.resolve(), scratch)
        for name in OUTPUTS:
            path = args.root / DIRECTORY / name
            data = (scratch / name).read_bytes()
            if args.check:
                if not path.is_file() or path.read_bytes() != data:
                    raise ValueError('stale donor Source capture: ' + name)
            else:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
        print(json.dumps({'raw': report['raw_source'], 'lua': report['lua_structure'],
                          'external_quest_research_allowed': False}, sort_keys=True))


if __name__ == '__main__':
    main()
