import unittest


class ManopScopeTests(unittest.TestCase):
    def test_manop_confirmation_uses_its_explicit_quest_scope_and_legacy_aliases(self):
        import capture_monk
        source = '''local MONK_QUEST = "the_way_of_the_monk_quest"
kv:set("questline", 1)
kv:set("questlog", 1)
player:setStorageValue(Storage.Quest.U15_00.TheWayOfTheMonk.Questline, 1)
player:setStorageValue(Storage.Quest.U15_00.TheWayOfTheMonk.Questlog, 1)'''
        aliases = capture_monk.manop_confirmation_aliases(source, 1)
        self.assertEqual(set(aliases), {"questline", "questlog"})
        self.assertTrue(all(key.startswith("oteryn:quest-progress/quest/u15_00/the_way_of_the_monk/")
                            for key in aliases.values()))
        with self.assertRaises(ValueError):
            capture_monk.manop_confirmation_aliases(source.replace('TheWayOfTheMonk.Questline, 1',
                                                                  'TheWayOfTheMonk.Questline, 2'), 1)
        with self.assertRaises(ValueError):
            capture_monk.manop_confirmation_aliases(source.replace('the_way_of_the_monk_quest', 'another_quest'), 1)


if __name__ == "__main__":
    unittest.main()
