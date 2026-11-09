"""Historical evidence reproduction never bypasses current source/Native guards."""

import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import check_stack_default_historical_context as check
import historical_world_owner_context as world_context
import refresh_spkg2_qualification_receipts as refresh


class ContextTests(unittest.TestCase):
    def test_own_rebuild_preserves_frozen_facts_and_rejects_foreign_lineage(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            source = "docs/source-v1.json"
            raw = b'{"records":[{"id":7,"value":"frozen fact"}],"closed_scope":[7]}\n'
            (root / "docs").mkdir()
            (root / source).write_bytes(raw)
            compiler = root / refresh.HERE / "fixture.py"
            compiler.parent.mkdir(parents=True)
            compiler.write_text(
                'PROOF="docs/source-v1.json"\nPROOF_SHA = "' + "0" * 64 + '"\n'
            )
            recipe = ("fixture", "PROOF", source, hashlib.sha256(raw).hexdigest(), 4)
            args = {
                "generator": "tools/content-schema/item-authoring/refresh_srvenc_qualification_receipts.py",
                "require_new_or_identical": True,
                "regenerate_owned_successor": True,
            }
            with patch.object(
                refresh, "original_marker", return_value=b"checked marker"
            ):
                refresh.refresh(root, recipe, **args)
                dest = root / "docs/source-v4.json"
                original = json.loads(dest.read_bytes())
                # Model stale producer input pins and an altered mutable successor
                # fact: recomputation must retain the immutable source facts.
                changed = dict(
                    original,
                    records=[{"id": 7, "value": "tampered successor"}],
                    input_pins=[],
                )
                dest.write_text(json.dumps(changed))
                refresh.refresh(root, recipe, **args)
                rebuilt = json.loads(dest.read_bytes())
                self.assertEqual(rebuilt["records"], json.loads(raw)["records"])
                self.assertEqual(rebuilt["closed_scope"], [7])
                for field, value in [
                    ("generator", "foreign.py"),
                    ("supersedes", "foreign-v1.json"),
                    ("sha256", "0" * 64),
                ]:
                    with self.subTest(field=field):
                        foreign = json.loads(dest.read_bytes())
                        foreign["input_footprint_refresh"][field] = value
                        dest.write_text(json.dumps(foreign))
                        before = dest.read_bytes()
                        with self.assertRaisesRegex(ValueError, "already exists"):
                            refresh.refresh(root, recipe, **args)
                        self.assertEqual(dest.read_bytes(), before)
                        dest.write_text(json.dumps(rebuilt))
                self.assertEqual((root / source).read_bytes(), raw)

    def test_sugar_shard_without_qualified_marker_cannot_be_hidden(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            marker = root / world_context.MARKER
            marker.parent.mkdir(parents=True)
            marker.write_bytes(world_context.original_marker(check.ROOT))
            shard = marker.parent / world_context.SUGAR_GROUND
            shard.write_bytes(
                (
                    check.ROOT / "content/world/terrain" / world_context.SUGAR_GROUND
                ).read_bytes()
            )
            with self.assertRaisesRegex(ValueError, "source digest drift"):
                world_context.original_marker(root)

    def test_soulpit_owner_successor_recovers_the_exact_frozen_marker(self):
        original = world_context.original_marker(check.ROOT)
        self.assertEqual(world_context.sha(original), world_context.BASE_SHA)

    def test_soulpit_owner_successor_rejects_changed_catalogue_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in (
                world_context.MARKER,
                "content/world/terrain/" + world_context.GROUND,
                "content/world/terrain/" + world_context.SOULPIT,
                "content/world/terrain/" + world_context.SUGAR_GROUND,
                "content/world/terrain/" + world_context.NPCFIX_GROUND,
            ):
                target = root / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes((check.ROOT / relative).read_bytes())
            path = root / "content/world/terrain" / world_context.SOULPIT
            path.write_bytes(path.read_bytes() + b" ")
            with self.assertRaisesRegex(ValueError, "unrecognized Soulpit"):
                world_context.original_marker(root)

    def test_historical_owner_receipts_reject_unqualified_ground_changes(self):
        for relative in (
            world_context.MARKER,
            "content/world/terrain/" + world_context.GROUND,
            "content/world/terrain/" + world_context.SOULPIT,
            "content/world/terrain/" + world_context.SUGAR_GROUND,
            "content/world/terrain/" + world_context.NPCFIX_GROUND,
        ):
            with (
                self.subTest(relative=relative),
                tempfile.TemporaryDirectory() as directory,
            ):
                root = Path(directory)
                for source in (
                    world_context.MARKER,
                    "content/world/terrain/" + world_context.GROUND,
                    "content/world/terrain/" + world_context.SOULPIT,
                    "content/world/terrain/" + world_context.SUGAR_GROUND,
                    "content/world/terrain/" + world_context.NPCFIX_GROUND,
                ):
                    target = root / source
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes((check.ROOT / source).read_bytes())
                self.assertEqual(
                    world_context.sha(world_context.original_marker(root)),
                    world_context.BASE_SHA,
                )
                target = root / relative
                target.write_bytes(target.read_bytes() + b" ")
                with self.assertRaisesRegex(ValueError, "source digest drift"):
                    world_context.original_marker(root)

    def test_historical_hit_receipt_rejects_changed_facts(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in (
                world_context.MARKER,
                "content/world/terrain/" + world_context.GROUND,
                "content/world/terrain/" + world_context.SOULPIT,
                "content/world/terrain/" + world_context.SUGAR_GROUND,
                "content/world/terrain/" + world_context.NPCFIX_GROUND,
                world_context.HIT_PACKET,
            ):
                target = root / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes((check.ROOT / relative).read_bytes())
            self.assertEqual(
                world_context.sha(world_context.original_hit_packet(root)),
                world_context.HIT_SHA,
            )
            target = root / world_context.HIT_PACKET
            packet = json.loads(target.read_bytes())
            packet["counts"]["fields"] += 1
            target.write_bytes(check.canonical(packet))
            with self.assertRaisesRegex(ValueError, "source digest drift"):
                world_context.original_hit_packet(root)

    def test_receipt_refresh_rejects_modified_frozen_qualification(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            recipe = refresh.RECIPES[0]
            target = root / recipe[2]
            target.parent.mkdir(parents=True)
            target.write_bytes((check.ROOT / recipe[2]).read_bytes() + b" ")
            with self.assertRaisesRegex(ValueError, "frozen qualification changed"):
                refresh.refresh(root, recipe)

    def test_packets_are_byte_exact_and_actual_current_cohort_stays_closed(self):
        result = check.check()
        self.assertEqual(result["strict_current_original_defaults"], 1487)
        self.assertEqual(result["strict_current_original_historical"], 7)
        self.assertEqual(result["outside_cohort_promotions_written"], 0)

    def test_frozen_context_byte_substitution_is_rejected(self):
        with tempfile.TemporaryDirectory(
            prefix="negative-frozen-context-"
        ) as directory:
            root = Path(directory)
            path = root / check.CONTEXT
            path.parent.mkdir(parents=True)
            path.write_bytes((check.ROOT / check.CONTEXT).read_bytes() + b" ")
            with self.assertRaisesRegex(ValueError, "source digest drift"):
                check.read_context(root)

    def test_original_proof_source_bytes_cannot_be_substituted(self):
        with tempfile.TemporaryDirectory(prefix="negative-source-bytes-") as directory:
            root = Path(directory)
            for path in (check.CONTEXT, check.base.PROOF):
                target = root / path
                target.parent.mkdir(parents=True, exist_ok=True)
                data = (check.ROOT / path).read_bytes()
                target.write_bytes(data + (b" " if path == check.base.PROOF else b""))
            with self.assertRaisesRegex(ValueError, "source digest drift"):
                check.read_context(root)

    def test_actual_current_name_conflict_rejects_the_original_cohort(self):
        source, _, _, _, _, current, _ = check.current_inputs(check.ROOT)
        packet = json.loads(check.base.OUTPUT.read_text())
        promoted = packet["promotions"][0]["item_key"]
        # Negative-only isolated fixture starts with actual current source-scope definitions.
        definitions = [copy.deepcopy(current[s["item_key"]]) for s in source["records"]]
        victim = next(d for d in definitions if d["identity"]["key"] == promoted)
        victim["semantics"]["presentation"] = {
            "state": "KNOWN",
            "value": {
                "name": {
                    "state": "KNOWN",
                    "value": "deliberately opposing current name",
                }
            },
        }
        with tempfile.TemporaryDirectory(
            prefix="negative-current-name-fixture-"
        ) as directory:
            negative_root = Path(directory)
            for name in ("docs", "tools", "imports"):
                (negative_root / name).symlink_to(
                    check.ROOT / name, target_is_directory=True
                )
            (negative_root / "content/items").mkdir(parents=True)
            for name in ("assets", "world"):
                (negative_root / "content" / name).symlink_to(
                    check.ROOT / "content" / name, target_is_directory=True
                )
            path = "content/items/negative-current-fixture-definitions.json"
            (negative_root / "content/items/index.json").write_bytes(
                check.canonical(
                    {"purpose": "NEGATIVE_TEST_ONLY_NOT_AUTHORITY", "shards": [path]}
                )
            )
            shard = negative_root / path
            shard.write_bytes(
                check.canonical({"records": [{"definition": d} for d in definitions]})
            )
            with self.assertRaisesRegex(ValueError, "KNOWN_PRESENTATION_NAME_CONFLICT"):
                check.current_validation(negative_root)
        self.assertNotEqual(
            check.current_inputs(check.ROOT)[5][promoted]["semantics"].get(
                "presentation"
            ),
            victim["semantics"]["presentation"],
        )


if __name__ == "__main__":
    unittest.main()
