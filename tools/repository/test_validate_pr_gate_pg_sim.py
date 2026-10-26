#!/usr/bin/env python3
"""Prove that removing PR PG/SIM duplication preserves MQ qualification."""
from __future__ import annotations

import importlib.util
import os
from pathlib import Path
import re
import subprocess
import tempfile
import textwrap
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

    def rust_fast_step(self, name):
        job = self.pr.split("  rust_fast:\n", 1)[1].split("\n  rust_supply_chain:\n", 1)[0]
        return job.split(f"      - name: {name}\n", 1)[1].split("\n      - name: ", 1)[0]

    def test_rust_fast_runner_toggle_and_hosted_cache(self):
        job = self.pr.split("  rust_fast:\n", 1)[1].split("\n  rust_supply_chain:\n", 1)[0]
        # Pin the complete expression: unset/off defaults to hosted, only the
        # Clippy leg uses the PC, and only same-repository, non-dependabot PRs
        # may execute on the owner's PC.
        expected = (
            "${{ (vars.OTERYN_SELF_HOSTED_RUST == 'on' && matrix.kind == 'clippy' && "
            "github.event.pull_request.head.repo.full_name == github.repository && "
            "github.actor != 'dependabot[bot]') && "
            "fromJSON('[\"self-hosted\",\"linux\",\"x64\",\"oteryn-molehill\"]') "
            "|| 'ubuntu-24.04' }}"
        )
        self.assertEqual(re.findall(r"(?m)^    runs-on: (.+)$", job), [expected])
        for name in (
            "Restore trimmed Cargo cache from protected main",
            "Normalize source mtimes for the restored Cargo cache",
        ):
            with self.subTest(step=name):
                self.assertIn(
                    "        if: runner.environment == 'github-hosted' && steps.jr_hit.outputs.hit != 'true'\n",
                    self.rust_fast_step(name),
                )

    def test_self_hosted_target_setup_and_stale_junit_cleanup(self):
        name = "Select persistent self-hosted Cargo target directory"
        step = self.rust_fast_step(name)
        self.assertIn("        if: runner.environment == 'self-hosted'\n", step)
        self.assertNotIn("matrix.kind", step)
        self.assertIn(
            "--component clippy,rustfmt\n\n      - name: " + name + "\n",
            self.pr.split("  rust_fast:\n", 1)[1],
        )
        script = textwrap.dedent(step.split("        run: |\n", 1)[1])
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            github_env = root / "github-env"
            cases = [
                (target_root, "molehill-game-1")
                for target_root in (None, "", "relative/path", str(root / "warm target"))
            ] + [
                (str(root / "warm target"), runner_name)
                for runner_name in (None, "", "../escape", "a/b", "molehill-game-2")
            ]
            for target_root, runner_name in cases:
                    with self.subTest(target_root=target_root, runner_name=runner_name):
                        env = {"PATH": os.defpath, "GITHUB_ENV": str(github_env)}
                        if target_root is not None:
                            env["OTERYN_CI_TARGET_ROOT"] = target_root
                        if runner_name is not None:
                            env["RUNNER_NAME"] = runner_name
                        root_ok = target_root is not None and target_root.startswith("/")
                        name_ok = runner_name is not None and re.fullmatch(r"[A-Za-z0-9._-]+", runner_name) is not None
                        valid = root_ok and name_ok
                        if valid:
                            target = Path(target_root) / runner_name / "merge-gate"
                            junit = target / "nextest/rust-fast/junit.xml"
                            junit.parent.mkdir(parents=True, exist_ok=True)
                            junit.write_text("stale report")
                            sentinel = target / "keep-artifacts"
                            sentinel.write_text("warm")
                        result = subprocess.run(["bash", "-c", script], env=env, capture_output=True, text=True)
                        if valid:
                            self.assertEqual(result.returncode, 0, result.stderr)
                            self.assertEqual(github_env.read_text(), f"CARGO_TARGET_DIR={target}\n")
                            self.assertFalse(junit.exists())
                            self.assertEqual(sentinel.read_text(), "warm")
                            github_env.unlink()
                        else:
                            self.assertNotEqual(result.returncode, 0)
                            expected_error = (
                                "OTERYN_CI_TARGET_ROOT must be set to an absolute path"
                                if not root_ok
                                else "RUNNER_NAME must be a plain runner name"
                            )
                            self.assertIn(expected_error, result.stderr)
                            self.assertFalse(github_env.exists())
        self.assertIn(
            'junit="${CARGO_TARGET_DIR:-target}/nextest/rust-fast/junit.xml"',
            self.rust_fast_step("Record passed crates in the job-result ledger"),
        )

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
        for command in ("cargo +1.99.1 test --locked --workspace", "github.event.merge_group.head_sha"):
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
