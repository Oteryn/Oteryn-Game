"""Literal-only recovery regressions; source integration uses existing local Git objects."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[4] / 'tools/content-schema/monster-authoring'))
sys.path.insert(0, str(HERE))
import partial_monster_loader as loader


class LiteralRecoveryTests(unittest.TestCase):
    def test_nested_path_ignores_identical_leaf_in_other_branch(self):
        text = 'Storage = {\nQuest = {\nOther = {Timer = 7},\nDream = {\nTimer = 42, -- exact leaf\n},\n},\n}'
        self.assertEqual(loader.integer_leaf(text, 'Storage', ['Quest', 'Dream', 'Timer']), 42)

    def test_no_arithmetic_or_function_execution(self):
        for value in ('42 + 1', 'dangerous()', '"42"'):
            with self.subTest(value=value), self.assertRaises(ValueError):
                loader.integer_leaf('Storage = {\nTimer = ' + value + ',\n}', 'Storage', ['Timer'])

    def test_duplicate_and_missing_paths_rejected(self):
        for text in ('Storage = {\nTimer = 1,\nTimer = 2,\n}', 'Storage = {\nOther = 1,\n}'):
            with self.subTest(text=text), self.assertRaises(ValueError):
                loader.integer_leaf(text, 'Storage', ['Timer'])

    def test_staged_library_must_match_pinned_object(self):
        if not Path('/workspace/spell-sources/crystal/.git').exists():
            self.skipTest('local pinned Crystal Git objects unavailable')
        relative = 'data-global/lib/core/storages.lua'
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'crystal'
            path = root / relative
            path.parent.mkdir(parents=True)
            path.write_text('Storage = {Timer = 1}')
            with self.assertRaisesRegex(ValueError, 'differs from pinned source'):
                loader.dependency_text(root, relative)

    def test_soulwar_pinned_interval_and_tampered_copy_refusal(self):
        for engine, directory in [('canary', 'data-otservbr-global'), ('crystal', 'data-global')]:
            with self.subTest(engine=engine), tempfile.TemporaryDirectory() as temporary:
                repo = Path('/workspace/spell-sources') / engine
                if not (repo / '.git').exists():
                    self.skipTest('local pinned ' + engine + ' Git objects unavailable')
                root = Path(temporary) / engine
                relative = directory + '/monster/quests/soul_war/goshnars_cruelty.lua'
                path = root / relative
                path.parent.mkdir(parents=True)
                path.write_bytes(subprocess.check_output(['git', '-C', str(repo), 'show', loader.REVISIONS[engine] + ':' + relative]))
                name, monster, callbacks = loader.load(path, [])
                dependencies = loader.RECOVERIES[str(path)]['source_literal_dependencies']
                interval = next(d for d in dependencies if d['field'] == 'SoulWarQuest.goshnarsCrueltyWaveInterval')
                self.assertEqual(interval['value'], 7)
                self.assertEqual(interval['revision'], loader.REVISIONS[engine])
                self.assertEqual(len(interval['sha256']), 64)
                lib_relative = directory + '/lib/quests/soul_war.lua'
                lib = root / lib_relative
                lib.parent.mkdir(parents=True)
                data = subprocess.check_output(['git', '-C', str(repo), 'show', loader.REVISIONS[engine] + ':' + lib_relative])
                lib.write_bytes(data.replace(b'goshnarsCrueltyWaveInterval = 7', b'goshnarsCrueltyWaveInterval = 8', 1))
                with self.assertRaisesRegex(ValueError, 'differs from pinned source'):
                    loader.load(path, [])

    def test_exact_pinned_crystal_recoveries_and_raw_string_refusal(self):
        repo = Path('/workspace/spell-sources/crystal')
        if not (repo / '.git').exists():
            self.skipTest('local pinned Crystal Git objects unavailable')
        revision = loader.REVISIONS['crystal']
        paths = [('data-global/monster/quests/a_piece_of_cake/cake_golem.lua', 'loot')]
        bosses = ('alptramun', 'izcandar_the_banished', 'malofur_mangrinder', 'maxxenius', 'plagueroot', 'the_nightmare_beast')
        paths += [('data-global/monster/quests/the_dream_courts/bosses/' + boss + '.lua', 'bosstiary') for boss in bosses]
        paths += [('data-global/monster/quests/rotten_blood_quest/chagorz.lua', None)]
        with tempfile.TemporaryDirectory() as directory:
            for relative, recovered_field in paths:
                with self.subTest(relative=relative):
                    path = Path(directory) / 'crystal' / relative
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(subprocess.check_output(['git', '-C', str(repo), 'show', revision + ':' + relative]))
                    name, monster, callbacks = loader.load(path, [])
                    recovery = loader.RECOVERIES[str(path)]
                    if recovered_field:
                        self.assertIn(recovered_field, monster)
                        self.assertEqual(recovery['omitted_fields'], [])
                        dependency = recovery['source_literal_dependencies'][0]
                        self.assertEqual(dependency['revision'], revision)
                        self.assertEqual(len(dependency['sha256']), 64)
                        if recovered_field == 'loot':
                            self.assertEqual(monster['loot'][0]['id'], 12143)
                        else:
                            self.assertIsInstance(monster['bosstiary']['storage'], int)
                    else:
                        self.assertNotIn('elements', monster)
                        self.assertEqual(recovery['omitted_fields'][0]['field'], 'elements')
                        raw = recovery['omitted_fields'][0]['raw_source_value']
                        self.assertEqual(len(raw), 10)
                        self.assertEqual(raw[6], {'type': 'COMBAT_EARTHDAMAGE', 'percent': 10})
                    self.assertEqual(recovery['mode'], 'declarative_assignments_only_encounter_setup_not_executed')


if __name__ == '__main__':
    unittest.main()
