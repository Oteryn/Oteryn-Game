import unittest

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


if __name__ == "__main__":
    unittest.main()
