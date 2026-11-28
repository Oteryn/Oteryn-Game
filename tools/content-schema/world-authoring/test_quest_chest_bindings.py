"""Startup UID restoration preserves selectors, attributes and nested contents."""

import copy
import hashlib
import json
import tarfile
import tempfile
import unittest
from unittest.mock import patch

import quest_chest_bindings as chests
import world_region_codec as codec


class QuestChestBindings(unittest.TestCase):
    def test_classic_quest_selectors_from_committed_regions(self):
        expected = (
            (32171, 32197, 7, 2472, "unique", 6013),
            (33131, 31624, 15, 2469, "unique", 6037),
            (33134, 31624, 15, 2469, "unique", 6038),
            (33048, 32630, 1, 3263, "unique", 3024),
            (33127, 31885, 9, 4024, "unique", 6069),
            (33163, 31603, 15, 2469, "unique", 6070),
            (32149, 32105, 11, 2472, "unique", 6113),
            (32357, 32130, 9, 2472, "unique", 6153),
            (32775, 32006, 11, 2476, "unique", 6155),
            (32568, 32085, 12, 3990, "unique", 6171),
            (33315, 32277, 11, 1983, "unique", 6174),
            (33315, 32282, 11, 1983, "unique", 6175),
            (32460, 31951, 5, 2434, "unique", 6176),
            (32464, 31957, 5, 2469, "unique", 6177),
            (32462, 31947, 4, 2473, "unique", 6178),
            (32467, 31962, 4, 2472, "unique", 6179),
            (32149, 32110, 11, 355, "action", 101),
            (32775, 32012, 10, 355, "action", 101),
            (32595, 32068, 9, 355, "action", 101),
            (32580, 32103, 11, 355, "action", 101),
            (32353, 32131, 8, 1930, "action", 102),
            (32354, 32131, 8, 1931, "action", 102),
        )
        world = chests.ROOT / chests.INDEX
        palette = json.loads(world.read_bytes())["palette"]
        regions = {}
        for x, y, z, appearance, attr, value in expected:
            with self.subTest(position=(x, y, z), appearance=appearance):
                name = codec.region_name(z, x // 256, y // 256)
                if name not in regions:
                    _, _, _, sectors = codec.decode_region(
                        world.with_name(name).read_bytes()
                    )
                    regions[name] = {
                        tile[:2]: tile[-1] for _, tiles in sectors for tile in tiles
                    }
                actual = [
                    (attrs or {}).get(attr)
                    for index, depth, attrs in regions[name][(x, y)]
                    if not depth and palette[index]["source_item_id"] == appearance
                ]
                self.assertEqual(actual, [value])

    def test_shipped_world_has_standalone_quest_reward_selectors(self):
        placements = chests.ROOT / "content/world/placements"
        palette = json.loads((placements / "index.json").read_bytes())["palette"]
        for x, y, z, appearance, uid in (
            (32015, 31409, 7, 2473, 6235),
            (32146, 31430, 8, 2473, 6217),
            (32456, 32008, 13, 2469, 6134),
            (32509, 32181, 13, 4285, 5015),
            (31973, 32209, 12, 2472, 6118),
            (31977, 32209, 12, 2472, 6119),
        ):
            with self.subTest(uid=uid):
                path = placements / codec.region_name(z, x // 256, y // 256)
                _, _, _, sectors = codec.decode_region(path.read_bytes())
                items = next(
                    tile[5]
                    for _, tiles in sectors
                    for tile in tiles
                    if tile[:2] == (x, y)
                )
                self.assertEqual(
                    [
                        (attrs or {}).get("unique")
                        for index, depth, attrs in items
                        if depth == 0 and palette[index]["source_item_id"] == appearance
                    ],
                    [uid],
                )

    def test_maze_of_lost_souls_has_four_level_30_entrance_doors(self):
        placements = chests.ROOT / "content/world/placements"
        palette = json.loads((placements / "index.json").read_bytes())["palette"]
        _, _, _, sectors = codec.decode_region(
            (placements / "region-z10-x126-y123.b3").read_bytes()
        )
        cells = {tile[:2]: tile[5] for _, tiles in sectors for tile in tiles}
        for x, y, appearance in (
            (32479, 31634, 5111),
            (32486, 31633, 5111),
            (32483, 31630, 5102),
            (32483, 31635, 5102),
        ):
            with self.subTest(x=x, y=y):
                self.assertEqual(
                    [
                        (attrs or {}).get("action")
                        for index, depth, attrs in cells[x, y]
                        if depth == 0 and palette[index]["source_item_id"] == appearance
                    ],
                    [1030],
                )

    def test_ghoul_room_landing_has_the_source_key_door(self):
        placements = chests.ROOT / "content/world/placements"
        palette = json.loads((placements / "index.json").read_bytes())["palette"]
        _, _, _, sectors = codec.decode_region(
            (placements / "region-z14-x126-y125.b3").read_bytes()
        )
        items = next(
            tile[5]
            for _, tiles in sectors
            for tile in tiles
            if tile[:2] == (32506, 32175)
        )
        self.assertEqual(
            [
                (attrs or {}).get("action")
                for index, depth, attrs in items
                if depth == 0 and palette[index]["source_item_id"] == 5106
            ],
            [3600],
        )

    def test_tyrsung_barrel_publishes_crystal_reward_uid(self):
        placements = chests.ROOT / chests.INDEX
        palette = json.loads(placements.read_bytes())["palette"]
        region = placements.with_name(codec.region_name(8, 126, 122))
        _, _, _, sectors = codec.decode_region(region.read_bytes())
        barrels = [
            (attributes or {}).get("unique")
            for _, tiles in sectors
            for x, y, _, _, _, items in tiles
            if (x, y) == (32431, 31235)
            for index, depth, attributes in items
            if depth == 0 and palette[index]["source_item_id"] == 2523
        ]
        self.assertEqual(barrels, [6216])

    def test_classic_quest_boxes_have_exact_shipped_selectors(self):
        index = json.loads((chests.ROOT / chests.INDEX).read_bytes())
        for z, rx, ry, expected in (
            (
                9,
                129,
                125,
                {
                    (33038, 32171): (2472, 6049),
                    (33039, 32171): (2472, 6050),
                    (33040, 32171): (2472, 6051),
                },
            ),
            (5, 126, 124, {(32467, 31970): (2469, 6257)}),
        ):
            path = (
                chests.ROOT / "content/world/placements" / codec.region_name(z, rx, ry)
            )
            raw = path.read_bytes()
            _, _, _, sectors = codec.parse_region(raw)
            selectors = {}
            for local, offset, length in sectors:
                payload = codec.zstandard.ZstdDecompressor().decompress(
                    raw[offset : offset + length],
                    max_output_size=codec.MAX_SECTOR_BYTES,
                )
                for x, y, _, _, _, items in codec.decode_sector(
                    payload, rx * 8 + local % 8, ry * 8 + local // 8
                ):
                    if (x, y) not in expected:
                        continue
                    for palette, depth, attrs in items:
                        appearance = index["palette"][palette]["source_item_id"]
                        if depth == 0 and appearance == expected[(x, y)][0]:
                            self.assertNotIn((x, y), selectors)
                            selectors[(x, y)] = (
                                appearance,
                                (attrs or {}).get("unique"),
                            )
            self.assertEqual(selectors, expected)

    def test_quest_reward_selectors_are_present_in_repository_regions(self):
        index = json.loads((chests.ROOT / chests.INDEX).read_bytes())
        selectors = (
            (32565, 32119, 3, 2472, 6160),
            (33143, 31719, 10, 2473, 6039),
            (33143, 31721, 10, 2473, 6040),
            (33189, 31688, 14, 2469, 6041),
            (33195, 31688, 14, 2469, 6042),
            (33188, 31682, 14, 2469, 6071),
            (33158, 31621, 15, 2469, 6043),
            (33158, 31622, 15, 2469, 6044),
            (32970, 32646, 8, 2472, 6258),
            (32972, 31461, 10, 1986, 6278),
        )
        for x, y, z, appearance, unique in selectors:
            with self.subTest(unique=unique):
                region = (
                    chests.ROOT
                    / "content/world/placements"
                    / codec.region_name(z, x // 256, y // 256)
                )
                _, _, _, sectors = codec.decode_region(region.read_bytes())
                found = [
                    (attrs or {}).get("unique")
                    for _, tiles in sectors
                    for tx, ty, _, _, _, items in tiles
                    if (tx, ty) == (x, y)
                    for palette, depth, attrs in items
                    if depth == 0
                    and index["palette"][palette]["source_item_id"] == appearance
                ]
                self.assertEqual(found, [unique])

    def test_committed_source_boxes_and_access_attributes(self):
        index = json.loads((chests.ROOT / chests.INDEX).read_bytes())
        for position, appearance, attribute, value in (
            ((32450, 32044, 8), 6248, "action", 3142),
            ((32601, 31776, 9), 2473, "unique", 6152),
            ((32769, 32302, 10), 2480, "unique", 6063),
            ((32774, 32289, 10), 355, "action", 101),
            ((32778, 32282, 11), 4024, "unique", 6062),
        ):
            with self.subTest(position=position):
                x, y, z = position
                name = codec.region_name(z, x // 256, y // 256)
                path = f"content/world/placements/{name}"
                (entry,) = [r for r in index["regions"] if r["path"] == path]
                raw = (chests.ROOT / path).read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(), entry["sha256"])
                matches = [
                    (attrs or {}).get(attribute)
                    for _, tiles in codec.decode_region(raw)[3]
                    for tx, ty, _, _, _, items in tiles
                    if (tx, ty) == (x, y)
                    for item, depth, attrs in items
                    if not depth
                    and index["palette"][item]["source_item_id"] == appearance
                ]
                self.assertEqual(matches, [value])

    def test_current_committed_corpus_captures_exact_elvenbane_startup_uids(self):
        from pathlib import Path

        revision = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
        paths = (
            "data-global/startup/tables/chest.lua",
            "data-global/startup/tables/door_level.lua",
            "data-global/startup/others/functions.lua",
        )
        keys = {
            f"oteryn:reward-claim.quest.u8_1.to_blind_the_enemy.{suffix}"
            for suffix in ("bp1", "bp2", "dwarven_shield", "morning_star")
        }
        archive = (
            chests.ROOT
            / "tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz"
        )
        with (
            tempfile.TemporaryDirectory() as temporary,
            tarfile.open(archive) as corpus,
        ):
            root = Path(temporary)
            manifest = json.load(corpus.extractfile("corpus-manifest.json"))
            for path in paths:
                row = next(
                    r
                    for r in manifest["files"]
                    if r["source"] == "crystalserver"
                    and r["revision"] == revision
                    and r["path"] == path
                )
                raw = corpus.extractfile(
                    row.get("blob_path", row.get("cache_path"))
                ).read()
                self.assertEqual(hashlib.sha256(raw).hexdigest(), row["sha256"])
                output = root / path
                output.parent.mkdir(parents=True, exist_ok=True)
                output.write_bytes(raw)
            document = chests.capture(root, keys)
            self.assertEqual(document["revision"], revision)
            self.assertEqual(
                document["table_sha256"],
                "0ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca",
            )
            self.assertEqual(
                {r["unique"] for r in document["bindings"]}, {6253, 6254, 6255, 6256}
            )
            self.assertEqual({r["claim"] for r in document["bindings"]}, keys)
            self.assertEqual(document["level_access"], [])
            startup = root / paths[2]
            original_startup = startup.read_bytes()
            startup.write_bytes(original_startup + b"changed")
            with self.assertRaisesRegex(ValueError, "QUEST_CHEST_STARTUP_SOURCE_PIN"):
                chests.capture(root, keys)
            startup.write_bytes(original_startup)
            chest = root / paths[0]
            chest.write_bytes(chest.read_bytes() + b"changed")
            with self.assertRaisesRegex(ValueError, "QUEST_CHEST_SOURCE_PIN"):
                chests.capture(root, keys)

    def test_annihilator_level_gate_requalified_from_retained_source_bytes(self):
        archive = (
            chests.ROOT
            / "tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz"
        )
        with tarfile.open(archive) as corpus:
            manifest = json.load(corpus.extractfile("corpus-manifest.json"))
            row = next(
                r
                for r in manifest["files"]
                if r["path"] == "data-global/startup/tables/door_level.lua"
                and r["sha256"] == chests.LEVEL_SHA256
            )
            raw = corpus.extractfile(row["cache_path"]).read()
        key = "oteryn:reward-claim.quest.u7_24.the_annihilator.reward"
        result = chests.captured_level_access(raw, {key})
        self.assertEqual(
            result,
            [
                {
                    "position": [33214, 31671, 13],
                    "appearance": 5111,
                    "action": 1100,
                    "source_line": 173,
                    "claims": [key],
                }
            ],
        )
        desert = chests.captured_level_access(raw, {"oteryn:reward-claim.kv.desert1"})
        self.assertEqual(
            (desert[0]["position"], desert[0]["appearance"], desert[0]["action"]),
            ([32673, 32100, 8], 5102, 1020),
        )
        with self.assertRaisesRegex(ValueError, "LEVEL_SOURCE_PIN"):
            chests.captured_level_access(raw + b"changed", {key})

    def test_fishnapped_goldfish_retains_its_unmovable_startup_uid(self):
        index = json.loads((chests.ROOT / chests.INDEX).read_bytes())
        path = chests.ROOT / "content/world/placements" / codec.region_name(6, 126, 128)
        _, _, _, sectors = codec.decode_region(path.read_bytes())
        fish = [
            attrs
            for _, tiles in sectors
            for x, y, _, _, _, items in tiles
            if (x, y) == (32359, 32791)
            for ordinal, depth, attrs in items
            if depth == 0 and index["palette"][ordinal]["source_item_id"] == 7936
        ]
        self.assertEqual(fish, [{"unique": 9266}])

    def test_annihilator_retains_exact_level100_startup_gate_with_reward(self):
        selected, _, _ = chests.selections()
        gate = selected["action"][(33214, 31671, 13)]
        self.assertEqual((gate["appearance"], gate["action"]), (5111, 1100))
        self.assertEqual(
            gate["claims"], ["oteryn:reward-claim.quest.u7_24.the_annihilator.reward"]
        )

    def test_restored_native_quest_chests_and_expertise_doors(self):
        selected, _, _ = chests.selections()
        for position, appearance, unique in (
            ((32800, 31959, 7), 3634, 5006),
            ((32813, 31964, 7), 3634, 5007),
            ((32868, 31955, 11), 3634, 6047),
            ((32880, 31955, 11), 3634, 6048),
            ((33295, 31658, 13), 2472, 6052),
            ((33297, 31658, 13), 2472, 6053),
            ((33298, 31658, 13), 2472, 6054),
            ((33199, 31923, 11), 2469, 6066),
            ((33294, 31658, 13), 2472, 6075),
            ((32490, 31192, 10), 4024, 6218),
        ):
            with self.subTest(position=position):
                row = selected["unique"][position]
                self.assertEqual(
                    (row["appearance"], row["unique"]), (appearance, unique)
                )
        for position, appearance, action in (
            ((32874, 31974, 12), 5102, 1050),
            ((32875, 31974, 12), 5102, 1050),
            ((33297, 31670, 14), 1648, 1080),
        ):
            with self.subTest(position=position):
                row = selected["action"][position]
                self.assertEqual(
                    (row["appearance"], row["action"]), (appearance, action)
                )

    def test_ape_city_mission_chests_have_startup_claim_selectors(self):
        selected, _, _ = chests.selections()
        for position, appearance, unique in (
            ((32782, 32910, 8), 2469, 6102),
            ((32935, 32886, 7), 2473, 6103),
        ):
            with self.subTest(position=position):
                row = selected["unique"][position]
                self.assertEqual(
                    (row["appearance"], row["unique"]), (appearance, unique)
                )
        chests.apply(check=True)

    def test_panpipe_key_uses_the_existing_complete_key_lowering(self):
        definition = chests.claim_definitions(chests.ROOT)[
            "oteryn:reward-claim.quest.key.id4055"
        ]
        self.assertTrue(chests.supported_claim(definition))
        for field in (
            "data_holds",
            "source_data",
            "achievement_grants",
            "charge_dispositions",
            "native_lowering",
            "key_binding",
        ):
            changed = copy.deepcopy(definition)
            if field == "data_holds":
                changed[field] = ["unresolved source fact"]
            elif field == "source_data":
                changed["source_variant"][field]["status"] = "INCOMPLETE"
            elif field == "native_lowering":
                changed["source_variant"][field]["pending_implementation"].append(
                    "OTHER_OWNER"
                )
            elif field == "key_binding":
                changed["placements"][0]["reward"][field] = "canary:door-key/4056"
            else:
                changed["source_variant"][field] = ["unproved grant"]
            with self.subTest(field=field):
                self.assertFalse(chests.supported_claim(changed))

    def test_panpipe_reward_retains_its_saved_key_door_binding(self):
        selected, _, _ = chests.selections()
        door = selected["action"][(32643, 32128, 8)]
        self.assertEqual((door["appearance"], door["action"]), (1631, 4055))

    def test_oasis_action_is_a_world_use_capture(self):
        index = json.loads((chests.ROOT / chests.INDEX).read_bytes())
        path = "content/quests/world-callbacks/qd-steal-from-thieves.json"
        captures = index["source"]["quest_world_uses"]
        self.assertEqual(captures["imported"], 1)
        self.assertEqual(
            captures["captures"],
            [
                {
                    "path": path,
                    "sha256": hashlib.sha256(
                        (chests.ROOT / path).read_bytes()
                    ).hexdigest(),
                }
            ],
        )
        self.assertNotIn(
            path, [pin["path"] for pin in index["source"]["quest_chests"]["captures"]]
        )

    def test_naginata_pick_access_is_selected_with_its_reward(self):
        selected, _, _ = chests.selections()
        rock = selected["action"][(32356, 32074, 10)]
        self.assertEqual((rock["appearance"], rock["action"]), (1791, 40031))
        self.assertEqual(rock["claims"], ["oteryn:reward-claim.kv.naginata"])

    def test_native_container_source_holds_cannot_be_bypassed_by_map_selection(self):
        definitions = chests.claim_definitions(chests.ROOT)
        for key in (
            "oteryn:reward-claim.kv.orcshaman",
            "oteryn:reward-claim.kv.powerbolt2",
        ):
            definition = definitions[key]
            self.assertTrue(chests.supported_claim(definition))
            for field in ("data_holds", "native_contents", "source_variant"):
                changed = copy.deepcopy(definition)
                if field == "data_holds":
                    changed[field] = ["unresolved source fact"]
                elif field == "native_contents":
                    changed[field]["source_sha256"] = "unverified"
                else:
                    changed[field]["source_data"]["status"] = "INCOMPLETE"
                self.assertFalse(chests.supported_claim(changed), (key, field))

    def setUp(self):
        self.palette = [{"source_item_id": 351}, {"source_item_id": 2472}]
        self.position = (32346, 32063, 12)
        self.binding = {self.position: {"appearance": 2472, "unique": 6135}}
        self.tiles = [
            (
                32346,
                32063,
                0,
                0,
                (),
                [
                    (0, 0, None),
                    (1, 0, {"text": "kept", "action": 0}),
                    (1, 1, {"unique": 99}),
                ],
            )
        ]

    def encode(self, tiles):
        x, y, z = self.position
        sx, sy = x // 32, y // 32
        return codec.encode_region(
            z,
            x // 256,
            y // 256,
            {(sy % 8) * 8 + sx % 8: codec.encode_sector_tiles(sx, sy, tiles)},
        )

    def test_exact_selector_restores_one_uid_and_is_byte_idempotent(self):
        before = self.encode(self.tiles)
        after, count, added = chests.overlay_region(before, self.palette, self.binding)
        self.assertEqual((count, added), (1, 1))
        decoded = codec.decode_region(after)[3][0][1]
        expected = copy.deepcopy(self.tiles)
        expected[0][5][1][2]["unique"] = 6135
        self.assertEqual(decoded, expected)
        self.assertEqual(
            chests.overlay_region(after, self.palette, self.binding), (after, 1, 0)
        )

    def test_other_region_is_byte_identical(self):
        before = self.encode(self.tiles)
        self.assertEqual(
            chests.overlay_region(
                before,
                self.palette,
                {(33000, 32063, 12): {"appearance": 2472, "unique": 6135}},
            ),
            (before, 0, 0),
        )

    def test_missing_appearance_nested_only_and_house_are_rejected(self):
        for mode in ("appearance", "nested", "house", "absent", "ambiguous"):
            tiles = copy.deepcopy(self.tiles)
            if mode == "appearance":
                bindings = {self.position: {"appearance": 2473, "unique": 6135}}
            else:
                bindings = self.binding
            if mode == "nested":
                tiles[0][5][1] = (1, 1, None)
            elif mode == "house":
                x, y, flags, _, zones, items = tiles[0]
                tiles[0] = (x, y, flags, 1, zones, items)
            elif mode == "absent":
                x, y, flags, house, zones, items = tiles[0]
                tiles[0] = (x - 1, y, flags, house, zones, items)
            elif mode == "ambiguous":
                tiles[0][5].append((1, 0, None))
            with self.subTest(mode=mode), self.assertRaises(ValueError):
                chests.overlay_region(self.encode(tiles), self.palette, bindings)

    def test_conflicting_unique_including_zero_is_never_overwritten(self):
        for unique in (0, 6134):
            tiles = copy.deepcopy(self.tiles)
            tiles[0][5][1][2]["unique"] = unique
            with (
                self.subTest(unique=unique),
                self.assertRaisesRegex(ValueError, "UID_CONFLICT"),
            ):
                chests.overlay_region(self.encode(tiles), self.palette, self.binding)

    def test_repository_overlay_updates_counts_and_hashes_once(self):
        raw = self.encode(self.tiles)
        path = "content/world/placements/region-z12-x126-y125.b3"
        outputs = {
            path: raw,
            chests.INDEX: chests.canonical(
                {
                    "palette": self.palette,
                    "source": {},
                    "regions": [
                        {"path": path, "sha256": hashlib.sha256(raw).hexdigest()}
                    ],
                }
            ),
            chests.SUMMARY: chests.canonical(
                {
                    "source": {},
                    "item_attributes": {"text": 1},
                    "bytes_on_disk": len(raw),
                }
            ),
        }
        _, _, world_use = chests.selections()
        with patch.object(
            chests,
            "selections",
            return_value=({"unique": self.binding, "action": {}}, [], world_use),
        ):
            self.assertEqual(chests.overlay_outputs(outputs, require_all=True), 1)
            once = copy.deepcopy(outputs)
            self.assertEqual(chests.overlay_outputs(outputs, require_all=True), 1)
        self.assertEqual(once, outputs)
        summary = json.loads(outputs[chests.SUMMARY])
        self.assertEqual(summary["item_attributes"], {"text": 1, "unique": 1})
        self.assertEqual(summary["bytes_on_disk"], len(outputs[path]))
        index = json.loads(outputs[chests.INDEX])
        self.assertEqual(
            index["regions"][0]["sha256"], hashlib.sha256(outputs[path]).hexdigest()
        )

    def test_repository_check_requires_regions_while_unrelated_world_is_unchanged(self):
        outputs = {chests.INDEX: chests.canonical({"source": {}, "regions": []})}
        before = copy.deepcopy(outputs)
        _, _, world_use = chests.selections()
        with patch.object(
            chests,
            "selections",
            return_value=({"unique": self.binding, "action": {}}, [], world_use),
        ):
            with self.assertRaisesRegex(ValueError, "REGION_MISSING"):
                chests.overlay_outputs(outputs, require_all=True)
            self.assertEqual(chests.overlay_outputs(outputs), 0)
        self.assertEqual(outputs, before)

    def test_level_gate_restoration_preserves_its_unique_and_rejects_another_action(
        self,
    ):
        tiles = copy.deepcopy(self.tiles)
        tiles[0][5][1] = (1, 0, {"unique": 42})
        bindings = {self.position: {"appearance": 2472, "action": 1040}}
        after, count, added = chests.overlay_region(
            self.encode(tiles), self.palette, bindings, "action"
        )
        self.assertEqual((count, added), (1, 1))
        attrs = codec.decode_region(after)[3][0][1][0][5][1][2]
        self.assertEqual(attrs, {"unique": 42, "action": 1040})
        bindings[self.position]["action"] = 1035
        with self.assertRaisesRegex(ValueError, "UID_CONFLICT"):
            chests.overlay_region(after, self.palette, bindings, "action")


if __name__ == "__main__":
    unittest.main()
