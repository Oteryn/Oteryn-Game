"""Exercise the shipped regions with the world placement decoder."""

import unittest

import quest_chest_bindings as world


def native_tiles(positions):
    index = world.json.loads((world.ROOT / world.INDEX).read_bytes())
    palette = index["palette"]
    targets = set(positions)
    for region in index["regions"]:
        if not any(
            region["path"].endswith(
                f"region-z{z:02d}-x{x // 256:03d}-y{y // 256:03d}.b3"
            )
            for x, y, z in targets
        ):
            continue
        z, _, _, sectors = world.codec.decode_region(
            (world.ROOT / region["path"]).read_bytes()
        )
        for _, tiles in sectors:
            for x, y, _, house, _, items in tiles:
                if (x, y, z) in targets:
                    assert house == 0
                    yield (
                        (x, y, z),
                        [
                            (palette[index]["source_item_id"], depth, attrs or {})
                            for index, depth, attrs in items
                        ],
                    )


class ClassicQuestPlacements(unittest.TestCase):
    def check(self, rows):
        tiles = dict(native_tiles(row[0] for row in rows))
        for position, appearance, attribute, value in rows:
            with self.subTest(position=position, appearance=appearance, value=value):
                matching = [
                    a.get(attribute)
                    for i, d, a in tiles[position]
                    if i == appearance and d == 0
                ]
                self.assertEqual(matching, [value])

    def test_devil_helmet_startup_carriers_plate_and_gates(self):
        self.check(
            [
                ((32576, 32216, 15), 4240, "unique", 5016),
                ((32589, 32100, 14), 387, "unique", 5017),
                ((32592, 32102, 14), 5106, "action", 3667),
                ((32462, 32153, 14), 1631, "action", 3610),
                ((32462, 32151, 15), 1628, "action", 3610),
                ((32468, 32119, 14), 431, "unique", 25030),
                ((32479, 32174, 15), 5111, "action", 1030),
            ]
        )

    def test_draconia_startup_keys_and_gates(self):
        doors = [
            (32814, 31597, 7),
            (32796, 31597, 7),
            (32811, 31597, 7),
            (32809, 31593, 6),
            (32794, 31582, 6),
            (32814, 31595, 6),
            (32797, 31592, 4),
            (32804, 31585, 1),
        ]
        keys = [
            ((32794, 31572, 7), 3001),
            ((32790, 31593, 7), 3003),
            ((32807, 31576, 6), 3004),
            ((32792, 31590, 6), 3005),
            ((32801, 31579, 6), 3006),
            ((32813, 31576, 5), 3007),
        ]
        self.check(
            [
                *(
                    (position, 5097, "action", number)
                    for number, position in enumerate(doors, 3001)
                ),
                *((position, 2969, "action", number) for position, number in keys),
                ((32802, 31576, 7), 2474, "unique", 6093),
                ((32800, 31582, 2), 2435, "unique", 5023),
                ((32804, 31583, 2), 5102, "action", 1025),
            ]
        )

    def test_draconia_startup_portals_and_levers(self):
        portals = {
            (32675, 31646, 10): (32725, 31589, 12),
            (32669, 31653, 10): (32679, 31673, 10),
            (32794, 31576, 5): (32812, 31577, 5),
            (32812, 31576, 5): (32794, 31577, 5),
        }
        self.check(
            [
                *(
                    (position, 1949, "teleport", destination)
                    for position, destination in portals.items()
                ),
                ((32792, 31595, 7), 2772, "unique", 30006),
                ((32792, 31579, 7), 2772, "unique", 30035),
                ((32805, 31587, 1), 1949, "unique", 35011),
                ((32815, 31599, 9), 1949, "unique", 35012),
            ]
        )
        root = world.ROOT
        index = world.json.loads(
            (root / "content/world/transitions/index.json").read_bytes()
        )
        transitions = {}
        for path in index["shards"]:
            for record in world.json.loads((root / path).read_bytes())["records"]:
                source = record["declaration"]["from"]
                position = (source["x"], source["y"], source["floor"])
                if position in portals:
                    destination = record["declaration"]["to"]
                    transitions[position] = (
                        destination["x"],
                        destination["y"],
                        destination["floor"],
                    )
        self.assertEqual(transitions, portals)

    def test_classic_quest_reward_carriers(self):
        self.check(
            [
                ((32803, 31582, 2), 2472, "unique", 6023),
                ((32804, 31582, 2), 2472, "unique", 6024),
                ((32179, 32224, 9), 4025, "unique", 6161),
                ((33072, 32169, 2), 2469, "unique", 6073),
                ((33078, 32169, 2), 2469, "unique", 6074),
            ]
        )


if __name__ == "__main__":
    unittest.main()
