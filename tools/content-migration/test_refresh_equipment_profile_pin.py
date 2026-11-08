import unittest
import copy
import json
import tempfile
from pathlib import Path
from unittest.mock import patch

import refresh_equipment_profile_pin as refresh

from refresh_equipment_profile_pin import regenerate


def document(*ids):
    return {
        "records": [{"production_definition": {"production_key": str(i)}} for i in ids]
    }


class EquipmentPinTests(unittest.TestCase):
    SOURCE = """const ORIGINAL_PROFILES: usize = 1;
const PROFILE_ADDITIONS: &[(&str, usize)] = &[("QNPCFX", 1)];
// cohorts add 1 + 1 keys
    assert_eq!(
        profiles.records.len(),
        ORIGINAL_PROFILES + PROFILE_ADDITIONS.iter().map(|(_, n)| n).sum::<usize>()
            + npc_additions.len()
    );
    assert_eq!(venison.attributes.speed_bonus, Some(0));
"""

    def generate(self, source=None, base=None):
        return regenerate(
            source or self.SOURCE,
            base or document(0, 1, 2, 3),
            document(4),
            document(0),
            document(1),
        )

    def test_registered_npc_is_counted_once_without_removing_identity_checks(self):
        fixed, count = self.generate()
        self.assertEqual(count, 5)
        self.assertNotIn("+ npc_additions.len()", fixed)
        self.assertIn("assert_eq!(venison.attributes.speed_bonus, Some(0))", fixed)
        self.assertEqual(self.generate(fixed), (fixed, 5))

    def test_missing_actual_profile_does_not_lower_the_declared_pin(self):
        with self.assertRaisesRegex(ValueError, "declared cohorts=5"):
            self.generate(base=document(0, 1, 2))

    def test_source_cohort_disagreement_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "NPC source proof"):
            self.generate(self.SOURCE.replace('("QNPCFX", 1)', '("QNPCFX", 2)'))

    def test_starting_loadout_overlay_is_included_in_the_source_union(self):
        source = self.SOURCE.replace(
            "ORIGINAL_PROFILES: usize = 1", "ORIGINAL_PROFILES: usize = 2"
        )
        _, count = regenerate(
            source,
            document(0, 1, 2, 3),
            document(4),
            document(0),
            document(1),
            document(5),
        )
        self.assertEqual(count, 6)

    def test_existing_registered_engine_is_counted_once(self):
        source = self.SOURCE.replace('("QNPCFX", 1)', '("QNPCFX", 1), ("QENGINE-REWARDS", 1)')
        source = source.replace('            + npc_additions.len()',
                                '            + (additions.len() - 1)\n            + npc_additions.len()')
        fixed, count = self.generate(source)
        self.assertEqual(count, 5)
        self.assertIn('(additions.len() - 1)', fixed)
        with self.assertRaisesRegex(ValueError, 'registered QENGINE cohort'):
            self.generate(source.replace('additions.len() - 1', 'additions.len() - 2'))

    def test_duplicate_profile_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "duplicate production key"):
            self.generate(base=document(0, 0, 1, 2, 3))


    def srx3_source(self):
        return self.SOURCE.replace("            + npc_additions.len()", "            + additions.len()\n            + npc_additions.len()")

    def test_source_packet_minus_declared_prior_identities_adds_only_new_keys(self):
        proof = [("SRX3-TEST", document(2, 5), {"2"})]
        fixed, count = regenerate(self.srx3_source(), document(0, 1, 2, 3, 5),
                                  document(4), document(0), document(1), srx3=proof)
        self.assertEqual(count, 6)
        self.assertIn('(\"SRX3-TEST\", 1)', fixed)
        self.assertIn('assert_eq!(venison.attributes.speed_bonus, Some(0))', fixed)
        self.assertEqual(regenerate(fixed, document(0, 1, 2, 3, 5), document(4),
                                    document(0), document(1), srx3=proof), (fixed, 6))

    def test_registered_engine_subtraction_composes_with_source_cohorts(self):
        source = self.srx3_source().replace(
            '("QNPCFX", 1)', '("QNPCFX", 1), ("QENGINE-REWARDS", 1)'
        ).replace('+ additions.len()', '+ (additions.len() - 1)')
        proof = [("SRX3-TEST", document(2, 5), {"2"})]
        args = (document(0, 1, 2, 3, 5), document(4), document(0), document(1))
        fixed, count = regenerate(source, *args, srx3=proof)
        self.assertEqual(count, 6)
        self.assertIn('+ (additions.len() - 1)', fixed)
        self.assertIn('+ SRX3_SOURCE_COHORTS.iter()', fixed)
        self.assertIn('assert_eq!(venison.attributes.speed_bonus, Some(0))', fixed)
        self.assertEqual(regenerate(fixed, *args, srx3=proof), (fixed, 6))

    def test_missing_new_source_profile_is_not_replaced_by_a_lower_count(self):
        with self.assertRaisesRegex(ValueError, "SRX3 source profile differs"):
            regenerate(self.srx3_source(), document(0, 1, 2, 3), document(4),
                       document(0), document(1), srx3=[("SRX3-TEST", document(2, 5), {"2"})])

    def test_unproven_actual_key_does_not_increase_source_expectation(self):
        with self.assertRaisesRegex(ValueError, "declared cohorts=6, generated profile union=7"):
            regenerate(self.srx3_source(), document(0, 1, 2, 3, 5, 6), document(4),
                       document(0), document(1), srx3=[("SRX3-TEST", document(2, 5), {"2"})])

    def test_changed_new_profile_provenance_is_refused(self):
        actual = document(0, 1, 2, 3, 5)
        actual["records"][-1]["production_binding"] = {"disposition": "forged"}
        with self.assertRaisesRegex(ValueError, "SRX3 source profile differs"):
            regenerate(self.srx3_source(), actual, document(4), document(0), document(1),
                       srx3=[("SRX3-TEST", document(2, 5), {"2"})])

    def test_false_prior_intersection_and_overlapping_source_cohorts_are_refused(self):
        with self.assertRaisesRegex(ValueError, "source-base intersection"):
            regenerate(self.srx3_source(), document(0, 1, 2, 3, 5), document(4), document(0), document(1),
                       srx3=[("SRX3-TEST", document(2, 5), {"3"})])
        with self.assertRaisesRegex(ValueError, "source cohorts overlap"):
            regenerate(self.srx3_source(), document(0, 1, 2, 3, 5), document(4), document(0), document(1),
                       srx3=[("SRX3-A", document(2, 5), {"2"}), ("SRX3-B", document(5), set())])


    def coowned(self, *, source=None, actual=None, proof=None, coowner=None, key="5"):
        source = source or self.srx3_source().replace(
            '("QNPCFX", 1)', '("QNPCFX", 1), ("W10-PRIMAL", 1)')
        return regenerate(source, actual or document(0, 1, 2, 3, key), document(4),
                          document(0), document(1),
                          srx3=[("SRX3-TEST", proof or document(2, key), {"2"})],
                          coowners=[("W10-PRIMAL", coowner or document(key), {key})])

    def test_independently_counted_coowner_is_not_counted_again(self):
        fixed, count = self.coowned()
        self.assertEqual(count, 6)
        self.assertIn('("SRX3-TEST", 0)', fixed)
        self.assertIn('("W10-PRIMAL", 1)', fixed)
        self.assertEqual(self.coowned(source=fixed), (fixed, 6))

    def test_missing_coowned_profile_is_refused(self):
        with self.assertRaisesRegex(ValueError, "coowner identity set"):
            self.coowned(actual=document(0, 1, 2, 3))

    def test_coowner_declared_count_must_match_its_independent_source(self):
        source = self.srx3_source().replace(
            '("QNPCFX", 1)', '("QNPCFX", 1), ("W10-PRIMAL", 2)')
        with self.assertRaisesRegex(ValueError, "coowner declaration"):
            self.coowned(source=source)

    def test_changed_shared_semantics_cannot_be_hidden_by_matching_actual_and_srx3(self):
        actual, proof = document(0, 1, 2, 3, 5), document(2, 5)
        actual['records'][-1]['semantics'] = {'invented': True}
        proof['records'][-1]['semantics'] = {'invented': True}
        with self.assertRaisesRegex(ValueError, "coowner profile differs"):
            self.coowned(actual=actual, proof=proof)

    def test_only_four_closed_candidate_revision_aliases_are_allowed(self):
        import copy
        for item in (39527, 39528, 39529, 39530):
            key = f'oteryn:item.tibia.i{item}'
            actual, proof, coowner = document(0, 1, 2, 3, key), document(2, key), document(key)
            for doc, revision in [(actual, 'srx3-boss-corpses-crystal-201189f'),
                                  (proof, 'srx3-boss-corpses-crystal-201189f'),
                                  (coowner, 'spell-p2-r21')]:
                doc['records'][-1]['authoring'] = {'item': {
                    'family': 'Item', 'key': f'candidate:item/{item}', 'revision': revision}}
            self.assertEqual(self.coowned(key=key, actual=actual, proof=proof, coowner=coowner)[1], 6)
            for field, value in [('extra', 'unqualified'), ('revision', 'forged')]:
                broken = copy.deepcopy(coowner)
                if field == 'revision': broken['records'][-1]['authoring']['item'][field] = value
                else: broken['records'][-1]['authoring'][field] = value
                with self.assertRaisesRegex(ValueError, "coowner profile differs"):
                    self.coowned(key=key, actual=actual, proof=proof, coowner=broken)
        key = 'oteryn:item.tibia.i39176'
        actual, proof, coowner = document(0, 1, 2, 3, key), document(2, key), document(key)
        for doc, revision in [(actual, 'srx3-boss-corpses-crystal-201189f'),
                              (proof, 'srx3-boss-corpses-crystal-201189f'),
                              (coowner, 'spell-p2-r21')]:
            doc['records'][-1]['authoring'] = {'item': {
                'family': 'Item', 'key': 'candidate:item/39176', 'revision': revision}}
        with self.assertRaisesRegex(ValueError, "coowner profile differs"):
            self.coowned(key=key, actual=actual, proof=proof, coowner=coowner)

    def test_coowner_must_actually_overlap_the_qualified_srx3_owner(self):
        with self.assertRaisesRegex(ValueError, "coowner source overlap"):
            self.coowned(proof=document(2))




class M25SourceCohortTests(unittest.TestCase):
    def test_primal_pod_replays_the_authoritative_registration_top_order(self):
        cohorts = refresh.source_cohorts(refresh.ROOT)
        document = next(document for name, document, _ in cohorts
                        if name == "SRX3-PRIMAL-POD")
        row = next(row for row in document["records"]
                   if row["production_definition"]["production_key"] == "oteryn:item.tibia.i39176")
        house = refresh.importlib.import_module("house_decoration_actions")
        overlay = json.loads((refresh.ROOT / house.PROFILES).read_bytes())
        self.assertEqual(overlay["always_on_top"]["39176"], 3)
        self.assertIs(row["attributes"].get("always_on_top"), True)
        self.assertEqual(row["attributes"].get("always_on_top_order"), 3)
        # Registration adds source order, never a new identity or custody rule.
        self.assertEqual(refresh.keys(document), {"oteryn:item.tibia.i39176"})
        self.assertEqual(row["admission"], {"materializable": True,
            "stack_class": "NonStackable", "legal_destinations": ["Ground"]})

    def test_m25_records_are_independently_qualified_before_counting(self):
        declaration = ("M25-GALTHEN-CORPSES", "m25_corpse_items", ())
        self.assertIn(declaration, refresh.SRX3_SOURCES)
        cohorts = refresh.source_cohorts(refresh.ROOT)
        selected = [row for row in cohorts if row[0] == declaration[0]]
        self.assertEqual(len(selected), 1)
        _, document, previous = selected[0]
        self.assertEqual(previous, set())
        self.assertEqual(refresh.keys(document), {
            f"oteryn:item.tibia.i{item}" for item in
            (42069, 42070, 42071, 42072, 44684, 44685, 44686, 44687)
        })
        owner = refresh.importlib.import_module("m25_corpse_items")
        proven, _ = owner.checked_packet(refresh.ROOT)
        taxonomy = {row["target"]["key"]: row["source_taxonomy"]
                    for row in json.loads((refresh.ROOT / "content/items/taxonomy/items.json").read_bytes())["records"]}
        expected = copy.deepcopy(proven)
        for row in expected["records"]:
            key = row["production_definition"]["production_key"]
            if key in taxonomy and "taxonomy" not in row["authoring"]:
                row["authoring"]["taxonomy"] = copy.deepcopy(taxonomy[key])
        house = refresh.importlib.import_module("house_decoration_actions")
        overlay = json.loads((refresh.ROOT / house.PROFILES).read_bytes())
        house.apply_top_order(expected["records"], overlay)
        self.assertEqual(document, {"records": sorted(expected["records"], key=lambda row: row["production_definition"]["production_key"])})

    def test_m25_corrupt_source_packet_never_becomes_a_count_only_cohort(self):
        # Load the ordinary owner chain first, as the production helper does.
        refresh.source_cohorts(refresh.ROOT)
        owner = refresh.importlib.import_module("m25_corpse_items")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in (owner.PATH, owner.PACKET, owner.PIN_PATH,
                         "content/items/taxonomy/items.json"):
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes((refresh.ROOT / name).read_bytes())
            data = json.loads((root / owner.PATH).read_bytes())
            data["records"].pop()
            (root / owner.PATH).write_bytes(owner.encode(data))
            with patch.object(refresh, "SRX3_SOURCES", (("M25-GALTHEN-CORPSES", "m25_corpse_items", ()),)):
                with self.assertRaisesRegex(ValueError, "M25_CAPTURE_DRIFT"):
                    refresh.source_cohorts(root)

if __name__ == "__main__":
    unittest.main()
