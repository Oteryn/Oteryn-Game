"""Compare the pinned Crystal S10 route gaps with native content."""

import json
import unittest
from pathlib import Path

from travel_semantics import canonical_keyword

ROOT = Path(__file__).resolve().parents[3]
SOURCE = Path(__file__).with_name("samples") / "s10-crystal-travel-bank.json"
CORRECTIONS = (
    ROOT / "crates/gameplay/content/src/content/npc_catalogue/travel_routes.json"
)


def read(path):
    return json.loads(path.read_bytes())



class S10TravelBankTests(unittest.TestCase):
    def test_s10_crystal_routes_are_declared_or_reported_held(self):
        routes = {route["key"]: route for route in read(CORRECTIONS)}
        for source in read(SOURCE)["routes"]:
            key = f"oteryn:service.travel.{source['npc']}/{canonical_keyword(source['keyword'])}"
            with self.subTest(route=key, source=source["source"]):
                route = routes.get(key)
                if source["hold"] == "quest gate":
                    for keyword in [source["keyword"], *source.get("aliases", [])]:
                        alias = f"oteryn:service.travel.{source['npc']}/{canonical_keyword(keyword)}"
                        self.assertTrue(alias not in routes or routes[alias]["held"], source["hold"])
                    continue
                self.assertIsNotNone(route, f"Missing route {key}")
                self.assertTrue(route["native_addition"])
                self.assertFalse(route.get("use_authored_fare_and_premium", False))
                self.assertNotIn("premium_override", route)
                self.assertNotIn("quest_gate", route)
                self.assertEqual(
                    (
                        route["keyword"],
                        [route["x"], route["y"], route["floor"]],
                        route["price"],
                        route["premium"],
                        route.get("min_level"),
                        route["postman"],
                        route["carpet"],
                        route["held"],
                    ),
                    (
                        source["keyword"],
                        source["destination"],
                        source["price"],
                        source["premium"],
                        source["min_level"],
                        source["postman"],
                        source["carpet"],
                        bool(source["hold"]),
                    ),
                )


if __name__ == "__main__":
    unittest.main()
