#!/usr/bin/env python3
"""Captured Global client weights survive legacy regeneration (ITEMGLOB)."""

import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import import_cyclopedia_global as cyclopedia


def definition(key, weight, state="KNOWN"):
    value = (
        {"weight": {"state": "KNOWN", "value": weight}} if state == "KNOWN" else None
    )
    physical = {"state": state, "value": value} if value else {"state": state}
    return {
        "identity": {"family": "Item", "key": key},
        "semantics": {"physical": physical},
    }


class ClientWeightAdmission(unittest.TestCase):
    def test_capture_weights_match_client(self):
        weights = cyclopedia.load_weights(ROOT)
        for identifier, expected in {
            3031: 10,
            3035: 10,
            3043: 10,
            9147: 100,
            19371: 1700,
            25729: 200,
            25730: 200,
        }.items():
            with self.subTest(identifier=identifier):
                self.assertEqual(weights[identifier], expected)
        self.assertEqual(weights[7439], 290)
        self.assertEqual(weights[7443], 290)

    def test_legacy_weight_is_replaced_and_unknown_is_left(self):
        records = [
            definition("oteryn:item.tibia.i7439", 200),
            definition("oteryn:item.tibia.i7443", 200, state="UNKNOWN"),
            definition("oteryn:item.tibia.i999999999", 7),
        ]
        cyclopedia.apply_admissions(records, ROOT)
        self.assertEqual(
            records[0]["semantics"]["physical"]["value"]["weight"],
            {"state": "KNOWN", "value": 290},
        )
        self.assertEqual(records[1]["semantics"]["physical"], {"state": "UNKNOWN"})
        self.assertEqual(
            records[2]["semantics"]["physical"]["value"]["weight"]["value"], 7
        )

    def test_only_weight_changed_accepts_weight_leaf_alone(self):
        before = definition("oteryn:item.tibia.i7439", 200)
        after = definition("oteryn:item.tibia.i7439", 290)
        self.assertTrue(cyclopedia.only_weight_changed(before, after))
        after["identity"]["key"] = "other"
        self.assertFalse(cyclopedia.only_weight_changed(before, after))
        self.assertFalse(
            cyclopedia.only_weight_changed(
                definition("k", 1, state="UNKNOWN"), definition("k", 2)
            )
        )

    def test_generated_tree_carries_client_weights(self):
        weights = cyclopedia.load_weights(ROOT)
        stale = []
        for path in sorted((ROOT / "content/items/definitions").glob("items-*.json")):
            for row in json.loads(path.read_bytes())["records"]:
                probe = json.loads(json.dumps(row["definition"]))
                if cyclopedia.apply_weight(probe, weights):
                    stale.append(row["definition"]["identity"]["key"])
        self.assertEqual(stale, [])

    def test_generated_native_profiles_carry_client_weights(self):
        weights = cyclopedia.load_weights(ROOT)
        document = json.loads(
            (ROOT / "content/items/definitions/spell-native-profiles.json").read_bytes()
        )
        stale = [
            identifier
            for identifier, weight in cyclopedia._overlay_profiles(document["records"])
            if identifier in weights and weights[identifier] != weight["value"]
        ]
        self.assertEqual(stale, [])

    def test_overlay_refresh_preserves_donor_captures(self):
        record = {
            "production_definition": {"production_key": "oteryn:item.tibia.i3031"},
            "semantics": {
                "physical": {
                    "state": "KNOWN",
                    "value": {"weight": {"state": "KNOWN", "value": 1}},
                }
            },
        }
        raw = (json.dumps({"records": [record]}, indent=2) + "\n").encode()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            overlays = root / cyclopedia.OVERLAY_DIR
            overlays.mkdir(parents=True)
            captures = [
                overlays / name
                for name in (
                    "boss-reward-item-profiles.json",
                    "hireling-item-profiles.json",
                    "soulpit-echo-item-profiles.json",
                    "writing-item-profiles.json",
                )
            ]
            coins = overlays / "coin-item-profiles.json"
            for capture in captures:
                capture.write_bytes(raw)
            coins.write_bytes(raw)
            self.assertEqual(
                cyclopedia.apply_overlay_weights(root, {3031: 10}),
                {"coin-item-profiles.json": 1},
            )
            for capture in captures:
                self.assertEqual(capture.read_bytes(), raw)
            self.assertEqual(
                json.loads(coins.read_bytes())["records"][0]["semantics"]["physical"][
                    "value"
                ]["weight"]["value"],
                10,
            )

    def test_profile_weights_replace_known_weight_only(self):
        record = {
            "production_definition": {"production_key": "oteryn:item.tibia.i7439"},
            "semantics": {
                "physical": {
                    "state": "KNOWN",
                    "value": {
                        "weight": {"state": "KNOWN", "value": 200},
                        "movable": {"state": "KNOWN", "value": True},
                    },
                }
            },
        }
        cyclopedia.apply_profile_weights([record], ROOT)
        self.assertEqual(
            record["semantics"]["physical"]["value"],
            {
                "weight": {"state": "KNOWN", "value": 290},
                "movable": {"state": "KNOWN", "value": True},
            },
        )


class SharedDetailsGeneration(unittest.TestCase):
    def test_shared_item_rebuild_replays_authoring_and_relations(self):
        from unittest.mock import patch

        import world_project_v2_to_tree as tree

        with (
            patch.object(tree, "write") as write,
            patch.object(tree, "write_registry"),
            patch.object(Path, "write_bytes"),
            patch.object(Path, "unlink"),
        ):
            self.assertEqual(tree.main(items_only=True), 0)
        outputs = {call.args[0]: call.args[1] for call in write.call_args_list}
        armor = next(
            row
            for shard in outputs["content/items/index.json"]["shards"]
            for row in outputs[shard]["records"]
            if row["definition"]["identity"]["key"] == "oteryn:item.tibia.i3361"
        )
        self.assertEqual(
            armor["authoring"]["forge"], {"classification": 1, "max_tier": 1}
        )
        for name in ("taxonomy", "relations"):
            relative = f"content/items/{name}/items.json"
            self.assertEqual(
                outputs[relative], json.loads((ROOT / relative).read_bytes())
            )

    def test_no_unqualified_fluid_quotes_in_trade_projection(self):
        ahmet = next(
            row["declaration"]
            for shard in sorted((ROOT / "content/services/trade").glob("trade-*.json"))
            for row in json.loads(shard.read_bytes())["records"]
            if row["declaration"]["identity"]["key"] == "oteryn:service.trade.ahmet"
        )
        self.assertFalse(
            any(
                row["item"]["key"] == "oteryn:item.tibia.i2874"
                and row.get("sub_type") is None
                for row in ahmet["offers"]
            )
        )

    def test_baltim_wrap_quote_has_exact_full_charge_evidence(self):
        baltim = next(
            row["declaration"]
            for shard in sorted((ROOT / "content/services/trade").glob("trade-*.json"))
            for row in json.loads(shard.read_bytes())["records"]
            if row["declaration"]["identity"]["key"] == "oteryn:service.trade.baltim"
        )
        wraps = [
            row
            for row in baltim["offers"]
            if row["item"]["key"] == "oteryn:item.tibia.i50293"
        ]
        self.assertEqual(
            [(r.get("count"), r.get("sub_type"), r["unit_price"]) for r in wraps],
            [(1, 500, 434028)],
        )


if __name__ == "__main__":
    unittest.main()
