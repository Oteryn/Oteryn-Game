"""Exercise completion and copyback fences; native serialization is checked by G4."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import native_world_canonicalization as canonical
import questpc7_thaian_authoring as thaian
import regenerate_content as regenerate


def encode(value):
    return (json.dumps(value, separators=(",", ":")) + "\n").encode()


class NativeWorldCompletion(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        root = Path(self.temporary.name)
        self.repo = root / "repo"
        self.world = root / "separate-package"
        self.calls = []
        packet = Path(thaian.ROOT) / thaian.PACKET
        self.write(self.repo / thaian.PACKET, packet.read_bytes())
        self.write(self.repo / "tools/content-migration/creature_admission_stage.py", b"fixture mapper\n")
        self.write(self.repo / "crates/gameplay/content/src/content/project/v2.rs", b"pub enum ProjectV2Family {\n    Presentation,\n    Creature,\n    Behavior,\n}\n")
        documents = {locator: {} for locator in canonical.DOCUMENTS}
        documents["definitions/reference.json"] = {"records": []}
        documents["definitions/declarations.json"] = {"records": [], "authoring_profiles": []}
        documents["provenance/sources.json"] = {"sources": [], "source_identity_bindings": []}
        documents["provenance/imports.json"] = {"batches": []}
        documents["project.json"] = {"project_revision": "fixture-r1"}
        for locator, value in documents.items():
            self.write(self.world / locator, encode(value))
        manifest = {
            "package_key": "oteryn:fixture", "package_revision": "fixture-r1",
            "semantic_schema_version": "fixture-v1", "licensing_metadata": "fixture",
            "documents": [{"locator": locator} for locator in canonical.DOCUMENTS
                          if locator not in {"manifest.json", "content.lock.json", "project.json"}],
        }
        manifest_raw = encode(manifest)
        digest = hashlib.sha256(manifest_raw).hexdigest()
        values = [manifest[k] for k in ["package_key", "package_revision", "semantic_schema_version", "licensing_metadata"]] + [digest]
        provenance = hashlib.sha256(b"".join(len(v.encode()).to_bytes(4, "big") + v.encode() for v in values)).hexdigest()
        lock_raw = encode({"entries": [{"package_key": "oteryn:fixture", "package_revision": "fixture-r1", "package_provenance_digest": provenance}]})
        self.write(self.world / "manifest.json", manifest_raw)
        self.write(self.world / "content.lock.json", lock_raw)
        self.write(self.world / "project.json", encode({
            "project_revision": "fixture-r1", "manifest_sha256": digest,
            "content_lock_sha256": hashlib.sha256(lock_raw).hexdigest(),
        }))
        self.write(self.world / "unmanaged/sentinel", b"retain caller sibling\n")
        self.original = canonical.read_documents(self.world)

    @staticmethod
    def write(path, raw):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)

    def native_output(self, command, *, cwd, check):
        self.assertEqual(cwd, self.repo)
        self.assertTrue(check)
        self.assertEqual(command[:8], ["cargo", "run", "--locked", "-p", "oteryn-gameplay-content", "--bin", "oteryn-world-project-canonicalize", "--"])
        source = Path(command[command.index("--input-root") + 1])
        output = Path(command[command.index("--output-root") + 1])
        self.assertNotEqual(source, self.world)
        self.assertFalse(output.exists())
        shutil.copytree(source, output)
        self.calls.append(output)
        return subprocess.CompletedProcess(command, 0)

    def test_standalone_uses_native_completion_for_the_explicit_package(self):
        self.write(self.repo / "content/world/project.json", b"do not borrow repository output\n")
        with patch.object(canonical.subprocess, "run", side_effect=self.native_output):
            extent = thaian.install(self.repo, self.world)
        self.assertEqual(len(self.calls), 1)
        self.assertEqual(extent["creatures"], 1)
        self.assertEqual(extent["references"], 3)
        self.assertEqual(extent["imports"], 1)
        completed = canonical.read_documents(self.world)
        changed = {locator for locator in completed if completed[locator] != self.original[locator]}
        self.assertEqual(changed, canonical.OWNED_DOCUMENTS)
        self.assertEqual((self.world / "unmanaged/sentinel").read_bytes(), b"retain caller sibling\n")
        self.assertEqual((self.repo / "content/world/project.json").read_bytes(), b"do not borrow repository output\n")

    def test_native_failure_cannot_leave_staged_python_bytes_in_the_input(self):
        default_world = self.repo / "content/world"
        shutil.copytree(self.world, default_world)
        with patch.object(canonical.subprocess, "run", side_effect=subprocess.CalledProcessError(1, "cargo")) as native:
            with self.assertRaises(subprocess.CalledProcessError):
                thaian.install(self.repo)
        native.assert_called_once()
        self.assertEqual(canonical.read_documents(default_world), self.original)

    def test_normal_regeneration_completes_natively_before_refreshing_pins(self):
        shutil.copytree(self.world, self.repo / "content/world")

        class Completed(Exception):
            pass

        def measured():
            self.assertEqual(len(self.calls), 1)
            raise Completed

        with patch.object(sys, "argv", ["regenerate_content.py"]), \
                patch.object(regenerate, "ROOT", self.repo), \
                patch.object(regenerate, "run"), \
                patch.object(regenerate, "refresh_quest_callback_proof"), \
                patch.object(regenerate, "materialize_world"), \
                patch.object(regenerate, "refresh_source_cohort_extent", side_effect=measured), \
                patch.object(regenerate, "repin_rust_inventory") as repin, \
                patch.object(canonical.subprocess, "run", side_effect=self.native_output):
            with self.assertRaises(Completed):
                regenerate.main()
        repin.assert_not_called()

    def test_missing_extra_or_changed_unowned_output_refuses_every_replacement(self):
        def missing(output):
            (output / "project.json").unlink()

        def extra(output):
            (output / "unexpected.json").write_bytes(b"{}\n")

        def unowned(output):
            (output / "editor/author.json").write_bytes(b'{"drift":true}\n')

        for mutate in [missing, extra, unowned]:
            with self.subTest(mutation=mutate.__name__):
                def native(command, **kwargs):
                    result = self.native_output(command, **kwargs)
                    mutate(self.calls[-1])
                    return result

                with patch.object(canonical.subprocess, "run", side_effect=native):
                    with self.assertRaises(ValueError):
                        canonical.complete_world_project(self.repo, self.world)
                self.assertEqual(canonical.read_documents(self.world), self.original)

    def test_native_output_cannot_drop_an_authored_source(self):
        def native(command, **kwargs):
            result = self.native_output(command, **kwargs)
            path = self.calls[-1] / "provenance/sources.json"
            document = json.loads(path.read_bytes())
            document["sources"] = []
            path.write_bytes(encode(document))
            return result

        with patch.object(canonical.subprocess, "run", side_effect=native):
            with self.assertRaisesRegex(ValueError, "source cohort"):
                canonical.complete_world_project(self.repo, self.world)
        self.assertEqual(canonical.read_documents(self.world), self.original)

    def test_live_input_change_is_not_overwritten_by_completed_output(self):
        replacement = b'{"concurrent":true}\n'

        def native(command, **kwargs):
            result = self.native_output(command, **kwargs)
            (self.world / "project.json").write_bytes(replacement)
            return result

        with patch.object(canonical.subprocess, "run", side_effect=native):
            with self.assertRaisesRegex(ValueError, "input changed"):
                canonical.complete_world_project(self.repo, self.world)
        expected = dict(self.original, **{"project.json": replacement})
        self.assertEqual(canonical.read_documents(self.world), expected)

    def test_staging_does_not_turn_a_managed_symlink_into_admissible_input(self):
        document = self.world / "editor/author.json"
        outside = self.repo / "outside.json"
        outside.write_bytes(document.read_bytes())
        document.unlink()
        document.symlink_to(outside)
        with patch.object(canonical.subprocess, "run") as native:
            with self.assertRaisesRegex(ValueError, "symlink"):
                canonical.complete_world_project(self.repo, self.world)
        native.assert_not_called()


if __name__ == "__main__":
    unittest.main()
