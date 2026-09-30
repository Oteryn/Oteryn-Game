"""The client 15.30 creatures that have no Canary monster file (evidence for §10.3 of the monster schema).

Evidence tooling only. Reads the staticdata creature names (imports/cipsoft-staticdata/creatures), the monster names
of Canary at its pinned commit and of CrystalServer at its pinned commit (from the commit trees, not a directory walk),
and lists every client creature Canary lacks, with the CrystalServer file that holds it outside the one
crystal_batch.py reads. --check compares the names and directories with the committed sample; the family, kind and
wiki fields of the sample are a preparation heuristic and are not recomputed here.

Usage: python wiki_only_candidates.py --canary <checkout> --crystal <checkout> [--check]
"""
import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

import canary_batch as cb
import crystal_batch

ROOT = Path(__file__).resolve().parent
STATICDATA = ROOT.parents[2] / 'imports' / 'cipsoft-staticdata' / 'creatures'
SAMPLE = ROOT / 'samples' / 'wiki-only-candidates-2026-09-30.json'
ADMITTED_FROM_WIKI = {'dark merudri'}  # D44, wiki_authored.py


def monster_names(checkout, revision, prefix):
    """{lowercase name: path} of every monster file under `prefix` in the commit tree."""
    head = subprocess.run(['git', '-C', checkout, 'rev-parse', 'HEAD'], capture_output=True, text=True, check=True).stdout.strip()
    if head != revision:
        raise SystemExit(f'{checkout}: HEAD {head} is not the pinned {revision}')
    paths = subprocess.run(['git', '-C', checkout, 'ls-tree', '-r', '--name-only', revision, prefix],
                           capture_output=True, text=True, check=True).stdout.split('\n')
    paths = [p for p in paths if p.endswith('.lua')]
    batch = subprocess.run(['git', '-C', checkout, 'cat-file', '--batch'], input=''.join(f'{revision}:{p}\n' for p in paths).encode(),
                           capture_output=True, check=True).stdout
    names, offset = {}, 0
    for path in paths:
        header_end = batch.index(b'\n', offset)
        size = int(batch[offset:header_end].split()[2])
        text = batch[header_end + 1:header_end + 1 + size].decode('utf-8', errors='replace')
        offset = header_end + 1 + size + 1
        match = re.search(r'Game\.createMonsterType\("([^"]+)"', text)
        if match:
            names.setdefault(match.group(1).lower(), path)
    return names


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--canary', required=True)
    parser.add_argument('--crystal', required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    client, digest = [], hashlib.sha256()
    for path in sorted(STATICDATA.glob('creatures-*.json')):
        digest.update(path.read_bytes())
        client += [record['name'].lower() for record in json.loads(path.read_text(encoding='utf-8'))['records']]
    canary = monster_names(args.canary, cb.REVISION, cb.MONSTER_DIR)
    crystal = monster_names(args.crystal, crystal_batch.REVISION, 'data-global/monster')
    rows = []
    for name in sorted(set(client)):
        if name in canary or name in ADMITTED_FROM_WIKI:
            continue
        path = crystal.get(name)
        if path and path.startswith(crystal_batch.MONSTER_DIR + '/'):
            continue
        rows.append({'name': name, 'crystal_other_dir': str(Path(path).relative_to('data-global/monster')) if path else None})
    result = {'staticdata_sha256': digest.hexdigest(), 'canary': cb.REVISION, 'crystal': crystal_batch.REVISION, 'monsters': rows}
    if args.check:
        sample = json.loads(SAMPLE.read_text(encoding='utf-8'))
        expected = [{'name': m['name'], 'crystal_other_dir': m['crystal_other_dir']} for m in sample['monsters']]
        if sorted(expected, key=lambda m: m['name']) != rows or sample.get('staticdata_sha256') != result['staticdata_sha256']:
            raise SystemExit('wiki-only candidates differ from the committed sample')
        print(json.dumps({'check': 'ok', 'monsters': len(rows)}))
        return
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
