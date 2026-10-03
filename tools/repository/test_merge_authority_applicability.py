#!/usr/bin/env python3
"""The protected main audit must not reject prepared native stack events."""
import json
from pathlib import Path
import re
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]


class MergeAuthorityApplicabilityTests(unittest.TestCase):
    def test_native_stack_event_keeps_main_audit_inapplicable(self):
        text = (ROOT / ".github/workflows/merge-authority-audit.yml").read_text()
        match = re.search(r"^    if: (.+)$", text, re.MULTILINE)
        self.assertIsNotNone(match)
        expression = match[1]
        self.assertEqual(expression, "github.event.pull_request.base.ref == 'main' || !github.event.pull_request.base.ref")
        script = (
            "const github={event:{pull_request:{base:{ref:JSON.parse(process.argv[1])}}}};"
            "process.stdout.write(JSON.stringify(Boolean(" + expression + ")));"
        )
        for base, expected in (("main", True), ("codex/prepared-parent", False), (None, True), ("", True)):
            with self.subTest(base=base):
                actual = json.loads(subprocess.check_output(["node", "-e", script, json.dumps(base)]))
                self.assertEqual(actual, expected)


if __name__ == "__main__":
    unittest.main()
