"""Contract tests for the offline elemental balance proposal (no Item mutations)."""
import unittest
from fractions import Fraction
from elemental_balance import propose


class ElementalBalanceTests(unittest.TestCase):
    def test_two_specialized_elements_are_discounted(self):
        result = propose({"FIRE": Fraction(9), "ICE": Fraction(5)})
        self.assertEqual(result["percent"], 3)
        self.assertEqual(result["mode"], "review")
        self.assertEqual(result["old_budget"], 14)
        self.assertEqual(result["new_budget"], 12)

    def test_all_four_equal_is_exact_not_nerfed(self):
        result = propose({element: Fraction(4) for element in ("FIRE", "ICE", "EARTH", "ENERGY")})
        self.assertEqual((result["percent"], result["mode"]), (4, "exact"))

    def test_one_element_stays_specialized(self):
        self.assertIsNone(propose({"FIRE": Fraction(15)}))

    def test_vulnerability_is_never_erased(self):
        self.assertIsNone(propose({"FIRE": Fraction(9), "ICE": Fraction(-5)}))

    def test_field_only_is_never_changed(self):
        self.assertIsNone(propose({"FIRE_FIELD": Fraction(90), "FIRE": Fraction(5), "ICE": Fraction(5)}))

    def test_non_four_element_types_are_ignored(self):
        result = propose({"FIRE": Fraction(9), "ICE": Fraction(5), "PHYSICAL": Fraction(8)})
        self.assertEqual(result["percent"], 3)

    def test_fractions_not_silently_rounded(self):
        self.assertIsNone(propose({"FIRE": Fraction(3, 2), "ICE": Fraction(3)}))

    def test_nonpositive_after_rounding_stays_specialized(self):
        self.assertIsNone(propose({"FIRE": Fraction(1), "ICE": Fraction(1)}))

    def test_three_elements_do_not_increase_average(self):
        result = propose({"FIRE": Fraction(10), "ICE": Fraction(10), "EARTH": Fraction(10)})
        self.assertEqual(result["percent"], 6)
        self.assertLessEqual(result["new_budget"], result["old_budget"])


if __name__ == "__main__":
    unittest.main()
