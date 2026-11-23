"""Replay explicitly named, previously reviewed Python test modules with a time bound.

This is audit evidence tooling, not CI registration or semantic review automation.
Use --direct only for a reviewed test script whose own main owns test selection.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--seconds', type=float, default=50)
    parser.add_argument('--direct', action='store_true')
    parser.add_argument('paths', nargs='+')
    args = parser.parse_args()
    args.repo = args.repo.resolve()
    rows = []
    for name in args.paths:
        path = args.repo / name
        command = ([sys.executable, str(path)] if args.direct else
                   [sys.executable, '-m', 'unittest', '-v', path.stem])
        started = time.monotonic()
        child = subprocess.Popen(command, cwd=path.parent, stdout=subprocess.PIPE,
                                 stderr=subprocess.STDOUT, text=True, start_new_session=True)
        timeout = False
        try:
            output, _ = child.communicate(timeout=args.seconds)
        except subprocess.TimeoutExpired:
            timeout = True
            os.killpg(child.pid, signal.SIGKILL)
            output, _ = child.communicate()
        found = re.search(r'Ran (\d+) tests? in ([0-9.]+)s', output)
        count = int(found[1]) if found else None
        skipped_match = re.search(r'\bskipped=(\d+)', output)
        skipped = int(skipped_match[1]) if skipped_match else 0
        completed_ok = sum(line.rstrip().endswith(' ... ok') for line in output.splitlines())
        status = ('TIMEOUT' if timeout else 'FAIL' if child.returncode else
                  'ZERO_UNITTEST_CASES' if count == 0 else 'PASS')
        row = {'path': name, 'source_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
               'command': command, 'cwd': str(path.parent), 'elapsed_seconds': round(time.monotonic() - started, 3),
               'timeout_seconds': args.seconds, 'exit_code': child.returncode,
               'status': status, 'unittest_case_count': count,
               'skipped_case_count': skipped, 'observed_ok_case_count': completed_ok,
               'suite_result': ('ALL_CASES_SKIPPED' if count and count == skipped else
                                'PASS_WITH_SKIPS' if status == 'PASS' and skipped else status),
               'case_results': [line for line in output.splitlines() if ' ... ' in line],
               'output_tail': output[-2000:]}
        rows.append(row)
        args.output.write_text(json.dumps({'limits': 'Execution only; no inferred semantic review or hosted CI proof.',
                                          'results': rows}, indent=2) + '\n')
        print(name, status, count, row['elapsed_seconds'], flush=True)


if __name__ == '__main__':
    main()
