"""The relocation ledger holds exactly, and fails closed on any other drift."""

import gzip
import json
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
        for (pinned, pinned_digest), (
            current,
            current_digest,
        ) in ledger.RELOCATIONS.items():
            self.assertNotEqual(pinned_digest, current_digest, pinned)
            archive = ledger.FROZEN_COMPRESSED_SOURCES.get((pinned, pinned_digest))
            expected_digest = pinned_digest
            if archive is None:
                archive = ledger.FROZEN_COMPRESSED_SOURCES.get((current, current_digest))
                expected_digest = current_digest
            if archive is None:
                expected = (ROOT / current).read_bytes()
            else:
                expected = gzip.decompress((ROOT / archive).read_bytes())
                self.assertEqual(ledger.digest(expected), expected_digest)
            self.assertEqual(
                ledger.read_pinned(ROOT, pinned, pinned_digest),
                expected,
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

    def test_a_verbatim_move_keeps_its_pin_digest_and_fails_closed(self):
        for pinned, current in ledger.VERBATIM_MOVES.items():
            self.assertFalse((ROOT / pinned).exists(), pinned)
            self.assertEqual(ledger.current_path(pinned), current)
            data = (ROOT / current).read_bytes()
            self.assertEqual(
                ledger.read_pinned(ROOT, pinned, ledger.digest(data)), data
            )
            self.assertIsNone(ledger.read_pinned(ROOT, pinned, "0" * 64))
            self.assertEqual(ledger.read_text(ROOT, pinned), data.decode())

    def test_unrelocated_pins_still_check_their_own_bytes(self):
        path = "tools/content-migration/source_relocations.py"
        data = (ROOT / path).read_bytes()
        self.assertEqual(ledger.read_pinned(ROOT, path, ledger.digest(data)), data)
        self.assertIsNone(ledger.read_pinned(ROOT, path, "0" * 64))
        self.assertEqual(ledger.current_path(path), path)

    def test_wiki_rekey_preserves_the_exact_predecessor(self):
        def snapshot(key, observations):
            records = {key: {"item_id": 8748, "observations": observations}}
            compact = json.dumps(records, sort_keys=True, separators=(",", ":"))
            document = {
                "records": records,
                "snapshot_sha256": ledger.digest(compact.encode()),
            }
            return (json.dumps(document, indent=2, sort_keys=True) + "\n").encode()

        path = "imports/tibiawiki/facts/items-stats.json"
        previous = ledger.digest(snapshot("8748", [1]))
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / path
            target.parent.mkdir(parents=True)
            rekeyed = snapshot("oteryn:item.tibia.i8748", [1])
            target.write_bytes(rekeyed)
            self.assertEqual(
                ledger.read_pinned(Path(directory), path, previous), rekeyed
            )
            target.write_bytes(snapshot("oteryn:item.tibia.i8748", [2]))
            self.assertIsNone(ledger.read_pinned(Path(directory), path, previous))


if __name__ == "__main__":
    unittest.main()
