"""Source replay and closed-admission checks for native world-change item profiles."""
import copy
import hashlib
import json
import os
from pathlib import Path
import sys
import unittest

import build_world_change_item_profiles as builder

sys.path.insert(0, str(builder.ROOT / "tools/content-migration"))
import world_change_profiles as admission


class WorldChangeItemProfiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        source_root = Path(os.environ.get("OTERYN_WORLD_CHANGE_SOURCE_ROOT", builder.ROOT))
        cls.source_root = source_root
        cls.document = builder.build(source_root)

    def test_committed_overlay_reproduces_exact_pinned_sources(self):
        self.assertEqual(builder.OUTPUT.read_bytes(), builder.encoded(self.document))
        self.assertEqual(len(admission.validate(self.document)), 42)
        self.assertEqual(len(admission.validate(self.document)), 43)
        self.assertEqual(self.document["reserved_ids_excluded"], [12064])
        self.assertNotIn("oteryn:item.tibia.i12064", {
            row["production_definition"]["production_key"] for row in self.document["records"]})

    def test_m31_royal_carpet_is_walkable_without_portable_custody(self):
        key = 'oteryn:item.tibia.i2576'
        records = {r['production_definition']['production_key']: r for r in self.document['records']}
        self.assertIn(key, records)
        record = records[key]
        self.assertFalse(record['attributes']['blocks_movement'])
        self.assertFalse(record['attributes']['blocks_projectile'])
        self.assertEqual(record['admission']['legal_destinations'], ['Ground'])
        physical = record['semantics']['physical']['value']
        self.assertEqual(physical['movable'], {'state': 'KNOWN', 'value': False})
        self.assertEqual(physical['pickupable'], {'state': 'KNOWN', 'value': False})
        row = next(r for r in self.document['source_rows'] if r['id'] == 2576)
        self.assertEqual(row['xml_name'], 'red carpet')
        self.assertFalse(row['projected_physical']['block_solid'])
        self.assertFalse(row['projected_physical']['block_pathfind'])
    def test_feroxa_trap_states_keep_exact_source_custody_and_have_no_invented_decay(self):
        records = {row["production_definition"]["production_key"]: row
                   for row in self.document["records"]}
        for number, portable, weight in [(22059, False, 0), (22074, True, 2200)]:
            row = records[f"oteryn:item.tibia.i{number}"]
            self.assertEqual(row["admission"], {
                "materializable": True, "stack_class": "NonStackable",
                "legal_destinations": ["CharacterInventory", "Ground"] if portable else ["Ground"]})
            self.assertEqual(row["semantics"]["physical"]["value"], {
                "weight": {"state": "KNOWN", "value": weight},
                "movable": {"state": "KNOWN", "value": portable},
                "pickupable": {"state": "KNOWN", "value": portable}})
            self.assertEqual(row["semantics"]["temporal"], {"state": "NOT_APPLICABLE"})

    def test_names_ground_speeds_and_tool_portability_come_from_sources(self):
        rows = {row["id"]: row for row in self.document["source_rows"]}
        self.assertEqual({number: rows[number]["projected_physical"]["ground_speed"]
                          for number in {12059,12060,12061,12062,12063,12065}},
                         {12059: 110, 12060: 130, 12061: 150, 12062: 150,
                          12063: 130, 12065: 100})
        for number in range(12059, 12064):
            self.assertEqual(rows[number]["xml_name"], "slime fungus")
            self.assertTrue(rows[number]["projected_physical"]["ground"])
        self.assertEqual(rows[12065]["xml_name"], "stone floor")
        self.assertEqual(rows[12077]["xml_name"], "slime gobbler")
        self.assertFalse(rows[12077]["projected_physical"]["ground"])
        self.assertEqual(rows[12077]["xml_attributes"]["weight"], "1000")
        records = {row["production_definition"]["production_key"]: row
                   for row in self.document["records"]}
        for number in builder.IDS:
            record = records[f"oteryn:item.tibia.i{number}"]
            self.assertEqual(record["semantics"]["presentation"]["value"]["name"],
                             {"state": "KNOWN", "value": rows[number]["xml_name"]})
        self.assertEqual(records["oteryn:item.tibia.i12077"]["semantics"]["physical"]["value"]["weight"],
                         {"state": "KNOWN", "value": 1000})
        self.assertEqual(records["oteryn:item.tibia.i12077"]["admission"]["legal_destinations"],
                         ["CharacterInventory", "Ground"])
        self.assertEqual(records["oteryn:item.tibia.i12059"]["admission"]["legal_destinations"],
                         ["Ground"])

    def test_overhunting_trap_and_captured_wolf_are_exact_source_facts(self):
        rows = {row["id"]: row for row in self.document["source_rows"]}
        records = {row["production_definition"]["production_key"]: row
                   for row in self.document["records"]}
        self.assertEqual(rows[12368]["xml_name"], "magic wolf trap")
        self.assertEqual(rows[12369]["xml_name"], "captured wolf")
        self.assertEqual(rows[12369]["xml_attributes"]["weight"], "5000")
        self.assertEqual(rows[12369]["xml_attributes"]["description"],
                         "The trap has imobilized and shrunken the wolf")
        for number, portable, weight in [(12368, False, 0), (12369, True, 5000)]:
            self.assertFalse(rows[number]["projected_physical"]["ground"])
            record = records[f"oteryn:item.tibia.i{number}"]
            for field in ("movable", "pickupable"):
                self.assertEqual(record["semantics"]["physical"]["value"][field],
                                 {"state": "KNOWN", "value": portable})
            self.assertEqual(record["semantics"]["physical"]["value"]["weight"],
                             {"state": "KNOWN", "value": weight})
            self.assertEqual(record["admission"]["legal_destinations"],
                             ["CharacterInventory", "Ground"] if portable else ["Ground"])
            self.assertEqual(record["admission"]["stack_class"], "NonStackable")
        fabricated = copy.deepcopy(self.document)
        next(row for row in fabricated["records"]
             if row["production_definition"]["production_key"].endswith("i12368"))["semantics"]["physical"]["value"]["movable"]["value"] = True
        with self.assertRaises(ValueError):
            admission.validate(fabricated)

    def test_shared_mission_tools_and_objects_keep_actual_source_physics(self):
        rows = {row["id"]: row for row in self.document["source_rows"]}
        records = {int(row["production_definition"]["production_key"].rsplit(".i", 1)[1]): row
                   for row in self.document["records"]}
        expected = {
            13974: ("strange powder", 500, True), 13976: ("blob bomb", 110, True),
            14018: ("Heart of the Sea", 390, True), 14020: ("soul net", 1250, True),
            14051: ("gooey substance", 120, True), 14052: ("flask of chitin dissolver", 155, True),
            14053: ("flask of dissolved chitin", 155, True), 14054: ("reagent flask", 150, True), 14055: ("aggressive fluid", 150, True),
            14064: ("hive pore", 0, False), 14074: ("insectoid monolith", 0, False),
            14163: ("fossilised coral", 0, False), 14164: ("ramp", 0, False),
            14255: ("swarmer drum", 3200, True),
            21752: ("broken orb of Light Undying", 250, True),
            21803: ("bottle of gloo", 200, True),
            21804: ("the orb of Darkness Eternal", 350, True),
            21805: ("the orb of Light Undying", 350, True),
            21812: ("orb of Life Everchanging", 350, True),
        }
        for number, (name, weight, portable) in expected.items():
            with self.subTest(number=number):
                record = records[number]
                self.assertEqual(rows[number]["xml_name"], name)
                self.assertEqual(record["semantics"]["physical"]["value"]["weight"],
                                 {"state": "KNOWN", "value": weight})
                self.assertEqual(record["admission"], {"materializable": True,
                    "stack_class": "StackCapable" if number in {14052,14053,14054} else "NonStackable",
                    "legal_destinations": ["CharacterInventory", "Ground"] if portable else ["Ground"]})
                self.assertEqual(record["attributes"]["blocks_movement"], number in {14074,14163})
                self.assertEqual(record["attributes"]["blocks_projectile"], False)
                self.assertEqual(record["attributes"]["has_height"], number == 14164)
        # Existing task-board/hireling overlays already own these portable profiles.
        self.assertTrue({3062,14076,14077,21747}.isdisjoint(builder.IDS))
        for number in (14052, 14074, 14164):
            fabricated = copy.deepcopy(self.document)
            record = next(row for row in fabricated["records"]
                          if row["production_definition"]["production_key"].endswith(f"i{number}"))
            if number == 14052:
                record["admission"]["stack_class"] = "NonStackable"
            else:
                record["attributes"]["blocks_movement"] = not record["attributes"]["blocks_movement"]
            with self.assertRaises(ValueError):
                admission.validate(fabricated)

    def test_three_loot_source_corpses_preserve_full_decay_and_source_capacity(self):
        records = {int(row["production_definition"]["production_key"].rsplit(".i",1)[1]):row
                   for row in self.document["records"]}
        chains = [(5963,3990,4103,4104),(13853,13854,13855,13856),(21893,21894,21895,21896)]
        for chain in chains:
            for index, number in enumerate(chain):
                temporal = records[number]["semantics"]["temporal"]["value"]
                self.assertEqual(temporal["duration"], {"state":"KNOWN","value":[10000,300000,300000,60000][index]})
                self.assertEqual(temporal["consumption_mode"]["value"], "DURABLE_ABSOLUTE_DEADLINE")
                if index < 3:
                    self.assertEqual(temporal["decay_target"]["value"]["key"], f"oteryn:item.tibia.i{chain[index+1]}")
                else:
                    self.assertEqual(temporal["decay_target"], {"state":"NOT_APPLICABLE"})
        self.assertEqual(records[21893]["semantics"]["container"]["value"]["capacity"]["value"],36)
        self.assertEqual(records[3583]["semantics"]["presentation"]["value"]["name"]["value"],"dragon ham")
        for number in (13854,21894):
            physical=records[number]["semantics"]["physical"]["value"]
            self.assertEqual(physical["movable"]["value"],True)
            self.assertEqual(physical["pickupable"]["value"],False)
        forged=copy.deepcopy(self.document)
        record=next(row for row in forged["records"] if row["production_definition"]["production_key"].endswith("i21893"))
        record["semantics"]["container"]["value"]["capacity"]["value"]=30
        with self.assertRaisesRegex(ValueError,"WORLD_CHANGE_ITEM_CORPSE_CHAIN"):
            admission.validate(forged)

    def test_missing_duplicate_reserved_and_fabricated_physical_profiles_refuse(self):
        for mutation in ("missing", "duplicate", "reserved", "portable_ground", "blocked"):
            document = copy.deepcopy(self.document)
            if mutation == "missing":
                document["records"].pop()
            elif mutation == "duplicate":
                document["records"].append(copy.deepcopy(document["records"][0]))
            elif mutation == "reserved":
                document["records"][0]["production_definition"]["production_key"] = "oteryn:item.tibia.i12064"
            elif mutation == "portable_ground":
                next(row for row in document["records"] if row["production_definition"]["production_key"] == "oteryn:item.tibia.i12059")["semantics"]["physical"]["value"]["movable"]["value"] = True
            else:
                document["records"][0]["attributes"]["blocks_movement"] = True
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                admission.validate(document)

    def test_unpinned_source_bytes_and_substituted_provenance_refuse(self):
        with self.assertRaisesRegex(ValueError, "WORLD_CHANGE_XML_DIGEST"):
            builder.verify_sources(b"substituted XML", b"substituted appearances")
        with self.assertRaisesRegex(ValueError, "WORLD_CHANGE_APPEARANCE_DIGEST"):
            builder.verify_sources(builder.read_xml(self.source_root), b"substituted appearances")
        document = copy.deepcopy(self.document)
        document["sources"][0]["sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "WORLD_CHANGE_ITEM_PROFILE_SOURCE"):
            admission.validate(document)

    def test_captured_wolf_reuses_existing_owner_and_rejects_conflicting_custody(self):
        canonical = lambda value: json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
        wolf = copy.deepcopy(next(row for row in self.document["records"]
                                  if row["production_definition"]["production_key"] == "oteryn:item.tibia.i12369"))
        wolf["semantics"]["presentation"] = {"state": "UNKNOWN"}
        order = lambda row: row["production_definition"]["production_key"]
        for conflict in (False, True):
            existing = copy.deepcopy(wolf)
            if conflict:
                existing["semantics"]["physical"]["value"]["weight"]["value"] = 1
            generated = {"items": canonical({"records": [existing]}),
                         "manifest": canonical({"item_profiles": {"sha256": "before"}})}
            if conflict:
                with self.assertRaisesRegex(ValueError, "WORLD_CHANGE_ITEM_PROFILE_DUPLICATE"):
                    admission.admit(builder.ROOT, generated, "items", "manifest", order, canonical)
            else:
                admission.admit(builder.ROOT, generated, "items", "manifest", order, canonical)
                records = json.loads(generated["items"])["records"]
                self.assertEqual(len(records), 42)
                self.assertEqual(len(records), 43)
                self.assertEqual(next(row for row in records if row["production_definition"] == wolf["production_definition"]), wolf)

    def test_existing_registry_admission_regenerates_manifest_and_rejects_duplicate(self):
        canonical = lambda value: json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
        generated = {"items": canonical({"records": []}),
                     "manifest": canonical({"item_profiles": {"sha256": "before"}})}
        order = lambda row: row["production_definition"]["production_key"]
        admission.admit(builder.ROOT, generated, "items", "manifest", order, canonical)
        self.assertEqual(len(json.loads(generated["items"])["records"]), 42)
        self.assertEqual(len(json.loads(generated["items"])["records"]), 43)
        self.assertEqual(json.loads(generated["manifest"])["item_profiles"]["sha256"],
                         hashlib.sha256(generated["items"]).hexdigest())
        with self.assertRaisesRegex(ValueError, "WORLD_CHANGE_ITEM_PROFILE_DUPLICATE"):
            admission.admit(builder.ROOT, generated, "items", "manifest", order, canonical)


if __name__ == "__main__":
    unittest.main()
