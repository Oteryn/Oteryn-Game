#!/usr/bin/env python3
"""Exercise real workflow regression tests against benign and failing changes.

Run with a checkout and optionally a historical source ref. The workflow remains
the current checkout's input; only the regression test implementation is swapped.
"""

import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


root = Path(sys.argv[1]).resolve()
ref = sys.argv[2] if len(sys.argv) > 2 else None
sys.path.insert(0, str(root / "tools/repository"))


def load(name):
    path = root / "tools/repository" / name
    source = (subprocess.check_output(["git", "show", f"{ref}:tools/repository/{name}"],
                                     cwd=root, text=True) if ref else path.read_text())
    spec = importlib.util.spec_from_loader(name.removesuffix(".py"), loader=None)
    module = importlib.util.module_from_spec(spec)
    module.__file__ = str(path)
    exec(compile(source, str(path), "exec"), module.__dict__)
    return module


def run(case):
    log = io.StringIO()
    result = unittest.TextTestRunner(stream=log).run(unittest.TestSuite([case]))
    return {"run": result.testsRun, "pass": result.wasSuccessful(),
            "failures": len(result.failures), "errors": len(result.errors),
            "log": log.getvalue()}


results = {"test_implementation_ref": ref or "working_tree"}
with tempfile.TemporaryDirectory() as directory:
    fixture = Path(directory)
    workflows = fixture / ".github/workflows"
    workflows.mkdir(parents=True)
    repository = fixture / "tools/repository"
    repository.mkdir(parents=True)
    for path in (root / "tools/repository").glob("test_*.py"):
        (repository / path.name).touch()

    merge = (root / ".github/workflows/merge-gate.yml").read_text()
    rust = (root / ".github/workflows/rust.yml").read_text()
    (workflows / "merge-gate.yml").write_text(merge)
    cache = load("test_protected_main_cache.py")
    cache.WORKFLOW = workflows / "rust.yml"
    variants = {
        "unchanged": rust,
        "build_display_label_only": rust.replace("- name: Build workspace\n", "- name: Compile workspace\n"),
        "mold_display_label_only": rust.replace("- name: Install pinned mold linker\n", "- name: Install verified linker\n"),
        "missing_build_linker": rust.replace("          LD_PRELOAD: ${{ env.MOLD_PRELOAD }}\n", "", 1),
    }
    results["cache"] = {}
    for name, source in variants.items():
        assert name == "unchanged" or source != rust, name
        cache.WORKFLOW.write_text(source)
        results["cache"][name] = run(cache.ProtectedMainCache(
            "test_linker_matches_the_archive_producer_without_adding_cargo_flags"))

    loop = load("test_repository_tests_run.py")
    loop.ROOT = fixture
    anchor = "          for test in tools/repository/test_*.py; do\n"
    variants = {
        "unchanged": merge,
        "skip_every_repository_test": merge.replace(anchor, anchor + "            continue\n", 1),
    }
    results["dispatch"] = {}
    for name, source in variants.items():
        assert name == "unchanged" or source != merge, name
        (workflows / "merge-gate.yml").write_text(source)
        results["dispatch"][name] = run(loop.RepositoryTestsRunTests(
            "test_governance_runs_every_repository_test"))

print(json.dumps(results, indent=2))
