#!/usr/bin/env python3
"""Prove benchmark isolation and honest sample grouping without compiling Rust."""
import hashlib
import json
from pathlib import Path
import os
import shutil
import subprocess
import sys
import tempfile
import textwrap
import time
import unittest

from benchmark_rust_cache import __file__ as controller_path
from benchmark_rust_cache import cargo_command, local_environment, run, summarize


class RustCacheBenchmarkTests(unittest.TestCase):
    def test_external_backends_and_wrappers_cannot_contaminate_plain_build(self):
        parent = {"PATH": "/bin", "RUSTFLAGS": "-Cdebuginfo=0", "CARGO_INCREMENTAL": "1",
                  "CARGO_BUILD_BUILD_DIR": "/foreign/artifacts",
                  "RUSTC_WRAPPER": "foreign", "RUSTC_WORKSPACE_WRAPPER": "other",
                  "SCCACHE_CONF": "/foreign/config", "SCCACHE_GHA_ENABLED": "on", "SCCACHE_BUCKET": "foreign",
                  "SCCACHE_REDIS": "foreign", "SCCACHE_SERVER_PORT": "123"}
        env = local_environment(parent, Path("/tmp/owned/target"), Path("/tmp/owned/cache"), Path("/tmp/owned/sccache.sock"))
        for name in ("SCCACHE_GHA_ENABLED", "SCCACHE_BUCKET", "SCCACHE_REDIS"):
            self.assertNotIn(name, env)
        self.assertEqual(env["RUSTC_WRAPPER"], "")
        self.assertEqual(env["RUSTC_WORKSPACE_WRAPPER"], "")
        self.assertEqual(env["CARGO_INCREMENTAL"], "0")
        self.assertEqual(env["CARGO_BUILD_BUILD_DIR"], "/tmp/owned/target")
        self.assertNotIn("SCCACHE_SERVER_PORT", env)
        self.assertEqual(env["SCCACHE_SERVER_UDS"], "/tmp/owned/sccache.sock")
        self.assertEqual(env["SCCACHE_DIR"], "/tmp/owned/cache")
        self.assertEqual(env["SCCACHE_CONF"], "/tmp/owned/sccache.toml")
        self.assertEqual(env["RUSTFLAGS"], parent["RUSTFLAGS"])
        self.assertEqual(parent["CARGO_INCREMENTAL"], "1")

    def test_actual_workspace_build_remains_locked_and_offline(self):
        workspace = cargo_command("cargo", None)
        self.assertIn("--locked", workspace)
        self.assertIn("--offline", workspace)
        self.assertEqual(workspace[-2:], ["--workspace", "--all-targets"])
        package = cargo_command("cargo", "oteryn-game-server")
        self.assertEqual(package[-3:], ["-p", "oteryn-game-server", "--lib"])

    @unittest.skipUnless(hasattr(os, "killpg") and Path("/proc").is_dir(), "Linux process groups")
    def test_timeout_stops_compiler_children_and_preserves_log(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "tree.log"
            code = ('import subprocess,sys,time; '
                    'p=subprocess.Popen([sys.executable,"-c","import time; time.sleep(30)"]); '
                    'print(p.pid,flush=True); time.sleep(30)')
            with self.assertRaises(subprocess.TimeoutExpired):
                run([sys.executable, "-c", code], os.environ.copy(), log, 0.5)
            child = int(log.read_text().strip())
            state = Path(f"/proc/{child}/stat")

            def process_state():
                # None once the child is gone: it can be reaped between an exists() check and
                # the read, which raises ProcessLookupError (or FileNotFoundError) instead.
                try:
                    return state.read_text().split()[2]
                except (FileNotFoundError, ProcessLookupError):
                    return None

            deadline = time.monotonic() + 1
            while process_state() not in (None, "Z") and time.monotonic() < deadline:
                time.sleep(0.01)
            self.assertIn(process_state(), (None, "Z"))

    def test_cold_and_warm_samples_are_not_pooled(self):
        rows = [{"case": "cold", "build_seconds": 20},
                {"case": "warm", "build_seconds": 2},
                {"case": "warm", "build_seconds": 4}]
        grouped = summarize(rows)
        self.assertEqual(grouped["cold"]["samples"], 1)
        self.assertEqual(grouped["cold"]["median_seconds"], 20)
        self.assertEqual(grouped["warm"], {"samples": 2, "median_seconds": 3,
                                           "min_seconds": 2, "max_seconds": 4})


class RustEditBenchmarkTests(unittest.TestCase):
    def run_edit(self, *, source_name="src/lib.rs", edited="pub fn answer() -> u32 { 2 }\n",
                 fail_edited_build=False, fail_stop=False, ignore_edit=False, legacy=False):
        with tempfile.TemporaryDirectory() as directory:
            temp = Path(directory)
            checkout = temp / "caller"
            controller = checkout / "tools/repository/benchmark_rust_cache.py"
            controller.parent.mkdir(parents=True)
            shutil.copyfile(controller_path, controller)
            source = checkout / source_name
            source.parent.mkdir(parents=True, exist_ok=True)
            base = "pub fn answer() -> u32 { 1 }\n"
            source.write_text(base)
            (checkout / "Cargo.lock").write_text("# fake lock\n")

            def git(*args):
                return subprocess.check_output(["git", "-C", str(checkout), *args], text=True).strip()

            git("init", "--quiet")
            git("add", "Cargo.lock", source_name)
            git("-c", "user.email=fixture@example.test", "-c", "user.name=Fixture",
                "commit", "--quiet", "-m", "benchmark fixture")
            base_tree = git("rev-parse", "HEAD^{tree}")
            source.write_text(edited)
            patch = temp / "change.patch"
            patch.write_text(git("diff", "--binary", "--full-index") + "\n")
            dirty = "pub fn answer() -> u32 { 99 }\n"
            source.write_text(dirty)

            fake = temp / "fake-tool"
            fake.write_text(f"#!{sys.executable}\n" + textwrap.dedent('''\
                import json
                import os
                from pathlib import Path
                import sys

                arg = sys.argv[1]
                cache = Path(os.environ["SCCACHE_DIR"])
                target = Path(os.environ["CARGO_TARGET_DIR"])
                endpoint = Path(os.environ["SCCACHE_SERVER_UDS"])
                if arg == "--start-server":
                    endpoint.touch()
                elif arg == "--stop-server":
                    if os.environ.get("FAKE_FAIL_STOP") == "1":
                        sys.exit(8)
                    endpoint.unlink()
                elif arg == "--show-stats":
                    assert endpoint.exists()
                    print(json.dumps({"cache_location": f'Local disk: "{cache}"', "stats": {}}))
                elif arg == "build":
                    source = Path("src/lib.rs").read_text()
                    marker = target / "previous-source"
                    previous = marker.read_text() if marker.exists() else None
                    cached = bool(os.environ["RUSTC_WRAPPER"])
                    seed = cache / "seed"
                    print(json.dumps({"source": source, "previous": previous, "cached": cached,
                                      "seed": seed.exists(), "cwd": str(Path.cwd()),
                                      "incremental": os.environ["CARGO_INCREMENTAL"]}))
                    if cached and "{ 2 }" in source and os.environ.get("FAKE_FAIL_EDIT") == "1":
                        print("simulated compilation failure")
                        sys.exit(7)
                    if "--message-format=json" in sys.argv:
                        fresh = previous == source or (previous is not None and os.environ.get("FAKE_IGNORE_EDIT") == "1")
                        print(json.dumps({"reason": "compiler-artifact", "fresh": fresh}))
                    target.mkdir(exist_ok=True)
                    marker.write_text(source)
                    if cached:
                        assert endpoint.exists()
                        cache.mkdir(exist_ok=True)
                        seed.touch()
                elif arg not in ("--version", "-vV", "fetch", "--zero-stats"):
                    sys.exit(9)
                '''))
            fake.chmod(0o755)
            scratch, artifacts = temp / "scratch", temp / "artifacts"
            scratch.mkdir()
            routing_override = {} if legacy else {"GIT_INDEX_FILE": str(checkout / ".git/index")}
            proc = subprocess.run(
                [sys.executable, str(controller), "--cargo", str(fake), "--sccache", str(fake),
                 "--package", "oteryn-game-server", "--temporary-root", str(scratch), "--output", str(artifacts)]
                + ([] if legacy else ["--edit-patch", str(patch)]),
                env={**os.environ, **routing_override, "RUSTC": str(fake), "FAKE_FAIL_EDIT": str(int(fail_edited_build)),
                     "FAKE_FAIL_STOP": str(int(fail_stop)), "FAKE_IGNORE_EDIT": str(int(ignore_edit))},
                text=True, capture_output=True, timeout=12)
            self.assertEqual(source.read_text(), dirty)
            self.assertEqual(git("rev-parse", "HEAD^{tree}"), base_tree)
            self.assertEqual(git("diff", "--cached", "--name-only"), "")
            self.assertEqual(list(scratch.iterdir()), [])
            self.assertTrue((artifacts / "results.json").exists(), proc.stderr)
            report = json.loads((artifacts / "results.json").read_text())
            if "source_edit" in report:
                source_edit = report["source_edit"]
                retained_patch = (artifacts / source_edit["patch"]).read_bytes()
                self.assertEqual(retained_patch, patch.read_bytes())
                self.assertEqual(source_edit["patch_sha256"], hashlib.sha256(retained_patch).hexdigest())
            logs = {path.stem: path.read_text() for path in artifacts.glob("*.log")}
            return proc, report, logs, base, base_tree, checkout

    def test_real_edit_uses_same_private_checkout_and_distinct_cache_states(self):
        proc, report, logs, base, base_tree, caller = self.run_edit()
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(report["status"], "success")
        self.assertEqual(report["source_edit"]["base_tree"], base_tree)
        edited_tree = report["source_edit"]["edited_tree"]
        self.assertNotEqual(edited_tree, base_tree)
        self.assertEqual(report["source_edit"]["paths"], ["src/lib.rs"])
        self.assertEqual(report["cleanup"], {"scratch": "removed", "sccache_server": "stopped"})
        cases = {
            "cargo_incremental_cold": (False, False, False, "1", False),
            "cargo_incremental_noop": (False, True, False, "1", False),
            "cargo_incremental_edit_retained_target": (True, True, False, "1", False),
            "sccache_cold": (False, False, True, "0", False),
            "sccache_noop": (False, True, True, "0", True),
            "sccache_edit_empty_target": (True, False, True, "0", True),
        }
        self.assertEqual(set(report["summary"]), set(cases))
        self.assertEqual(len(report["rows"]), 12)
        locations = set()
        for row in report["rows"]:
            case = row["case"]
            edited, retained, cached, incremental, seeded = cases[case]
            observed = json.loads(logs[f"{case}-{row['iteration']}"].splitlines()[0])
            self.assertEqual(row["source_tree"], edited_tree if edited else base_tree)
            self.assertEqual(observed["source"], base.replace("1", "2") if edited else base)
            self.assertEqual(observed["previous"], base if retained else None)
            self.assertEqual((observed["cached"], observed["incremental"], observed["seed"]),
                             (cached, incremental, seeded))
            self.assertEqual(row["rebuilt_artifacts"], int(not case.endswith("_noop")))
            locations.add(observed["cwd"])
        self.assertEqual(len(locations), 1)
        self.assertNotIn(str(caller), locations)

    def test_no_rust_patch_is_rejected_before_measuring(self):
        proc, report, _, _, _, _ = self.run_edit(source_name="README.md")
        self.assertNotEqual(proc.returncode, 0)
        self.assertEqual(report["rows"], [])
        self.assertIn("tracked Rust", report["error"])

    def test_whitespace_or_line_comment_patch_is_rejected(self):
        proc, report, _, _, _, _ = self.run_edit(edited="// comment only\npub fn answer() -> u32 { 1 }\n")
        self.assertNotEqual(proc.returncode, 0)
        self.assertEqual(report["rows"], [])
        self.assertIn("whitespace or line comments", report["error"])

    def test_edit_must_trigger_a_rebuild_with_the_retained_target(self):
        proc, report, _, _, _, _ = self.run_edit(ignore_edit=True)
        self.assertNotEqual(proc.returncode, 0)
        self.assertEqual(report["active_log"], "cargo_incremental_edit_retained_target-1.log")
        self.assertIn("edited build did not rebuild artifacts", report["error"])

    def test_failed_edited_build_preserves_log_and_cleans_up(self):
        proc, report, logs, _, _, _ = self.run_edit(fail_edited_build=True)
        self.assertNotEqual(proc.returncode, 0)
        self.assertEqual(report["status"], "failure")
        self.assertEqual(report["active_log"], "sccache_edit_empty_target-1.log")
        self.assertIn("command failed (7)", report["error"])
        self.assertIn("simulated compilation failure", logs["sccache_edit_empty_target-1"])
        self.assertEqual(report["cleanup"], {"scratch": "removed", "sccache_server": "stopped"})

    def test_failed_daemon_shutdown_cannot_produce_success(self):
        proc, report, _, _, _, _ = self.run_edit(fail_stop=True)
        self.assertNotEqual(proc.returncode, 0)
        self.assertEqual(report["status"], "failure")
        self.assertEqual(report["cleanup"], {"scratch": "removed", "sccache_server": "failed"})

    def test_original_pilot_keeps_its_case_names_and_incremental_disabled(self):
        proc, report, _, _, _, _ = self.run_edit(legacy=True)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(report["schema_version"], 1)
        self.assertFalse(report["incremental"])
        self.assertEqual(set(report["summary"]), {"plain_cold", "plain_warm_target", "sccache_cold",
                                                  "sccache_warm_empty_target", "sccache_warm_target"})


if __name__ == "__main__":
    unittest.main()
