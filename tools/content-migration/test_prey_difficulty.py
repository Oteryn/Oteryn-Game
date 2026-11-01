"""PREY-DIFF content decisions, exact valuation and source eligibility."""
import json
import tempfile
import unittest
from fractions import Fraction
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import prey_difficulty as prey


def creature(key, xp, health=100, **flags):
    return {"profile": {"target": {"key": key}, "data": {"kind": "Creature", "profile": {
        "experience": xp, "health": health, "bestiary": {}, "details": {
            "display_name": key, "system_eligibility": {"prey": True, "exclusive_prey": False, **flags}}}}}}


class PreyDifficultyTests(unittest.TestCase):
    def setUp(self):
        self.config = {"experience_thresholds": [100, 1000, 5000, 20000], "minimum_levels": [1, 50, 100, 250, 500],
                       "list_size": 9, "expanded_list_size": 12, "proposal_status": "awaiting_owner_approval"}

    def test_tiers_follow_authored_experience_boundaries_and_stable_hp_order(self):
        document = {"records": [creature(f"oteryn:{i}", xp, health) for i, (xp, health) in enumerate(
            [(99, 100), (100, 200), (999, 100), (1000, 300), (4999, 400), (5000, 500), (19999, 600), (20000, 700)])]}
        result = prey.classify(document, self.config)
        self.assertEqual([r["tier"] for r in result["records"]], [1, 2, 2, 3, 3, 4, 4, 5])
        self.config["experience_thresholds"][0] = 200
        changed = prey.classify(document, self.config)
        self.assertEqual(changed["records"][1]["tier"], 1)

    def test_membership_matches_native_bestiary_positive_xp_prey_guard(self):
        admitted = creature("oteryn:allowed", 1)
        no_bestiary = creature("oteryn:none", 10)
        no_bestiary["profile"]["data"]["profile"]["bestiary"] = None
        rows = [admitted, no_bestiary, creature("canary:reference", 10), creature("oteryn:zero", 0),
                creature("oteryn:disabled", 10, prey=False), creature("oteryn:exclusive", 10, exclusive_prey=True)]
        self.assertEqual(list(prey.eligible_profiles({"records": rows})), ["oteryn:allowed", "oteryn:exclusive"])
        result = prey.classify({"records": rows}, self.config)
        self.assertEqual(result["tiers"][0]["nonexclusive_count"], 1)

    def test_expansion_capacity_is_reported_instead_of_inventing_creatures(self):
        document = {"records": [creature(f"oteryn:{i}", 20000) for i in range(33)]}
        tier = prey.classify(document, self.config)["tiers"][4]
        self.assertTrue(tier["normal_three_slot_capacity"])
        self.assertFalse(tier["expanded_three_slot_capacity"])

    def test_duplicate_identity_is_refused(self):
        row = creature("oteryn:same", 10)
        with self.assertRaisesRegex(ValueError, "PREY_DUPLICATE_CREATURE"):
            prey.eligible_profiles({"records": [row, row]})

    def test_loot_value_uses_exact_chance_count_and_npc_value(self):
        entries = [{"item": {"key": "coin"}, "probability_ppm": 500000, "min_count": 1, "max_count": 4},
                   {"item": {"key": "rare"}, "probability_ppm": 125000, "min_count": 1, "max_count": 1},
                   {"item": {"key": "no_npc_offer"}, "probability_ppm": 1000000, "min_count": 1, "max_count": 100}]
        self.assertEqual(prey.expected_loot(entries, {"coin": 1, "rare": 100}), Fraction(55, 4))
        loot = {"tables": [{"identity": {"key": "table"}, "algorithm": "IndependentBernoulliPpm", "entries": entries}],
                "creature_loot": [{"creature": {"key": "creature"}, "loot": {"key": "table"}}]}
        self.assertEqual(prey.loot_values(loot, {"coin": 1, "rare": 100}, ["creature"]), {"creature": 13750000})

    def test_nested_loot_is_conditional_on_parent(self):
        child = {"item": {"key": "child"}, "probability_ppm": 500000, "min_count": 2, "max_count": 2}
        parent = {"item": {"key": "container"}, "probability_ppm": 500000, "min_count": 1, "max_count": 1, "children": [child]}
        self.assertEqual(prey.expected_loot([parent], {"container": 10, "child": 100}), Fraction(55))

    def test_source_balance_change_rebuilds_model_without_editing_native_artifact(self):
        import bind_monster_source_definitions as binding

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            key = "oteryn:creature.rat"
            baseline = {"records": [creature(key, 100)]}
            baseline["records"][0]["profile"]["target"]["family"] = "Creature"
            original = prey.encoded(baseline)
            prey.write_files(root, {prey.PROFILES: original})
            config = json.loads((prey.ROOT / prey.CONFIG).read_bytes())
            declarations = {"records": [], "authoring_profiles": [baseline["records"][0]["profile"]]}
            prey.write_files(root, {
                prey.CONFIG: prey.encoded(config),
                "content/world/definitions/declarations.json": prey.encoded(declarations),
                "content/world/definitions/reference.json": prey.encoded({"records": []}),
                "content/world/provenance/sources.json": prey.encoded({"sources": [], "source_identity_bindings": []}),
                "content/creatures/official-balance.json": prey.encoded({"overrides": [{"creature": "rat", "field": "experience"}]}),
                prey.LOOT: prey.encoded({"tables": [], "creature_loot": [{"creature": {"key": key}, "loot": None}]}),
                "crates/gameplay/domain/src/domain/currency.rs": b'Self::Gold => "gold", Self::Platinum => "platinum", Self::Crystal => "crystal",\nSelf::Gold => 1, Self::Platinum => 100, Self::Crystal => 10_000,',
            })
            seen = []

            def registration(task_root, *, include_prey):
                self.assertFalse(include_prey, "bootstrap must not recurse into its own result")
                # Exercise the actual canonical balance bridge used by registration.
                return binding.bind_definitions(task_root, {
                    prey.PROFILES: original,
                    "content/spells.manifest.json": prey.encoded({"creature_profiles": {}}),
                })

            def model(task_root, profiles):
                rows = prey.classify(profiles, prey.authoring(task_root))["records"]
                return {"input_sha256": prey.digest(profiles), "tiers": {row["creature"]: row["tier"] for row in rows}}

            def cargo(command, *, cwd, check):
                self.assertEqual(cwd, root)
                self.assertTrue(check)
                native = json.loads(Path(command[command.index("--creature-profiles") + 1]).read_bytes())
                data = json.loads(Path(command[command.index("--input") + 1]).read_bytes())
                self.assertEqual(data["input_sha256"], prey.digest(native))
                seen.append(native["records"][0]["profile"]["data"]["profile"]["experience"])
                # The test substitutes only the external Cargo process. Combat
                # arithmetic itself is covered by the real Rust example tests.
                result = {"schema_version": 1, "input_sha256": data["input_sha256"],
                          "seed": config["model"]["seed"], "prey_seconds": 7200,
                          "presets": [], "models": [], "cases": [], "kills_per_hour_micros": {key: 1_000_000},
                          "no_physical_kill": [], "no_source_melee": [], "available_creatures": [key],
                          "unsettleable_creatures": []}
                output = Path(command[command.index("--output") + 1])
                output.parent.mkdir(parents=True, exist_ok=True)
                output.write_bytes(prey.encoded(result))

            with patch.dict("sys.modules", {"register_spell_families": SimpleNamespace(outputs=registration)}), \
                    patch.object(prey, "model_input", side_effect=model), \
                    patch.object(prey.subprocess, "run", side_effect=cargo):
                prey.regenerate(root)
                first = prey.read(root, prey.RESULTS)["input_sha256"]
                declarations["authoring_profiles"][0]["data"]["profile"]["experience"] = 1000
                (root / "content/world/definitions/declarations.json").write_bytes(prey.encoded(declarations))
                prey.regenerate(root)
                self.assertNotEqual(first, prey.read(root, prey.RESULTS)["input_sha256"])
                self.assertEqual(prey.read(root, prey.TIERS)["records"][0]["tier"], 3)
                prey.regenerate(root)
            self.assertEqual(seen, [100, 1000], "unchanged inputs reuse the validated result")
            self.assertEqual((root / prey.PROFILES).read_bytes(), original, "the stale artifact was never manually patched")

    def test_npc_price_policy_is_data_and_coins_come_from_owner(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            trade = root / "content/services/trade/trade-00000-00000.json"
            trade.parent.mkdir(parents=True)
            trade.write_text(json.dumps({"records": [{"declaration": {"offers": [
                {"direction": "BuyFromPlayer", "item": {"key": "item"}, "unit_price": 8},
                {"direction": "BuyFromPlayer", "item": {"key": "item"}, "unit_price": 15},
                {"direction": "SellToPlayer", "item": {"key": "item"}, "unit_price": 1000}]}}]}))
            owner = root / "crates/gameplay/domain/src/domain/currency.rs"
            owner.parent.mkdir(parents=True)
            owner.write_text('Self::Gold => "gold", Self::Platinum => "platinum", Self::Crystal => "crystal",\n'
                             'Self::Gold => 1, Self::Platinum => 100, Self::Crystal => 10_000,')
            self.assertEqual(prey.npc_prices(root, "maximum_catalogue_offer"), {"crystal": 10000, "gold": 1, "item": 15, "platinum": 100})
            self.assertEqual(prey.npc_prices(root, "minimum_catalogue_offer")["item"], 8)

    def test_merge_owner_recognizes_generated_model_and_current_dated_reports(self):
        import regenerate_content as regeneration

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            prey.write_files(root, {"content/manifest.json": prey.encoded({"managed_files": []})})

            def git(*args):
                if args == ("show", f"HEAD:{regeneration.PIN_PATH}"):
                    return 'const DOCUMENTS: [(&str, usize, &str); 0] = [\n];'
                return ""

            with patch.object(regeneration, "ROOT", root), patch.object(regeneration, "git", side_effect=git):
                for date in ("2026-11-01", "2026-11-02"):
                    prey.write_files(root, {prey.CONFIG: prey.encoded({"model": {"date": date}})})
                    derived = regeneration.derived_paths()
                    for path in (prey.RESULTS, f"docs/balance/PREY-TIME-SIM-{date}.md",
                                 f"docs/balance/PREY-SLOT-VALUE-{date}.md"):
                        self.assertIn(path, derived)
                        self.assertFalse(regeneration.needs_person(path, derived))
                    self.assertTrue(regeneration.needs_person(prey.CONFIG, derived))
                (root / prey.CONFIG).write_text("<<<<<<< unresolved authoring\n")
                derived = regeneration.derived_paths()
                self.assertTrue(regeneration.needs_person(prey.CONFIG, derived))


if __name__ == "__main__":
    unittest.main()
