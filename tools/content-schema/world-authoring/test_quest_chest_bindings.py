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

    def test_annihilator_retains_exact_level100_startup_gate_with_reward(self):
        selected, _ = chests.selections()
        gate = selected["action"][(33214, 31671, 13)]
        self.assertEqual((gate["appearance"], gate["action"]), (5111, 1100))
        self.assertEqual(
            gate["claims"], ["oteryn:reward-claim.quest.u7_24.the_annihilator.reward"]
        )

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
        selected, _ = chests.selections()
        door = selected["action"][(32643, 32128, 8)]
        self.assertEqual((door["appearance"], door["action"]), (1631, 4055))

    def test_naginata_pick_access_is_selected_with_its_reward(self):
        selected, _ = chests.selections()
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
        with patch.object(
            chests,
            "selections",
            return_value=({"unique": self.binding, "action": {}}, []),
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
        with patch.object(
            chests,
            "selections",
            return_value=({"unique": self.binding, "action": {}}, []),
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
