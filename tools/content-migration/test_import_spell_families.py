#!/usr/bin/env python3
"""Check import integrity and refusals without writing repository content."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch

import import_spell_families as importer


class SpellFamilyImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.runtime, cls.sidecars = importer.verified_inputs()
        cls.planned = importer.outputs()

    def test_changed_pinned_provider_is_rejected_before_import(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            shutil.copytree(importer.ROOT / importer.PACK, root / importer.PACK)
            provider = root / importer.PACK / "presentation_profiles.json"
            provider.write_bytes(provider.read_bytes() + b"\n")
            with self.assertRaisesRegex(ValueError, "INPUT_DIGEST_MISMATCH:presentation_profiles.json"):
                importer.outputs(root)
            self.assertFalse((root / importer.CATALOG).exists())

    def test_changed_reference_sidecar_is_also_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            shutil.copytree(importer.ROOT / importer.PACK, root / importer.PACK)
            provider = root / importer.PACK / "sidecars/player-presentation.json"
            provider.write_bytes(b"{}\n")
            with self.assertRaisesRegex(ValueError, "INPUT_DIGEST_MISMATCH:sidecars/player-presentation.json"):
                importer.verified_inputs(root)

    def test_provider_locator_cannot_escape_source_pack(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pack = root / importer.PACK
            pack.mkdir(parents=True)
            outside = pack.parent / "outside.json"
            outside.write_bytes(b"{}\n")
            with self.assertRaisesRegex(ValueError, "INPUT_PATH_ESCAPE"):
                importer.pinned_file(root, "../outside.json", importer.digest(outside.read_bytes()))

    def test_conflicting_dependency_identity_is_rejected(self):
        catalog = json.loads(self.runtime["catalog"])
        owner = next(entry for entry in catalog["bundles"] if entry["dependencies"]["effects"])
        conflict = copy.deepcopy(owner["dependencies"]["effects"][0])
        conflict["operation"] = "conflicting-source-value"
        other = next(entry for entry in catalog["bundles"] if entry is not owner)
        other["dependencies"]["effects"].append(conflict)
        # Exercise the cross-bundle conflict check independently of bundle validation.
        with patch.object(importer, "validate_catalog", return_value=catalog):
            with self.assertRaisesRegex(ValueError, "CONFLICTING_DEPENDENCY:Effect"):
                importer.outputs()

    def test_duplicate_spell_identity_is_rejected(self):
        catalog = json.loads(self.runtime["catalog"])
        catalog["bundles"][1]["bundle"]["spell"]["identity"] = copy.deepcopy(
            catalog["bundles"][0]["bundle"]["spell"]["identity"]
        )
        with self.assertRaisesRegex(ValueError, "DUPLICATE_SPELL_IDENTITY"):
            importer.validate_catalog(importer.ROOT, importer.canonical_bytes(catalog))

    def test_complete_catalog_and_sidecars_remain_byte_identical(self):
        self.assertEqual(self.planned[importer.CATALOG], self.runtime["catalog"])
        self.assertEqual(self.planned[importer.SELECTION], self.runtime["source_selection"])
        for original, data in self.sidecars.items():
            with self.subTest(path=original):
                self.assertEqual(self.planned["imports/spells/r25/" + Path(original).name], data)

    def test_dependencies_preserve_candidate_identities_and_do_not_replace_legacy_shards(self):
        catalog = json.loads(self.runtime["catalog"])
        for family, (section, path) in importer.DEPENDENCIES.items():
            collection = json.loads(self.planned[path])
            expected = {
                (record["identity"]["key"], record["identity"]["revision"])
                for entry in catalog["bundles"] for record in entry["dependencies"][section]
            }
            actual = {
                (record["identity"]["key"], record["identity"]["revision"])
                for record in collection["records"]
            }
            self.assertEqual(actual, expected, family)
            self.assertEqual(len(collection["records"]), len(actual), family)
            self.assertFalse(collection["runtime_activation"])
            self.assertEqual(collection["schema"], "OTERYN_SPELL_DEPENDENCY_COLLECTION/v1")
        manifest = json.loads((importer.ROOT / "content/manifest.json").read_bytes())
        legacy_shards = set()
        for family in manifest["families"].values():
            index = json.loads((importer.ROOT / family["index"]).read_bytes())
            legacy_shards.update(index.get("shards", []))
        self.assertFalse(legacy_shards.intersection(self.planned))
        for protected in ("content/project.json", "content/manifest.json", "content/content.lock.json"):
            self.assertNotIn(protected, self.planned)
        self.assertFalse(any(path.startswith("content/world/") for path in self.planned))

    def test_runtime_manifest_uses_exact_imported_providers_within_repository(self):
        path = "content/spells.manifest.json"
        manifest = json.loads(self.planned[path])
        old = json.loads((importer.ROOT / importer.PACK / "manifest.json").read_bytes())
        self.assertEqual(manifest["schema"], old["schema"])
        self.assertEqual(manifest.get("native_map_profile"), old.get("native_map_profile"))
        keys = {key for key, value in manifest.items() if isinstance(value, dict)}
        self.assertEqual(keys, set(self.runtime))
        for key in keys:
            with self.subTest(provider=key):
                pin = manifest[key]
                resolved = (importer.ROOT / Path(path).parent / pin["path"]).resolve()
                self.assertTrue(resolved.is_relative_to(importer.ROOT.resolve()))
                relative = resolved.relative_to(importer.ROOT.resolve()).as_posix()
                self.assertNotIn("test-packs", relative)
                self.assertFalse(relative.startswith("docs/"))
                self.assertEqual(self.planned[relative], self.runtime[key])
                self.assertEqual(hashlib.sha256(self.planned[relative]).hexdigest(), pin["sha256"])
                self.assertEqual({k: v for k, v in pin.items() if k != "path"},
                                 {k: v for k, v in old[key].items() if k != "path"})

    def test_generation_and_write_check_are_deterministic_and_idempotent(self):
        self.assertEqual(importer.outputs(), self.planned)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.object(importer, "ROOT", root), patch.object(importer, "outputs", return_value=self.planned):
                with patch.object(sys, "argv", ["import_spell_families.py", "content"]):
                    self.assertEqual(importer.main(), 0)
                    first = {path: (root / path).read_bytes() for path in self.planned}
                    self.assertEqual(importer.main(), 0)
                    self.assertEqual(first, {path: (root / path).read_bytes() for path in self.planned})
                with patch.object(sys, "argv", ["import_spell_families.py", "content", "--check"]):
                    self.assertEqual(importer.main(), 0)
                    target = root / importer.CATALOG
                    target.write_bytes(b"changed\n")
                    with self.assertRaisesRegex(ValueError, "CANONICAL_IMPORT_MISMATCH"):
                        importer.main()
                    self.assertEqual(target.read_bytes(), b"changed\n")


if __name__ == "__main__":
    unittest.main()
