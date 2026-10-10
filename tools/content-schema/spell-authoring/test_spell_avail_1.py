"""SPELL-AVAIL-1: locate_message, companion_haste and mass_spirit_mend donor rows are bound, not held."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
IMPORTS = ROOT / "content/abilities/source-imports"
CATALOG = ROOT / "tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"

# mechanism -> (native key, [(source_id, donor file)])
MECHANISMS = {
    "locate_message": ("locate_message", [
        ("canary-main-current", "support/find_person.lua"), ("crystal-summer-current", "support/find_person.lua"),
        ("canary-main-current", "support/find_fiend.lua"), ("crystal-summer-current", "support/find_fiend.lua")]),
    "companion_haste": ("companion_haste", [("canary-main-current", "support/swift_foot.lua")]),
    "mass_spirit_mend": ("mass_spirit_mend", [
        ("canary-main-current", "healing/mass_spirit_mend.lua"),
        ("crystal-summer-current", "healing/mass_spirit_mend.lua")]),
}


def registration(source_id, donor):
    return "%s/data/scripts/spells/%s#1" % (source_id, donor)


class SpellAvail1Test(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        sources = json.loads((IMPORTS / "current-sources.json").read_text())
        cls.bindings = {b["registration"]: b for b in sources["bindings"]}
        cls.held = {json.loads(line)["registration"] for line in (IMPORTS / "unavailable-spells.jsonl").read_text().splitlines()}
        cls.natives = {e["bundle"]["spell"]["identity"]["key"]: e["bundle"]["spell"]["execution"].get("native_behavior", {}).get("key")
                       for e in json.loads(CATALOG.read_text())["bundles"]}

    def test_each_mechanism_is_bound_and_released(self):
        for mechanism, (native, donors) in MECHANISMS.items():
            for source_id, donor in donors:
                key = registration(source_id, donor)
                with self.subTest(mechanism=mechanism, registration=key):
                    self.assertNotIn(key, self.held)
                    binding = self.bindings[key]
                    self.assertEqual(binding["projection"], "identity_only")
                    self.assertEqual(self.natives[binding["target"]["key"]], native)
                    qualification = binding["qualification"]
                    self.assertEqual(qualification["scope"], "accepted_normalized_model")
                    self.assertFalse(qualification["runtime_activation"])
                    self.assertFalse(qualification["raw_source_parity"])
                    self.assertTrue(qualification["preserved_fields"])

    def test_conflicts_retain_accepted_value(self):
        for mechanism, (_, donors) in MECHANISMS.items():
            for source_id, donor in donors:
                for field in self.bindings[registration(source_id, donor)]["qualification"]["preserved_fields"]:
                    with self.subTest(mechanism=mechanism, path=field["path"]):
                        self.assertTrue(field["path"].startswith("/spell/"))
                        self.assertTrue(field["reason"].startswith("S24_"))

    def test_out_of_scope_rows_stay_held(self):
        for key in ("crystal-main-current/data/scripts/spells/healing/mass_spirit_mend.lua#1",
                    "crystal-main-current/data/scripts/spells/support/swift_foot.lua#1",
                    "crystal-summer-current/data/scripts/spells/support/swift_foot.lua#1",
                    "canary-main-current/data/scripts/spells/support/magic_shield.lua#1"):
            self.assertIn(key, self.held)


if __name__ == "__main__":
    unittest.main()
