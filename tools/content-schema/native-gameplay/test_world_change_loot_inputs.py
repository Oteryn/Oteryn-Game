"""Only three existing source loot rows gain their exact native Item closure."""
import copy
import hashlib
import json
import os
from pathlib import Path
import sys
import unittest

import build_world_change_item_profiles as builder
sys.path.insert(0, str(builder.ROOT / "tools/content-migration"))
import world_change_loot_inputs as admission


class WorldChangeLootInputs(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source_root = Path(os.environ.get("OTERYN_WORLD_CHANGE_SOURCE_ROOT", builder.ROOT))
        cls.overlay = builder.build(cls.source_root)
        cls.original = json.loads((cls.source_root / admission.PATH).read_bytes())
        native = json.loads((cls.source_root / "content/items/definitions/spell-native-profiles.json").read_bytes())
        present = {row["production_definition"]["production_key"] for row in native["records"]}
        native["records"] += [row for row in cls.overlay["records"]
                              if row["production_definition"]["production_key"] not in present]
        cls.native = native

    def run_admission(self, native=None, source=None):
        canonical = lambda value: json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
        generated = {"profiles": canonical(native or self.native),
                     "manifest": canonical({"loot_tables": {"sha256": "before"}}),
                     admission.PATH: canonical(source or self.original)}
        admission.apply(self.source_root, generated, "profiles", "manifest", canonical)
        return generated

    def test_exact_three_rows_35_outputs_and_38_facts_with_unchanged_drops(self):
        generated = self.run_admission()
        result = json.loads(generated[admission.PATH])
        self.assertEqual(result["tables"], self.original["tables"])
        self.assertEqual(result["creature_loot"], self.original["creature_loot"])
        selected = {f"oteryn:item.tibia.i{number}" for number in admission.CORPSES}
        for name in admission.TABLES:
            row = next(t for t in result["tables"] if t["identity"]["key"] == f"oteryn:loot.creature.{name}")
            selected.update(entry["item"]["key"] for entry in row["entries"])
        self.assertEqual(len(selected), 38)
        original = {r["item"]["key"]: r for r in self.original["items"]}
        profiles = {r["production_definition"]["production_key"]:r for r in self.native["records"]}
        for facts in result["items"]:
            key = facts["item"]["key"]
            if key not in selected:
                self.assertEqual(facts, original[key])
            else:
                self.assertTrue(facts["materializable"])
                self.assertEqual(facts["stackable"], profiles[key]["admission"]["stack_class"] == "StackCapable")
        corpses = {r["item"]["key"]:r for r in result["items"]}
        self.assertEqual(corpses["oteryn:item.tibia.i21893"]["container_capacity"],36)
        self.assertEqual(json.loads(generated["manifest"])["loot_tables"]["sha256"],
                         hashlib.sha256(generated[admission.PATH]).hexdigest())
        self.assertEqual(self.run_admission(source=result)[admission.PATH], generated[admission.PATH])

    def test_missing_unadmitted_item_changed_drops_and_capacity_clamp_refuse(self):
        for mutation in ("missing", "unadmitted", "drop", "capacity"):
            native = copy.deepcopy(self.native)
            source = copy.deepcopy(self.original)
            row = next(r for r in native["records"] if r["production_definition"]["production_key"] == "oteryn:item.tibia.i21893")
            if mutation == "missing":
                native["records"].remove(row)
            elif mutation == "unadmitted":
                row["admission"]["materializable"] = False
            elif mutation == "capacity":
                row["semantics"]["container"]["value"]["capacity"]["value"] = 30
            else:
                table = next(t for t in source["tables"] if t["identity"]["key"] == "oteryn:loot.creature.seacrest_serpent")
                table["entries"][0]["probability_ppm"] += 1
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                self.run_admission(native, source)


if __name__ == "__main__":
    unittest.main()
