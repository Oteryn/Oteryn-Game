"""Qualification of the bounded completion import and its reference joins."""
import copy
import json
import tempfile
import unittest
from pathlib import Path

import quest_completion_import as tool

ROOT = Path(__file__).resolve().parents[3]


class CompletionImportTests(unittest.TestCase):
    def test_all68_bindings_reference_actual_owned_transitions(self):
        outputs = tool.expected(ROOT)
        state = json.loads(outputs[tool.OUTPUT])
        plan = json.loads(outputs[tool.PLAN])
        transitions = {t['key']: t for q in state['quests'] for t in q['transitions']}
        self.assertEqual(plan['counts'], {'quests': 68, 'stages': 416})
        for quest in plan['records']:
            self.assertFalse(quest['runtime_enabled'])
            self.assertIsNone(quest['native_reward_delivery_binding'])
            for stage in quest['stages']:
                transition = transitions[stage['quest_transition_key']]
                self.assertEqual(transition['quest'], quest['quest'])
                self.assertEqual(transition['source']['chosen_stage']['key'], stage['stage_key'])
                self.assertIsNone(stage['native_dispatch_binding'])
                self.assertIsNone(stage['selected_NPC_branch'])

    def test_existing_main_records_change_only_approved_five_effects(self):
        base = json.loads((ROOT / tool.BASE).read_bytes())
        state = json.loads(tool.expected(ROOT)[tool.OUTPUT])
        actual = {q['quest']: q for q in state['quests']}
        changed = []
        for quest in base['quests']:
            after = actual[quest['quest']]
            self.assertEqual(after['tracks'], quest['tracks'])
            self.assertEqual(len(after['transitions']), len(quest['transitions']))
            for before, new in zip(quest['transitions'], after['transitions']):
                if before != new:
                    allowed = copy.deepcopy(before)
                    allowed['effects'][0]['effect'] = new['effects'][0]['effect']
                    self.assertEqual(allowed, new)
                    self.assertTrue(new['effects'][0]['from_exact'])
                    changed.append(new['key'])
        self.assertEqual(len(changed), 5)

    def test_readonly_determinism_and_pinned_input_drift_rejection(self):
        before = {path: (ROOT / path).stat().st_mtime_ns for path in [tool.BASE, tool.EVENTS, tool.NPC]}
        self.assertEqual(tool.expected(ROOT), tool.expected(ROOT))
        self.assertEqual(before, {path: (ROOT / path).stat().st_mtime_ns for path in before})
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / tool.BASE
            path.parent.mkdir(parents=True)
            path.write_bytes(b'{}')
            with self.assertRaisesRegex(ValueError, 'catalogue drift'):
                tool.expected(root)
            with self.assertRaisesRegex(ValueError, 'packet drift'):
                tool.pinned(root, tool.BASE, '0' * 64)


if __name__ == '__main__':
    unittest.main()
