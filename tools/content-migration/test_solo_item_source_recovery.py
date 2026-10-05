"""Reject corrupted or escaping recovery members before any project write."""

import gzip
import tempfile
import unittest
from pathlib import Path

from verify_solo_item_source_recovery import MAX_RAW_FILE, recovered_digest, safe_path


class RecoveryBoundaryTests(unittest.TestCase):
    def test_archive_truncation_and_inflation_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory) / "member.gz"
            archive.write_bytes(gzip.compress(b"verified source bytes"))
            self.assertEqual(len(recovered_digest(archive, 21)), 64)
            for declared in (0, 20, 22, -1, MAX_RAW_FILE + 1, True):
                with self.subTest(declared=declared), self.assertRaises(ValueError):
                    recovered_digest(archive, declared)
            archive.write_bytes(archive.read_bytes()[:-5])
            with self.assertRaises((EOFError, OSError)):
                recovered_digest(archive, 21)

    def test_members_cannot_escape_or_read_a_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for member in ("/absolute.gz", "../outside.gz", ".git/config", ""):
                with self.subTest(member=member), self.assertRaises(ValueError):
                    safe_path(root, member)
            (root / "linked.gz").symlink_to(root / "member.gz")
            with self.assertRaises(ValueError):
                safe_path(root, "linked.gz")
            self.assertEqual(
                safe_path(root, "recovery/files/definition.json.gz"),
                root / "recovery/files/definition.json.gz",
            )


if __name__ == "__main__":
    unittest.main()
