"""One archive can finish independently without certifying another archive."""

import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import textwrap
import unittest


HELPER = Path(__file__).with_name("run_rust_fast_archive.py")
WORKFLOW = HELPER.parents[2] / ".github/workflows/rust-fast-archive.yml"


class ArchiveShard(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.artifact = self.root / "artifact"
        self.artifact.mkdir()
        for name, content in (("runner-image", "image-a\n"), ("runner-platform", "Linux-X64\n"),
                              ("unit-tests.tar.zst", "archive"), ("cargo-nextest", "nextest")):
            (self.artifact / name).write_text(content)
        binaries = self.root / "bin"
        binaries.mkdir()
        (binaries / "git").write_text("#!/bin/sh\nprintf '%s\\n' " + "a" * 40 + "\n")
        (binaries / "cargo").write_text('''#!/usr/bin/env python3
import json, os
from pathlib import Path
import sys
Path(os.environ['CALLS']).write_text(json.dumps(sys.argv[1:]))
Path(os.environ['CALLS'] + '-cwd').write_text(str(Path.cwd()))
if os.environ.get('WRITE_REPORT', '1') == '1':
    report = Path(os.environ['CARGO_TARGET_DIR']) / 'nextest/rust-fast/junit.xml'
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text('<testsuites><testsuite name="current"/></testsuites>')
code = int(os.environ.get('NEXT_EXIT', '0'))
if code < 0:
    os.kill(os.getpid(), -code)
sys.exit(code)
''')
        for binary in binaries.iterdir():
            binary.chmod(0o700)
        self.env = {
            **os.environ, "PATH": str(binaries) + os.pathsep + os.environ["PATH"],
            "EXPECTED_SHA": "a" * 40, "BASE_SHA": "b" * 40,
            "RUST_SELECTION": json.dumps({"mode": "packages", "packages": ["crate-a", "crate-b"]}),
            "TEST_SHARDS": json.dumps({f"tests-{shard}-5": ["crate-a"] for shard in range(1, 6)}),
            "ARCHIVE": "dispatch-2", "SHARD": "2/5", "SLUG": "tests-2-5",
            "PLAN_IMAGE": "image-a", "PLAN_PLATFORM": "Linux-X64",
            "BUILD_IMAGE": "image-a", "BUILD_PLATFORM": "Linux-X64",
            "ImageVersion": "image-a", "RUNNER_OS": "Linux", "RUNNER_ARCH": "X64",
            "GITHUB_RUN_ID": "123", "GITHUB_RUN_ATTEMPT": "2",
            "GITHUB_WORKSPACE": str(self.root),
            "GITHUB_OUTPUT": str(self.root / "outputs"),
            "CARGO_TARGET_DIR": str(self.root / "target"), "CALLS": str(self.root / "calls"),
        }
        self.result = self.root / "result"

    def invoke(self, command):
        args = [sys.executable, "-I", str(HELPER), command, "--out", str(self.result)]
        if command == "run":
            args += ["--artifact", str(self.artifact), "--extract", str(self.root / "extract")]
        return subprocess.run(args, env=self.env, capture_output=True, text=True, timeout=10, check=False)

    def manifest(self):
        return json.loads((self.result / "result.json").read_text())

    def plan(self):
        result = self.invoke("plan")
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_planned_selection_is_reused_only_on_both_planned_runner_identities(self):
        for changed in ({}, {"ImageVersion": "image-b"}, {"BUILD_IMAGE": "image-b"},
                        {"RUNNER_ARCH": "ARM64"}, {"BUILD_PLATFORM": "Linux-ARM64"}):
            with self.subTest(changed=changed):
                original = self.env.copy()
                self.env.update(changed)
                self.plan()
                expected = ["crate-a", "crate-b"] if changed else ["crate-a"]
                self.assertEqual(self.manifest()["pending_packages"], expected)
                self.assertFalse(self.manifest()["completed"])
                self.env = original

    def test_empty_ledger_selection_needs_no_archive_or_nextest(self):
        shards = json.loads(self.env["TEST_SHARDS"])
        shards["tests-2-5"] = []
        self.env["TEST_SHARDS"] = json.dumps(shards)
        self.plan()
        self.assertTrue(self.manifest()["completed"])
        self.assertEqual(self.manifest()["exit_code"], 0)
        self.assertIn("hit=true\n", (self.root / "outputs").read_text())
        self.assertFalse((self.root / "calls").exists())

    def test_control_checkout_helper_still_executes_in_the_exact_candidate_workspace(self):
        control = self.root / "control/tools/repository/run_rust_fast_archive.py"
        control.parent.mkdir(parents=True)
        control.write_bytes(HELPER.read_bytes())
        args = [sys.executable, "-I", str(control)]
        planned = subprocess.run([*args, "plan", "--out", str(self.result)], env=self.env,
                                 capture_output=True, text=True, timeout=10, check=False)
        self.assertEqual(planned.returncode, 0, planned.stderr)
        executed = subprocess.run([*args, "run", "--out", str(self.result), "--artifact", str(self.artifact),
                                   "--extract", str(self.root / "extract")], env=self.env,
                                  capture_output=True, text=True, timeout=10, check=False)
        self.assertEqual(executed.returncode, 0, executed.stderr)
        self.assertEqual(Path(self.env["CALLS"] + "-cwd").read_text(), str(self.root))
        self.assertEqual(self.manifest()["head"], "a" * 40)

    def test_one_archive_runs_same_partition_and_filter_then_keeps_current_junit(self):
        self.plan()
        result = self.invoke("run")
        self.assertEqual(result.returncode, 0, result.stderr)
        args = json.loads((self.root / "calls").read_text())
        self.assertEqual(args[:3], ["+1.100.0", "nextest", "run"])
        self.assertEqual(args[args.index("--archive-file") + 1], str(self.artifact / "unit-tests.tar.zst"))
        self.assertEqual(args[args.index("--partition") + 1], "hash:2/5")
        self.assertEqual(args[args.index("-E") + 1],
                         "(package(=crate-a)) & test(gameplay_transport::monk_save::postgres_tests::)")
        self.assertIn("--no-fail-fast", args)
        self.assertIn("--no-tests=pass", args)
        self.assertTrue(self.manifest()["completed"])
        self.assertIn('name="current"', (self.result / "junit.xml").read_text())

    def test_test_failures_complete_but_runner_errors_cannot_reuse_a_stale_report(self):
        for code in (100, 70, -15):
            with self.subTest(code=code):
                self.plan()
                self.env.update(NEXT_EXIT=str(code), WRITE_REPORT="1" if code == 100 else "0")
                stale = Path(self.env["CARGO_TARGET_DIR"]) / "nextest/rust-fast/junit.xml"
                stale.parent.mkdir(parents=True, exist_ok=True)
                stale.write_text("stale report")
                result = self.invoke("run")
                self.assertEqual(result.returncode, code % 256, result.stderr)
                self.assertEqual(self.manifest()["exit_code"], code)
                self.assertEqual(self.manifest()["completed"], code == 100)
                self.assertEqual((self.result / "junit.xml").exists(), code == 100)

    def test_wrong_checkout_pending_package_and_archive_identity_fail_closed(self):
        for changed in ({"EXPECTED_SHA": "c" * 40},
                        {"TEST_SHARDS": json.dumps({"tests-2-5": ["unselected"]})}):
            with self.subTest(changed=changed):
                original = self.env.copy()
                self.env.update(changed)
                self.assertNotEqual(self.invoke("plan").returncode, 0)
                self.env = original
        self.plan()
        (self.artifact / "runner-image").write_text("wrong-builder\n")
        self.assertNotEqual(self.invoke("run").returncode, 0)
        self.assertFalse(self.manifest()["completed"])
        self.assertFalse((self.root / "calls").exists())

    def test_partial_rerun_verifies_the_original_build_attempt_and_immutable_id(self):
        source = WORKFLOW.read_text()
        def step(name):
            return source.split(f"      - name: {name}\n", 1)[1].split("\n      - name: ", 1)[0]
        def script(name):
            body = step(name).split("        run: |\n", 1)[1]
            return textwrap.dedent("\n".join(line for line in body.splitlines()
                                           if not line.strip() or line.startswith("          ")))
        self.assertIn("run_attempt: ${{ steps.identity.outputs.run_attempt }}", source)
        self.assertIn("BUILD_ATTEMPT: ${{ needs.build.outputs.run_attempt }}", source)
        self.assertIn("artifact-ids: ${{ needs.build.outputs.artifact_id }}",
                      step("Download this unit-test archive by immutable ID"))
        output = self.root / "identity-output"
        for attempt in ("2", "", "0", "-1", "invalid"):
            with self.subTest(attempt=attempt):
                output.unlink(missing_ok=True)
                env = dict(self.env, GITHUB_OUTPUT=str(output), GITHUB_RUN_ATTEMPT=attempt)
                result = subprocess.run(["bash", "-c", script("Report archive runner identity")],
                                        env=env, capture_output=True, text=True, check=False)
                self.assertEqual(result.returncode == 0, attempt == "2", result.stderr)
                if attempt == "2":
                    self.assertIn("run_attempt=2\n", output.read_text())
                else:
                    self.assertFalse(output.exists())
        # A successful attempt-2 build can be retained when only tests rerun
        # in attempt 3. The verifier must receive the builder's actual attempt.
        stub = 'python3() { printf "%s\\n" "$@"; }\n'
        env = dict(self.env, RUNNER_TEMP=str(self.root), BUILD_ATTEMPT="2", GITHUB_RUN_ATTEMPT="3")
        result = subprocess.run(["bash", "-c", stub + script("Verify canonical Dispatch archive")],
                                env=env, capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(), ["tools/ci/shared_dispatch_archive.py", "verify",
            str(self.root / "rust-fast-archive/dispatch-2"), "dispatch-2", "--producer-image", "image-a",
            "--producer-attempt", "2"])

    def test_native_side_build_and_upload_both_require_explicit_demand(self):
        source = WORKFLOW.read_text()
        self.assertIn("      native_entry_required:\n        default: false\n        type: boolean\n", source)
        expected = '''        if: >-
          inputs.native_entry_required
          && contains(fromJSON('["lib","all","lib-1"]'), inputs.archive)
          && (fromJSON(inputs.rust_selection).mode == 'workspace'
              || contains(fromJSON(inputs.rust_selection).packages, 'oteryn-game-server'))
          && hashFiles('tools/qualification/native_entry_room/prebuilt.py') != ''
'''
        for name in ("Build native entry room on the common library graph", "Upload native entry room binary"):
            with self.subTest(step=name):
                step = source.split(f"      - name: {name}\n", 1)[1].split("\n      - name: ", 1)[0]
                self.assertIn(expected, step)

    def test_native_producer_demand_tracks_consumer_paths_and_incomplete_scope(self):
        directory = WORKFLOW.parent
        consumer = (directory / "native-entry-room-qualification.yml").read_text()
        paths = consumer.split("    paths:\n", 1)[1].split("  workflow_dispatch:", 1)[0]
        patterns = re.findall(r"(?m)^      - (.+)$", paths)
        self.assertTrue(patterns)
        self.assertEqual(len(patterns), len(set(patterns)))
        # This consumer uses exact files and complete directory subtrees.
        # A new pattern kind needs an explicit producer-demand implementation.
        expected = []
        for pattern in patterns:
            marker = pattern[:-2] if pattern.endswith("/**") else pattern
            self.assertNotRegex(marker, r"[?*\[\]{}!]", pattern)
            expected.append(marker)
        parent = (directory / "merge-gate.yml").read_text()
        expression = parent.split("      native_entry_required: >-\n", 1)[1].split("\n\n", 1)[0]
        actual = re.findall(r"contains\(needs\.scope\.outputs\.file_records, '([^']+)'\)", expression)
        self.assertCountEqual(actual, expected)
        self.assertIn("needs.scope.outputs.complete != 'true'", expression)
        # Scope deliberately transports [] when immutable enumeration is
        # incomplete. Missing evidence must keep the producer available.
        for complete, records, wanted in (
            ("true", [{"filename": "apps/game-server/src/main.rs"}], False),
            ("false", [], True), ("", [], True),
            *(("true", [{"filename": marker + "nested/example.rs" if marker.endswith("/") else marker}], True)
              for marker in expected),
        ):
            with self.subTest(complete=complete, records=records):
                serialized = json.dumps(records, separators=(",", ":"))
                demanded = complete != "true" or any(marker in serialized for marker in actual)
                self.assertEqual(demanded, wanted)


if __name__ == "__main__":
    unittest.main()
