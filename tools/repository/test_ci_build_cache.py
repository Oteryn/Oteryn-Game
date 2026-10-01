#!/usr/bin/env python3
"""Mutation controls for CI cache trust, profile and cancellation boundaries."""
from __future__ import annotations

import importlib.util
import hashlib
import re
import subprocess
import sys
import tempfile
import textwrap
from pathlib import Path
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "policy_core", ROOT / "tools/repository/validate_repository_policy_core.py"
)
assert SPEC and SPEC.loader
core = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(core)


class CacheContractTests(unittest.TestCase):
    def setUp(self):
        self.workflows = {
            name: (ROOT / ".github/workflows" / name).read_text(encoding="utf-8")
            for name in ("rust.yml", "merge-gate.yml", "merge-group-gate.yml")
        }

    def reject(self, name, old, new):
        self.assertIn(old, self.workflows[name])
        self.workflows[name] = self.workflows[name].replace(old, new, 1)
        self.assertTrue(core.validate_ci_build_cache(self.workflows))

    def test_reviewed_contract(self):
        self.assertEqual(core.validate_ci_build_cache(self.workflows), [])

    def test_pull_requests_cannot_publish(self):
        self.reject("merge-gate.yml", "save-if: 'false'", "save-if: 'true'")

    def test_queue_cannot_publish(self):
        self.reject("merge-group-gate.yml", "save-if: 'false'", "save-if: 'true'")

    def test_manual_main_run_cannot_publish(self):
        self.reject("rust.yml", "save-if: ${{ github.event_name == 'push' && github.ref == 'refs/heads/main' && github.ref_protected }}", "save-if: ${{ github.ref == 'refs/heads/main' }}")

    def test_no_workspace_or_incremental_cache(self):
        for setting in ("cache-workspace-crates", "cache-all-crates"):
            with self.subTest(setting=setting):
                self.setUp()
                self.reject("rust.yml", f"{setting}: 'false'", f"{setting}: 'true'")

    def test_windows_has_separate_partition(self):
        self.reject("merge-group-gate.yml", "shared-key: windows-msvc", "shared-key: linux")

    def test_exact_toolchain_and_profile(self):
        for old, new in (("RUSTUP_TOOLCHAIN: '1.94.0'", "RUSTUP_TOOLCHAIN: stable"), ("CARGO_PROFILE_TEST_DEBUG: line-tables-only", "CARGO_PROFILE_TEST_DEBUG: '0'")):
            with self.subTest(setting=old):
                self.setUp()
                self.reject("merge-gate.yml", old, new)

    def test_release_profile_remains_unchanged(self):
        self.reject("rust.yml", "CARGO_PROFILE_DEV_DEBUG: line-tables-only", "CARGO_PROFILE_RELEASE_DEBUG: '0'")

    def test_manual_dispatch_does_not_cancel_push(self):
        self.reject("rust.yml", "cancel-in-progress: ${{ github.event_name == 'push' }}", "cancel-in-progress: true")

    def test_action_is_immutable(self):
        self.reject("merge-gate.yml", "Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6", "Swatinem/rust-cache@v2")

    def test_missing_queue_cache_is_rejected(self):
        self.reject("merge-group-gate.yml", "        uses: Swatinem/rust-cache@", "        uses: removed/cache@")

    def test_unselected_push_does_not_cancel_selected_jobs(self):
        self.reject("rust.yml", "jobs:\n", "concurrency:\n  group: rust-main\n  cancel-in-progress: true\n\njobs:\n")

    def test_reviewed_pins_reject_unrotated_code(self):
        original = Path.read_text
        target = ROOT / ".github/workflows/merge-gate.yml"
        changed = self.workflows["merge-gate.yml"].replace("  scope:\n", "  scope:\n    if: false\n", 1)
        with patch.object(Path, "read_text", lambda path, *a, **kw: changed if path == target else original(path, *a, **kw)):
            errors = core.validate_control_pins()
        self.assertTrue(any("scope job must exactly match" in error for error in errors))

    def test_rotated_pins_do_not_authorize_removing_pr_postgres_or_sim(self):
        spec = importlib.util.spec_from_file_location(
            "pr_semantic_policy", ROOT / "tools/repository/validate_pr_gate_pg_sim.py"
        )
        assert spec and spec.loader
        validator = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(validator)
        original = Path.read_text
        cases = (
            ("rust_linux", '              cargo +1.94.0 test --locked -p oteryn-game-server --test "$name"\n'),
            ("rust_windows", "        run: cargo +1.94.0 test --locked -p oteryn-simulation-determinism --target x86_64-pc-windows-msvc\n"),
        )
        for job, command in cases:
            with self.subTest(job=job):
                self.assertEqual(self.workflows["merge-gate.yml"].count(command), 1)
                changed = self.workflows["merge-gate.yml"].replace(command, "", 1)
                forged = dict(validator.EXPECTED_EVIDENCE_JOB_SHA256)
                forged[job] = hashlib.sha256(validator.job_block(changed, job).encode()).hexdigest()
                with patch.object(validator, "EXPECTED_EVIDENCE_JOB_SHA256", forged), patch.object(
                    Path, "read_text", lambda path, *a, **kw: changed if path == validator.MERGE_GATE else original(path, *a, **kw)
                ):
                    errors = validator.validate()
                self.assertTrue(errors)
                self.assertFalse(any("exactly match" in error for error in errors), errors)

    def test_hosted_pin_reader_uses_inert_registry_and_rejects_drift(self):
        match = re.search(
            r"      - name: Verify routing validator blob\n        run: \|\n(.*?)(?=\n      - name:)",
            self.workflows["merge-gate.yml"], re.S,
        )
        self.assertIsNotNone(match)
        shell = textwrap.dedent(match.group(1))
        source = shell.split("<<'PY'\n", 1)[1].rsplit("\nPY", 1)[0]
        data = b"print('validator fixture')\n"
        digest = core.git_blob_sha(data)
        target = "tools/repository/validate_pr_routing_contract.py"
        row = (target, "blob", None, digest, "fixture")
        cases = (
            (row, data, False, True),
            (row, data + b"# drift\n", False, False),
            (("other.py", *row[1:]), data, False, False),
            ((target, "sha256", *row[2:]), data, False, False),
            ((target, "blob", None, "not-a-sha", "fixture"), data, False, False),
            (row, data, True, False),
        )
        for binding, content, duplicate, allowed in cases:
            with self.subTest(binding=binding, duplicate=duplicate), tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                (root / target).parent.mkdir(parents=True)
                (root / target).write_bytes(content)
                registry = f"CONTROL_CONTRACT_PINS = {{'routing-validator': {binding!r}}}\n"
                if duplicate:
                    registry += registry
                # Candidate-core code must remain inert in this pre-execution check.
                registry += "from pathlib import Path\nPath('EXECUTED').touch()\n"
                (root / "tools/repository/validate_repository_policy_core.py").write_text(registry, encoding="utf-8")
                result = subprocess.run([sys.executable, "-I", "-c", source], cwd=root, capture_output=True, check=False)
                self.assertEqual(result.returncode == 0, allowed, result.stderr.decode())
                self.assertFalse((root / "EXECUTED").exists())


if __name__ == "__main__":
    unittest.main()
