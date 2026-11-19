import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from normalize_cargo_mtimes import LEGACY_MARKER, SNAPSHOT, normalize, record_snapshot


def git(cwd, *args):
    return subprocess.check_output(
        ["git", "-c", "user.name=t", "-c", "user.email=t@t", *args], cwd=cwd, text=True
    ).strip()


class NormalizeCargoMtimes(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        git(self.root, "init", "-q")
        (self.root / "target").mkdir()
        (self.root / "a.rs").write_text("a")
        (self.root / "b.rs").write_text("b")
        self.commit("one")

    def commit(self, message):
        git(self.root, "add", "-A", "--", ".", ":!target")
        git(self.root, "commit", "-qm", message)

    def mtime(self, path):
        return (self.root / path).stat().st_mtime_ns

    def test_only_changed_files_get_new_mtimes(self):
        unchanged, changed = self.mtime("a.rs"), self.mtime("b.rs")
        record_snapshot(self.root)
        (self.root / "a.rs").touch()
        (self.root / "b.rs").write_text("b2")
        self.commit("two")

        self.assertIn("restored 1 mtimes", normalize(self.root))
        self.assertEqual(self.mtime("a.rs"), unchanged)
        self.assertGreater(self.mtime("b.rs"), changed)

    def test_changed_or_removed_entries_refresh_parent_directories(self):
        for name in ("same", "edited", "removed"):
            (self.root / name).mkdir()
            (self.root / name / "keep.rs").write_text(name)
        (self.root / "removed/gone.rs").write_text("gone")
        self.commit("three")
        record_snapshot(self.root)
        previous = {path: self.mtime(path) for path in (".", "same", "edited", "removed")}
        (self.root / "edited/keep.rs").write_text("edited2")
        (self.root / "removed/gone.rs").unlink()
        self.commit("four")

        normalize(self.root)
        self.assertEqual(self.mtime("same"), previous["same"])
        for path in (".", "edited", "removed"):
            self.assertGreater(self.mtime(path), previous[path])

    def test_missing_or_legacy_snapshot_discards_only_the_ci_target(self):
        (self.root / LEGACY_MARKER).write_text(git(self.root, "rev-parse", "HEAD"))
        (self.root / "registry").touch()
        before = self.mtime("a.rs")
        self.assertIn("unavailable", normalize(self.root))
        self.assertTrue((self.root / LEGACY_MARKER).is_file())
        self.assertIn("discarded", normalize(self.root, discard_invalid=True))
        self.assertFalse((self.root / "target").exists())
        self.assertTrue((self.root / "registry").exists())
        self.assertEqual(self.mtime("a.rs"), before)

    def test_incomplete_snapshot_does_not_age_sources(self):
        record_snapshot(self.root)
        path = self.root / SNAPSHOT
        snapshot = json.loads(path.read_text())
        del snapshot["mtimes"]["b.rs"]
        path.write_text(json.dumps(snapshot))
        (self.root / "a.rs").touch()
        before = self.mtime("a.rs")

        self.assertIn("missing mtimes", normalize(self.root, discard_invalid=True))
        self.assertEqual(self.mtime("a.rs"), before)
        self.assertFalse((self.root / "target").exists())

    def test_future_recorded_at_cannot_make_a_new_mtime_older_than_the_cache(self):
        record_snapshot(self.root)
        path = self.root / SNAPSHOT
        snapshot = json.loads(path.read_text())
        snapshot["recorded_at_ns"] += 60_000_000_000
        path.write_text(json.dumps(snapshot))

        self.assertIn("invalid cache source snapshot capture time", normalize(self.root, discard_invalid=True))
        self.assertFalse((self.root / "target").exists())

    def test_dirty_worktree_cannot_be_recorded_or_restored(self):
        record_snapshot(self.root)
        (self.root / "b.rs").write_text("changed outside the snapshot")

        with self.assertRaisesRegex(ValueError, "tracked worktree differs"):
            record_snapshot(self.root)
        self.assertIn("tracked worktree differs", normalize(self.root, discard_invalid=True))
        self.assertFalse((self.root / "target").exists())

    def test_record_removes_legacy_marker(self):
        (self.root / LEGACY_MARKER).write_text(git(self.root, "rev-parse", "HEAD"))

        record_snapshot(self.root)

        self.assertFalse((self.root / LEGACY_MARKER).exists())
        self.assertTrue((self.root / SNAPSHOT).is_file())

    @unittest.skipUnless(shutil.which("cargo"), "Cargo is unavailable")
    def test_inherited_unbuilt_test_is_recompiled_after_another_package_warms_the_cache(self):
        def cargo(*args, check=True):
            return subprocess.run(
                ["cargo", *args, "--offline"], cwd=self.root,
                env=dict(os.environ, CARGO_TARGET_DIR=str(self.root / "target")),
                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, check=check,
            )

        (self.root / "Cargo.toml").write_text(
            '[workspace]\nmembers = ["dispatch", "retained"]\nresolver = "2"\n'
        )
        for package in ("dispatch", "retained"):
            (self.root / package / "src").mkdir(parents=True)
            (self.root / package / "Cargo.toml").write_text(
                f'[package]\nname = "{package}"\nversion = "0.1.0"\nedition = "2021"\n'
            )
            (self.root / package / "src/lib.rs").write_text("#[test] fn witness() {}\n")
        cargo("generate-lockfile")
        self.commit("baseline")
        cargo("test", "--locked", "-p", "dispatch")
        record_snapshot(self.root)
        dispatch = self.root / "dispatch/src/lib.rs"
        dispatch.write_text('#[test] fn witness() { panic!("new source compiled"); }\n')
        retained = self.root / "retained/Cargo.toml"
        retained.write_text(retained.read_text().replace("0.1.0", "0.1.1"))
        cargo("generate-lockfile")
        self.commit("change dispatch and lockfile")
        self.assertIn("cache source", normalize(self.root))
        cargo("test", "--locked", "--workspace", "--exclude", "dispatch", "--no-run")
        expected = self.mtime("dispatch/src/lib.rs")
        record_snapshot(self.root)

        # A fresh checkout restores the same changed-path timestamp as the producer.
        dispatch.touch()
        self.assertIn("cache source", normalize(self.root))
        self.assertEqual(self.mtime("dispatch/src/lib.rs"), expected)
        result = cargo("test", "--locked", "-p", "dispatch", check=False)
        self.assertEqual(result.returncode, 101, result.stdout)
        self.assertIn("test witness ... FAILED", result.stdout)
        self.assertIn("new source compiled", result.stdout)


if __name__ == "__main__":
    unittest.main()
