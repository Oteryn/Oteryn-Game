#!/usr/bin/env python3
"""Replay the scoped regeneration test probe; never invoke production generation."""
import argparse, contextlib, hashlib, io, json, pathlib, subprocess, sys, tempfile, time, types
from unittest.mock import patch

parser = argparse.ArgumentParser()
parser.add_argument('--repo', type=pathlib.Path, required=True)
parser.add_argument('--candidate', type=pathlib.Path, required=True)
parser.add_argument('--producer-commit', required=True)
args = parser.parse_args()
repo = args.repo.resolve()
source_path = 'tools/content-migration/regenerate_content.py'
test_path = 'tools/content-migration/test_regenerate_content.py'
sys.path.insert(0, str(repo / 'tools/content-migration'))
producer = subprocess.check_output(['git', 'show', f'{args.producer_commit}:{source_path}'], cwd=repo)
candidate = args.candidate.read_bytes()
rc = types.ModuleType('regenerate_content')
rc.__file__ = str(repo / source_path)
sys.modules['regenerate_content'] = rc
exec(compile(producer, rc.__file__, 'exec'), rc.__dict__)
test_module = types.ModuleType('test_regenerate_content_probe')
test_module.__file__ = str(repo / test_path)
sys.modules[test_module.__name__] = test_module
exec(compile(candidate, test_module.__file__, 'exec'), test_module.__dict__)
executed = []
def profile(frame, event, arg):
    if event == 'call' and frame.f_code.co_filename == test_module.__file__ and frame.f_code.co_name.startswith('test_'):
        executed.append(frame.f_code.co_name)
output, errors = io.StringIO(), io.StringIO()
started = time.monotonic()
with tempfile.TemporaryDirectory(prefix='content-regenerate-outer-') as temporary:
    root = pathlib.Path(temporary)
    (root / 'content/world').mkdir(parents=True)
    with patch.object(rc, 'ROOT', root), patch.object(rc, 'WORLD', root / 'content/world'), patch.object(rc, 'PIN_TEST', root / rc.PIN_PATH), contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
        sys.setprofile(profile)
        try:
            test_module.main()
        finally:
            sys.setprofile(None)
print(json.dumps({
    'repo': str(repo), 'producer_commit': args.producer_commit,
    'producer_sha256': hashlib.sha256(producer).hexdigest(),
    'candidate_sha256': hashlib.sha256(candidate).hexdigest(),
    'status': 'PASS', 'elapsed_seconds': round(time.monotonic() - started, 3),
    'manual_executed_ids': executed, 'manual_execution_count': len(executed),
    'stdout_tail': output.getvalue()[-2200:], 'stderr_tail': errors.getvalue()[-3200:],
    'isolation': 'In-memory producer and candidate with actual test module main. Outer ROOT/WORLD/PIN_TEST temporary; tests retain their own temporary repositories/mocks. Imported Item bootstrap tests use their existing isolated fixtures.'
}, indent=2))
