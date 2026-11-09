"""M26 source parsing retains cooldowns and distinguishes real extra monsters."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('m26_levers', Path(__file__).with_name('import_boss_levers.py'))
IMPORTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORTER)


class EndgameCapture(unittest.TestCase):
    def test_bakragore_duration_ratio_is_exact_and_bounded(self):
        self.assertEqual(IMPORTER.number('ParseDuration("68h") / ParseDuration("1s")'), 244800)
        self.assertEqual(IMPORTER.number('ParseDuration("20h") / ParseDuration("1s")'), 72000)
        for text in ['ParseDuration("68h") / ParseDuration("2s")',
                     'ParseDuration("68h") / ParseDuration("1s") + 1',
                     'ParseDuration("99999999999d") / ParseDuration("1s")']:
            with self.subTest(expression=text), self.assertRaises(ValueError):
                IMPORTER.number(text)

    def test_lua_last_literal_policy_wins_and_unknown_last_value_stays_unknown(self):
        self.assertEqual(IMPORTER.policy('{ timeToFightAgain = 0, timeToFightAgain = 20 * 60 * 60 }',
                                         'timeToFightAgain', 1), 72000)
        with self.assertRaises(ValueError):
            IMPORTER.policy('{ timeToFightAgain = 0, timeToFightAgain = unknown() }',
                            'timeToFightAgain', 1)

    def test_comment_only_monsters_are_empty_but_real_extras_remain_blocked(self):
        for body, blocked in [('-- {name="Mushroom", pos=Position(1,2,3)}\n', False),
                              ('{name="Mushroom", pos=Position(1,2,3)}', True)]:
            with self.subTest(monsters=body), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                path = root / 'lever.lua'
                path.write_text('local config = { monsters = {\n' + body + '\n} }\nBossLever(config)')
                row = IMPORTER.capture(root, path, {}, {'cooldown': 72000, 'timeout': 1200})
                self.assertEqual('extra_monster_closure' in row['blockers'], blocked)
                self.assertFalse(row['callbacks_known'], 'parser facts never qualify unknown native callbacks')


class SoulLeverCapture(unittest.TestCase):
    def test_six_registered_configurations_retain_identity_line_and_policy(self):
        names = ['goshnarsSpite', 'goshnarsMalice', 'goshnarsGreed',
                 'goshnarsHatred', 'goshnarsCruelty', 'goshnarsMegalomania']
        configurations = []
        for index, name in enumerate(names):
            configurations.append(name + ' = { boss = { name = "Test ' + str(index) +
                '", position = Position(1,2,3) }, requiredLevel = 250, ' +
                'playerPositions = {{ pos = Position(4,2,3), teleport = Position(1,2,3) }}, ' +
                'specPos = { from = Position(0,0,3), to = Position(6,6,3) }, ' +
                'exit = Position(9,9,3), timeToFightAgain = ' +
                ('72' if index == 5 else '20') + ' * 60 * 60 }')
        text = 'SoulWarQuest = { levers = {\n' + ',\n'.join(configurations) + '\n} }\n'
        for name in names:
            text += ('local ' + name + 'Lever = BossLever(SoulWarQuest.levers.' + name + ')\n' +
                     name + 'Lever:position(SoulWarQuest.levers.' + name + 'Position)\n')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'data-global/lib/quests/soul_war.lua'
            source.parent.mkdir(parents=True)
            source.write_text(text)
            with patch.object(IMPORTER.m26_endgame_capture, 'SOUL_SHA',
                              hashlib.sha256(text.encode()).hexdigest(), create=True):
                rows = IMPORTER.capture_records(root, source, {}, {'cooldown':72000,'timeout':1200})
            self.assertEqual(len(rows), 6, 'one candidate per actual registered lever')
            self.assertEqual([row['line'] for row in rows], list(range(2,8)))
            self.assertEqual([row['creature_key'] for row in rows],
                             ['oteryn:creature.test_' + str(i) for i in range(6)])
            self.assertEqual([row['cooldown_seconds'] for row in rows], [72000]*5+[259200])
            self.assertTrue(all(row['complete_literal_layout'] for row in rows))
            self.assertTrue(all(not row['callbacks_known'] for row in rows))
            self.assertTrue(all(row['path'] == source.relative_to(root).as_posix() for row in rows))
            self.assertTrue(all(row['sha256'] == hashlib.sha256(text.encode()).hexdigest() for row in rows))


class SpiteHookCustody(unittest.TestCase):
    def test_exact_source_death_hook_and_absent_think_are_captured_without_native_authority(self):
        capture = IMPORTER.m26_endgame_capture
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            self.assertEqual(capture.spite_hooks(root,IMPORTER.table),([],False))
            path=root/capture.SPITE_MONSTER_PATH
            path.parent.mkdir(parents=True)
            raw=b'monster.events = {"SoulWarBossesDeath"}\nmType:register(monster)'
            path.write_bytes(raw)
            blob=hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()
            mechanics=b'local bossesDeath = CreatureEvent("SoulWarBossesDeath")\nfunction bossesDeath.onDeath(creature) end\nbossesDeath:register()'
            target=root/capture.SPITE_MECHANICS_PATH
            target.parent.mkdir(parents=True,exist_ok=True)
            target.write_bytes(mechanics)
            mechanics_blob=hashlib.sha1(b'blob '+str(len(mechanics)).encode()+b'\0'+mechanics).hexdigest()
            with patch.object(capture,'SPITE_MONSTER_BLOB',blob), patch.object(capture,'SPITE_MECHANICS_BLOB',mechanics_blob):
                self.assertEqual(capture.spite_hooks(root,IMPORTER.table),(['SoulWarBossesDeath'],True))
                path.write_bytes(raw+b'\nmType.onThink = function() end')
                self.assertEqual(capture.spite_hooks(root,IMPORTER.table),([],False))


if __name__ == '__main__':
    unittest.main()
