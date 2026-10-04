"""Regression checks for data import without runtime or identity admission."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import imbuement_content as importer


class ContentImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = importer.inputs()
        cls.files = importer.content_files(cls.data)
        cls.docs = [importer.load(importer.ROOT / f"content/{n}.json")
                    for n in ("project", "manifest", "content.lock")]

    def test_preserves_all_tiers_including_basic(self):
        catalogue = json.loads(self.files[importer.PREFIX + "catalogue.json"])
        self.assertEqual(catalogue, self.data["candidate"])
        self.assertEqual(len(catalogue["definitions"]), 24)
        self.assertEqual([t["tier"] for d in catalogue["definitions"] for t in d["tiers"]],
                         [1, 2, 3] * 24)

    def test_unknown_and_unregistered_equipment_are_held(self):
        equipment = json.loads(self.files[importer.PREFIX + "equipment.json"])
        self.assertEqual(len(equipment["profiles"]), 627)
        self.assertTrue(all(r["item_ref"] and r["allowed_types"] for r in equipment["profiles"]))
        held = equipment["held_observations"]
        self.assertEqual(len(held), 36)
        self.assertEqual({r["client_id"] for r in held if r["status"] == "CANONICAL_ITEM_ABSENT"},
                         {49160, 53192})
        source = self.data["packets"]["imbuement-eligibility.json"]["items"]
        self.assertEqual(sorted(r["client_id"] for r in source),
                         sorted(r["client_id"] for r in held + equipment["profiles"]))

    def test_registration_preserves_other_families_and_rulesets(self):
        original = copy.deepcopy(self.docs)
        original[1].setdefault("rulesets", {})["Other"] = {"index": "rulesets/other/index.json"}
        project, manifest, lock = importer.registered(*original, self.files)
        self.assertEqual(project["migrated_families"], original[0]["migrated_families"])
        self.assertEqual(manifest["families"], original[1]["families"])
        self.assertEqual(lock["family_counts"], original[2]["family_counts"])
        self.assertEqual(manifest["rulesets"]["Other"], original[1]["rulesets"]["Other"])
        self.assertFalse(manifest["compatibility"]["runtime_switch_authorized"])
        self.assertEqual(original[1]["rulesets"].get("Imbuements"), self.docs[1].get("rulesets", {}).get("Imbuements"))

    def test_refuses_runtime_activation(self):
        for which in (0, 1):
            docs = copy.deepcopy(self.docs)
            if which == 0:
                docs[0]["runtime_source"] = "new_runtime"
            else:
                docs[1]["compatibility"]["runtime_switch_authorized"] = True
            with self.assertRaisesRegex(ValueError, "cannot authorize runtime"):
                importer.registered(*docs, self.files)

    def test_payload_hashes_bind_all_imported_bytes(self):
        index = json.loads(self.files[importer.PREFIX + "index.json"])
        self.assertEqual({r["path"] for r in index["payloads"]},
                         set(self.files) - {importer.PREFIX + "index.json"})
        for row in index["payloads"]:
            self.assertEqual(row["sha256"], importer.sha(self.files[row["path"]]))

    def test_check_rejects_tampered_import_without_writing(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name, doc in zip(("project", "manifest", "content.lock"), self.docs):
                p = root / f"content/{name}.json"
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_bytes(importer.encoded(doc))
            with (patch.object(importer, "ROOT", root),
                  patch.object(importer, "inputs", return_value=self.data),
                  patch.object(importer, "content_files", side_effect=lambda _: dict(self.files))):
                importer.command(False)
                importer.command(True)
                p = root / (importer.PREFIX + "owner-policy.json")
                altered = b'{"runtime_switch_authorized":true}\n'
                p.write_bytes(altered)
                with self.assertRaisesRegex(ValueError, "stale imported"):
                    importer.command(True)
                self.assertEqual(p.read_bytes(), altered)


if __name__ == "__main__":
    unittest.main()
