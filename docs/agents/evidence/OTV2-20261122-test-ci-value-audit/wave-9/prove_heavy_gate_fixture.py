#!/usr/bin/env python3
"""Replay the real aggregate fixture against current and legacy summarizers.

Read repository trees only. The legacy replay removes expected_rows solely
from the serialized plan because its old validator rejects unknown fields.
Scenario evidence and the actual workflow aggregate script are unchanged.
"""

import argparse
from collections import Counter
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from unittest.mock import patch


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', type=Path, required=True)
    parser.add_argument('--after', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    before, after = args.before.resolve(), args.after.resolve()
    target = Path('tools/repository/test_heavy_gate.py')
    matrix_path = Path('tools/qualification/node_boot/matrix.py')
    sys.path.insert(0, str(after / target.parent))
    spec = importlib.util.spec_from_file_location('review_heavy_gate', after / target)
    test_module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(test_module)
    case = test_module.HeavyGateTests('test_node_boot_aggregate_fails_closed')
    original_run = subprocess.run
    result = {
        'scope': 'Actual fixture helper and workflow aggregate subprocesses; no live Game or GitHub Actions run',
        'before': str(before), 'after': str(after),
        'candidate_test_sha256': digest(after / target),
        'candidate_matrix_sha256': digest(after / matrix_path),
        'baseline_matrix_sha256': digest(before / matrix_path),
        'legacy_schema_adapter': 'Delete expected_rows only from each plan item; preserve scenario evidence and workflow script',
        'workflows': [],
    }
    for filename in ('pr-postgres-harness.yml', 'merge-group-gate.yml'):
        workflow = Path('.github/workflows') / filename
        assert (before / workflow).read_bytes() == (after / workflow).read_bytes(), filename
        snapshots = []

        def observe(command, **kwargs):
            env = kwargs['env']
            planned = json.loads(env['SHARDS'])
            directory = Path(env['RUNNER_TEMP'])
            artifacts = {path.parent.name: json.loads(path.read_text())
                         for path in sorted((directory / 'shards').glob('*/result.json'))}
            assert env['PLAN_RESULT'] == env['BUILD_RESULT'] == env['SHARD_RESULT'] == 'success'
            assert env['PLATFORM_RESULT'] == env['WORLD_RESULT'] == 'success'
            assert env['DEFAULT_REQUIRED'] == 'false' and env['DEFAULT_RESULT'] == 'skipped'
            assert all(row['exit_code'] == 0 and row['result'] == 'NODE_BOOT_PASS'
                       and not row['incomplete'] for row in artifacts.values())
            assert all(row['scenarios'] for row in artifacts.values()
                       if next(item for item in planned if item['name'] == row['shard'])['world'])
            current = original_run(command, **kwargs)
            with tempfile.TemporaryDirectory(prefix='bb02-legacy-') as temp:
                legacy_dir = Path(temp)
                shutil.copytree(directory / 'shards', legacy_dir / 'shards')
                legacy_plan = [{key: value for key, value in item.items() if key != 'expected_rows'}
                               for item in planned]
                legacy_env = dict(env, RUNNER_TEMP=str(legacy_dir),
                                  GITHUB_STEP_SUMMARY=str(legacy_dir / 'summary'),
                                  SHARDS=json.dumps(legacy_plan))
                legacy = original_run(command, **dict(kwargs, cwd=before, env=legacy_env))
            snapshots.append({'plan': planned, 'artifacts': artifacts,
                              'current': current, 'legacy': legacy})
            return current

        with patch.object(test_module.subprocess, 'run', side_effect=observe):
            positive = case.run_aggregate(filename)
            missing = case.run_aggregate(filename, omit_required_row=True)
        assert len(snapshots) == 2
        complete, mutated = snapshots
        assert complete['plan'] == mutated['plan']
        before_ids = Counter(scenario['id'] for artifact in complete['artifacts'].values()
                             for scenario in artifact['scenarios'])
        after_ids = Counter(scenario['id'] for artifact in mutated['artifacts'].values()
                            for scenario in artifact['scenarios'])
        removed = list((before_ids - after_ids).elements())
        required = {row['scenario'] for item in complete['plan'] for row in item['expected_rows']}
        assert len(removed) == 1 and removed[0] in required and not after_ids - before_ids
        assert positive.returncode == 0, positive.stdout + positive.stderr
        assert missing.returncode != 0, missing.stdout + missing.stderr
        diagnostic = f'missing terminal verdict for {removed[0]}'
        assert diagnostic in missing.stdout
        assert complete['legacy'].returncode == mutated['legacy'].returncode == 0
        changed = [key for key in complete['artifacts']
                   if complete['artifacts'][key] != mutated['artifacts'][key]]
        assert len(changed) == 1
        shard = changed[0]
        normalized = dict(mutated['artifacts'][shard], scenarios=complete['artifacts'][shard]['scenarios'])
        assert normalized == complete['artifacts'][shard]
        result['workflows'].append({
            'path': str(workflow), 'sha256': digest(after / workflow),
            'candidate_positive_exit_code': positive.returncode,
            'candidate_missing_required_exit_code': missing.returncode,
            'legacy_positive_exit_code': complete['legacy'].returncode,
            'legacy_missing_required_exit_code': mutated['legacy'].returncode,
            'only_removed_id': removed[0], 'changed_artifacts': changed,
            'changed_shard_counts': {'before': len(complete['artifacts'][shard]['scenarios']),
                                     'after': len(mutated['artifacts'][shard]['scenarios'])},
            'all_required_producer_statuses_success': True, 'all_child_exit_codes_zero': True,
            'all_child_results_node_boot_pass': True,
            'candidate_diagnostic': diagnostic,
        })
    result['verdict'] = 'PASS: both real aggregate scripts reject exactly one missing required ID only under the new coverage contract'
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
