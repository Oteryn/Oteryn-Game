"""Execute the hosted metadata check against live-PR response fixtures."""

import copy
import io
import json
import os
from pathlib import Path
import tempfile
import textwrap
import unittest
from test_main_job_applicability import MainJobApplicabilityTests  # noqa: F401
from contextlib import redirect_stderr, redirect_stdout
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/agent-governance.yml"
SHA = "1" * 40
REPOSITORY = "Oteryn/Oteryn-Game"


class GovernanceTargets(unittest.TestCase):
    def run_metadata(self, event_base="main", live_base=None, dispatch=False, **changes):
        workflow = WORKFLOW.read_text()
        script = textwrap.dedent(workflow.split("python - <<'PY'\n", 1)[1].split("\n          PY", 1)[0])
        pull = {
            "state": "open", "title": "fix(ci): validate prepared branches",
            "body": "## Summary\n## Scope\n## Validation",
            "head": {"sha": SHA, "repo": {"full_name": REPOSITORY}},
            "base": {"ref": event_base if live_base is None else live_base},
        }
        pull.update(changes)
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "env"
            environment = {
                "EVENT_NAME": "workflow_dispatch" if dispatch else "pull_request",
                "REPOSITORY": REPOSITORY, "GH_TOKEN": "synthetic-token",
                "EVENT_SHA": SHA, "EVENT_PR_NUMBER": "123",
                "EVENT_PR_HEAD_SHA": SHA, "EVENT_PR_BASE_REF": event_base,
                "DISPATCH_PR_NUMBER": "123", "DISPATCH_EXPECTED_HEAD_SHA": SHA,
                "GITHUB_ENV": str(target),
            }
            stdout, stderr = io.StringIO(), io.StringIO()
            response = io.StringIO(json.dumps(pull))
            with patch.dict(os.environ, environment, clear=True), \
                    patch("urllib.request.urlopen", return_value=response), \
                    redirect_stdout(stdout), redirect_stderr(stderr):
                try:
                    exec(compile(script, str(WORKFLOW), "exec"), {})
                    code = 0
                except SystemExit as error:
                    code = error.code
            return code, stdout.getvalue(), stderr.getvalue(), target.read_text() if target.exists() else ""

    def test_main_and_dispatch_keep_exact_head_qualification(self):
        for dispatch in (False, True):
            with self.subTest(dispatch=dispatch):
                code, _, _, target = self.run_metadata(dispatch=dispatch)
                self.assertEqual(code, 0)
                self.assertEqual(target, f"TARGET_SHA={SHA}\n")

    def test_stack_runs_are_explicit_preflight(self):
        code, stdout, _, target = self.run_metadata("codex/parent")
        self.assertEqual(code, 0)
        self.assertIn("preflight", stdout)
        self.assertIn("not integration qualification", stdout)
        self.assertEqual(target, f"TARGET_SHA={SHA}\n")
        workflow = WORKFLOW.read_text()
        self.assertIn("github.event.pull_request.base.ref != 'main'", workflow)
        self.assertIn("'Agent governance / stack preflight'", workflow)
        self.assertIn("'Agent governance / validate'", workflow)

    def test_dispatch_cannot_qualify_a_stack(self):
        code, _, stderr, target = self.run_metadata("codex/parent", dispatch=True)
        self.assertEqual(code, 1)
        self.assertIn("only accepts pull requests targeting main", stderr)
        self.assertEqual(target, "")

    def test_retargeted_event_fails_before_checkout(self):
        for event_base, live_base in [("main", "codex/parent"), ("codex/parent", "main"),
                                      ("codex/parent", "codex/other"), ("", "main")]:
            with self.subTest(event_base=event_base, live_base=live_base):
                code, _, stderr, target = self.run_metadata(event_base, live_base)
                self.assertEqual(code, 1)
                self.assertIn("base", stderr)
                self.assertEqual(target, "")

    def test_invalid_head_repository_and_state_fail_for_both_modes(self):
        bad_heads = [{"sha": "2" * 40, "repo": {"full_name": REPOSITORY}},
                     {"sha": SHA, "repo": {"full_name": "outside/fork"}},
                     {"sha": "not-a-sha", "repo": {"full_name": REPOSITORY}}]
        for base in ("main", "codex/parent"):
            for changes in [*({"head": copy.deepcopy(h)} for h in bad_heads), {"state": "closed"}]:
                with self.subTest(base=base, changes=changes):
                    code, _, _, target = self.run_metadata(base, **changes)
                    self.assertEqual(code, 1)
                    self.assertEqual(target, "")


if __name__ == "__main__":
    unittest.main()
