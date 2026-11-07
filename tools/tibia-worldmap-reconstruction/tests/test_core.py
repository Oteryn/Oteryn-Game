from __future__ import annotations

import unittest

from tibia_worldmap_reconstruction.core import (
    ValidationError,
    build_otbm_export_plan,
    compare_documents,
    merge_documents,
    validate_document,
)


def content(stack_index, client_id, semantic_class, server_id=None, mapping_status=None):
    if mapping_status is None:
        mapping_status = "MAPPED" if server_id is not None else "UNMAPPED"
    return {
        "stack_index": stack_index,
        "client_appearance_id": client_id,
        "semantic_class": semantic_class,
        "server_otb_id": server_id,
        "mapping_status": mapping_status,
    }


def tile(x=100, y=200, z=7, *, observed=True, sequence=1, contents=None):
    return {
        "coordinate": {"x": x, "y": y, "z": z},
        "observed": observed,
        "sequence": sequence,
        "contents": list(contents or []),
    }


def document(*tiles, name="test", entities=None):
    return {
        "format": "oteryn-worldmap-normalized-v1",
        "source": {"name": name, "client_version": "synthetic"},
        "tiles": list(tiles),
        "entities": list(entities or []),
    }


class ValidationTests(unittest.TestCase):
    def test_valid_document(self):
        validate_document(document(tile(contents=[content(0, 10, "ground", 100)])))

    def test_invalid_semantic_class_fails(self):
        value = document(tile(contents=[content(0, 10, "guessed", 100)]))
        with self.assertRaises(ValidationError):
            validate_document(value)

    def test_mapped_requires_server_id(self):
        value = document(tile(contents=[content(0, 10, "ground", None, "MAPPED")]))
        with self.assertRaises(ValidationError):
            validate_document(value)

    def test_duplicate_coordinate_fails(self):
        value = document(tile(), tile())
        with self.assertRaises(ValidationError):
            validate_document(value)

    def test_boolean_coordinate_is_rejected(self):
        value = document(tile())
        value["tiles"][0]["coordinate"]["x"] = True
        with self.assertRaises(ValidationError):
            validate_document(value)

    def test_boolean_numeric_content_fields_are_rejected(self):
        fields = ("stack_index", "client_appearance_id", "server_otb_id")
        for field in fields:
            with self.subTest(field=field):
                value = document(tile(contents=[content(0, 10, "ground", 100)]))
                value["tiles"][0]["contents"][0][field] = True
                with self.assertRaises(ValidationError):
                    validate_document(value)


class ComparisonTests(unittest.TestCase):
    def setUp(self):
        self.base_contents = [
            content(0, 4407, "ground", 1000),
            content(1, 313, "ground_border", 1001),
            content(2, 6379, "static_item", 1002),
        ]

    def status(self, left_tile, right_tile):
        result = compare_documents(document(left_tile, name="capture"), document(right_tile, name="reference"))
        return result["tiles"][0]["status"]

    def test_match(self):
        self.assertEqual(self.status(tile(contents=self.base_contents), tile(contents=self.base_contents)), "MATCH")

    def test_ground_mismatch(self):
        changed = [content(0, 9999, "ground", 2000), *self.base_contents[1:]]
        self.assertEqual(self.status(tile(contents=self.base_contents), tile(contents=changed)), "GROUND_MISMATCH")

    def test_stack_order_mismatch(self):
        left = self.base_contents
        right = [left[0], content(1, 6379, "static_item", 1002), content(2, 313, "ground_border", 1001)]
        self.assertEqual(self.status(tile(contents=left), tile(contents=right)), "STACK_ORDER_MISMATCH")

    def test_item_mismatch(self):
        changed = [self.base_contents[0], self.base_contents[1], content(2, 8888, "static_item", 3000)]
        self.assertEqual(self.status(tile(contents=self.base_contents), tile(contents=changed)), "ITEM_MISMATCH")

    def test_unmapped_id(self):
        unmapped = [content(0, 4407, "ground", None, "UNMAPPED")]
        self.assertEqual(self.status(tile(contents=unmapped), tile(contents=self.base_contents)), "UNMAPPED_ID")

    def test_not_observed_is_not_empty(self):
        self.assertEqual(self.status(tile(observed=False), tile(contents=self.base_contents)), "NOT_OBSERVED")

    def test_missing_coordinate_statuses(self):
        result = compare_documents(
            document(tile(x=1, contents=self.base_contents), name="capture"),
            document(tile(x=2, contents=self.base_contents), name="reference"),
        )
        self.assertEqual([entry["status"] for entry in result["tiles"]], ["MISSING_IN_REFERENCE", "MISSING_IN_RECONSTRUCTION"])


class MergeTests(unittest.TestCase):
    def test_newer_sequence_replaces_and_records_history(self):
        old = document(tile(sequence=1, contents=[content(0, 10, "ground", 100)]), name="old")
        new = document(tile(sequence=2, contents=[content(0, 20, "ground", 200)]), name="new")
        merged = merge_documents(old, new)
        self.assertEqual(merged["tiles"][0]["sequence"], 2)
        self.assertEqual(merged["tiles"][0]["contents"][0]["server_otb_id"], 200)
        self.assertEqual(merged["merge_history"][0]["replaced_sequence"], 1)

    def test_same_sequence_conflict_fails(self):
        old = document(tile(sequence=1, contents=[content(0, 10, "ground", 100)]))
        conflict = document(tile(sequence=1, contents=[content(0, 20, "ground", 200)]))
        with self.assertRaises(ValidationError):
            merge_documents(old, conflict)

    def test_unobserved_update_does_not_erase_observed_tile(self):
        old = document(tile(sequence=1, contents=[content(0, 10, "ground", 100)]), name="old")
        unseen = document(tile(sequence=2, observed=False), name="unseen")
        merged = merge_documents(old, unseen)
        self.assertTrue(merged["tiles"][0]["observed"])
        self.assertEqual(merged["tiles"][0]["sequence"], 1)
        self.assertEqual(merged["tiles"][0]["contents"][0]["server_otb_id"], 100)

    def test_incoming_entities_are_rejected_until_merge_semantics_exist(self):
        with self.assertRaisesRegex(ValidationError, "entity merge semantics"):
            merge_documents(document(tile()), document(tile(x=101), entities=[{"kind": "npc"}]))


class ExportPlanTests(unittest.TestCase):
    def test_ready_only_when_ground_and_static_ids_are_mapped(self):
        ready = build_otbm_export_plan(document(tile(contents=[content(0, 10, "ground", 100), content(1, 11, "static_item", 101)])))
        self.assertTrue(ready["ready"])
        self.assertEqual(ready["tiles"][0]["ground_server_otb_id"], 100)
        self.assertEqual(ready["tiles"][0]["ordered_static_server_otb_ids"], [101])

    def test_unmapped_blocks_export(self):
        blocked = build_otbm_export_plan(document(tile(contents=[content(0, 10, "ground", None, "UNMAPPED")])))
        self.assertFalse(blocked["ready"])
        self.assertEqual(blocked["blockers"][0]["reason"], "UNMAPPED_ID")

    def test_missing_ground_blocks_export(self):
        blocked = build_otbm_export_plan(document(tile(contents=[content(0, 11, "static_item", 101)])))
        self.assertFalse(blocked["ready"])
        self.assertEqual(blocked["blockers"][0]["reason"], "GROUND_NOT_PROVEN")

    def test_unobserved_tiles_do_not_block_export_readiness(self):
        ready = build_otbm_export_plan(
            document(
                tile(contents=[content(0, 10, "ground", 100)]),
                tile(x=101, observed=False),
            )
        )
        self.assertTrue(ready["ready"])
        self.assertEqual(len(ready["tiles"]), 1)
        self.assertEqual(ready["blockers"], [])


if __name__ == "__main__":
    unittest.main()
