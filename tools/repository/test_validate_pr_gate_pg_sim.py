#!/usr/bin/env python3
"""Prove that removing PR PG/SIM duplication preserves MQ qualification."""
from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import textwrap
import time
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

    def archive_step(self, name):
        build = self.pr.split("  rust_fast_build:\n", 1)[1].split("\n  rust_fast:\n", 1)[0]
        return build.split(f"      - name: {name}\n", 1)[1].split("\n      - name: ", 1)[0]

    @staticmethod
    def route_script(text):
        block = text.split("      - name: Resolve Molehill routing\n", 1)[1].split("        run: |\n", 1)[1]
        lines = []
        for line in block.splitlines(keepends=True):
            if line.strip() and not line.startswith("          "):
                break
            lines.append(line)
        return textwrap.dedent("".join(lines)).strip() + "\n"

    def test_molehill_routing_follows_heartbeat(self):
        # on/split always route to Molehill; auto/auto-split only while the PC heartbeat
        # (OTERYN_MOLEHILL_SEEN, Unix seconds) is at most ten minutes old; anything else is hosted.
        harness = (ROOT / ".github/workflows/pr-postgres-harness.yml").read_text()
        script = self.route_script(self.pr)
        self.assertEqual(script, self.route_script(harness))
        now = int(time.time())
        fresh, stale, future = str(now - 60), str(now - 700), str(now + 3600)
        cases = (
            ("on", "", "on"), ("on", stale, "on"), ("split", "", "split"),
            ("auto", fresh, "on"), ("auto", stale, "off"), ("auto", "", "off"),
            ("auto", "junk", "off"), ("auto", future, "off"),
            ("auto-split", fresh, "split"), ("auto-split", stale, "off"),
            ("off", fresh, "off"), ("", fresh, "off"),
        )
        for mode, seen, expected in cases:
            with self.subTest(mode=mode, seen=seen), tempfile.TemporaryDirectory() as directory:
                output = Path(directory) / "output"
                env = dict(os.environ, MODE=mode, SEEN=seen, MAX_AGE="600", GITHUB_OUTPUT=str(output))
                subprocess.run(["bash", "-c", script], env=env, check=True, capture_output=True)
                self.assertEqual(output.read_text(), f"mode={expected}\n")

    def test_rust_fast_runner_toggle_and_hosted_cache(self):
        job = self.pr.split("  rust_fast:\n", 1)[1].split("\n  rust_supply_chain:\n", 1)[0]
        # Pin the complete expression: unset/off defaults to hosted, only the
        # Clippy leg uses the PC, and only same-repository, non-dependabot PRs
        # may execute on the owner's PC.
        expected = (
            "${{ (needs.runner_route.outputs.rust_runner_mode == 'on' && matrix.kind == 'clippy' && "
            "github.event.pull_request.head.repo.full_name == github.repository && "
            "github.actor != 'dependabot[bot]') && "
            "fromJSON('[\"self-hosted\",\"linux\",\"x64\",\"oteryn-molehill\"]') "
            "|| 'ubuntu-24.04' }}"
        )
        self.assertEqual(re.findall(r"(?m)^    runs-on: (.+)$", job), [expected])
        build = self.pr.split("  rust_fast_build:\n", 1)[1].split("\n  rust_fast:\n", 1)[0]
        self.assertIn("    runs-on: ubuntu-24.04\n", build)
        self.assertIn("needs.runner_route.outputs.rust_test_build_required == 'true'", build)
        for name in (
            "Restore trimmed Cargo cache from protected main",
            "Normalize source mtimes for the restored Cargo cache",
        ):
            with self.subTest(step=name):
                self.assertIn(f"      - name: {name}\n", build)
                self.assertIn(
                    "        if: matrix.kind == 'clippy' && runner.environment == 'github-hosted' "
                    "&& steps.jr_hit.outputs.hit != 'true'\n",
                    self.rust_fast_step(name),
                )
        self.assertIn("    needs: [scope, lanes, runner_route, rust_fast_build]\n", job)
        self.assertIn('nextest archive --locked "${package_args[@]}" --lib --bins', build)
        self.assertIn('nextest run --archive-file "$artifact/unit-tests.tar.zst"', job)
        self.assertIn('--workspace-remap . --extract-to .', job)
        self.assertIn('--partition "hash:${SHARD}" --no-tests=pass --no-fail-fast', job)
        self.assertIn("          retention-days: 30\n", self.archive_step("Upload unit-test archive"))
        for name in ("Add swap for the library test build", "Install pinned mold linker"):
            with self.subTest(step=name):
                self.assertIn(f"      - name: {name}\n", build)
                self.assertNotIn(f"      - name: {name}\n", job)

    def test_unit_test_archive_uses_full_cargo_selection_on_partial_ledger_hit(self):
        step = self.archive_step("Archive changed-crate unit tests")
        source = textwrap.dedent(
            step.split("          python - <<'PY' > \"$package_args_file\"\n", 1)[1]
            .split("          PY\n", 1)[0]
        )
        for mode, expected in (
            ("workspace", ["--workspace"]),
            ("packages", ["-p", "first", "-p", "second"]),
        ):
            with self.subTest(mode=mode):
                env = dict(os.environ, TEST_PACKAGES='["first"]',
                           RUST_SELECTION=f'{{"mode": "{mode}", "packages": ["first", "second"]}}')
                result = subprocess.run(
                    [sys.executable, "-c", source], env=env, check=True, capture_output=True, text=True
                )
                self.assertEqual(result.stdout.splitlines(), expected)

    def test_unit_test_ledger_handles_mixed_runner_images(self):
        archive_step = self.archive_step("Report archive runner identity")
        plan_step = self.rust_fast_step("Plan job-result keys")
        select_step = self.rust_fast_step("Select crates without a passed result")
        record_step = self.rust_fast_step("Record passed crates in the job-result ledger")
        self.assertIn('record --plan "$RUNNER_TEMP/jr-plan.json"', record_step)
        self.assertIn("BYPASS_LEDGER: ${{ steps.jr_plan.outputs.bypass }}", select_step)
        self.assertIn("PLAN_IMAGE: ${{ needs.runner_route.outputs.rust_test_image }}", plan_step)
        archive = textwrap.dedent(archive_step.split("        run: |\n", 1)[1])
        plan = textwrap.dedent(plan_step.split("        run: |\n", 1)[1])
        select = textwrap.dedent(select_step.split("        run: |\n", 1)[1].split("\n\n      #", 1)[0])
        # Print planner arguments to verify the recording identity without writing cache files.
        plan = "python() { printf '%s\\n' \"$@\"; }\n" + plan
        planned_image = "20261111.370.1"
        new_image = "20261116.376.1"
        for image, arch, archive_result in (
            (planned_image, "X64", "success"),
            (new_image, "X64", "success"),
            (planned_image, "ARM64", "success"),
            (new_image, "X64", "skipped"),
        ):
            with self.subTest(image=image, arch=arch, archive=archive_result), tempfile.TemporaryDirectory() as directory:
                temp = Path(directory)
                output = temp / "output"
                full = "cached\t--lib\npending\t--bins\n"
                (temp / "rust-fast-packages.tsv").write_text(full + "no-unit-target\t\n")
                env = dict(
                    os.environ, KIND="tests", JOB="rust_fast-tests 1/5", BASE_SHA="base",
                    PER_PACKAGE="--per-package", RUNNER_TEMP=directory, GITHUB_OUTPUT=str(output),
                    PLAN_IMAGE=planned_image, PLAN_PLATFORM="Linux-X64", ARCHIVE_RESULT=archive_result,
                    ARCHIVE_IMAGE=image, ARCHIVE_PLATFORM=f"Linux-{arch}",
                    ImageVersion=image, RUNNER_OS="Linux", RUNNER_ARCH=arch,
                    SLUG="tests-1-5", TEST_SHARDS='{"tests-1-5": ["pending"]}',
                )
                if archive_result == "success":
                    subprocess.run(["bash", "-c", archive], env=env, check=True)
                    self.assertEqual(output.read_text(), f"image={image}\nplatform=Linux-{arch}\n")
                    output.unlink()

                result = subprocess.run(["bash", "-c", plan], env=env, capture_output=True, text=True)
                if archive_result == "skipped":
                    self.assertEqual(result.returncode, 1)
                    self.assertIn(
                        f"::error::complete ledger hit planned on image {planned_image}/Linux-X64 "
                        f"but shard runs on {image}/Linux-{arch}; re-run all jobs", result.stdout,
                    )
                    self.assertNotIn("job_result_key.py", result.stdout)
                    self.assertFalse(output.exists())
                    continue

                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn(f"image={image}\n", result.stdout)
                self.assertIn(f"os=Linux-{arch}\n", result.stdout)
                mismatch = (image, arch) != (planned_image, "X64")
                if mismatch:
                    self.assertEqual(result.stdout.count("::notice::"), 1)
                    self.assertEqual(output.read_text(), "bypass=true\n")
                else:
                    self.assertNotIn("::notice::", result.stdout)
                    self.assertFalse(output.exists())
                env["BYPASS_LEDGER"] = "true" if output.exists() else ""
                subprocess.run(["bash", "-c", select], env=env, check=True)
                self.assertTrue(output.read_text().endswith("hit=false\n"))
                self.assertEqual((temp / "rust-fast-run.tsv").read_text(), full if mismatch else "pending\t--bins\n")

    def test_unit_test_ledger_requires_matching_archive_builder(self):
        route = self.pr.split("      - name: Plan unit-test job-result keys\n", 1)[1]
        route = textwrap.dedent(route.split("        run: |\n", 1)[1].split("\n      - name: ", 1)[0])
        plan = self.rust_fast_step("Plan job-result keys").split("        run: |\n", 1)[1]
        record = self.rust_fast_step("Record passed crates in the job-result ledger").split("        run: |\n", 1)[1]
        name = "oteryn-simulation-determinism"
        image = "20261111.370.1"
        platform = "Linux-X64"
        rows = f"{name}\t--lib\n"
        for builder_image, builder_platform in (
            (image, platform), ("20261116.376.1", platform), (image, "Linux-ARM64"),
        ):
            with self.subTest(builder_image=builder_image, builder_platform=builder_platform), \
                 tempfile.TemporaryDirectory() as directory:
                temp = Path(directory)
                packages = temp / "rust-fast-packages.tsv"
                packages.write_text(rows)
                (temp / "rust-fast-test-packages.tsv").write_text(rows)
                (temp / "rust-fast-run.tsv").write_text(rows)
                (temp / "rust-fast-metadata.json").write_text(json.dumps({
                    "workspace_root": str(ROOT), "workspace_members": [name],
                    "packages": [{
                        "id": name, "name": name,
                        "manifest_path": str(ROOT / "crates/simulation-determinism/Cargo.toml"),
                    }],
                }))
                junit = temp / "nextest/rust-fast/junit.xml"
                junit.parent.mkdir(parents=True)
                junit.write_text(f'<testsuite name="{name}" tests="1"><testcase name="ok"/></testsuite>')
                output = temp / "output"
                env = dict(
                    os.environ, KIND="tests", JOB="rust_fast-tests 1/5", BASE_SHA="base",
                    PER_PACKAGE="--per-package", RUNNER_TEMP=directory, GITHUB_OUTPUT=str(output),
                    GITHUB_STEP_SUMMARY=str(temp / "summary"), CARGO_TARGET_DIR=directory,
                    PLAN_IMAGE=image, PLAN_PLATFORM=platform, ARCHIVE_RESULT="success",
                    ARCHIVE_IMAGE=builder_image, ARCHIVE_PLATFORM=builder_platform,
                    ImageVersion=image, RUNNER_OS="Linux", RUNNER_ARCH="X64",
                )
                subprocess.run(["bash", "-c", textwrap.dedent(plan)], cwd=ROOT, env=env,
                               check=True, capture_output=True)
                recorded = subprocess.run(["bash", "-c", textwrap.dedent(record)], cwd=ROOT, env=env,
                                          check=True, capture_output=True, text=True)
                subprocess.run(["bash", "-c", route], cwd=ROOT, env=env, check=True, capture_output=True)
                remaining = temp / "remaining.tsv"
                later_plan = temp / "rust-fast-test-plan-1.json"
                subprocess.run(
                    [sys.executable, "-I", str(ROOT / "tools/repository/job_result_key.py"), "select",
                     "--plan", str(later_plan), "--packages", str(packages),
                     "--dir", str(temp / "jobresult"), "--out", str(remaining)],
                    cwd=ROOT, env=env, check=True, capture_output=True,
                )
                matched = (builder_image, builder_platform) == (image, platform)
                self.assertEqual(remaining.read_text(), "" if matched else rows)
                self.assertEqual((temp / "jobresult/ledger.json").exists(), matched)
                self.assertEqual("recorded=true\n" in output.read_text(), matched)
                self.assertEqual(recorded.stdout.count("::notice::"), int(not matched))
                recorded_key = json.loads((temp / "jr-plan.json").read_text())["units"][name]
                lookup_key = json.loads(later_plan.read_text())["units"][name]
                self.assertTrue(recorded_key and lookup_key)
                self.assertEqual(recorded_key == lookup_key, matched)
        self.assertIn("ARCHIVE_IMAGE: ${{ needs.rust_fast_build.outputs.runner_image }}", self.pr)
        self.assertIn("ARCHIVE_PLATFORM: ${{ needs.rust_fast_build.outputs.runner_platform }}", self.pr)

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
        for command in ("cargo +1.100.0 test --locked --workspace", "github.event.merge_group.head_sha"):
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
