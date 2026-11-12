import hashlib
import unittest
from pathlib import Path

from refresh_workflow_pin import workflow_digest


class WorkflowPinTests(unittest.TestCase):
    def test_only_the_approved_workflow_addition_can_refresh_the_pin(self):
        root = Path(__file__).resolve().parents[2]
        workflow = (root / ".github/workflows/rust.yml").read_text()
        self.assertEqual(workflow_digest(workflow), hashlib.sha256(workflow.encode()).hexdigest())
        for old, new in (
            ("contents: read", "contents: write"),
            ("workflow_dispatch:", "workflow_dispatch:\n  pull_request:"),
            ("name: game-platform-catalog", "name: unexpected-artifact"),
        ):
            with self.subTest(mutation=old):
                self.assertIn(old, workflow)
                with self.assertRaises(ValueError):
                    workflow_digest(workflow.replace(old, new, 1))
