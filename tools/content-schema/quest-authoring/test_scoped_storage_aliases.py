"""SOURCE alias regressions; exact use sites never confer runtime authority."""
import unittest
from pathlib import Path

from scoped_storage_aliases import expand_scoped_storage_aliases as expand


class ScopedAliasTests(unittest.TestCase):
    def test_top_level_alias_after_closed_function(self):
        source = 'local function revert()\nend\nlocal A = Storage.Quest.A\nfunction a.onUse(p)\np:setStorageValue(A.Stage, 4)\nend\n'
        self.assertIn('p:setStorageValue(Storage.Quest.A.Stage, 4)', expand(source))
        self.assertEqual(expand(source).count('\n'), source.count('\n'))

    def test_loop_branch_constant_target_computed_value_stays_computed(self):
        source = 'function a.onUse(p)\nfor index, shrine in ipairs(rows) do\nif ready then\nlocal Key = Storage.Quest.Shrines\np:setStorageValue(Key, index + 1)\nend\nend\nend\n'
        self.assertIn('p:setStorageValue(Storage.Quest.Shrines, index + 1)', expand(source))

    def test_character_and_global_receivers_do_not_merge(self):
        source = 'function a.onDeath(p)\nlocal K = 775558\nlocal W = 673003\np:setStorageValue(K, p:getStorageValue(K) + 1)\nGame.setStorageValue(W, 0)\nend\n'
        result = expand(source)
        self.assertIn('p:setStorageValue(775558, p:getStorageValue(775558) + 1)', result)
        self.assertIn('Game.setStorageValue(W, 0)', result)
        self.assertNotIn('setStorageValue(673003', result)

    def test_sibling_callback_and_branch_do_not_leak(self):
        for source in ('function a.first(p)\nlocal K = 12\nend\nfunction a.second(p)\np:setStorageValue(K, 1)\nend\n', 'if ready then\nlocal K = 12\nelse\np:setStorageValue(K, 1)\nend\n', 'if ready then\nlocal K = 12\nend\np:setStorageValue(K, 1)\n'):
            self.assertEqual(expand(source), source)

    def test_declaration_after_use_and_forward_alias_do_not_resolve(self):
        for source in ('function a.onUse(p)\np:setStorageValue(K, 1)\nlocal K = 12\nend\n', 'local K = LATE\nlocal LATE = 12\np:setStorageValue(K, 1)\n'):
            self.assertEqual(expand(source), source)
        source = 'local K = ' + '9' * 5000 + '\np:setStorageValue(K, 1)\n'
        self.assertEqual(expand(source), source)

    def test_mutation_and_escape_fail_closed_including_earlier_reads(self):
        for tail in ('K = 13', 'local other = K', 'mutate(K)', 'K.Stage = 13'):
            source = 'local K = Storage.Quest.A\np:setStorageValue(K.Stage, 1)\n' + tail + '\n'
            self.assertEqual(expand(source), source)

    def test_parameter_and_loop_variable_shadow_alias(self):
        for source in ('local K = 12\nfunction a.onUse(K)\nK:setStorageValue(K, 1)\nend\n', 'local K = 12\nfor K in pairs(rows) do\np:setStorageValue(K, 1)\nend\n'):
            self.assertEqual(expand(source), source)

    def test_computed_local_unknown_and_builtin_shadow_held(self):
        for source in ('local K = storageForPlayer(p)\np:setStorageValue(K, 1)\n', 'local K = BASE + 1\np:setStorageValue(K, 1)\n', 'local K = Storage.Quest.A\nStorage.Quest.A = 99\np:setStorageValue(K, 1)\n', 'local K = Storage.Quest.A\nmutate(Storage.Quest.A)\np:setStorageValue(K.Stage, 1)\n', 'local K = Storage.Quest.A\nStorage["Quest"].A = other\np:setStorageValue(K.Stage, 1)\n', 'local K = Storage.Quest.A\nfunction f()\nlocal K = K\nK.Stage = 99\nend\np:setStorageValue(K.Stage, 1)\n'):
            self.assertEqual(expand(source), source)

    def test_comments_strings_and_unsupported_blocks_unchanged(self):
        for source in ('-- local K = 12\np:setStorageValue(K, 1)\n', 'local text = "local K = 12"\np:setStorageValue(K, 1)\n', 'repeat\nlocal K = 12\np:setStorageValue(K, 1)\nuntil ready\n'):
            self.assertEqual(expand(source), source)

    def test_exact_pinned_source_positive_and_world_negative(self):
        root = Path('/tmp/quest-sources/canary/data-otservbr-global/scripts/quests')
        if not root.is_dir(): self.skipTest('pinned donor checkout is an optional local fixture')
        for relative, wanted in (('an_uneasy_alliance/actions_crystal_ball.lua', 'setStorageValue(Storage.Quest.U8_54.AnUneasyAlliance.Questline, 4)'), ('the_way_of_the_monk/shrines.lua', 'setStorageValue(Storage.Quest.U14_15.TheWayOfTheMonk.ShrinesCount, index + 1)'), ('raging_mage_tower/creaturescripts_raging_mage_1.lua', 'setStorageValue(775558, mostDamageKiller:getStorageValue(775558) + 1)')):
            source = (root / relative).read_text(); result = expand(source)
            self.assertIn(wanted, result)
            self.assertEqual(result.count('\n'), source.count('\n'))
            self.assertNotIn('Game.setStorageValue(673003', result)


if __name__ == '__main__': unittest.main()
