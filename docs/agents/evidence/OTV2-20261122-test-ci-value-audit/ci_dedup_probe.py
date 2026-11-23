"""Execute the affected test cases and record normalized IDs, without modifying source."""
import importlib
import json
from pathlib import Path
import sys
import time
import unittest

root = Path(sys.argv[1]).resolve()
output = Path(sys.argv[2])
sys.path.insert(0, str(root / 'tools/repository'))
records = []
original_start = unittest.TestResult.startTest

def start_test(self, test):
    filename = Path(getattr(type(test), test._testMethodName).__code__.co_filename).name
    records.append(f'{filename}::{type(test).__qualname__}.{test._testMethodName}')
    return original_start(self, test)

unittest.TestResult.startTest = start_test
results = []
started = time.monotonic()
for name in (
    'test_validate_pr_gate_pg_sim',
    'test_run_rust_fast_archive',
    'test_agent_governance_pr_targets',
    'test_main_job_applicability',
    'test_merge_authority_applicability',
):
    module = importlib.import_module(name)
    suite = (unittest.defaultTestLoader.loadTestsFromTestCase(module.QualificationPlacementTests)
             if name == 'test_validate_pr_gate_pg_sim' else
             unittest.defaultTestLoader.loadTestsFromModule(module))
    before = len(records)
    result = unittest.TextTestRunner(verbosity=1).run(suite)
    results.append({'module': name, 'top_level_tests': result.testsRun,
                    'executions_including_nested_cases': len(records) - before,
                    'failures': len(result.failures), 'errors': len(result.errors),
                    'skipped': len(result.skipped)})
payload = {'source_root': str(root), 'elapsed_seconds': time.monotonic() - started,
           'scope': 'Actual affected unittest cases; sibling plain-main suites excluded from this focused probe.',
           'executions_including_nested_cases': len(records), 'unique_case_count': len(set(records)),
           'unique_case_ids': sorted(set(records)), 'case_execution_ids': records, 'modules': results}
output.write_text(json.dumps(payload, indent=2) + '\n')
print(json.dumps({k: v for k, v in payload.items() if k not in ('unique_case_ids', 'case_execution_ids')}, indent=2))
raise SystemExit(any(x['failures'] or x['errors'] for x in results))
