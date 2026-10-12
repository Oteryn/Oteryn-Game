#!/usr/bin/env python3
"""Regressions for the duplicate test source guard and the Merge Queue test shard planner."""
from __future__ import annotations

import contextlib
import io
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import check_duplicate_test_sources as guard
import plan_rust_test_shards as shards

LIB = ("app", "lib", "app")
COPY = ("app", "test", "copy_pg")
WRAPPER = ("app", "test", "wrapper_pg")
PLAIN = ("app", "test", "plain")


class DuplicateTestSources(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name).resolve()
        self.write("app/src/core.rs", "pub fn one() {}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn one() {}\n}\n")
        self.write("app/src/plain.rs", "pub fn two() {}\n")
        self.write("app/tests/support/cases.rs", "#[tokio::test]\nasync fn case() {}\n")
        self.cwd = os.getcwd()

    def tearDown(self):
        os.chdir(self.cwd)
        self.tmp.cleanup()

    def write(self, relative, text):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        return path

    def units(self, extra=None):
        units = {
            LIB: ["app/src/lib.rs", "app/src/core.rs", "app/src/plain.rs"],
            PLAIN: ["app/tests/plain.rs"],
        }
        units.update(extra or {})
        return units

    def check(self, units, allowlist):
        with patch.object(guard, "ALLOWED_DUPLICATING_TARGETS", allowlist):
            return guard.find_duplicates(units, self.root)

    def test_single_holder_per_test_source_passes(self):
        self.assertEqual(self.check(self.units(), {}), ([], []))

    def test_copied_test_bearing_source_fails(self):
        violations, stale = self.check(self.units({COPY: ["app/tests/copy_pg.rs", str(self.root / "app/src/core.rs")]}), {})
        self.assertEqual(stale, [])
        self.assertEqual(len(violations), 1)
        self.assertIn("app/src/core.rs defines tests compiled into 2 test targets", violations[0])
        self.assertIn("app:test:copy_pg", violations[0])

    def test_copied_source_without_tests_passes(self):
        units = self.units({COPY: ["app/tests/copy_pg.rs", "app/src/plain.rs"]})
        self.assertEqual(self.check(units, {}), ([], []))

    def test_shared_case_file_fails_unless_one_holder(self):
        units = self.units({COPY: ["app/tests/support/cases.rs"], WRAPPER: ["app/tests/support/cases.rs"]})
        violations, _ = self.check(units, {})
        self.assertEqual(len(violations), 1)
        self.assertIn("app/tests/support/cases.rs", violations[0])
        units.pop(WRAPPER)
        self.assertEqual(self.check(units, {}), ([], []))

    def test_allowlisted_target_is_exempt_only_under_its_prefix(self):
        allowlist = {("app", "copy_pg"): ("app/src/", "reason")}
        units = self.units({COPY: ["app/src/core.rs", "app/tests/support/cases.rs"], WRAPPER: ["app/tests/support/cases.rs"]})
        violations, stale = self.check(units, allowlist)
        self.assertEqual(stale, [])
        self.assertEqual(len(violations), 1)
        self.assertIn("app/tests/support/cases.rs", violations[0])

    def test_unused_allowlist_entry_is_stale(self):
        allowlist = {("app", "copy_pg"): ("app/src/", "reason"), ("app", "gone"): ("app/src/", "reason")}
        units = self.units({COPY: ["app/src/core.rs"]})
        self.assertEqual(self.check(units, allowlist), ([], [("app", "gone")]))

    def test_dep_info_paths_follow_rustc_naming(self):
        self.assertEqual(guard.dep_info_path("/t/debug/deps/libcopy_pg-0123.rmeta"), Path("/t/debug/deps/copy_pg-0123.d"))
        self.assertEqual(guard.dep_info_path("/t/debug/deps/libapp-0123.rlib"), Path("/t/debug/deps/app-0123.d"))
        self.assertEqual(guard.dep_info_path("/t/debug/deps/copy_pg-0123"), Path("/t/debug/deps/copy_pg-0123.d"))
        self.assertEqual(guard.dep_info_path("C:/t/deps/copy_pg-0123.exe"), Path("C:/t/deps/copy_pg-0123.d"))

    def test_dep_info_reads_first_rule_with_escaped_spaces(self):
        path = self.write(
            "deps/copy_pg-1.d",
            "/t/deps/libcopy_pg-1.rmeta: app/tests/copy_pg.rs app/src/my\\ dir/core.rs \\\n app/src/plain.rs\n\n"
            "/t/deps/copy_pg-1.d: app/tests/copy_pg.rs\n\napp/tests/copy_pg.rs:\n",
        )
        self.assertEqual(guard.dep_info_sources(path), ["app/tests/copy_pg.rs", "app/src/my dir/core.rs", "app/src/plain.rs"])
        with self.assertRaises(guard.EvidenceError):
            guard.dep_info_sources(self.write("deps/bad.d", "no rule here\n"))
        with self.assertRaises(guard.EvidenceError):
            guard.dep_info_sources(self.root / "deps/missing.d")

    def artifact(self, name, kind, test, manifest="app/Cargo.toml", package_id=None):
        deps = self.root / "target/debug/deps"
        deps.mkdir(parents=True, exist_ok=True)
        (deps / f"{name}-1.d").write_text(f"{deps}/lib{name}-1.rmeta: app/src/core.rs\n", encoding="utf-8")
        return {
            "reason": "compiler-artifact",
            "package_id": package_id or f"path+file://{self.root}/app#app@0.1.0",
            "manifest_path": str(self.root / manifest),
            "target": {"name": name, "kind": [kind]},
            "profile": {"test": test},
            "filenames": [str(deps / f"lib{name}-1.rmeta")],
            "fresh": True,
        }

    def test_harness_units_keep_only_workspace_test_harnesses(self):
        messages = [
            self.artifact("app", "lib", False),
            self.artifact("app", "lib", True),
            self.artifact("copy_pg", "test", True),
            self.artifact("dep", "lib", True, manifest="../registry/dep/Cargo.toml"),
            {"reason": "build-finished", "success": True},
        ]
        units = guard.harness_units(messages, self.root)
        self.assertEqual(sorted(units), [("app", "lib", "app"), ("app", "test", "copy_pg")])
        legacy = self.artifact("other", "test", True, package_id="other 0.1.0 (path+file:///x)")
        self.assertEqual(sorted(guard.harness_units([legacy], self.root)), [("app", "test", "other")])

    def run_main(self, messages_text):
        evidence = self.write("messages.jsonl", messages_text)
        os.chdir(self.root)
        stdout, stderr = io.StringIO(), io.StringIO()
        with patch.object(guard, "ALLOWED_DUPLICATING_TARGETS", {}), contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            code = guard.main(["guard", str(evidence)])
        return code, stdout.getvalue(), stderr.getvalue()

    def test_main_passes_and_fails_closed(self):
        lib = json.dumps(self.artifact("app", "lib", True))
        code, out, _ = self.run_main("warning: noise\n" + lib + "\n")
        self.assertEqual(code, 0)
        self.assertIn("PASS (1 test-harness units", out)
        code, _, err = self.run_main(lib + "\n" + json.dumps(self.artifact("copy_pg", "test", True)) + "\n")
        self.assertEqual(code, 1)
        self.assertIn("app/src/core.rs defines tests compiled into 2 test targets", err)
        code, _, err = self.run_main(json.dumps(self.artifact("app", "lib", False)) + "\n")
        self.assertEqual(code, 1)
        self.assertIn("no workspace test-harness units", err)
        code, _, err = self.run_main("{not json\n")
        self.assertEqual(code, 1)
        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(guard.main(["guard"]), 2)

    def test_repository_allowlist_entries_are_explained(self):
        self.assertTrue(guard.ALLOWED_DUPLICATING_TARGETS)
        for (package, target), (prefix, reason) in guard.ALLOWED_DUPLICATING_TARGETS.items():
            self.assertTrue(package and target and reason.strip(), (package, target))
            self.assertTrue(prefix.startswith("apps/") and not prefix.startswith("/"), prefix)


def metadata(targets, members=("app 0.1.0",)):
    return {
        "workspace_members": list(members),
        "packages": [
            {"id": "app 0.1.0", "targets": targets},
            {"id": "dep 1.0.0", "targets": [{"name": "dep_test", "kind": ["test"], "test": True}]},
        ],
    }


def target(name, kind="test", test=True):
    return {"name": name, "kind": [kind], "test": test}


class RustTestShards(unittest.TestCase):
    def full(self, *extra):
        return metadata([target("app", "lib"), target("wrapper_pg", test=False), target("plain"), target("other"),
                         *[target(name) for name in shards.HEAVY_TARGETS], *extra])

    def test_shards_partition_tested_integration_targets(self):
        heavy = shards.plan(self.full(), "integration-heavy")
        rest = shards.plan(self.full(), "integration-rest")
        self.assertEqual(heavy[0::2], ["--test"] * len(shards.HEAVY_TARGETS))
        self.assertEqual(rest, ["--test", "other", "--test", "plain"])
        self.assertEqual(set(heavy[1::2]) | set(rest[1::2]), shards.tested_integration_targets(self.full()))
        self.assertFalse(set(heavy[1::2]) & set(rest[1::2]))

    def test_planner_fails_closed(self):
        with self.assertRaises(shards.PlanError):
            shards.plan(self.full(), "integration")
        with self.assertRaises(shards.PlanError):
            shards.plan(metadata([target("plain")]), "integration-heavy")
        only_heavy = metadata([target(name) for name in shards.HEAVY_TARGETS])
        with self.assertRaises(shards.PlanError):
            shards.plan(only_heavy, "integration-rest")
        with self.assertRaises(shards.PlanError):
            shards.plan(self.full(target("bench_me", "bench")), "integration-rest")
        with self.assertRaises(shards.PlanError):
            shards.plan(self.full({"name": "x", "kind": ["test"]}), "integration-rest")
        with self.assertRaises(shards.PlanError):
            shards.plan(metadata([], members=()), "integration-rest")
        self.assertEqual(shards.plan(self.full(target("example_me", "example", False)), "integration-rest")[1::2], ["other", "plain"])


if __name__ == "__main__":
    unittest.main()
