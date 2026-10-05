"""Retained replay cannot silently replace live inputs or alter the product tree."""

import fnmatch
import json
import tempfile
import unittest
from pathlib import Path

import check_tibiawiki165_historical_context as existing
import replay_closed_item_context as replay
from lower_elemental_magic_modifier_packet import native_order


class ClosedContextTests(unittest.TestCase):
    def test_workflow_selects_every_pinned_current_and_retained_input(self):
        document = json.loads(
            (replay.ROOT / replay.CONTEXT / "context.json").read_bytes()
        )
        inputs = set(document["current_world_inputs"]) | set(
            document["retained_inputs"]
        )
        inputs |= {
            replay.CONTEXT + "/context.json",
            replay.CONTEXT + "/retained-inputs.zip",
            replay.TOOL + "/replay_closed_item_context.py",
        }
        lines = (
            (replay.ROOT / ".github/workflows/item-authoring-schema.yml")
            .read_text()
            .splitlines()
        )
        start = lines.index("    paths:") + 1
        patterns = []
        for line in lines[start:]:
            if not line.startswith("      - "):
                break
            patterns.append(line.removeprefix("      - ").strip("'\""))
        missing = sorted(
            p
            for p in inputs
            if not any(fnmatch.fnmatchcase(p, pattern) for pattern in patterns)
        )
        self.assertEqual(
            missing, [], f"pinned inputs cannot skip qualification: {missing}"
        )

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

    def test_nested_existing_replay_keeps_census_source_witness(self):
        with (
            replay.qualification_context() as root,
            existing.historical_context(root, weapon=True) as nested,
        ):
            witness = "tools/content-census/item_wiki_family_capture.py"
            self.assertEqual(
                (nested / witness).read_bytes(),
                (replay.ROOT / witness).read_bytes(),
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
