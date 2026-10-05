"""Retained replay cannot silently replace live inputs or alter the product tree."""

import json
import tempfile
import unittest
from pathlib import Path

import replay_closed_item_context as replay
from lower_elemental_magic_modifier_packet import native_order


class ClosedContextTests(unittest.TestCase):
    def test_actual_context_preserves_current_items_bindings_owners_and_tools(self):
        before = (replay.ROOT / replay.TOOL / "item.schema.json").read_bytes()
        with replay.qualification_context() as root:
            for name in (
                "content/items",
                "imports/tibiawiki/bindings/items.json",
                "content/world/definitions/declarations.json",
                "content/world/provenance/sources.json",
            ):
                self.assertEqual(
                    (root / name).resolve(), (replay.ROOT / name).resolve()
                )
            packet = json.loads(
                (
                    root
                    / "docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json"
                ).read_bytes()
            )
            for path, digest in packet["source"]["map_owner_inputs"].items():
                self.assertEqual(replay.sha((root / path).read_bytes()), digest)
            self.assertEqual(native_order(root), native_order(replay.ROOT))
            self.assertEqual(len(native_order(root)), 37)
            (root / replay.TOOL / "item.schema.json").write_bytes(b"test sentinel")
        self.assertEqual(
            (replay.ROOT / replay.TOOL / "item.schema.json").read_bytes(), before
        )

    def assert_substitution_rejected(self, name, data, message):
        with tempfile.TemporaryDirectory(prefix="negative-item-replay-") as directory:
            root = Path(directory)
            replay.mirror(replay.ROOT, root, {name: data})
            with self.assertRaisesRegex(ValueError, message):
                replay.checked_inputs(root)

    def test_current_world_manifest_substitution_is_rejected(self):
        path = "content/world/objects/index.json"
        document = json.loads((replay.ROOT / path).read_bytes())
        document["deliberate_invalid_manifest"] = True
        self.assert_substitution_rejected(
            path,
            json.dumps(document).encode(),
            "current World qualification inputs drift",
        )

    def test_current_runtime_source_substitution_is_rejected(self):
        self.assert_substitution_rejected(
            replay.RUST,
            (replay.ROOT / replay.RUST).read_bytes() + b"\n",
            "closed/current input witness drift",
        )

    def test_historical_archive_and_context_substitutions_are_rejected(self):
        for name, message in (
            ("retained-inputs.zip", "archive digest drift"),
            ("context.json", "context digest drift"),
        ):
            path = replay.CONTEXT + "/" + name
            self.assert_substitution_rejected(
                path, (replay.ROOT / path).read_bytes() + b" ", message
            )


if __name__ == "__main__":
    unittest.main()
