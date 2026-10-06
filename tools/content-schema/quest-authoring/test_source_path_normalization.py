import unittest

from ots_questlog import script_of


class SourcePathNormalizationTest(unittest.TestCase):
    def test_script_of_normalizes_windows_and_posix_paths(self):
        expected = "scripts/quests/demon_helmet/action-lever.lua"
        self.assertEqual(
            expected,
            script_of(r"data-otservbr-global\scripts\quests\demon_helmet\action-lever.lua"),
        )
        self.assertEqual(
            expected,
            script_of("data-otservbr-global/scripts/quests/demon_helmet/action-lever.lua"),
        )
        self.assertEqual(
            expected,
            script_of(r"data-global\scripts\quests\demon_helmet\action-lever.lua"),
        )


if __name__ == "__main__":
    unittest.main()
