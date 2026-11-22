"""World qualification executables retain their exact source and build binding."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import prebuilt


class WorldPrebuilt(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.output = self.root / "bundle"
        self.output.mkdir()
        self.identity = {"head": "1" * 40, "workspace": str(self.root),
                         "variant": prebuilt.VARIANT, "toolchain": "1.100.0",
                         "profile": "dev"}
        self.manifest = dict(self.identity, executables={})
        for name, target in prebuilt.TARGETS.items():
            executable = self.output / target["name"]
            executable.write_bytes(b"#!/bin/sh\nexit 0\n")
            executable.chmod(0o700)
            self.manifest["executables"][name] = dict(
                target, sha256=hashlib.sha256(executable.read_bytes()).hexdigest())
        self.path = self.output / "manifest.json"
        self.save(self.manifest)
        self.binding = patch.object(prebuilt, "identity", return_value=self.identity)
        self.binding.start()
        self.addCleanup(self.binding.stop)

    def save(self, manifest):
        self.path.write_text(json.dumps(manifest))

    def test_accepts_only_the_bound_executable_inventory(self):
        for name, target in prebuilt.TARGETS.items():
            self.assertEqual(prebuilt.executable(name, self.path, self.root),
                             self.output / target["name"])

    def test_wrong_source_profile_features_or_inventory_is_refused(self):
        def different_head(value): value["head"] = "2" * 40
        def different_workspace(value): value["workspace"] += "/other"
        def release(value): value["profile"] = "release"
        def features(value): value["executables"]["canonicalizer"]["features"] = []
        def extra(value): value["executables"]["unexpected"] = {}
        def missing(value): del value["executables"]["materializer"]
        for mutate in (different_head, different_workspace, release, features, extra, missing):
            with self.subTest(mutation=mutate.__name__):
                value = copy.deepcopy(self.manifest)
                mutate(value)
                self.save(value)
                with self.assertRaises(ValueError):
                    prebuilt.executable("canonicalizer", self.path, self.root)

    def test_changed_bytes_and_symlink_cannot_borrow_a_valid_manifest(self):
        path = self.output / prebuilt.TARGETS["canonicalizer"]["name"]
        path.write_bytes(b"#!/bin/sh\nexit 9\n")
        with self.assertRaisesRegex(ValueError, "checksum"):
            prebuilt.executable("canonicalizer", self.path, self.root)
        path.unlink()
        path.symlink_to(self.output / prebuilt.TARGETS["materializer"]["name"])
        with self.assertRaisesRegex(ValueError, "canonical"):
            prebuilt.executable("canonicalizer", self.path, self.root)

    def cargo_messages(self):
        return [dict(reason="compiler-artifact", executable=str(self.output / target["name"]),
                     target={"name": target["name"], "kind": [target["kind"]]},
                     features=target["features"], profile={"test": False})
                for target in prebuilt.TARGETS.values()]

    def test_single_build_collects_both_compatible_targets(self):
        destination = self.root / "built"
        messages = "\n".join(json.dumps(message) for message in self.cargo_messages())
        with patch.object(prebuilt.subprocess, "check_output", return_value=messages) as cargo:
            prebuilt.build(destination, self.root)
        cargo.assert_called_once()
        command = cargo.call_args.args[0]
        self.assertIn("--example", command)
        self.assertIn("--bin", command)
        self.assertIn("oteryn-gameplay-content/test-support", command)
        self.assertTrue(prebuilt.executable("materializer", destination / "manifest.json", self.root).is_file())

    def test_incomplete_duplicate_or_wrong_feature_build_cannot_publish(self):
        valid = self.cargo_messages()
        wrong = copy.deepcopy(valid)
        wrong[0]["features"] = []
        for messages in (valid[:-1], valid + valid[:1], wrong):
            with self.subTest(messages=messages):
                raw = "\n".join(json.dumps(message) for message in messages)
                with patch.object(prebuilt.subprocess, "check_output", return_value=raw):
                    with self.assertRaises(ValueError):
                        prebuilt.build(self.root / "refused", self.root)
                self.assertFalse((self.root / "refused/manifest.json").exists())

    def test_source_move_during_build_cannot_issue_a_manifest(self):
        output = "\n".join(json.dumps(message) for message in self.cargo_messages())
        moved = dict(self.identity, head="2" * 40)
        with patch.object(prebuilt.subprocess, "check_output", return_value=output), \
                patch.object(prebuilt, "identity", side_effect=[self.identity, moved]):
            with self.assertRaisesRegex(ValueError, "source changed"):
                prebuilt.build(self.root / "refused", self.root)
        self.assertFalse((self.root / "refused/manifest.json").exists())


if __name__ == "__main__":
    unittest.main()
