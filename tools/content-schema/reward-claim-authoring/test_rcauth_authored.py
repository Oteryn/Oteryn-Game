import unittest

import rcauth_authored as overlay


def record(key, item):
    return {"definition": {"identity": {"key": key},
                           "placements": [{"reward": overlay._donor_reward(item)}]}}


def donor():
    return [record(key, item) for key, (item, _, _) in overlay.BINDINGS.items()]


class RcauthAuthoredTest(unittest.TestCase):
    def test_apply_binds_text_and_strip_restores_the_donor_record(self):
        bound = overlay.apply(donor())
        for row in bound:
            key = row["definition"]["identity"]["key"]
            reward = row["definition"]["placements"][0]["reward"]
            self.assertEqual(reward["written_text"], {"text": overlay.BINDINGS[key][1]})
        restored, errors = overlay.strip(bound)
        self.assertEqual((restored, errors), (donor(), []))

    def test_changed_donor_reward_or_stale_text_is_refused(self):
        changed = donor()
        changed[0]["definition"]["placements"][0]["reward"]["items"][0]["count"] = 2
        with self.assertRaises(ValueError):
            overlay.apply(changed)
        stale = overlay.apply(donor())
        stale[0]["definition"]["placements"][0]["reward"]["written_text"]["text"] = "other"
        self.assertTrue(overlay.strip(stale)[1])
        self.assertTrue(overlay.strip(donor())[1])


if __name__ == "__main__":
    unittest.main()
