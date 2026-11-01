"""Market parity: the Oteryn marketable set equals Crystal's market set on defined Items."""

import json
import unittest
from pathlib import Path

import lower_client_market_crystal_parity_packet as parity

ROOT = Path(__file__).resolve().parents[3]

# Crystal market ids (flags.market, zimbadev/crystalserver@201189f) with no Oteryn Item
# definition. E06 (owner decision 2026-10-31) minted the last 68 from the pinned client
# (OTV2-20261031-item-client-market-mint-v1.json), so none remain.
NO_DEFINITION = ()

# Wave 1 Wiki `marketable: false` facts that Crystal's market flag supersedes (owner decision
# 2026-10-31): the parity row names the superseded fact.
SUPERSEDED_WAVE1_FALSE = {3321}

# Oteryn-only marketable ids: the newer official client defines them, Crystal's appearance
# file ends below them (its highest market id is 54651), so Crystal cannot confirm them.
OTERYN_ONLY = {
    54640: "newer client appearance; absent from the pinned Crystal appearances.dat"
}


def content_marketable():
    """Item id -> True/False for every definition whose marketable state is KNOWN."""
    index = json.loads((ROOT / "content/items/index.json").read_text())
    defined, known = set(), {}
    for shard in index["shards"]:
        for record in json.loads((ROOT / shard).read_text())["records"]:
            definition = record["definition"]
            item_id = int(definition["identity"]["key"].rsplit("i", 1)[1])
            defined.add(item_id)
            group = definition.get("semantics", {}).get("trade_restrictions", {})
            marketable = group.get("value", {}).get("marketable", {})
            if marketable.get("state") == "KNOWN":
                known[item_id] = marketable["value"]
    return defined, known


class MarketCrystalParity(unittest.TestCase):
    def setUp(self):
        self.packet = json.loads(parity.OUTPUT.read_text())
        self.crystal = set(self.packet["crystal_market_ids"])

    def test_content_marketable_equals_crystal_market_on_defined_items(self):
        defined, known = content_marketable()
        true_ids = {i for i, value in known.items() if value}
        self.assertEqual(true_ids - self.crystal, set(OTERYN_ONLY))
        self.assertEqual(
            true_ids & self.crystal, {i for i in self.crystal if i in defined}
        )
        self.assertEqual(self.crystal - defined, set(NO_DEFINITION))
        self.assertEqual(
            {i for i, value in known.items() if not value} & self.crystal, set()
        )

    def test_packet_exceptions_and_counts_match_the_explicit_lists(self):
        reasons = {e["source_item_id"]: e["reason"] for e in self.packet["exceptions"]}
        self.assertEqual(reasons, {i: parity.NO_DEFINITION for i in NO_DEFINITION})
        self.assertEqual(
            {
                r["source_item_id"]: r["supersedes"]
                for r in self.packet["promotions"]
                if "supersedes" in r
            },
            {i: parity.SUPERSEDES_WAVE1_FALSE for i in SUPERSEDED_WAVE1_FALSE},
        )
        counts = self.packet["counts"]
        self.assertEqual(counts["crystal_market_ids"], len(self.crystal))
        self.assertEqual(counts["promotions"], len(self.packet["promotions"]))
        self.assertEqual(counts["exceptions"], len(reasons))
        self.assertTrue(
            {r["source_item_id"] for r in self.packet["promotions"]} <= self.crystal
        )


if __name__ == "__main__":
    unittest.main()
