"""M19's declared inputs, pinned native facts and installed cauldron stay reproducible."""

import json
from pathlib import Path
import sys
import unittest

import qready1_clock_callbacks as clocks

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
import qready1_clock_landmarks as landmarks
import world_region_codec as codec


class Qready1ClockBindings(unittest.TestCase):
    def test_committed_bindings_match_the_chosen_recipe_and_literal_source_interval(
        self,
    ):
        for path, expected in clocks.build(ROOT).items():
            self.assertEqual(expected, json.loads((ROOT / path).read_bytes()), path)
        callbacks = clocks.build(ROOT)[clocks.OUTPUT]["callbacks"]
        self.assertEqual(
            [86400, 60 * 30 * 1000],
            [c["state_ops"][0]["from"]["seconds"] for c in callbacks],
        )
        self.assertEqual([3, 3], [o["count"] for o in callbacks[0]["offerings"]])
        self.assertEqual(
            "oteryn:item.tibia.i5953", callbacks[1]["effects"]["item_reward"]["item"]
        )

    def test_authored_delta_does_not_relax_the_donor_pin(self):
        import copy
        import convert_world_base as base

        summary = json.loads((ROOT / landmarks.SUMMARY).read_bytes())
        index = json.loads((ROOT / landmarks.INDEX).read_bytes())
        self.assertEqual(
            base.PINNED_TOTALS, base.world_otbm_totals(index["totals"], summary)
        )
        landmarks.validate_installed(ROOT, index, summary)
        for field, value in [("items_added", 2), ("sha256", "0" * 64)]:
            bad = copy.deepcopy(summary)
            bad["source"]["qready1_clock_landmark"][field] = value
            with self.assertRaisesRegex(ValueError, "M19_LANDMARK_SOURCE_CHANGED"):
                base.world_otbm_totals(index["totals"], bad)

    def test_native_item_facts_come_from_the_pinned_sources(self):
        packet = json.loads(
            (
                ROOT
                / "tools/content-schema/native-gameplay/qready1-quest-item-profiles.json"
            ).read_bytes()
        )
        rows = {
            r["production_definition"]["production_key"]: r for r in packet["records"]
        }
        self.assertEqual(
            {5953, 11982, 12023, 25783}, {int(k.rsplit(".i", 1)[1]) for k in rows}
        )
        self.assertEqual(
            100,
            rows["oteryn:item.tibia.i5953"]["semantics"]["physical"]["value"]["weight"][
                "value"
            ],
        )
        self.assertEqual(
            205,
            rows["oteryn:item.tibia.i11982"]["semantics"]["physical"]["value"][
                "weight"
            ]["value"],
        )
        for number in [12023, 25783]:
            self.assertEqual(
                ["Ground"],
                rows[f"oteryn:item.tibia.i{number}"]["admission"]["legal_destinations"],
            )

    def test_equipment_cohort_requires_all_four_exact_source_identities(self):
        import copy

        sys.path.insert(0, str(ROOT / "tools/content-migration"))
        import qready1_clock_equipment_pin as pin

        packet = json.loads(
            (
                ROOT
                / "tools/content-schema/native-gameplay/qready1-quest-item-profiles.json"
            ).read_bytes()
        )
        source = (
            ROOT
            / "crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/ability_tests.rs"
        ).read_text()
        self.assertEqual(source, pin.refresh(source, packet))
        bad = copy.deepcopy(packet)
        bad["records"][0] = copy.deepcopy(bad["records"][1])
        with self.assertRaisesRegex(ValueError, "M19_CLOCK_SOURCE_COHORT"):
            pin.refresh(source, bad)
        bad = copy.deepcopy(packet)
        bad["records"][0]["attributes"]["speed_bonus"] = 1
        with self.assertRaisesRegex(ValueError, "M19_CLOCK_SPEED_SOURCE_CHANGED"):
            pin.refresh(source, bad)

    def test_authored_cauldron_is_installed_below_the_swamp_and_overlay_is_idempotent(
        self,
    ):
        capture = json.loads((ROOT / clocks.LANDMARK).read_bytes())
        x, y, floor = capture["position"]
        self.assertEqual(-8, floor)
        path = "content/world/placements/" + codec.region_name(
            -floor, x // 256, y // 256
        )
        index = json.loads((ROOT / landmarks.INDEX).read_bytes())
        tile = next(
            t
            for _, tiles in codec.decode_region((ROOT / path).read_bytes())[3]
            for t in tiles
            if t[:2] == (x, y)
        )
        self.assertEqual(0, tile[3], "never install the public station in a house")
        self.assertEqual(
            [(1210, 0), (12023, 0)],
            [(index["palette"][p]["source_item_id"], depth) for p, depth, _ in tile[5]],
        )
        outputs = {
            p: (ROOT / p).read_bytes()
            for p in [landmarks.INDEX, landmarks.SUMMARY, path]
        }
        before = outputs.copy()
        landmarks.overlay_outputs(outputs)
        self.assertEqual(before, outputs)
