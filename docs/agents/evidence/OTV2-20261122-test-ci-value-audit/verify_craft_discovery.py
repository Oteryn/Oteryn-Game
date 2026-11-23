"""Enumerate normal import discovery and the actual direct entry point without running cases."""
from pathlib import Path
import argparse
import subprocess
import tempfile
import ast
import importlib.util
import json
import runpy
import sys
import unittest
from unittest.mock import patch

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--repo', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--baseline', default='8e9f0e16631aab1beb8416f8d5ca707aa3031083')
args = parser.parse_args()
root = args.repo.resolve()
temporary = tempfile.TemporaryDirectory()
before = Path(temporary.name) / 'test_tainted_souls.py'
before.write_text(subprocess.check_output(['git', 'show', args.baseline + ':tools/content-schema/craft-authoring/test_tainted_souls.py'], cwd=root, text=True))
sys.path.insert(0, str(root / 'tools/content-schema/craft-authoring'))
files = {
    'before': before,
    'after': root / 'tools/content-schema/craft-authoring/test_tainted_souls.py',
}

def ids(suite):
    for test in suite:
        if isinstance(test, unittest.TestSuite):
            yield from ids(test)
        else:
            yield test.id().split('.', 1)[1]

class EnumerationComplete(Exception):
    pass

results = {}
for tag, path in files.items():
    spec = importlib.util.spec_from_file_location('audit_' + tag, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    discovered = list(ids(unittest.defaultTestLoader.loadTestsFromModule(module)))
    direct = []
    def record(program):
        direct.extend(ids(program.test))
        raise EnumerationComplete
    with patch.object(unittest.TestProgram, 'runTests', record), patch.object(sys, 'argv', [str(path)]):
        try:
            runpy.run_path(str(path), run_name='__main__')
        except EnumerationComplete:
            pass
    tree = ast.parse(path.read_text())
    bodies = [ast.dump(ast.Module(body=node.body, type_ignores=[]), include_attributes=False)
              for node in ast.walk(tree) if isinstance(node, ast.FunctionDef) and node.name.startswith('test_')]
    results[tag] = {'discover_count': len(discovered), 'direct_count': len(direct),
                    'unique_bodies': len(set(bodies)), 'discover': discovered, 'direct': direct,
                    '_bodies': set(bodies)}

assert results['before']['discover_count'] == 10
assert results['before']['direct_count'] == 5
assert results['after']['discover_count'] == 7
assert results['before']['_bodies'] == results['after']['_bodies']
assert set(results['after']['discover']) == set(results['after']['direct'])
for result in results.values():
    result.pop('_bodies')
args.output.write_text(json.dumps(results, indent=2) + '\n')
print(json.dumps({tag: {key: value for key, value in result.items() if not isinstance(value, list)}
                  for tag, result in results.items()}, indent=2))
print('PASS: unique test bodies preserved, after direct/discover IDs identical. Enumeration only; no test bodies executed.')
