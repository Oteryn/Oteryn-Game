import unittest
import tempfile
from pathlib import Path
from unittest.mock import patch

from refresh_equipment_profile_pin import regenerate, declared_cohorts, source_speed_pin, TEST


def document(*ids):
    return {
        "records": [{"production_definition": {"production_key": str(i)}} for i in ids]
    }


class EquipmentPinTests(unittest.TestCase):
    def test_source_speed_pin_retains_unknown_pick_and_rejects_other_unknowns(self):
        packet = document('oteryn:item.tibia.i3456', 'oteryn:item.tibia.i123')
        packet['records'][0]['attributes'] = {}
        packet['records'][1]['attributes'] = {'speed_bonus': 0}
        source = TEST.read_text()
        fixed = source_speed_pin(source, packet)
        self.assertIn('("qengine-reward-item-profiles.json", 2)', fixed)
        self.assertIn('assert_eq!(actual.attributes.speed_bonus, None', fixed)
        self.assertIn('assert_eq!(actual.attributes.speed_bonus, Some(0)', fixed)
        self.assertEqual(source_speed_pin(fixed, packet), fixed)
        packet['records'][1]['attributes'] = {}
        with self.assertRaisesRegex(ValueError, 'unknown-speed pick'):
            source_speed_pin(source, packet)
        packet['records'][1]['attributes'] = {'speed_bonus': 1}
        with self.assertRaisesRegex(ValueError, 'unknown-speed pick'):
            source_speed_pin(source, packet)

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

    def test_source_identity_expression_is_counted_and_unknown_expressions_refuse(self):
        shared = [1791, 1948, 2822, 2823, 2848, 3593, 5102, 5103, 21982]
        with tempfile.TemporaryDirectory() as temporary:
            test = Path(temporary) / 'ability_tests.rs'
            test.with_name('questpc2_source_profile_cohort.rs').write_text(
                'const IDS: &[u16] = &[' + ','.join(map(str, shared + [100, 101])) + '];\n'
                'const SHARED_WITH_MAIN: &[u16] = &[' + ','.join(map(str, shared)) + '];\n')
            source = self.SOURCE.replace('[("QNPCFX", 1)];', '''[
    ("QNPCFX", 1),
    ("QUESTPC2-SOURCE-CLOSURE", questpc2_source_profile_cohort::IDS.len()
        - questpc2_source_profile_cohort::SHARED_WITH_MAIN.len()),
];''')
            with patch('refresh_equipment_profile_pin.TEST', test):
                self.assertEqual(declared_cohorts(source), {
                    'QNPCFX': 1, 'QUESTPC2-SOURCE-CLOSURE': 2})
                with self.assertRaisesRegex(ValueError, 'unresolved declared profile cohort'):
                    declared_cohorts(source.replace('QUESTPC2-SOURCE-CLOSURE', 'UNKNOWN-COHORT'))

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
    def test_merged_census_uses_frozen_keys_and_inserts_only_declared_cohort(self):
        source = self.SOURCE.replace(
            '[("QNPCFX", 1)];',
            '[\n    ("QNPCFX", 1),\n    ("QUICKLOOT-1", 0),\n];',
        ).replace(
            "    assert_eq!(\n        profiles.records.len(),\n"
            "        ORIGINAL_PROFILES + PROFILE_ADDITIONS.iter().map(|(_, n)| n).sum::<usize>()\n"
            "            + npc_additions.len()\n    );",
            "    assert_eq!(profiles.records.len(), baseline + custody_keys.len());",
        )
        frozen = ({str(i) for i in range(7)}, 2)
        fixed, count = regenerate(source, document(0, 1, 2, 3, 4, 5), document(6),
                                  document(0), document(1), frozen=frozen)
        self.assertEqual(count, 7)
        self.assertIn('("QUESTPC8-MERGED", 2)', fixed)
        self.assertIn('baseline + custody_keys.len()', fixed)
        self.assertIn('assert_eq!(venison.attributes.speed_bonus, Some(0))', fixed)
        self.assertEqual(regenerate(fixed, document(0, 1, 2, 3, 4, 5), document(6),
                                    document(0), document(1), frozen=frozen), (fixed, 7))
        with self.assertRaisesRegex(ValueError, "frozen published union"):
            regenerate(source, document(0, 1, 2, 3, 4, 8), document(6),
                       document(0), document(1), frozen=frozen)

    def test_declared_new_recipe_profile_preserves_baseline_partition(self):
        source = self.SOURCE.replace(
            '[("QNPCFX", 1)];',
            '[\n    ("QNPCFX", 1),\n    ("QUICKLOOT-1", 0),\n];',
        ) + """
    let later_main_profiles = PROFILE_ADDITIONS.iter().filter(|(name, _)| matches!(*name, "QUESTPC8-MERGED")).count();
    assert_eq!(later_main_profiles, 999);
    let baseline = 3663;
    assert_eq!(baseline, 3663);
"""
        fixed, count = regenerate(source, document(0, 1, 2, 3, 4, 5), document(6),
                                  document(0), document(1), frozen=({str(i) for i in range(7)}, 2))
        self.assertEqual(count, 7)
        self.assertIn('assert_eq!(later_main_profiles, 3);', fixed)
        self.assertIn('assert_eq!(baseline, 3663);', fixed)

    def test_declared_pick_overlap_requires_both_source_owners(self):
        source = self.SOURCE.replace(
            'ORIGINAL_PROFILES: usize = 1', 'ORIGINAL_PROFILES: usize = 2')
        source += '\n- 1 // the pick already belongs to the base provider\n'
        pick = 'oteryn:item.tibia.i3456'
        _, count = regenerate(source, document(pick, 1, 2, 3),
                              document(4), document(pick), document(1))
        self.assertEqual(count, 5)
        with self.assertRaisesRegex(ValueError, 'pick overlap'):
            regenerate(source, document(0, 1, 2, 3),
                       document(4), document(pick), document(1))

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

    def test_new_source_cohort_does_not_change_immutable_baseline_or_drop_engine_pin(self):
        source = self.SOURCE.replace('[("QNPCFX", 1)];',
            '[\n    ("QNPCFX", 1),\n    ("QUICKLOOT-1", 0),\n    ("QPC7-CORPSE", 2),\n];')
        source += '''
    let later_main_profiles = 0;
    const QENGINE2_BASE_PROFILES: usize = 1;
    assert_eq!(additions.len(), QENGINE2_BASE_PROFILES);
    let baseline = 3663;
    assert_eq!(baseline, 3663);
'''
        census = {'schema': 1, 'base': '1ae39c7fb9bef09c3cd50d16410374f4134b1676',
                  'base_count': 3663, 'added_keys': ['3663', '3664']}
        kwargs = dict(custody=census, frozen=({str(i) for i in range(3665)}, 2),
                      baseline_cohorts={'QUICKLOOT-1'}, baseline_qengine_count=1)
        args = (document(*range(3665)), document(), document(0), document(1))
        fixed, count = regenerate(source, *args, **kwargs)
        self.assertEqual(count, 3665)
        self.assertIn('"QPC7-CORPSE"', fixed.split('let later_main_profiles =')[1].split('let baseline =')[0])
        self.assertIn('const QENGINE2_BASE_PROFILES: usize = 1;', fixed)
        self.assertIn('assert_eq!(baseline, 3663);', fixed)
        self.assertEqual(regenerate(fixed, *args, **kwargs), (fixed, count))


if __name__ == "__main__":
    unittest.main()
