"""Closed provenance/name override and complete raw own-ID witness guards."""

import copy
import unittest

import lower_item_name_packet as names


class Names(unittest.TestCase):
    def args(self):
        target = {"family": "Item", "key": "item", "revision": "definition-r1"}
        return [
            {
                "target": target,
                "official_name": "bow",
                "facts": {"name": "bow"},
                "imported_name_proof": {
                    "typed_value": {"kind": "TEXT", "value": "weapon of carving"}
                },
            },
            {
                "identity": target,
                "semantics": {
                    "presentation": {
                        "state": "KNOWN",
                        "value": {
                            "name": {"state": "KNOWN", "value": "weapon of carving"}
                        },
                    }
                },
            },
            [],
        ]

    def test_closed_import_and_corrected_idempotence(self):
        args = self.args()
        row = names.qualify(*args)
        self.assertEqual(row["previous_imported_name"], "weapon of carving")
        args[1]["semantics"]["presentation"]["value"]["name"] = {
            "state": "KNOWN",
            "value": "bow",
        }
        self.assertEqual(names.qualify(*args), row)
        for state in [
            {"state": "UNKNOWN"},
            {"state": "CONFLICT"},
            {"state": "NOT_APPLICABLE"},
            {"state": "KNOWN", "value": "custom Game name"},
        ]:
            args = self.args()
            args[1]["semantics"]["presentation"]["value"]["name"] = state
            with self.assertRaisesRegex(ValueError, "Known-name"):
                names.qualify(*args)

    def test_event_literal_is_per_target_not_global_generic_exception(self):
        args = self.args()
        args[0]["imported_name_proof"]["typed_value"]["value"] = "event item"
        args[1]["semantics"]["presentation"]["value"]["name"]["value"] = "event item"
        self.assertEqual(names.qualify(*args)["previous_imported_name"], "event item")
        args[1]["semantics"]["presentation"]["value"]["name"]["value"] = (
            "weapon of carving"
        )
        with self.assertRaisesRegex(ValueError, "Known-name"):
            names.qualify(*args)

    def test_game_owned_duplicate_and_full_owner_identity(self):
        for owner in [
            {"item": self.args()[0]["target"], "presentation": {"key": "GameOwned"}},
            {"item": self.args()[0]["target"] | {"revision": "other"}},
        ]:
            args = self.args()
            args[2] = [owner]
            with self.assertRaisesRegex(ValueError, "GameOwned"):
                names.qualify(*args)
        args = self.args()
        args[2] = [{"item": args[0]["target"]}] * 2
        with self.assertRaises(ValueError):
            names.qualify(*args)

    def witness(self):
        raw = "{{Infobox Object|itemid=7|name=Bow (Charged)|actualname=bow}}"
        page = {
            "revision_id": 1,
            "revision_timestamp": "2026-09-01T00:00:00Z",
            "content_sha256": "declared article digest",
            "source_part": "part",
            "source_part_sha256": "part sha",
            "source_page_ordinal": 0,
            "own_objects": [{"box_index": 0, "raw_itemid_values": ["7"]}],
        }
        witness = {k: v for k, v in page.items() if k != "own_objects"}
        witness.update(
            page_id=1,
            box_index=0,
            raw_infobox=raw,
            raw_infobox_sha256=names.base.sha(raw.encode()),
            balanced=True,
            inside_comment=False,
            positive_exact_infobox_object_match=True,
        )
        return [
            {"source_item_id": 7, "official_name": "bow", "sources": [witness]},
            {7: {1}},
            {1: page},
            "2026-09-27T23:59:59Z",
        ]

    def test_raw_units_identity_opposition_coordinate_and_name(self):
        self.assertEqual(
            names.witness_params(*self.witness())[0]["actualname"], ["bow"]
        )
        for change in [
            "shared",
            "empty",
            "duplicate",
            "malformed",
            "different name",
            "opposition",
            "rawdigest",
            "aftercut",
        ]:
            args = self.witness()
            w = args[0]["sources"][0]
            replacements = {
                "shared": ("itemid=7", "itemid=7, 8"),
                "empty": ("itemid=7", "itemid="),
                "duplicate": ("actualname=bow", "actualname=bow|actualname=bow"),
                "malformed": ("itemid=7", "itemid=7x"),
                "different name": ("actualname=bow", "actualname=other"),
            }
            if change in replacements:
                w["raw_infobox"] = w["raw_infobox"].replace(*replacements[change])
                w["raw_infobox_sha256"] = names.base.sha(w["raw_infobox"].encode())
            elif change == "opposition":
                args[1][7].add(2)
                args[2][2] = copy.deepcopy(args[2][1])
            elif change == "rawdigest":
                w["raw_infobox_sha256"] = "0" * 64
            else:
                w["revision_timestamp"] = args[2][1]["revision_timestamp"] = (
                    "2026-10-01T00:00:00Z"
                )
            with self.subTest(change=change), self.assertRaises(ValueError):
                names.witness_params(*args)


if __name__ == "__main__":
    unittest.main()
