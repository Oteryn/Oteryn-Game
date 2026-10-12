#!/usr/bin/env python3
"""Tests stage_npc_data.py on a synthetic project, and the committed pin against the repository.

usage: python3 deploy/synology-game/test_stage_npc_data.py
"""
import hashlib
import json
import pathlib
import re
import shutil
import struct
import tempfile
import unittest

import stage_npc_data as stage

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[1]
REVISION = "rev-test-1"


def sha(data):
    return hashlib.sha256(data).hexdigest()


class Fixture:
    """A repository root with a minimal WorldProject and a three-directory catalogue."""

    def __init__(self, root):
        self.root = pathlib.Path(root)
        self.content = self.root / "content"
        world = self.content / "world"
        documents = {"definitions/declarations.json": b'{"d":1}', "worlds/world.json": b'{"w":1}'}
        for locator, data in documents.items():
            self.put(world / locator, data)
        manifest = json.dumps({
            "schema": stage.MANIFEST_SCHEMA,
            "documents": [
                {"locator": locator, "byte_length": len(data), "sha256": sha(data)}
                for locator, data in documents.items()
            ],
        }).encode()
        lock = b'{"lock":1}'
        project = json.dumps({
            "schema": stage.PROJECT_SCHEMA,
            "project_revision": REVISION,
            "manifest_locator": "manifest.json",
            "manifest_sha256": sha(manifest),
            "content_lock_locator": "content.lock.json",
            "content_lock_sha256": sha(lock),
        }).encode()
        self.put(world / "manifest.json", manifest)
        self.put(world / "content.lock.json", lock)
        self.put(world / "project.json", project)
        self.put(world / "terrain/ignored.json", b"not part of the staged project")
        self.put(self.content / "npcs/definitions/npcs-0.json", b"npcs")
        self.put(self.content / "npcs/index.json", b"not in the catalogue")
        self.put(self.content / "dialogues/definitions/dialogues-0.json", b"dialogues")
        self.put(self.content / "services/trade/trade-0.json", b"trade")
        self.put(self.content / "services/index.json", b"services")
        self.pin_path = self.root / "pin.json"
        self.write_pin(catalogue=stage.catalogue_digest(stage.collect_catalogue(self.content)))

    @staticmethod
    def put(path, data):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)

    def write_pin(self, **override):
        pin = {
            "schema": stage.PIN_SCHEMA,
            "project_revision": REVISION,
            "project_sha256": "ab" * 32,
            "catalogue_sha256": override.pop("catalogue", None),
        }
        pin.update(override)
        self.pin_path.write_text(json.dumps(pin))

    def run(self):
        out = self.root / "out"
        stage.stage(self.pin_path, self.root, out)
        return out


class StageTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.tmp)
        self.fx = Fixture(self.tmp)

    def test_stages_exactly_the_project_documents_and_the_catalogue(self):
        out = self.fx.run()
        staged = sorted(p.relative_to(out).as_posix() for p in out.rglob("*") if p.is_file())
        self.assertEqual(staged, [
            "PROJECT_SHA256",
            "content/dialogues/definitions/dialogues-0.json",
            "content/npcs/definitions/npcs-0.json",
            "content/services/index.json",
            "content/services/trade/trade-0.json",
            "content/world/content.lock.json",
            "content/world/definitions/declarations.json",
            "content/world/manifest.json",
            "content/world/project.json",
            "content/world/worlds/world.json",
        ])
        self.assertEqual((out / "PROJECT_SHA256").read_text(), "ab" * 32 + "\n")

    def test_staged_catalogue_hashes_to_the_pin_as_the_node_does(self):
        out = self.fx.run()
        again = stage.catalogue_digest(stage.collect_catalogue(out / "content"))
        self.assertEqual(again, json.loads(self.fx.pin_path.read_text())["catalogue_sha256"])

    def test_digest_layout_is_the_compiler_record_format(self):
        expected = hashlib.sha256()
        for path, data in (("a/b", b"xy"), ("a/c", b"")):
            expected.update(struct.pack(">Q", len(path)) + path.encode())
            expected.update(struct.pack(">Q", len(data)) + data)
        self.assertEqual(stage.catalogue_digest([("a/c", b""), ("a/b", b"xy")]), expected.hexdigest())

    def refused(self):
        with self.assertRaises(ValueError):
            self.fx.run()

    def test_refuses_a_changed_catalogue_file(self):
        (self.fx.content / "npcs/definitions/npcs-0.json").write_bytes(b"changed")
        self.refused()

    def test_refuses_an_added_catalogue_file(self):
        Fixture.put(self.fx.content / "services/extra.json", b"x")
        self.refused()

    def test_refuses_a_missing_catalogue_file(self):
        (self.fx.content / "dialogues/definitions/dialogues-0.json").unlink()
        self.refused()

    def test_refuses_a_changed_project_document(self):
        (self.fx.content / "world/definitions/declarations.json").write_bytes(b'{"d":2}')
        self.refused()

    def test_refuses_a_changed_manifest_or_lock(self):
        (self.fx.content / "world/content.lock.json").write_bytes(b'{"lock":2}')
        self.refused()

    def test_refuses_another_project_revision(self):
        self.fx.write_pin(catalogue=stage.catalogue_digest(stage.collect_catalogue(self.fx.content)),
                          project_revision="other")
        self.refused()

    def test_refuses_a_malformed_pin(self):
        for key, value in (("project_sha256", "AB" * 32), ("catalogue_sha256", "ab" * 31), ("schema", "x")):
            self.fx.write_pin(catalogue=stage.catalogue_digest(stage.collect_catalogue(self.fx.content)),
                              **{key: value})
            self.refused()

    def test_refuses_a_symlinked_catalogue_file(self):
        link = self.fx.content / "services/link.json"
        link.symlink_to(self.fx.content / "services/index.json")
        self.refused()

    def test_refuses_an_unsafe_manifest_locator(self):
        for bad in ("../x", "/abs", "a//b", "a\\b", "./a", ""):
            with self.assertRaises(ValueError):
                stage.safe_locator(bad)

    def test_refuses_an_existing_destination(self):
        self.fx.run()
        with self.assertRaises(FileExistsError):
            self.fx.run()


class PinTests(unittest.TestCase):
    """The committed pin against the repository, so a content or World change cannot leave it stale."""

    def setUp(self):
        self.pin = stage.load_pin(HERE / "npc-data.pin.json")

    def test_project_pin_is_the_value_the_server_test_measures(self):
        source = REPO / "apps/game-server/tests/content_npc_catalogue.rs"
        if not source.is_file():
            self.skipTest("not a full checkout")
        text = source.read_text()
        self.assertIn(f'"{self.pin["project_sha256"]}"', text)
        self.assertIn(f'"{self.pin["project_revision"]}"', text)

    def test_revision_is_the_world_pin_content_revision(self):
        source = REPO / "content/world/pins/oteryn.json"
        if not source.is_file():
            self.skipTest("not a full checkout")
        self.assertEqual(json.loads(source.read_text())["content_revision"], self.pin["project_revision"])

    def test_catalogue_pin_is_the_repository_catalogue(self):
        content = REPO / "content"
        if not (content / "npcs/definitions").is_dir():
            self.skipTest("not a full checkout")
        got = stage.catalogue_digest(stage.collect_catalogue(content))
        self.assertEqual(got, self.pin["catalogue_sha256"], "refresh deploy/synology-game/npc-data.pin.json")

    def test_stages_the_repository_project(self):
        if not (REPO / "content/world/project.json").is_file():
            self.skipTest("not a full checkout")
        with tempfile.TemporaryDirectory() as tmp:
            out = pathlib.Path(tmp) / "npc-data"
            stage.stage(HERE / "npc-data.pin.json", REPO, out)
            self.assertTrue(re.fullmatch(r"[0-9a-f]{64}\n", (out / "PROJECT_SHA256").read_text()))
            self.assertTrue((out / "content/world/project.json").is_file())


if __name__ == "__main__":
    unittest.main()
