import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import unittest
from qualify_monster_wiki_fields import qualify, count_value, received_damage_percent
from fractions import Fraction

class ActualRegressionTests(unittest.TestCase):
    def test_br_magma_bubble_experience_uses_thousands_not_eighty(self):
        self.assertEqual(qualify('Magma Bubble', 'Magma Bubble', 'experience', '80.000', 'pt-BR')['value'], 80000)
        self.assertEqual(count_value('1.234.567', 'pt-BR'), 1234567)

    def test_dragon_pack_does_not_supply_dragon_hoard_stats(self):
        fact = qualify('Dragon Hoard', 'Dragon Pack', 'experience', '0', 'pt-BR')
        self.assertEqual(fact['reason'], 'PAGE_IDENTITY_MISMATCH')
        self.assertNotIn('value', fact)

    def test_percent_not_count_has_decimal_semantics(self):
        self.assertEqual(received_damage_percent('1.23%'), Fraction(123, 100))
        self.assertEqual(received_damage_percent('80,5%'), Fraction(161, 2))
        self.assertIsNone(count_value('1.23', 'pt-BR'))

    def test_missing_and_uncertain_are_not_neutral_zero(self):
        for raw in [None, '', '?', '100%?', '0-100%', '~100%']:
            self.assertIsNone(received_damage_percent(raw))
        self.assertEqual(received_damage_percent('100%'), 100)
        self.assertEqual(received_damage_percent('0%'), 0)

    def test_variant_title_mismatch_is_not_an_alias_guess(self):
        self.assertEqual(qualify('Pink Butterfly', 'Butterfly (Purple)', 'armor', '0', 'en')['reason'], 'PAGE_IDENTITY_MISMATCH')
        self.assertEqual(qualify("Fallen Mooh'Tah Master Ghar", "Fallen Mooh'Tah Master Ghar", 'armor', '10', 'en')['value'], 10)

if __name__ == '__main__':
    unittest.main()
