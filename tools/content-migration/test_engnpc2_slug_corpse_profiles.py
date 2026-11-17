"""Native Slug death must yield an admitted first corpse and exact decay chain."""
import sys
import unittest
import engnpc2_slug_corpse_profiles as corpse
sys.path.insert(0, str(corpse.ROOT / 'tools/content-schema/native-gameplay'))
from build_full_spell_artifact import native_corpse_capacity, native_loot_yield_facts


class SlugCorpse(unittest.TestCase):
    def test_source_chain_is_ground_only_and_retains_exact_first_form(self):
        records = corpse.capture()['records']
        self.assertEqual([int(r['production_binding']['external_id']) for r in records], list(corpse.IDS))
        for row, duration in zip(records, (10000, 300000, 300000, 60000)):
            self.assertEqual(row['admission'], {'materializable': True, 'stack_class': 'NonStackable', 'legal_destinations': ['Ground']})
            self.assertEqual(row['semantics']['temporal']['value']['duration'], {'state': 'KNOWN', 'value': duration})
            self.assertIsNone(native_loot_yield_facts(row['production_binding']['target'], row))
        self.assertEqual(native_corpse_capacity(records[0]['production_binding']['target'], records[0]), 10)

    def test_registered_native_profiles_admit_the_real_slug_death(self):
        import json
        rows = {r['production_definition']['production_key']: r for r in json.loads(
            (corpse.ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']}
        for item in corpse.IDS:
            self.assertTrue(f'oteryn:item.tibia.i{item}' in rows, f'absent native profile: {item}')
        first = rows['oteryn:item.tibia.i12526']
        self.assertEqual(native_corpse_capacity(first['production_binding']['target'], first), 10)

    def test_cohort_is_source_checked_and_idempotent(self):
        packet = corpse.capture()
        source = 'const PROFILE_ADDITIONS: &[(&str, usize)] = &[\n];\n'
        updated = corpse.refresh_cohort(source, packet)
        self.assertIn('("ENGNPC2-SLUG-CORPSES", 4)', updated)
        self.assertEqual(corpse.refresh_cohort(updated, packet), updated)
        packet['records'].pop()
        with self.assertRaisesRegex(ValueError, 'ENGNPC2_CORPSE_PIN_COHORT'):
            corpse.refresh_cohort(source, packet)
