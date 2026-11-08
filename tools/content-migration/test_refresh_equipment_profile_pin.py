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

    def test_collector_census_retains_the_frozen_base_and_exact_added_identities(self):
        census = {'schema': 1, 'base': '1ae39c7fb9bef09c3cd50d16410374f4134b1676',
                  'base_count': 3663, 'added_keys': ['3663', '3664']}
        args = (self.SOURCE, document(*range(3665)), document(), document(0), document(1))
        self.assertEqual(regenerate(*args, custody=census), (self.SOURCE, 3665))
        with self.assertRaisesRegex(ValueError, 'source identity absent'):
            regenerate(*args, custody={**census, 'added_keys': ['3663', 'missing']})
        with self.assertRaisesRegex(ValueError, 'declared cohorts=3664'):
            regenerate(*args, custody={**census, 'added_keys': ['3663']})

    def test_collector_census_rejects_changed_baselines_and_duplicate_additions(self):
        census = {'schema': 1, 'base': '1ae39c7fb9bef09c3cd50d16410374f4134b1676',
                  'base_count': 3663, 'added_keys': ['3663']}
        args = (self.SOURCE, document(*range(3664)), document(), document(0), document(1))
        with self.assertRaisesRegex(ValueError, 'pinned custody baseline'):
            regenerate(*args, custody={**census, 'base_count': 3662})
        with self.assertRaisesRegex(ValueError, 'duplicate custody addition'):
            regenerate(*args, custody={**census, 'added_keys': ['3663', '3663']})


if __name__ == "__main__":
    unittest.main()
