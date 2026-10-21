"""The relocation ledger holds exactly, and fails closed on any other drift."""

import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import source_relocations as ledger

ROOT = Path(__file__).resolve().parents[2]


class SourceRelocationTests(unittest.TestCase):
    def test_every_entry_names_the_exact_current_source(self):
        for (pinned, pinned_digest), (current, current_digest) in (
            ledger.RELOCATIONS.items()
        ):
            self.assertNotEqual(pinned_digest, current_digest, pinned)
            self.assertEqual(
                ledger.read_pinned(ROOT, pinned, pinned_digest),
                (ROOT / current).read_bytes(),
            )
            if pinned != current:
                self.assertFalse((ROOT / pinned).exists(), pinned)
                self.assertEqual(ledger.current_path(pinned), current)

    def test_an_unrecorded_digest_or_a_further_edit_fails_closed(self):
        (pinned, pinned_digest), (current, _) = next(
            (key, value)
            for key, value in ledger.RELOCATIONS.items()
            if key[0] != value[0]
        )
        self.assertIsNone(ledger.read_pinned(ROOT, pinned, "0" * 64))
        with tempfile.TemporaryDirectory() as directory:
            copy = Path(directory) / current
            copy.parent.mkdir(parents=True)
            shutil.copyfile(ROOT / current, copy)
            self.assertIsNotNone(ledger.read_pinned(directory, pinned, pinned_digest))
            copy.write_bytes(copy.read_bytes() + b"\n")
            self.assertIsNone(ledger.read_pinned(directory, pinned, pinned_digest))
            copy.unlink()
            self.assertIsNone(ledger.read_pinned(directory, pinned, pinned_digest))

    def test_unrelocated_pins_still_check_their_own_bytes(self):
        path = "tools/content-migration/source_relocations.py"
        data = (ROOT / path).read_bytes()
        self.assertEqual(ledger.read_pinned(ROOT, path, ledger.digest(data)), data)
        self.assertIsNone(ledger.read_pinned(ROOT, path, "0" * 64))
        self.assertEqual(ledger.current_path(path), path)


if __name__ == "__main__":
    unittest.main()
