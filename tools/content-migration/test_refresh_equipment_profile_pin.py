import unittest

from refresh_equipment_profile_pin import regenerate, refresh_npcfix_cohort


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

    def test_shared_captured_sugar_keys_are_counted_once(self):
        source = '// i3456/i6528. original\n("NPCFIX", 4),'
        source = "    " + source
        npcfix = document("oteryn:item.tibia.i3456", "oteryn:item.tibia.i6528", 1, 2, 3, 4)
        fixed = refresh_npcfix_cohort(source, npcfix, document(1, 2, 3, 9))
        self.assertIn('("NPCFIX", 1),', fixed)
        self.assertIn('Sugar shares 3 captured keys', fixed)
        self.assertEqual(refresh_npcfix_cohort(fixed, npcfix, document(1, 2, 3, 9)), fixed)

    def test_shared_sources_never_invent_baseline_membership(self):
        with self.assertRaisesRegex(ValueError, "baseline source keys missing"):
            refresh_npcfix_cohort('("NPCFIX", 4),', document(1, 2), document(1))

    def test_duplicate_profile_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "duplicate production key"):
            self.generate(base=document(0, 0, 1, 2, 3))


if __name__ == "__main__":
    unittest.main()
