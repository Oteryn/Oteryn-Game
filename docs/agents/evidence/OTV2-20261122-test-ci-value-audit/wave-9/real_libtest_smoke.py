"""Compile a tiny real Rust libtest and feed its real outcomes through the S3B tail.

No game, PostgreSQL or Platform topology is started. Requires Rust 1.100.0.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shlex
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--repo', type=Path, required=True)
parser.add_argument('--expected', choices=('baseline', 'fixed'), required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
source_path = args.repo.resolve() / 'tools/qualification/wp5_s3b/run.sh'
source = source_path.read_text()
cleanup = source[source.index('cleanup() {\n'):source.index('\n# Ephemeral fresh-admission signing key')]
tail = source[source.index('(\n  cd "$SEAM_TEST_CWD"\n'):]
rust = '''#[test]
#[ignore]
fn selected_success() {
    println!("probe stdout emitted during an ignored test");
    eprintln!("probe stderr emitted during an ignored test");
}
#[test]
#[ignore]
fn selected_failure() { assert_eq!(1, 2, "intentional smoke failure"); }
'''
cases = []
with tempfile.TemporaryDirectory(prefix='oteryn-s3b-real-libtest-') as directory:
    work = Path(directory)
    fixture = work / 'fixture.rs'
    fixture.write_text(rust)
    executable = work / 'libtest-probe'
    compiler = ['rustc', '+1.100.0']
    version = subprocess.check_output([*compiler, '--version'], text=True).strip()
    compile_result = subprocess.run([*compiler, '--edition=2024', '--test', str(fixture), '-o', str(executable)],
                                    text=True, capture_output=True, timeout=60)
    if compile_result.returncode:
        raise SystemExit(compile_result.stderr)
    for name, test, ignored, baseline_rc, fixed_rc in (
        ('success', 'selected_success', True, 0, 0),
        ('no_exact_match', 'missing_name', True, 0, 1),
        ('selected_but_ignored', 'selected_success', False, 0, 1),
        ('failed_test', 'selected_failure', True, 101, 101),
    ):
        cleanup_dir = work / ('cleanup-' + name)
        cleanup_dir.mkdir()
        command = [str(executable), test, '--exact', '--nocapture', '--color', 'never']
        if ignored:
            command.append('--ignored')
        setup = '\n'.join([
            'set -Eeuo pipefail', 'result=COMPOSED_FAIL', 'PASS_RESULT=SEAM_PASS', 'cleaned=true',
            'WORK=' + shlex.quote(str(cleanup_dir)), 'SEAM_TEST_CWD=' + shlex.quote(str(work)),
            'SEAM_TEST_COMMAND=(' + ' '.join(map(shlex.quote, command)) + ')',
            'FRESH_SEED_HEX=synthetic-unused-value', 'RECOVERY_SEED_HEX=synthetic-unused-value',
            'evidence() { printf "S3B_EVIDENCE %s\\n" "$*"; }',
        ])
        done = subprocess.run(['bash'], input=setup + '\n' + cleanup + '\n' + tail,
                              text=True, capture_output=True, timeout=10)
        expected_rc = fixed_rc if args.expected == 'fixed' else baseline_rc
        expected_verdict = 'SEAM_PASS' if expected_rc == 0 else 'COMPOSED_FAIL'
        if done.returncode != expected_rc or f'S3B_RESULT={expected_verdict}\n' not in done.stdout:
            raise AssertionError((name, done.returncode, expected_rc, done.stdout, done.stderr))
        cases.append({'case': name, 'argv': command[1:], 'exit_code': done.returncode,
                      'expected_exit_code': expected_rc, 'stdout': done.stdout, 'stderr': done.stderr})
result = {'scope': 'Actual tiny Rust libtest and actual S3B cleanup/verdict tail; no Game/PG/Platform run',
          'source_sha256': hashlib.sha256(source.encode()).hexdigest(), 'expected': args.expected,
          'rustc': version, 'fixture': rust, 'compile_exit_code': compile_result.returncode, 'cases': cases}
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'rustc': version, 'source_sha256': result['source_sha256'],
                  'cases': [{k: c[k] for k in ('case', 'exit_code')} for c in cases]}, indent=2))
