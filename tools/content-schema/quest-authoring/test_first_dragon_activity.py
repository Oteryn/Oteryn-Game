"""M18's selected final fight retains exact source identities and bounded timers."""
import unittest
import first_dragon_activity as activity


class FirstDragonActivityTest(unittest.TestCase):
    def test_committed_packet_matches_generator(self):
        self.assertEqual(activity.TARGET.read_bytes(), activity.canonical(activity.build()) + b'\n')

    def test_selected_stage_keeps_donor_timings_and_native_profile_fingerprint(self):
        packet = activity.build()
        candidate = packet['candidate']
        self.assertEqual(packet['basis'], 'CHOSEN_OTERYN_APPROXIMATION')
        self.assertEqual(packet['stage'], 's5')
        self.assertEqual(candidate['creature_key'], activity.CREATURE)
        self.assertEqual(candidate['minimum_level'], 40)
        self.assertEqual(candidate['cooldown_seconds'], 72000)
        self.assertEqual(candidate['time_limit_seconds'], 1200)
        self.assertEqual(len(candidate['entry_positions']), 15)
        self.assertEqual(len(candidate['native_profile_sha256']), 64)
        expected = dict(packet)
        expected['candidate'] = dict(candidate)
        expected['candidate'].pop('sha256')
        self.assertEqual(candidate['sha256'], activity.sha(activity.canonical(expected)))


if __name__ == '__main__':
    unittest.main()
