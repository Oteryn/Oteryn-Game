"""Static generator checks. Native behavior is tested by native_soul_war_progression.rs."""
import copy
import json
import tempfile
import unittest
from pathlib import Path

import soul_war_native_progression as tool

ROOT = Path(__file__).resolve().parents[3]


class SoulWarNativeProgressionTests(unittest.TestCase):
    def test_committed_fixture_is_reproducible_without_mutation(self):
        path = ROOT / tool.OUTPUT
        before = path.read_bytes(), path.stat().st_mtime_ns
        self.assertEqual(before[0], tool.encoded(tool.expected(ROOT)))
        self.assertEqual(before, (path.read_bytes(), path.stat().st_mtime_ns))

    def test_fixture_is_not_a_completion_or_native_admission(self):
        packet = tool.expected(ROOT)
        self.assertFalse(packet['contract']['runtime_enabled'])
        self.assertEqual(packet['contract']['holds'], list(tool.HOLDS))
        self.assertEqual(packet['counts'], {'quests': 1, 'tracks': 10, 'transitions': 29,
                                            'native_dispatch_bindings': 0, 'completes': 0})
        quest = packet['quests'][0]
        self.assertTrue(all(not t['completes'] and t['requested_by'] is None
                            and t['source']['native_dispatch_binding'] is None for t in quest['transitions']))
        self.assertEqual(quest['transitions'][-1]['effects'][0]['effect'], {'op': 'COMPUTED'})

    def test_final_gate_checks_five_flags_in_one_bounded_transition(self):
        transitions = tool.expected(ROOT)['quests'][0]['transitions']
        gate = next(t for t in transitions if t['key'] == tool.TRANSITION_PREFIX + 'final-access')
        self.assertEqual(len(gate['effects']), 8)
        guards = {e['track']: e['from'] for e in gate['effects']}
        for boss in tool.BOSSES:
            self.assertEqual(guards[tool.TRACK_PREFIX + boss], {'op': 'EQ', 'value': 1})
        for t in transitions:
            self.assertLessEqual(len(t['effects']), 8)
            self.assertEqual(len(t['effects']), len({e['track'] for e in t['effects']}))

    def test_every_boss_rank_requires_its_own_unset_flag(self):
        transitions = {t['key']: t for t in tool.expected(ROOT)['quests'][0]['transitions']}
        for boss in tool.BOSSES:
            for rank in range(5):
                t = transitions[tool.TRANSITION_PREFIX + f'{boss}/taint-{rank + 1}']
                effects = {e['track']: e for e in t['effects']}
                own = effects[tool.TRACK_PREFIX + boss]
                self.assertEqual(own['from'], {'op': 'EQ', 'value': 0})
                self.assertEqual(own['effect'], {'op': 'SET', 'value': 1})
                taint = effects[tool.TRACK_PREFIX + 'taints']
                self.assertEqual(taint['from'], {'op': 'EQ', 'value': rank})
                self.assertEqual(taint['effect'], {'op': 'ADD', 'value': 1})
                self.assertEqual(tool.TRACK_PREFIX + 'first-taint-at' in effects, rank == 0)

    def test_invalid_source_scope_is_rejected_without_changing_source(self):
        source = json.loads((ROOT / tool.PACKET).read_bytes())
        mutations = [lambda p: p.update(runtime_activation=True),
                     lambda p: p['quest'].update(key='oteryn:quest.other'),
                     lambda p: p['source_policy'].update(primary_revision='0' * 40),
                     lambda p: p['progression'].update(mini_boss_order='FIXED_ORDER'),
                     lambda p: p['progression']['mini_bosses'].append("Goshnar's Malice"),
                     lambda p: p['progression'].update(final_access='any boss is enough')]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / tool.PACKET
            path.parent.mkdir(parents=True)
            for mutate in mutations:
                changed = copy.deepcopy(source); mutate(changed)
                raw = tool.encoded(changed); path.write_bytes(raw)
                with self.assertRaises(ValueError): tool.expected(root)
                self.assertEqual(path.read_bytes(), raw)


if __name__ == '__main__':
    unittest.main()
