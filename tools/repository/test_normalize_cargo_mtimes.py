import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from normalize_cargo_mtimes import normalize


def run(cwd, *args):
    return subprocess.check_output(["git", "-c", "user.name=t", "-c", "user.email=t@t", *args], cwd=cwd, text=True).strip()


class NormalizeCargoMtimes(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        run(self.root, "init", "-q")
        (self.root / "a.rs").write_text("a")
        (self.root / "b.rs").write_text("b")
        run(self.root, "add", ".")
        run(self.root, "commit", "-qm", "one")
        self.cached = run(self.root, "rev-parse", "HEAD")
        (self.root / "b.rs").write_text("b2")
        run(self.root, "commit", "-qam", "two")
        self.cwd = os.getcwd()

    def tearDown(self):
        os.chdir(self.cwd)
        self.tmp.cleanup()

    def test_only_changed_files_keep_fresh_mtime(self):
        (self.root / "target").mkdir()
        (self.root / "target/.oteryn-cache-source-sha").write_text(self.cached + "\n")
        message = normalize(self.root)
        self.assertIn("1 unchanged", message)
        self.assertEqual(int((self.root / "a.rs").stat().st_mtime), 946684800)
        self.assertGreater((self.root / "b.rs").stat().st_mtime, 946684800)

    def test_directories_age_unless_an_entry_changed(self):
        # Build scripts watch directories (cargo:rerun-if-changed=<dir>); Cargo compares
        # the newest mtime of the directory and every entry under it, so a directory
        # left with its checkout mtime rebuilt the crate even when nothing in it changed.
        for name in ("same", "edited", "removed"):
            (self.root / name).mkdir()
            (self.root / name / "keep.rs").write_text(name)
        (self.root / "removed/gone.rs").write_text("gone")
        run(self.root, "add", ".")
        run(self.root, "commit", "-qm", "three")
        cached = run(self.root, "rev-parse", "HEAD")
        (self.root / "edited/keep.rs").write_text("edited2")
        run(self.root, "rm", "-q", "removed/gone.rs")
        run(self.root, "commit", "-qam", "four")
        (self.root / "target").mkdir()
        (self.root / "target/.oteryn-cache-source-sha").write_text(cached + "\n")
        message = normalize(self.root)
        self.assertIn("1 unchanged director", message)
        self.assertEqual(int((self.root / "same").stat().st_mtime), 946684800)
        self.assertGreater((self.root / "edited").stat().st_mtime, 946684800)
        # A deleted entry leaves no file behind, so only its directory's fresh mtime
        # tells Cargo that the watched directory changed.
        self.assertGreater((self.root / "removed").stat().st_mtime, 946684800)
        self.assertGreater(self.root.stat().st_mtime, 946684800)
        self.assertGreater((self.root / "target").stat().st_mtime, 946684800)

    def test_fails_open_without_marker(self):
        before = (self.root / "a.rs").stat().st_mtime
        self.assertIn("no cache source marker", normalize(self.root))
        self.assertEqual((self.root / "a.rs").stat().st_mtime, before)

    def test_rejects_malformed_marker(self):
        (self.root / "target").mkdir()
        (self.root / "target/.oteryn-cache-source-sha").write_text("../etc\n")
        self.assertIn("invalid", normalize(self.root))


if __name__ == "__main__":
    unittest.main()
