import unittest
from build_srx3_soul_war_appearances import extract_outfits


LITERALS = '''
waterElementalOutfit = {
 lookType = 286, lookHead = 0, lookBody = 0, lookLegs = 0,
 lookFeet = 0, lookAddons = 0, lookMount = 0,
},
function Creature:applyZoneEffect(var, combat, zoneName)
 local outfitConfig = {
  outfit = { lookType = 242, lookHead = 0, lookBody = 0, lookLegs = 0, lookFeet = 0, lookAddons = 0 },
  time = 7000,
 }
end
'''


class SoulWarAppearanceTests(unittest.TestCase):
    def test_source_water_and_elemental_literals_preserve_explicit_and_default_fields(self):
        rows = extract_outfits(LITERALS)
        self.assertEqual(set(rows), {242, 286})
        self.assertNotIn('lookMount', rows[242])
        self.assertEqual(rows[286]['lookMount'], 0)
        self.assertTrue(all(value == 0 for row in rows.values()
                            for key, value in row.items() if key != 'lookType'))

    def test_duplicate_or_dynamic_source_outfit_is_refused(self):
        for changed in [LITERALS + LITERALS,
                        LITERALS.replace('lookType = 242', 'lookType = player:getOutfit()'),
                        LITERALS.replace('lookType = 286', 'lookType = 285')]:
            with self.subTest(changed=changed[-30:]):
                with self.assertRaises(ValueError):
                    extract_outfits(changed)


if __name__ == '__main__':
    unittest.main()
