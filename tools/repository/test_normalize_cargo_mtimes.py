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
