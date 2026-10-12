#!/usr/bin/env python3
"""Prove that removing PR PG/SIM duplication preserves MQ qualification."""
from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
VALIDATOR = Path(__file__).with_name("validate_pr_gate_pg_sim.py")


class QualificationPlacementTests(unittest.TestCase):
    def setUp(self):
        spec = importlib.util.spec_from_file_location("placement", VALIDATOR)
        self.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.module)
        self.pr = (ROOT / ".github/workflows/merge-gate.yml").read_text()
        self.mq = (ROOT / ".github/workflows/merge-group-gate.yml").read_text()

    def validate(self, pr=None, mq=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.module.MERGE_GATE = root / "pr.yml"
            self.module.MERGE_GROUP_GATE = root / "mq.yml"
            self.module.MERGE_GATE.write_text(self.pr if pr is None else pr)
            self.module.MERGE_GROUP_GATE.write_text(self.mq if mq is None else mq)
            return self.module.validate()

    def test_current_contract(self):
        self.assertEqual(self.validate(), [])

    def test_no_workflows(self):
        self.module.MERGE_GATE = Path("/nonexistent/oteryn-pr.yml")
        self.assertTrue(self.module.validate())

    def test_heavy_jobs_cannot_return_to_pr_even_when_skipped(self):
        for job in ("rust_linux", "rust_windows", "codeql", "atlas_fullworld", "node_boot", "server_seam"):
            with self.subTest(job=job):
                self.assertTrue(self.validate(pr=self.pr + f"  {job}:\n    if: false\n"))

    def test_unknown_job_is_not_a_heavy_job_escape(self):
        self.assertTrue(self.validate(pr=self.pr + "  extra_workspace_tests:\n    runs-on: ubuntu-24.04\n"))

    def test_labels_cannot_restore_heavy_pr_selection(self):
        self.assertTrue(self.validate(pr=self.pr + "# full-ci\n"))
        self.assertTrue(self.validate(pr=self.pr + "# full_ci\n"))

    def test_mq_postgres_and_windows_cannot_be_deselected(self):
        for job in ("durability_postgres", "rust_windows"):
            marker = f"  {job}:\n"
            self.assertIn(marker, self.mq)
            with self.subTest(job=job):
                self.assertTrue(self.validate(mq=self.mq.replace(marker, marker + "    if: false\n", 1)))

    def test_mq_cannot_drop_full_tests_or_exact_candidate_checkout(self):
        for command in ("cargo +1.95.0 test --locked --workspace", "github.event.merge_group.head_sha"):
            self.assertIn(command, self.mq)
            with self.subTest(command=command):
                self.assertTrue(self.validate(mq=self.mq.replace(command, "removed", 1)))


def main() -> int:
    result = unittest.TextTestRunner(verbosity=1).run(unittest.defaultTestLoader.loadTestsFromTestCase(QualificationPlacementTests))
    if not result.wasSuccessful():
        return 1
    # Keep real registered-target, Windows native failure, risk and content
    # routing regressions on their owning existing MQ/routing entry points.
    for filename in ("test_validate_merge_group_pg_sim.py", "test_classify_pr_test_lanes.py", "test_classify_content_routing.py"):
        spec = importlib.util.spec_from_file_location("qualification_regressions", VALIDATOR.with_name(filename))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        if module.main() != 0:
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
