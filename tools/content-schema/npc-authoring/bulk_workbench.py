#!/usr/bin/env python3
"""One workspace for batch NPC planning and existing offline validation tools."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
WORK = None
LEDGER = ROOT / 'docs/agents/evidence/OTV2-20261001-npc-source-audit-r19/original160-progress.json'
TOOLS = ROOT / 'tools/content-schema/npc-authoring'
PYTHON = Path(sys.executable)


def prepare():
    data = json.loads(LEDGER.read_text())
    admitted = {'DEFINITION_ADMITTED_PREDECESSOR', 'QUALIFIED_EIGHT_CANDIDATE'}
    targets = [r for r in data['actors'] if r['status'] not in admitted]
    assert len(targets) == len({r['key'] for r in targets})
    WORK.mkdir(parents=True, exist_ok=True)
    if (WORK / 'targets.json').exists():
        raise SystemExit('targets.json already exists; use check or choose a fresh workspace to preserve progress.')
    parts = [targets[i:i + 45] for i in range(0, len(targets), 45)]
    rows = []
    for batch, group in enumerate(parts, 1):
        for actor in group:
            rows.append({'key': actor['key'], 'name': actor['name'], 'batch': batch,
                         'state': 'IMPORT_PENDING', 'previous_hold': actor['status'],
                         'field_quality': {}, 'remaining_tasks': actor.get('reasons', []),
                         'native_runtime_loaded': False})
    result = {'schema': 'NPC_BULK_WORKSPACE/v1', 'base_head': subprocess.check_output(
        ['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        'ledger_sha256': hashlib.sha256(LEDGER.read_bytes()).hexdigest(),
        'batch_sizes': [len(group) for group in parts], 'records': rows,
        'field_quality_values': ['verified', 'donor', 'defaulted', 'placeholder', 'todo'],
        'policy': 'Approximate NPC content is allowed with explicit per-field flags; missing features do not hold the whole NPC.',
        'server_environment': 'Native NPC runtime integration and dev-map setup are planned, not running.'}
    (WORK / 'targets.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'targets': len(targets), 'batch_sizes': result['batch_sizes'],
                      'runtime_loaded': 0, 'state': 'IMPORT_PENDING'}))


def check():
    data = json.loads((WORK / 'targets.json').read_text())
    assert data['ledger_sha256'] == hashlib.sha256(LEDGER.read_bytes()).hexdigest()
    assert len(data['records']) == len({r['key'] for r in data['records']})
    assert all(q in data['field_quality_values'] for r in data['records'] for q in r['field_quality'].values())
    assert not any(r['native_runtime_loaded'] for r in data['records']), 'This tool does not load runtime NPCs.'
    assert [sum(r['batch'] == i for r in data['records']) for i in range(1, len(data['batch_sizes']) + 1)] == data['batch_sizes']
    counts = {state: sum(r['state'] == state for r in data['records'])
              for state in sorted({r['state'] for r in data['records']})}
    print(json.dumps({'targets': len(data['records']), 'batch_sizes': data['batch_sizes'], 'states': counts,
                      'runtime_loaded': sum(r['native_runtime_loaded'] for r in data['records'])}))


def smoke():
    env = dict(os.environ)
    WORK.mkdir(parents=True, exist_ok=True)
    command = [str(PYTHON), '-m', 'unittest', 'test_npc_authoring', 'test_schema_regressions']
    with (WORK / 'smoke.log').open('w') as log:
        result = subprocess.run(command, cwd=TOOLS, env=env, stdout=log, stderr=subprocess.STDOUT)
    receipt = {'command': command, 'exit_code': result.returncode,
               'log_sha256': hashlib.sha256((WORK / 'smoke.log').read_bytes()).hexdigest(),
               'scope': 'Existing offline source schema and NPC authoring tools; not a gameplay server.'}
    (WORK / 'smoke-result.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(receipt))
    sys.exit(result.returncode)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['prepare', 'check', 'smoke'])
    parser.add_argument('--workdir', type=Path, required=True, help='External workspace for target files and logs; do not use content/.')
    args = parser.parse_args()
    WORK = args.workdir.resolve()
    if WORK == ROOT or ROOT in WORK.parents:
        parser.error('--workdir must be outside the repository')
    {'prepare': prepare, 'check': check, 'smoke': smoke}[args.command]()
