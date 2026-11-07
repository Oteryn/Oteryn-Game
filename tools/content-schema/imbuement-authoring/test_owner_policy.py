"""Protect approved behavior and the distinction from public Global evidence."""
import json
import unittest

import owner_policy


class OwnerPolicyTests(unittest.TestCase):
    def setUp(self):
        self.packet = json.loads(owner_policy.PACKET.read_text())

    def test_approved_policy_validates(self):
        owner_policy.validate(self.packet)

    def test_approval_cannot_activate_runtime_or_certify_global(self):
        self.packet["full_global_parity_proven"] = True
        with self.assertRaisesRegex(ValueError, "certify Global"):
            owner_policy.validate(self.packet)

    def test_owner_selection_cannot_become_public_observation(self):
        self.packet["decisions"]["etcher_consumption"]["global_public_confirmation"] = True
        with self.assertRaisesRegex(ValueError, "public Global observation"):
            owner_policy.validate(self.packet)

    def test_late_crash_consumption_is_not_approved_by_zero_rejection(self):
        self.packet["decisions"]["filled_scroll_consumption"]["late_internal_failure_or_crash_net_consumption"] = 0
        with self.assertRaisesRegex(ValueError, "late failure"):
            owner_policy.validate(self.packet)

    def test_armor_order_cannot_be_reversed(self):
        self.packet["decisions"]["physical_armor_and_percentage_order"]["stage_order"].reverse()
        with self.assertRaisesRegex(ValueError, "armor order"):
            owner_policy.validate(self.packet)

    def test_zero_hits_and_empty_aoe(self):
        self.assertEqual(owner_policy.illustrated_life_heal([100, 0], 2500, 1000), 25)
        self.assertEqual(owner_policy.illustrated_life_heal([0, 0], 2500, 1000), 0)

    def test_unequal_hits_and_historical_rounding(self):
        self.assertEqual(owner_policy.illustrated_life_heal([100, 900], 2500, 1000), 138)
        self.assertEqual(owner_policy.illustrated_life_heal([387, 387], 2500, 1000), 108)

    def test_missing_health_cap_and_damage_basis(self):
        self.assertEqual(owner_policy.illustrated_life_heal([1000], 2500, 30), 30)
        self.packet["decisions"]["life_leech_damage_basis"]["overkill_damage_counts"] = False
        with self.assertRaisesRegex(ValueError, "damage basis"):
            owner_policy.validate(self.packet)

    def test_pz_does_not_start_a_new_sixty_second_deadline(self):
        self.packet["decisions"]["imbuement_timer_pz_rules"]["combat_imbuements"]["residual_origin"] = "PZ_ENTRY"
        with self.assertRaisesRegex(ValueError, "deadline"):
            owner_policy.validate(self.packet)


if __name__ == "__main__":
    unittest.main()
