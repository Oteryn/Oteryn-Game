"""The real source overlay is bounded, replayable and refuses partial admission."""

import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import quest_disguise_item_profiles as disguise

ROOT = Path(__file__).resolve().parents[2]


class QuestDisguiseItemProfileTests(unittest.TestCase):
    def overlay(self):
        return json.loads((ROOT / disguise.PATH).read_bytes())

    def temporary_root(self, directory, overlay):
        root = Path(directory)
        for descriptor in overlay["canonical_inputs"]:
            target = root / descriptor["path"]
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes((ROOT / descriptor["path"]).read_bytes())
        bindings = root / disguise.BINDINGS_PATH
        bindings.parent.mkdir(parents=True, exist_ok=True)
        bindings.write_bytes((ROOT / disguise.BINDINGS_PATH).read_bytes())
        target = root / disguise.PATH
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(disguise.encoded(overlay))
        return root

    def planned(self, records=None):
        return {
            "profiles": disguise.encoded(
                {
                    "schema": "OTERYN_NATIVE_ITEM_PROFILES/v1",
                    "records": records or [],
                }
            ),
            "manifest": b'{"item_profiles":{"path":"profiles","sha256":"before"}}',
        }

    def admit(self, root, planned):
        disguise.admit(
            root,
            planned,
            "profiles",
            "manifest",
            lambda row: row["production_definition"]["production_key"],
            disguise.encoded,
        )

    def test_real_overlay_admits_only_disguise_and_preserves_identity_and_unknown_systems(
        self,
    ):
        overlay = self.overlay()
        planned = self.planned()
        self.admit(ROOT, planned)
        records = json.loads(planned["profiles"])["records"]
        self.assertEqual(records, overlay["records"])
        self.assertEqual(len(records), 1)
        record = records[0]
        self.assertEqual(
            record["production_definition"],
            {
                "family": "Item",
                "production_key": disguise.ITEM_KEY,
                "revision_ref": "definition-r1",
            },
        )
        self.assertEqual(record["production_binding"]["external_id"], "144")
        self.assertEqual(
            record["production_binding"]["identity_namespace"], "ots/item_server_id"
        )
        self.assertEqual(
            record["production_binding"]["source_revision"],
            "ff7ede593c69d4c658b382c97443e8155926924a",
        )
        self.assertIsNone(record["production_binding_qualification"])
        self.assertTrue(record["admission"]["materializable"])
        self.assertEqual(record["admission"]["stack_class"], "NonStackable")
        for section in disguise.UNKNOWN_SECTIONS:
            self.assertEqual(record["semantics"][section], {"state": "UNKNOWN"})
        self.assertNotIn("consumption", record["semantics"])
        self.assertNotIn("use_requirements", record["semantics"])
        self.assertEqual(
            json.loads(planned["manifest"])["item_profiles"]["sha256"],
            hashlib.sha256(planned["profiles"]).hexdigest(),
        )
        # This overlay is a native provider, not a rewrite of the legacy declaration.
        definition, _ = disguise.canonical_identity(ROOT)
        self.assertFalse(definition["materializable"])
        self.assertEqual(definition["identity"]["key"], disguise.ITEM_KEY)

    def test_duplicate_refuses_without_modifying_either_output(self):
        planned = self.planned(self.overlay()["records"])
        before = dict(planned)
        with self.assertRaisesRegex(ValueError, "QUEST_DISGUISE_PROFILE_DUPLICATE"):
            self.admit(ROOT, planned)
        self.assertEqual(planned, before)

    def test_unsupported_transform_cannot_be_silently_declared_absent(self):
        overlay = self.overlay()
        overlay["records"][0]["semantics"]["use_transform"] = {
            "state": "NOT_APPLICABLE"
        }
        with tempfile.TemporaryDirectory() as directory:
            root = self.temporary_root(directory, overlay)
            planned = self.planned()
            before = dict(planned)
            with self.assertRaisesRegex(
                ValueError, "QUEST_DISGUISE_PROFILE_QUALIFICATION"
            ):
                self.admit(root, planned)
            self.assertEqual(planned, before)

    def test_missing_record_and_wrong_source_pin_are_rejected_before_output(self):
        for mutate, error in (
            (lambda data: data.update(records=[]), "QUEST_DISGUISE_PROFILE_SET"),
            (
                lambda data: data.update(source_revision="0" * 40),
                "QUEST_DISGUISE_SOURCE_PROVENANCE",
            ),
        ):
            overlay = copy.deepcopy(self.overlay())
            mutate(overlay)
            with self.subTest(error=error), tempfile.TemporaryDirectory() as directory:
                root = self.temporary_root(directory, overlay)
                planned = self.planned()
                before = dict(planned)
                with self.assertRaisesRegex(ValueError, error):
                    self.admit(root, planned)
                self.assertEqual(planned, before)

    def test_canonical_input_drift_is_not_accepted_as_new_permission(self):
        overlay = self.overlay()
        with tempfile.TemporaryDirectory() as directory:
            root = self.temporary_root(directory, overlay)
            path = root / overlay["canonical_inputs"][0]["path"]
            path.write_bytes(path.read_bytes() + b"\n")
            planned = self.planned()
            before = dict(planned)
            with self.assertRaisesRegex(ValueError, "QUEST_DISGUISE_CANONICAL_DRIFT"):
                self.admit(root, planned)
            self.assertEqual(planned, before)

    def test_manifest_error_does_not_partially_admit_the_profile(self):
        planned = self.planned()
        planned["manifest"] = b"{}"
        before = dict(planned)
        with self.assertRaises(KeyError):
            self.admit(ROOT, planned)
        self.assertEqual(planned, before)


if __name__ == "__main__":
    unittest.main()
