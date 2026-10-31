"""Market parity: the Oteryn marketable set equals Crystal's market set on defined Items."""

import json
import unittest
from pathlib import Path

import lower_client_market_crystal_parity_packet as parity

ROOT = Path(__file__).resolve().parents[3]

# Crystal market ids (flags.market, zimbadev/crystalserver@201189f) with no Oteryn Item
# definition: adding definitions is outside this change, so these cannot be marketable.
NO_DEFINITION = (
    42085, 42175, 43892, 43893, 43948, 43949, 43950, 44157, 44179, 44755,
    47899, 48115, 48179, 48197, 48480, 48481, 48482, 48483, 48582, 48586,
    49160, 49176, 49272, 49273, 50051, 50053, 50062, 50063, 50312, 50335,
    50336, 50337, 50338, 51314, 51315, 51316, 51317, 51318, 51319, 51751,
    51752, 51753, 51754, 51755, 53142, 53143, 53144, 53145, 53146, 53147,
    53148, 53149, 53150, 53151, 53152, 53153, 53154, 53155, 53633, 53634,
    53635, 53636, 53637, 54510, 54511, 54512, 54513, 54514,
)  # fmt: skip

# Defined Crystal market ids that stay unmarketable: the accepted definitions are digest-sealed
# by other evidence (D289 native core hold of the quest reward admission, D310 sealed reward
# stack proof), so a marketable fact breaks the seal until the owner re-seals them.
SEALED = {
    901: "D289 native core hold; quest-interaction-item-admission pins the definition digest",
    3450: "D310 sealed state; reward_stack_normalization.json pins the definition",
}

# Crystal marks i3321 market, but Oteryn keeps the explicit Wiki `marketable: false`: the
# Market-true v2 hold list reads that false and the sealed successor8 current-parent receipt pins
# the v2 bytes, so flipping it needs an owner re-seal of that receipt first.
KNOWN_FALSE = {3321: "Wiki false pinned through the Market-true v2 packet and its receipt"}

# Oteryn-only marketable ids: the newer official client defines them, Crystal's appearance
# file ends below them (its highest market id is 54651), so Crystal cannot confirm them.
OTERYN_ONLY = {54640: "newer client appearance; absent from the pinned Crystal appearances.dat"}


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
            true_ids & self.crystal,
            {i for i in self.crystal if i in defined} - set(SEALED) - set(KNOWN_FALSE),
        )
        self.assertEqual(self.crystal - defined, set(NO_DEFINITION))
        self.assertTrue((set(SEALED) | set(KNOWN_FALSE)) <= defined & self.crystal)
        self.assertEqual(
            {i for i, value in known.items() if not value} & self.crystal,
            set(KNOWN_FALSE),
        )

    def test_packet_exceptions_and_counts_match_the_explicit_lists(self):
        reasons = {e["source_item_id"]: e["reason"] for e in self.packet["exceptions"]}
        self.assertEqual(
            reasons,
            {i: parity.NO_DEFINITION for i in NO_DEFINITION}
            | {i: parity.SEALED for i in SEALED}
            | {i: parity.KNOWN_FALSE_PINNED for i in KNOWN_FALSE},
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
