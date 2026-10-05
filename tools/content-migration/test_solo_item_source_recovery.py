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
    ability_row_sizes,
    lexeme_bytes,
    recovered_digest,
    safe_path,
    verify_charge_observation,
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


def charge_observation(**changes):
    value = {
        "current_world_owner_pointers": [
            {
                "family": "Terrain",
                "key": "oteryn:terrain.tibia.i100",
                "revision": "definition-r1",
            }
        ],
        "external_item_id": 100,
        "ordered_assignments": [],
        "parameter": {
            "charges_default_u32": 0,
            "charges_origin": "OWN_CPP_INITIALIZER",
            "kind": "CHARGES_AND_LEVEL_DOOR",
            "level_door_origin": "OWN_CPP_INITIALIZER",
            "level_door_u32": 0,
        },
        "phase": "FRESH_CPP_PROTOBUF_THEN_FULL_ORDERED_XML_BEFORE_LUA",
        "prototype_message_sha256": "a" * 64,
        "source_cut": "CRYSTAL_FF7",
        "source_group": "ITEM_GROUP_GROUND",
        "xml_record_ordinal": 20,
        "xml_record_sha256": "b" * 64,
    }
    return value | changes


class ChargeObservationShapeTests(unittest.TestCase):
    def test_closed_shape_is_accepted(self):
        verify_charge_observation(charge_observation())
        explicit = charge_observation(
            ordered_assignments=[
                {"attribute_ordinal": 1, "key": "charges", "value_lexeme": "5"}
            ]
        )
        explicit["parameter"] = explicit["parameter"] | {
            "charges_origin": "EXPLICIT_ORDERED_XML"
        }
        verify_charge_observation(explicit)

    def test_malformed_observations_are_rejected(self):
        bad_parameter = charge_observation()["parameter"]
        del bad_parameter["charges_origin"]
        extra = charge_observation()
        extra["unexpected"] = 1
        for name, value in {
            "missing origin": charge_observation(parameter=bad_parameter),
            "arbitrary cut": charge_observation(source_cut="OTHER"),
            "bad phase": charge_observation(phase="LUA"),
            "bad digest": charge_observation(xml_record_sha256="xyz"),
            "bad ordinal": charge_observation(xml_record_ordinal=-1),
            "bad owner family": charge_observation(
                current_world_owner_pointers=[
                    {"family": "Item", "key": "k", "revision": "definition-r1"}
                ]
            ),
            "assignment without explicit origin": charge_observation(
                ordered_assignments=[
                    {"attribute_ordinal": 1, "key": "k", "value_lexeme": "v"}
                ]
            ),
            "extra key": extra,
        }.items():
            with self.subTest(name=name), self.assertRaises(ValueError):
                verify_charge_observation(value)


class AbilityMaximaTests(unittest.TestCase):
    def test_lexeme_bytes_counts_nested_utf8(self):
        row = {
            "ordered_assignments": [{"value_lexeme": "ab"}],
            "ordered_events": [{"values": [{"value_lexeme": "\u00e9\u00e9"}]}],
        }
        self.assertEqual(sorted(lexeme_bytes(row)), [2, 4])

    def test_row_sizes_are_recomputed_from_the_row(self):
        row = {
            "parameter": {
                "ordered_assignments": [{"value_lexeme": "x" * 300}],
                "ordered_events": [{}, {}],
            }
        }
        line = json.dumps(row) + "\n"
        self.assertEqual(
            ability_row_sizes(row, line),
            {
                "events": 2,
                "direct_assignments": 1,
                "lexeme_UTF8_bytes": 300,
                "serialized_bytes": len(line),
            },
        )


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
