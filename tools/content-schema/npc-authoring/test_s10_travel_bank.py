"""Compare the pinned Crystal S10 route gaps and bank population with native content."""

import json
import unittest
from pathlib import Path

from travel_semantics import canonical_keyword

ROOT = Path(__file__).resolve().parents[3]
SOURCE = Path(__file__).with_name("samples") / "s10-crystal-travel-bank.json"
CORRECTIONS = (
    ROOT / "crates/gameplay/content/src/content/npc_catalogue/travel_routes.json"
)


# Global fare evidence supersedes the historical Crystal quote, even on a held route.
# https://tibia.fandom.com/wiki/Magic_Carpet and https://tibia.fandom.com/wiki/Petros
GLOBAL_FARES = {
    "chemar": {"edron": 40, "femor_hills": 60, "issavi": 100, "kazordoon": 80, "svargrond": 60},
    "petros": {
        "ankrahmun": 100, "gray_island": 150, "krailos": 110,
        "liberty_bay": 200, "port_hope": 180, "venore": 60,
    },
}


def read(path):
    return json.loads(path.read_bytes())


def declarations(directory, pattern):
    for path in sorted((ROOT / directory).glob(pattern)):
        yield from (record["declaration"] for record in read(path)["records"])


class S10TravelBankTests(unittest.TestCase):
    def test_s10_crystal_routes_are_declared_or_reported_held(self):
        routes = {route["key"]: route for route in read(CORRECTIONS)}
        for source in read(SOURCE)["routes"]:
            service = f"oteryn:service.travel.{source['npc']}"
            slug = canonical_keyword(source["keyword"])
            key = f"{service}/{slug}"
            global_fare = GLOBAL_FARES.get(source["npc"], {}).get(slug)
            with self.subTest(route=key, source=source["source"]):
                route = routes.get(key)
                if (source["npc"], slug) == ("chemar", "farmine"):
                    self.assertEqual(route["aliases"], ["zao"])
                    self.assertEqual(route["quest_tracks"], [{
                        "track": "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission10_1",
                        "minimum": 2,
                        "maximum": 2,
                    }])
                if (source["npc"], slug) == ("petros", "yalahar"):
                    self.assertEqual(route["yalahar_town_gate"], "darashia")
                if source["hold"] == "quest gate" and source["npc"] not in {"chemar", "petros"}:
                    for keyword in [source["keyword"], *source.get("aliases", [])]:
                        alias = f"{service}/{canonical_keyword(keyword)}"
                        self.assertTrue(
                            alias not in routes or routes[alias]["held"], source["hold"]
                        )
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
                        source["price"] if global_fare is None else global_fare,
                        (slug != "marapur" if source["npc"] == "chemar" else source["premium"]),
                        source["min_level"],
                        source["postman"],
                        source["carpet"],
                        (slug == "marapur" if source["npc"] in {"chemar", "petros"} else bool(source["hold"])),
                    ),
                )

    def test_s10_placed_crystal_bankers_have_a_bank_binding(self):
        placements = {
            row["npc"]
            for row in declarations("content/world/npc-placements", "placements-*.json")
        }
        records = read(ROOT / "content/world/definitions/declarations.json")["records"]
        npcs = {row["identity"]["key"]: row for row in records if row["kind"] == "NPC"}
        bank_services = {
            row["identity"]["key"]
            for row in records
            if row["kind"] == "Service" and row.get("bank")
        }
        absent = []
        for banker in read(SOURCE)["bankers"]:
            key = f"oteryn:npc.{banker['npc']}"
            if key not in placements:
                absent.append(banker["npc"])
                continue
            with self.subTest(npc=key, source=banker["source"]):
                self.assertTrue(
                    any(service["key"] in bank_services for service in npcs[key]["services"]),
                    f"Missing bank service for {key}",
                )
        self.assertEqual(absent, ["elgar", "hireling", "plunderpurse"])


if __name__ == "__main__":
    unittest.main()
