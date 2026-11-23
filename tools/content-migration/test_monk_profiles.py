"""Focused production overlays; historical test-pack inputs remain unchanged."""

import hashlib
import json
import unittest
from pathlib import Path

import register_spell_families as register
import timed_item_profiles

ROOT = Path(__file__).resolve().parents[2]
PROFILE = register.spell_import.PROVIDER_DESTINATIONS["item_profiles"]
MANIFEST = register.NATIVE_MANIFEST


class MonkProfiles(unittest.TestCase):
    def test_staff_overlay_uses_current_authoring_only(self):
        historical = ROOT / "content/test-packs/spells/r25/item_profiles.json"
        original = historical.read_bytes()
        document = json.loads(original)
        staff = next(
            r
            for r in document["records"]
            if r["production_definition"]["production_key"] == "oteryn:item.tibia.i3289"
        )
        generated = {
            PROFILE: json.dumps({"records": [staff]}).encode(),
            MANIFEST: b'{"item_profiles":{}}',
        }
        register.admit_current_staff_profile(ROOT, generated)
        semantics = json.loads(generated[PROFILE])["records"][0]["semantics"]
        weapon = semantics["weapon"]["value"]
        for field, value in (("weapon_type", "FIST"), ("attack", 22), ("defense", 14)):
            self.assertEqual(weapon[field], {"state": "KNOWN", "value": value})
        for field, value in staff["semantics"]["weapon"]["value"].items():
            if field not in ("weapon_type", "attack", "defense"):
                self.assertEqual(weapon[field], value)
        patterns = semantics["equipment"]["value"]["patterns"]["value"]
        self.assertEqual(len(patterns), 1)
        for field, value in (
            ("vocations", ["MONK"]),
            ("level", 15),
            ("primary_slot", "WEAPON"),
            ("additional_reserved_slots", ["SHIELD"]),
        ):
            self.assertEqual(patterns[0][field], {"state": "KNOWN", "value": value})
        self.assertEqual(
            json.loads(generated[MANIFEST])["item_profiles"]["sha256"],
            hashlib.sha256(generated[PROFILE]).hexdigest(),
        )
        self.assertEqual(historical.read_bytes(), original)

    def test_timed_power_ring_keeps_current_active_bonus_and_inactive_base(self):
        generated = {PROFILE: b'{"records":[]}', MANIFEST: b'{"item_profiles":{}}'}
        timed_item_profiles.admit(
            ROOT,
            generated,
            PROFILE,
            MANIFEST,
            register.provider_order,
            register.spell_import.canonical_bytes,
        )
        records = {
            r["production_definition"]["production_key"]: r["semantics"]
            for r in json.loads(generated[PROFILE])["records"]
        }
        self.assertEqual(
            records["oteryn:item.tibia.i3050"]["skill_modifiers"], {"state": "UNKNOWN"}
        )
        active = records["oteryn:item.tibia.i3087"]["skill_modifiers"]["value"][
            "modifiers"
        ]["value"]
        # Monk Vocation Update 2025-04-08, TibiaWiki Power Ring history: +6 -> +4.
        self.assertEqual(
            [(r["kind"], r["parameter"]["value"]) for r in active],
            [("SKILL_FIST", {"kind": "SIGNED_POINTS", "value": 4})],
        )

    def test_timed_rings_admit_reciprocal_forms_with_crystal_active_effects(self):
        generated = {PROFILE: b'{"records":[]}', MANIFEST: b'{"item_profiles":{}}'}
        timed_item_profiles.admit(
            ROOT,
            generated,
            PROFILE,
            MANIFEST,
            register.provider_order,
            register.spell_import.canonical_bytes,
        )
        records = {
            r["production_definition"]["production_key"]: r["semantics"]
            for r in json.loads(generated[PROFILE])["records"]
        }
        for base, active, duration, effects in (
            (3049, 3086, 600_000, {"INVISIBILITY": True}),
            (
                3052,
                3089,
                1_200_000,
                {
                    "HEALTH_GAIN": 2,
                    "MANA_GAIN": 8,
                    "HEALTH_TICKS": 6_000,
                    "MANA_TICKS": 6_000,
                },
            ),
            (
                3098,
                3100,
                450_000,
                {
                    "HEALTH_GAIN": 6,
                    "MANA_GAIN": 24,
                    "HEALTH_TICKS": 6_000,
                    "MANA_TICKS": 6_000,
                },
            ),
        ):
            with self.subTest(base=base):
                base_key = f"oteryn:item.tibia.i{base}"
                active_key = f"oteryn:item.tibia.i{active}"
                inactive, worn = records[base_key], records[active_key]
                for profile, kind, target in (
                    (inactive, "EQUIP", active_key),
                    (worn, "DEEQUIP", base_key),
                ):
                    transforms = profile["use_transform"]["value"]["targets"]
                    pair = next(row for row in transforms if row["kind"] == kind)
                    self.assertEqual(pair["target"]["value"]["key"], target)
                    (pattern,) = profile["equipment"]["value"]["patterns"]["value"]
                    self.assertEqual(pattern["primary_slot"]["value"], "RING")
                temporal = worn["temporal"]["value"]
                self.assertEqual(temporal["duration"]["value"], duration)
                self.assertEqual(
                    temporal["consumption_mode"]["value"],
                    "AUTHORITATIVE_ACTIVE_TIME_BUDGET",
                )
                self.assertIs(temporal["stop_duration"]["value"], True)
                self.assertEqual(inactive["skill_modifiers"], {"state": "UNKNOWN"})
                modifiers = worn["skill_modifiers"]["value"]["modifiers"]["value"]
                self.assertEqual(
                    {r["kind"]: r["parameter"]["value"]["value"] for r in modifiers},
                    effects,
                )
                # Reference Item SkillModifier kinds are ordered by wire ID.
                expected_order = (
                    ["INVISIBILITY"]
                    if active == 3086
                    else [
                        "HEALTH_GAIN",
                        "HEALTH_TICKS",
                        "MANA_GAIN",
                        "MANA_TICKS",
                    ]
                )
                self.assertEqual([row["kind"] for row in modifiers], expected_order)


if __name__ == "__main__":
    unittest.main()
