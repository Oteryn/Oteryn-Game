import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[3]


class CosmeticMountOffers(unittest.TestCase):
    def test_store_mounts_with_wiki_prices(self):
        document = json.loads((ROOT / "rulesets/economy/store/cosmetics.json").read_text())
        offers = {row["key"]: row for row in document["offers"]}
        for name, look, price, revision, line in (
            ("battle_werewolf", 1873, 870, 427794, 274),
            ("boisterous_bull", 1672, 690, 394898, 242),
            ("obstinate_ox", 1674, 690, 394900, 244),
            ("surly_steer", 1673, 690, 394899, 243),
        ):
            with self.subTest(name=name):
                offer = offers["store:mount:" + name.replace("_", "-")]
                key = "oteryn:content.mount." + name
                self.assertEqual(offer["unlock_key"], key)
                self.assertEqual(offer["preview"], {"kind": 3, "key": key})
                self.assertEqual(offer["price"], price)
                self.assertEqual(offer["currency"], "TIBIA_COIN")
                self.assertEqual(offer["tibiawiki_revision"], revision)
                self.assertEqual(offer["mount_xml_line"], line)
                self.assertEqual(document["source"]["mount_looks"][key], look)
