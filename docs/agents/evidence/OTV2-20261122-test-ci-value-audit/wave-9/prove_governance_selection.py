"""Observe real CLI entrypoints and discovery; no fixture/assertion replacement."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--before', type=Path, required=True)
parser.add_argument('--after', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
RENAMES = {'test_main_job_applicability.py': 'governance_main_job_cases.py',
           'test_merge_authority_applicability.py': 'governance_merge_authority_cases.py'}
TARGETS = ('test_agent_governance_pr_targets.py', 'test_main_job_applicability.py',
           'test_merge_authority_applicability.py')
CHILD = r'''
import json, pathlib, runpy, sys, unittest
source, record = map(pathlib.Path, sys.argv[1:])
sys.path.insert(0, str(source.parent))
ids = []
original = unittest.TestResult.startTest
def observed(self, test):
    ids.append(pathlib.Path(getattr(type(test), test._testMethodName).__code__.co_filename).name + '::' + type(test).__qualname__ + '.' + test._testMethodName)
    return original(self, test)
unittest.TestResult.startTest = observed
sys.argv = [str(source)]
code = 0
try:
    runpy.run_path(str(source), run_name='__main__')
except SystemExit as error:
    code = error.code or 0
record.write_text(json.dumps({'ids': ids, 'exit_code': code}))
raise SystemExit(code)
'''
DISCOVERY = r'''
import json, pathlib, sys, unittest
root = pathlib.Path(sys.argv[1])
loader = unittest.TestLoader()
suite = unittest.TestSuite()
for pattern in ('test_agent_governance_pr_targets.py', 'test_*applicability.py'):
    suite.addTests(loader.discover(str(root / 'tools/repository'), pattern=pattern))
def cases(node):
    for test in node:
        if isinstance(test, unittest.TestSuite): yield from cases(test)
        else: yield test
ids = [pathlib.Path(getattr(type(test), test._testMethodName).__code__.co_filename).name + '::' + type(test).__qualname__ + '.' + test._testMethodName for test in cases(suite)]
result = unittest.TextTestRunner(verbosity=1).run(suite)
print(json.dumps({'ids': ids, 'tests': result.testsRun, 'errors': len(result.errors),
                  'failures': len(result.failures), 'skips': len(result.skipped)}))
raise SystemExit(not result.wasSuccessful())
'''


def normalized(ids):
    return [RENAMES.get(case.split('::', 1)[0], case.split('::', 1)[0]) + '::' + case.split('::', 1)[1] for case in ids]


def run(repo: Path) -> dict:
    repo = repo.resolve()
    started = time.perf_counter()
    ids, commands = [], []
    with tempfile.TemporaryDirectory(prefix='governance-case-count-') as temp:
        for name in TARGETS:
            source = repo / 'tools/repository' / name
            if not source.is_file():
                continue
            record = Path(temp) / (name + '.json')
            done = subprocess.run([sys.executable, '-B', '-c', CHILD, str(source), str(record)],
                                  capture_output=True, text=True, cwd=repo)
            observed = json.loads(record.read_text())
            ids.extend(observed['ids'])
            commands.append({'entrypoint': name, 'rc': done.returncode,
                             'ids': observed['ids'], 'stdout': done.stdout,
                             'stderr': done.stderr})
            assert done.returncode == 0, done.stderr
        discovery = subprocess.run([sys.executable, '-B', '-c', DISCOVERY, str(repo)],
                                   capture_output=True, text=True, cwd=repo)
        assert discovery.returncode == 0, discovery.stderr
        discover_result = json.loads(discovery.stdout)
    return {'cli': {'executions': len(ids), 'unique_cases': len(set(ids)), 'ids': ids, 'normalized_ids': normalized(ids),
                    'commands': commands}, 'discovery': discover_result,
            'seconds': round(time.perf_counter() - started, 4)}


before, after = run(args.before), run(args.after)
assert before['cli']['executions'] == 27
assert after['cli']['executions'] == 16
assert set(normalized(before['cli']['ids'])) == set(normalized(after['cli']['ids']))
assert before['discovery']['tests'] == 27
assert after['discovery']['tests'] == 16
assert set(normalized(before['discovery']['ids'])) == set(normalized(after['discovery']['ids']))
byte_identity = {}
for old, new in RENAMES.items():
    original = (args.before / 'tools/repository' / old).read_bytes()
    candidate = (args.after / 'tools/repository' / new).read_bytes()
    assert original == candidate
    byte_identity[old] = {'new_path': new, 'sha256': hashlib.sha256(candidate).hexdigest()}
workflow = (args.before / '.github/workflows/agent-governance.yml').read_bytes()
assert workflow == (args.after / '.github/workflows/agent-governance.yml').read_bytes()
wrapper = (args.before / 'tools/repository/test_agent_governance_pr_targets.py').read_text()
expected_wrapper = wrapper.replace('from test_main_job_applicability import', 'from governance_main_job_cases import').replace('from test_merge_authority_applicability import', 'from governance_merge_authority_cases import')
assert expected_wrapper == (args.after / 'tools/repository/test_agent_governance_pr_targets.py').read_text()
result = {'baseline_root': str(args.before.resolve()), 'candidate_root': str(args.after.resolve()), 'explicit_module_id_renames': RENAMES,
          'scope': 'Actual three CLI entrypoints before; sole existing owner after. '
                   'Discovery of the same governance slice. Not full CI timing.',
          'before': before, 'after': after,
          'unique_case_ids_unchanged': True, 'renamed_helpers_identical_bytes': byte_identity,
          'protected_workflow_git_blob': hashlib.sha1(f'blob {len(workflow)}\0'.encode() + workflow).hexdigest()}
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'cli': [before['cli']['executions'], after['cli']['executions']],
                  'discovery': [before['discovery']['tests'], after['discovery']['tests']],
                  'unique_cases': len(set(after['cli']['ids'])),
                  'workflow_git_blob': result['protected_workflow_git_blob']}, indent=2))
