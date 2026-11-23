#!/usr/bin/env python3
"""Retain actual discovery/execution and body identity for one duplicate removal."""
import ast
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import sys
import unittest

root = Path(sys.argv[1]).resolve()
path = root / "tools/agents/tests/test_enqueue_readiness.py"
source = path.read_text()
spec = importlib.util.spec_from_file_location("test_enqueue_readiness", path)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
suite = unittest.defaultTestLoader.loadTestsFromModule(module)


def cases(suite):
    for item in suite:
        if isinstance(item, unittest.TestSuite):
            yield from cases(item)
        else:
            yield item.id()


ids = sorted(cases(suite))
log = io.StringIO()
result = unittest.TextTestRunner(stream=log).run(suite)
scopes = []
for cls in ast.parse(source).body:
    if isinstance(cls, ast.ClassDef) and cls.name in {
        "EnqueueReadinessTest", "FailClosedTimestampAndCountTest"
    }:
        methods = [n for n in cls.body if isinstance(n, ast.FunctionDef)]
        for method in methods:
            if method.name in {"test_unresolved_threads_block", "test_unresolved_positive_integer_blocks"}:
                body = ast.dump(ast.Module(body=method.body, type_ignores=[]), include_attributes=False)
                scopes.append({"class": cls.name, "case": method.name,
                               "body_sha256": hashlib.sha256(body.encode()).hexdigest(),
                               "fixture_hooks": [m.name for m in methods if m.name.startswith(("setUp", "tearDown"))],
                               "bases": [ast.unparse(base) for base in cls.bases]})
print(json.dumps({"path": str(path.relative_to(root)),
                  "source_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                  "ids": ids, "tests_run": result.testsRun, "passed": result.wasSuccessful(),
                  "failures": len(result.failures), "errors": len(result.errors),
                  "skipped": len(result.skipped), "duplicate_scopes": scopes,
                  "log": log.getvalue()}, indent=2))
