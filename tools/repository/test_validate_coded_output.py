#!/usr/bin/env python3
"""Coded failure output of the two governance validators (ERR-TOOLS-3)."""
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
REGISTRY = ROOT / "docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json"
VALIDATORS = (
    ("tools/agents/validate_governance.py", "E8001", "GOVERNANCE_CHECK_FAILED"),
    ("tools/repository/validate_repository_policy.py", "E8002", "REPOSITORY_POLICY_CHECK_FAILED"),
)


def load(relative: str):
    path = ROOT / relative
    spec = importlib.util.spec_from_file_location(path.stem, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class CodedOutputTests(unittest.TestCase):
    def test_codes_match_registry(self) -> None:
        names = {entry["code"]: entry["name"] for entry in json.loads(REGISTRY.read_text())["codes"]}
        for _, code, name in VALIDATORS:
            self.assertEqual(names[int(code[1:])], name)

    def test_failure_line_and_annotation(self) -> None:
        for relative, code, name in VALIDATORS:
            module = load(relative)
            for actions in ("false", "true"):
                stream = io.StringIO()
                with patch.dict(os.environ, {"GITHUB_ACTIONS": actions}):
                    module.report_failure(code, name, ["bad 100%\nthing"], stream)
                lines = stream.getvalue().splitlines()
                self.assertEqual(lines[0], f"{code} {name}: bad 100%")
                if actions == "true":
                    self.assertIn(f"::error title={code} {name}::bad 100%25%0Athing", lines)
                else:
                    self.assertFalse(any(line.startswith("::error") for line in lines))

    def test_core_failures_are_recoded(self) -> None:
        module = load("tools/repository/validate_repository_policy.py")

        def core_failure() -> int:
            print("Repository policy validation failed:", file=sys.stderr)
            print("- missing thing", file=sys.stderr)
            return 1

        err = io.StringIO()
        with patch.dict(os.environ, {"GITHUB_ACTIONS": "true"}), contextlib.redirect_stderr(err):
            self.assertEqual(module.run_core(core_failure), 1)
        lines = err.getvalue().splitlines()
        self.assertIn("Repository policy validation failed:", lines)
        self.assertIn("E8002 REPOSITORY_POLICY_CHECK_FAILED: missing thing", lines)
        self.assertIn("::error title=E8002 REPOSITORY_POLICY_CHECK_FAILED::missing thing", lines)
        self.assertNotIn("- missing thing", lines)

    def test_core_success_passes_through(self) -> None:
        module = load("tools/repository/validate_repository_policy.py")
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            self.assertEqual(module.run_core(lambda: print("passed") or 0), 0)
        self.assertEqual(out.getvalue(), "passed\n")


if __name__ == "__main__":
    unittest.main()
