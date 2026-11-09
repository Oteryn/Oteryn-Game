"""Qualification of the bounded completion import and its reference joins."""
import copy
import json
import tempfile
import unittest
from pathlib import Path

import quest_completion_import as tool

ROOT = Path(__file__).resolve().parents[3]


class CompletionImportTests(unittest.TestCase):
    def test_all304_bindings_reference_actual_owned_transitions(self):
        outputs = tool.expected(ROOT)
        state = json.loads(outputs[tool.OUTPUT])
        plan = json.loads(outputs[tool.PLAN])
        transitions = {t['key']: t for q in state['quests'] for t in q['transitions']}
        self.assertEqual(plan['counts'], {'quests': 304, 'stages': 1887})
        native = {}
        for quest in plan['records']:
            self.assertFalse(quest['runtime_enabled'])
            self.assertIsNone(quest['native_reward_delivery_binding'])
            for stage in quest['stages']:
                transition = transitions[stage['quest_transition_key']]
                self.assertEqual(transition['quest'], quest['quest'])
                self.assertEqual(transition['source']['chosen_stage']['key'], stage['stage_key'])
                if (quest['quest'] == 'oteryn:quest.adventures_of_galthen_quest'
                        and stage['stage_key'] in ('s5', 's8')):
                    predecessor, creature = {
                        's5': ('s4', 'oteryn:creature.ahau'),
                        's8': ('s7', 'oteryn:creature.mitmah_vanguard'),
                    }[stage['stage_key']]
                    self.assertEqual(stage['native_dispatch_binding'], {
                        'owner': 'native_kills',
                        'creature': {'family': 'Creature', 'key': creature, 'revision': 'definition-r1'},
                        'transition': stage['quest_transition_key'],
                        'predecessor': {'track': 'oteryn:quest-progress/chosen-source/adventures_of_galthen_quest/' + predecessor, 'value': 1},
                    })
                    self.assertFalse(transition['completes'])
                    native[stage['stage_key']] = stage['native_dispatch_binding']
                elif (quest['quest'], stage['stage_key']) == ('oteryn:quest.authored.a_pirate_s_death_to_me', 's1'):
                    self.assertTrue(stage['runtime_enabled'])
                    self.assertEqual(stage['native_dispatch_binding'], {
                        'kind': 'npc_dialogue', 'document': 'content/quests/authored-bindings/m01a-a-pirates-death-to-me.json',
                        'stage': 's1', 'callback': 'oteryn:quest-reward/native-talk/a-pirates-death-to-me/s1'})
                else:
                    self.assertIsNone(stage['native_dispatch_binding'])
                    self.assertFalse(stage['runtime_enabled'])
                self.assertIsNone(stage['selected_NPC_branch'])
        self.assertEqual(set(native), {'s5', 's8'})

    def test_existing_main_source_prefix_is_preserved_and_only_five_source_effects_refine(self):
        base = json.loads((ROOT / tool.BASE).read_bytes())
        state = json.loads(tool.expected(ROOT)[tool.OUTPUT])
        actual = {q['quest']: q for q in state['quests']}
        source_stages = tool.module(ROOT, tool.SOURCE_STAGES, 'chosen_source_test_projection')
        packet = source_stages.build(ROOT)
        overlays = {q['quest']: q for q in packet['overlays']}
        changed = []
        for quest in base['quests']:
            after = actual[quest['quest']]
            self.assertEqual(
                quest['tracks'],
                after['tracks'][:len(quest['tracks'])],
                quest['quest'],
            )
            source_transitions = after['transitions'][:len(quest['transitions'])]
            for before, new in zip(quest['transitions'], source_transitions):
                if before != new:
                    allowed = copy.deepcopy(before)
                    allowed['effects'][0]['effect'] = new['effects'][0]['effect']
                    self.assertEqual(allowed, new)
                    self.assertTrue(new['effects'][0]['from_exact'])
                    changed.append(new['key'])
            if quest['quest'] in overlays:
                overlay = overlays[quest['quest']]
                self.assertEqual(
                    overlay['tracks'],
                    after['tracks'][len(quest['tracks']):],
                    quest['quest'],
                )
                self.assertEqual(
                    overlay['transitions'],
                    after['transitions'][len(quest['transitions']):],
                    quest['quest'],
                )
                self.assertEqual(
                    'SOURCE_PLUS_CHOSEN_TYPED_PROGRESS_ONLY',
                    after['completion']['state'],
                )
                self.assertTrue(after['completion']['source_progress_preserved'])
            else:
                self.assertEqual(len(quest['tracks']), len(after['tracks']))
                self.assertEqual(len(quest['transitions']), len(after['transitions']))
        self.assertEqual(90, len(overlays))
        self.assertEqual(len(changed), 5)


    def test_terminal_refinement_helper_is_pinned_in_candidate_provenance(self):
        outputs = tool.expected(ROOT)
        receipt = json.loads(outputs[tool.RECEIPT])
        by_path = {row['path']: row['sha256'] for row in receipt['input_provenance']}
        self.assertIn(tool.TERMINAL_REFINEMENTS, by_path)
        self.assertEqual(
            tool.sha((ROOT / tool.TERMINAL_REFINEMENTS).read_bytes()),
            by_path[tool.TERMINAL_REFINEMENTS],
        )

    def test_make_believe_post_release_counts_are_finite_overlay(self):
        outputs = tool.expected(ROOT)
        plan = json.loads(outputs[tool.PLAN])
        quest = next(q for q in plan['records']
                     if q['quest'] == 'oteryn:quest.authored.make_believe_quest')
        stages = {s['stage_key']: s['event_identity_associations'] for s in quest['stages']}
        self.assertEqual((stages['s8']['kind'], stages['s8']['count']), ('use', 5))
        self.assertEqual((stages['s14']['kind'], stages['s14']['count']), ('use', 8))
        self.assertEqual(plan['input_packets'][1],
                         {'path': tool.EVENT_CORRECTIONS, 'sha256': tool.EVENT_CORRECTIONS_SHA})
        historical = tool.pinned(ROOT, tool.EVENTS, tool.EVENTS_SHA)
        source = next(q for q in historical['records']
                      if q['quest_ref']['key'] == 'oteryn:quest.authored.make_believe_quest')
        old = {s['stage_key']: (s['kind'], s['count']) for s in source['stages']}
        self.assertEqual(old['s8'], ('use', 3))
        self.assertEqual(old['s14'], ('use', 6))

    def test_completion_outputs_use_portable_repository_paths(self):
        outputs = tool.expected(ROOT)
        for output_path, encoded in outputs.items():
            value = json.loads(encoded)
            stack = [value]
            while stack:
                current = stack.pop()
                if isinstance(current, dict):
                    for key, item in current.items():
                        if key == 'path' and isinstance(item, str):
                            self.assertNotIn('\\', item, (output_path, item))
                            self.assertFalse(Path(item).is_absolute(), (output_path, item))
                            self.assertEqual(item, Path(item).as_posix(), (output_path, item))
                        stack.append(item)
                elif isinstance(current, list):
                    stack.extend(current)

    def test_readonly_determinism_and_pinned_input_drift_rejection(self):
        before = {path: (ROOT / path).stat().st_mtime_ns
                  for path in [tool.BASE, tool.EVENTS, tool.EVENT_CORRECTIONS, tool.SOURCE_EVENTS, tool.NPC]}
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
