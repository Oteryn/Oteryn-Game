import unittest

import capture_yalahar


class PalimuthConditionTests(unittest.TestCase):
    def test_palimuth_job_fix_is_exact_and_does_not_rewrite_other_conditions(self):
        source = 'if MsgContains(message, "job") and not player:getStorageValue(Storage.Quest.U8_4.InServiceOfYalahar.Questline) == 54 then'
        fixed = capture_yalahar.repair_palimuth_job_line(source)
        self.assertEqual(fixed, source.replace('not player:', 'player:').replace('== 54', '~= 54'))
        with self.assertRaises(ValueError):
            capture_yalahar.repair_palimuth_job_line(source.replace('54', '55'))


if __name__ == "__main__":
    unittest.main()
