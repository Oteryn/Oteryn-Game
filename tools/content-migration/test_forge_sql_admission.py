"""The database policy must be a checked projection of admitted Content and rules."""
import unittest

import forge_sql_admission as forge


class ForgeSqlAdmissionTests(unittest.TestCase):
    def test_committed_projection_matches_all_admitted_profiles_and_prices(self):
        actual = forge.MIGRATION.read_text()
        start = actual.index(forge.BEGIN)
        end = actual.index(forge.END, start) + len(forge.END)
        self.assertEqual(actual[start:end], forge.generated())

    def test_current_native_class_caps_and_exact_transfer_prices_are_preserved(self):
        profiles, prices, materials = forge.policy()
        by_key = {row[0]: row[1:] for row in profiles}
        self.assertEqual(by_key['oteryn:item.tibia.i3264'], ('definition-r1', 1, 1, 'hand'))
        self.assertEqual(by_key['oteryn:item.tibia.i34082'], ('definition-r1', 4, 10, 'hand'))
        self.assertNotIn('oteryn:item.tibia.i37110', by_key)
        self.assertNotIn('oteryn:item.tibia.i51475', by_key)
        self.assertEqual(materials, [('oteryn:item.tibia.i37109', 'definition-r1'),
                                     ('oteryn:item.tibia.i37110', 'definition-r1')])
        self.assertIn((4, 1, 8_000_000, 1, 55_000_000, 65_000_000), prices)
        self.assertIn((4, 10, 15_000_000_000, 85, 125_000_000_000, 300_000_000_000), prices)


if __name__ == '__main__':
    unittest.main()
