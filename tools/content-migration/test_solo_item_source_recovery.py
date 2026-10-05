"""Reject corrupted or escaping recovery members before any project write."""

import copy
import gzip
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from verify_solo_item_source_recovery import (
    MAX_RAW_FILE,
    OLD_REGISTRY,
    recovered_digest,
    safe_path,
    verify_definition_successor,
)

SCHEMA = "OTERYN_SOURCE_DEFINITION_OBSERVATIONS_CLOSED/v1"
KEY = "oteryn:item.tibia.i100"


def observation(kind, assignments=0):
    return {
        "parameter": {"kind": kind},
        "ordered_assignments": [{}] * assignments,
    }


def write_package(root, old_rows, new_rows, charges, census=None):
    old = {"schema": SCHEMA, "targets": {KEY: {"definition-r1": old_rows}}}
    new = {"schema": SCHEMA, "targets": {KEY: {"definition-r1": new_rows}}}
    raw = json.dumps(old).encode()
    (root / "recovery").mkdir()
    (root / "recovery/old.json.gz").write_bytes(gzip.compress(raw))
    manifest = {
        "files": [
            {
                "path": OLD_REGISTRY,
                "archive": "recovery/old.json.gz",
                "raw_bytes": len(raw),
                "raw_sha256": hashlib.sha256(raw).hexdigest(),
            }
        ]
    }
    (root / "recovery-manifest.json").write_text(json.dumps(manifest))
    (root / "source-batches").mkdir()
    (root / "source-batches/definition-registry-successor.json.gz").write_bytes(
        gzip.compress(json.dumps(new).encode())
    )
    census = census or {
        "targets": 1,
        "observations": len(new_rows),
        "new_observations": len(charges),
        "max_observations": len(new_rows),
        "max_assignments": max(
            (len(r["ordered_assignments"]) for r in new_rows), default=0
        ),
        "old_registry_inverse": "EXACT",
    }
    (root / "source-batches/definition-successor-census.json").write_text(
        json.dumps(census)
    )
    return {KEY: charges}


class DefinitionSuccessorTests(unittest.TestCase):
    def check(self, old_rows, new_rows, charges, census=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            records = write_package(root, old_rows, new_rows, charges, census)
            return verify_definition_successor(root, records)

    def test_exact_successor_is_accepted(self):
        old = [observation("DURATION", 1)]
        charge = [observation("CHARGES_AND_LEVEL_DOOR")]
        result = self.check(old, old + charge, charge)
        self.assertEqual(result["observations"], 2)
        self.assertEqual(result["added"], 1)

    def test_dropped_reordered_or_altered_old_observations_are_rejected(self):
        old = [observation("DURATION", 1), observation("FLUID_SOURCE")]
        charge = [observation("CHARGES_AND_LEVEL_DOOR")]
        for changed in (old[1:], old[::-1], [observation("DURATION", 2), old[1]]):
            with self.subTest(changed=changed), self.assertRaises(ValueError):
                self.check(old, changed + charge, charge)

    def test_suffix_must_equal_the_charge_batch(self):
        old = [observation("DURATION")]
        charge = [observation("CHARGES_AND_LEVEL_DOOR")]
        other = [observation("CHARGES_AND_LEVEL_DOOR", 1)]
        for new_rows, charges in (
            (old + other, charge),
            (old, charge),
            (old + charge + charge, charge),
        ):
            with self.subTest(rows=len(new_rows)), self.assertRaises(ValueError):
                self.check(old, new_rows, charges)

    def test_census_must_match_the_data(self):
        old = [observation("DURATION")]
        charge = [observation("CHARGES_AND_LEVEL_DOOR")]
        good = {
            "targets": 1,
            "observations": 2,
            "new_observations": 1,
            "max_observations": 2,
            "max_assignments": 0,
            "old_registry_inverse": "EXACT",
        }
        self.check(old, old + charge, charge, good)
        for field, value in (
            ("targets", 2),
            ("observations", 3),
            ("new_observations", 2),
            ("max_observations", 1),
            ("max_assignments", 1),
            ("old_registry_inverse", "UNKNOWN"),
        ):
            census = copy.deepcopy(good) | {field: value}
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.check(old, old + charge, charge, census)


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
