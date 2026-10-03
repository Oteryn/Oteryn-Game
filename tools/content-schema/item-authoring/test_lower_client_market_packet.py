"""Market evidence requires an affirmative independent flag and safe identity."""

import copy
import unittest

import lower_client_market_packet as market


class MarketSources(unittest.TestCase):
    def setUp(self):
        self.identity = {"family": "Item", "key": "item", "revision": "definition-r1"}
        self.definition = {"identity": self.identity, "semantics": {}}
        self.binding = {"target": self.identity}
        self.obj = {
            "name": "thing",
            "flags": {"flags.market": True, "flags.take": True},
        }

    def qualified(self, **changes):
        args = {
            "definition": self.definition,
            "binding": self.binding,
            "obj": self.obj,
            "routed": set(),
            "absent": {"item"},
            "witness_reasons": [],
            "retained": [],
        }
        args.update(changes)
        return market.qualify(**args)

    def test_positive_metadata_and_idempotence(self):
        self.assertEqual(self.qualified(), [])
        self.definition["semantics"]["trade_restrictions"] = {
            "state": "KNOWN",
            "value": {"marketable": {"state": "KNOWN", "value": True}},
        }
        self.assertEqual(self.qualified(), [])

    def test_each_current_source_guard_holds(self):
        for flag in ("flags.market", "flags.take"):
            obj = copy.deepcopy(self.obj)
            del obj["flags"][flag]
            self.assertTrue(self.qualified(obj=obj))
        for flag in (*market.WORLD_FLAGS, "flags.unmove"):
            obj = copy.deepcopy(self.obj)
            obj["flags"][flag] = True
            self.assertTrue(self.qualified(obj=obj))
        for changes in (
            {"binding": None},
            {"binding": {"target": self.identity | {"revision": "other"}}},
            {"routed": {"item"}},
            {"absent": set()},
            {"retained": [{"fields": {"marketable": "no"}}]},
            {"witness_reasons": ["opposed"]},
        ):
            self.assertTrue(self.qualified(**changes))
        self.definition["semantics"]["presentation"] = {
            "state": "KNOWN",
            "value": {"name": {"state": "KNOWN", "value": "other"}},
        }
        self.assertTrue(self.qualified())

    def test_every_native_group_and_leaf_block_holds(self):
        for group in (
            {"state": "CONFLICT"},
            {"state": "NOT_APPLICABLE"},
            *(
                {"state": "KNOWN", "value": {"marketable": value}}
                for value in (
                    {"state": "CONFLICT"},
                    {"state": "NOT_APPLICABLE"},
                    {"state": "KNOWN", "value": False},
                )
            ),
        ):
            self.definition["semantics"]["trade_restrictions"] = group
            self.assertTrue(self.qualified())

    def test_own_template_opposition_and_source_integrity(self):
        def witness(value="yes", ids="7"):
            raw = "{{Infobox Object|itemid=" + ids + "|marketable=" + value + "}}"
            return {
                "own_object_raw": raw,
                "raw_sha256": market.sha(raw.encode()),
                "source_item_ids": [7],
                "revision_timestamp": "2026-09-01T00:00:00Z",
            }

        self.assertEqual(market.witness_holds(witness(), "2026-09-27"), [])
        for value in ("no", "sometimes", "yes|marketable=no"):
            self.assertTrue(market.witness_holds(witness(value), "2026-09-27"))
        self.assertTrue(market.witness_holds(witness(ids="7 (variant)"), "2026-09-27"))
        self.assertTrue(market.witness_holds(witness(), "2026-08-01"))
        for change in ({"raw_sha256": "0" * 64}, {"source_item_ids": [8]}):
            with self.assertRaises(ValueError):
                market.witness_holds(witness() | change, "2026-09-27")


if __name__ == "__main__":
    unittest.main()
