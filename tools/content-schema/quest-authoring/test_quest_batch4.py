"""Offline regression tests for exact batch ownership and source-only emission."""
import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import quest_batch4 as tool


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def callback(directory, name):
    return {
        'key': 'oteryn:quest-callback/' + name,
        'quest': tool.QUEST_OWNERS[directory],
        'trigger': 'step_in',
        'selector': {'position': {'x': 100, 'y': 200, 'floor': -7},
                     'unique': None, 'action': None, 'appearance': None},
        'lever': None, 'reversible': False, 'passages': [],
        'teleport': {'x': 101, 'y': 200, 'floor': -7},
        'storage': [], 'offerings': [], 'item_on_target': None,
        'return_on_failure': False, 'failure_message': None,
        'sources': [tool.SOURCE_PREFIX + f'data-global/scripts/quests/{directory}/fixture.lua:1-12'],
    }


def packet(directory):
    return {
        'schema': 1, 'quest_directory': directory,
        'source_repository_revision': tool.SOURCE_REVISION,
        'callbacks': [], 'runtime_exclusions': [],
        'npc_source': {
            'quest': directory, 'runtime_admission': False,
            'entries': [],
        },
        'step_inventory': [], 'preserved_claims': [],
    }


class QuestBatch4Tests(unittest.TestCase):
    def test_paid_step_requires_explicit_native_world_admission(self):
        row = callback('waterfall', 'paid-step')
        row['requires_world_admission'] = True
        row['selector']['native'] = {'position': row['selector']['position'],
                                     'ordinal': 0, 'appearance': 353}
        row['storage'] = [{'track': 'oteryn:quest-progress/paid-step',
                           'minimum': 0, 'maximum': 1, 'set': 1}]
        row['item_on_target'] = {'item': 'oteryn:item.tibia.i3595', 'count': 1,
                                 'on_step': True}
        tool.validate_callback(row, 'waterfall')
        for field, value in (('on_step', False), ('on_step', 1),
                             ('reusable', True), ('target_replacement', 1949)):
            changed = copy.deepcopy(row)
            changed['item_on_target'][field] = value
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                tool.validate_callback(changed, 'waterfall')
        for field, value in (('ordinal', 256), ('ordinal', True),
                             ('appearance', 0), ('appearance', True)):
            changed = copy.deepcopy(row)
            changed['selector']['native'][field] = value
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                tool.validate_callback(changed, 'waterfall')
        changed = copy.deepcopy(row)
        changed['requires_world_admission'] = False
        with self.assertRaises(ValueError):
            tool.validate_callback(changed, 'waterfall')

    def test_read_only_step_remote_teleport_is_a_closed_predicate(self):
        row = callback('formogar_mine_hoist', 'hoist-remote')
        row['return_on_failure'] = True
        row['storage'] = [{'track': 'oteryn:quest-progress/fixture/instruction',
                           'minimum': 1, 'maximum': 1, 'set': 1, 'read_only': True}]
        row['remote_teleport'] = {
            'position': {'x': 32156, 'y': 31125, 'floor': -10},
            'appearance': 2772,
            'destination': {'x': 32157, 'y': 31125, 'floor': -11},
        }
        tool.validate_callback(row, 'formogar_mine_hoist')
        for invalid in ('appearance', 'position', 'destination', 'no_gate', 'mutation', 'no_default', 'no_return'):
            bad = copy.deepcopy(row)
            if invalid == 'appearance':
                bad['remote_teleport']['appearance'] = 0
            elif invalid in ('position', 'destination'):
                bad['remote_teleport'][invalid]['floor'] = 1
            elif invalid == 'no_gate':
                bad['storage'] = []
            elif invalid == 'mutation':
                bad['storage'][0]['preserve'] = True
            elif invalid == 'no_return':
                bad['return_on_failure'] = False
            else:
                bad['teleport'] = None
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                tool.validate_callback(bad, 'formogar_mine_hoist')

    def test_passage_state_guard_requires_a_reversible_removal_lever(self):
        row = callback('cults_of_tibia', 'guarded-passage')
        row.update(trigger='use', lever=[2772, 2773], reversible=True)
        row['passages'] = [{
            'position': {'x': 101, 'y': 200, 'floor': -7},
            'appearance': 389, 'require_state': True,
        }]
        tool.validate_callback(row, 'cults_of_tibia')
        for invalid in ('non_boolean', 'replacement', 'one_way', 'no_lever', 'step'):
            bad = copy.deepcopy(row)
            if invalid == 'non_boolean':
                bad['passages'][0]['require_state'] = 1
            elif invalid == 'replacement':
                bad['passages'][0]['replacement'] = 390
            elif invalid == 'one_way':
                bad['reversible'] = False
            elif invalid == 'no_lever':
                bad['lever'] = None
            else:
                bad['trigger'] = 'step_in'
                bad['lever'] = None
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                tool.validate_callback(bad, 'cults_of_tibia')
    def test_callback_source_presentation_is_bounded_and_validated(self):
        row = callback('waterfall', 'waterfall-presentation')
        row['facing'] = 2
        row['effects'] = [{'position': {'x': 32968, 'y': 32626, 'floor': -5}, 'effect': 54}]
        tool.validate_callback(row, 'waterfall')
        for field, invalid in (
                ('facing', 4), ('facing', True), ('teleport', None),
                ('effects', [{'position': {'x': 1, 'y': 2, 'floor': 1}, 'effect': 54}]),
                ('effects', [{'position': {'x': 1, 'y': 2, 'floor': -1}, 'effect': 0}]),
                ('effects', row['effects'] * 9)):
            with self.subTest(field=field, invalid=invalid):
                changed = copy.deepcopy(row)
                changed[field] = invalid
                with self.assertRaises(ValueError):
                    tool.validate_callback(changed, 'waterfall')

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.packets = {name: packet(name) for name in tool.QUEST_ORDER}
        self.seed = {'schema': 1, 'callbacks': []}
        for index in range(20):
            row = callback('cults_of_tibia', f'foreign-seed-{index}')
            row['quest'] = f'oteryn:quest.foreign_seed_{index}'
            row['sources'] = ['canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:fixture.lua:1']
            self.seed['callbacks'].append(row)
        write_json(self.root / tool.CALLBACK_OUTPUT, self.seed)
        self.save_packets()

    def save_packets(self):
        for name, value in self.packets.items():
            write_json(self.root / tool.INPUT_DIRECTORY / (name + '.json'), value)

    def run_generator(self, check=False):
        self.save_packets()
        return tool.generate(self.root, check=check)

    def keep_packets(self, names):
        self.save_packets()
        for path in (self.root / tool.INPUT_DIRECTORY).glob('*.json'):
            if path.stem not in names:
                path.unlink()

    def output(self):
        return json.loads((self.root / tool.CALLBACK_OUTPUT).read_text())

    def test_deterministic_regeneration_sorts_union_and_keeps_complete_npc_source(self):
        cults = self.packets['cults_of_tibia']
        cults['callbacks'] = [callback('cults_of_tibia', 'cults-a'), callback('cults_of_tibia', 'cults-b')]
        dreamer = self.packets['dreamers_challenge_quest']
        dreamer['callbacks'] = [callback('dreamers_challenge_quest', 'dreamer-a')]
        cults['runtime_exclusions'] = [{
            'key': cults['callbacks'][1]['key'], 'reason': 'Pinned map binding is absent.',
            'question': 'https://github.com/Oteryn/Oteryn-Game/pull/2842#issuecomment-fixture',
        }]
        cults['npc_source']['entries'] = [{
            'runtime_admission': False,
            'guarded_source_lua': 'if mission == 4 then\n  say("Pełny tekst — \\"quote\\"")\nend\n',
            'topic': {'before': 2, 'after': 0},
        }]
        original_seed = copy.deepcopy(self.seed)
        self.run_generator()
        result = self.output()
        self.assertEqual(len(result['callbacks']), 22)
        foreign = [row for row in result['callbacks'] if row['key'].startswith('oteryn:quest-callback/foreign-seed-')]
        expected_foreign = sorted(original_seed['callbacks'], key=lambda row: row['key'])
        self.assertEqual(foreign, expected_foreign)
        for before, after in zip(expected_foreign, foreign):
            self.assertEqual(json.dumps(before), json.dumps(after))
        self.assertEqual(result['callbacks'], sorted(
            original_seed['callbacks'] + [cults['callbacks'][0], dreamer['callbacks'][0]],
            key=lambda row: row['key'],
        ))
        output = self.root / tool.NPC_OUTPUT_DIRECTORY / 'cults_of_tibia.json'
        self.assertEqual(json.loads(output.read_text()), cults['npc_source'])
        saved = {path: path.read_bytes() for path in self.root.rglob('*.json')}
        self.assertEqual(self.run_generator(), [])
        self.assertEqual(self.run_generator(check=True), [])
        self.assertEqual(saved, {path: path.read_bytes() for path in self.root.rglob('*.json')})

    def test_duplicate_callback_keys_and_foreign_owner_are_rejected(self):
        row = callback('cults_of_tibia', 'same')
        for mode in ('duplicate_local', 'duplicate_batch', 'foreign_owner', 'foreign_prefix', 'foreign_existing'):
            with self.subTest(mode=mode):
                self.packets = {name: packet(name) for name in tool.QUEST_ORDER}
                self.packets['cults_of_tibia']['callbacks'] = [copy.deepcopy(row)]
                if mode == 'duplicate_local':
                    self.packets['cults_of_tibia']['callbacks'].append(copy.deepcopy(row))
                elif mode == 'duplicate_batch':
                    self.packets['dreamers_challenge_quest']['callbacks'] = [callback('dreamers_challenge_quest', 'same')]
                elif mode == 'foreign_owner':
                    self.packets['cults_of_tibia']['callbacks'][0]['quest'] = tool.QUEST_OWNERS['kilmaresh']
                elif mode == 'foreign_prefix':
                    self.packets['cults_of_tibia']['callbacks'][0]['key'] = 'oteryn:item.foreign'
                else:
                    self.packets['cults_of_tibia']['callbacks'][0]['key'] = self.seed['callbacks'][0]['key']
                original = (self.root / tool.CALLBACK_OUTPUT).read_bytes()
                with self.assertRaises(ValueError):
                    self.run_generator()
                self.assertEqual((self.root / tool.CALLBACK_OUTPUT).read_bytes(), original)

    def test_forged_packet_and_callback_pins_are_rejected(self):
        for mode in ('packet', 'callback', 'lookalike_prefix', 'unversioned'):
            with self.subTest(mode=mode):
                self.packets = {name: packet(name) for name in tool.QUEST_ORDER}
                current = self.packets['cults_of_tibia']
                current['callbacks'] = [callback('cults_of_tibia', 'pinned')]
                if mode == 'packet':
                    current['source_repository_revision'] = tool.SOURCE_REVISION[:-1] + '0'
                elif mode == 'callback':
                    current['callbacks'][0]['sources'] = ['crystal:unverified:fixture.lua:1']
                elif mode == 'lookalike_prefix':
                    current['callbacks'][0]['sources'] = [tool.SOURCE_PREFIX[:-1] + '0:fixture.lua:1']
                else:
                    current['callbacks'][0]['sources'] = ['fixture.lua:1']
                with self.assertRaises(ValueError):
                    self.run_generator()

    def test_unmatched_exclusion_fails_and_new_exclusion_removes_prior_runtime_record(self):
        current = self.packets['cults_of_tibia']
        current['callbacks'] = [callback('cults_of_tibia', 'now-excluded')]
        self.run_generator()
        self.assertEqual(len(self.output()['callbacks']), 21)
        current['runtime_exclusions'] = [{
            'key': current['callbacks'][0]['key'] + '-typo',
            'reason': 'Absent map metadata.', 'question': 'QUESTION: Q4-FIXTURE',
        }]
        with self.assertRaises(ValueError):
            self.run_generator()
        current['runtime_exclusions'][0]['key'] = current['callbacks'][0]['key']
        self.run_generator()
        expected = copy.deepcopy(self.seed)
        expected['callbacks'].sort(key=lambda row: row['key'])
        self.assertEqual(self.output(), expected)

    def test_runtime_admission_true_is_rejected_at_every_npc_packet_level(self):
        for nested in (False, True):
            with self.subTest(nested=nested):
                current = self.packets['cults_of_tibia']['npc_source']
                current['runtime_admission'] = False
                current['entries'] = []
                if nested:
                    current['entries'] = [{'sections': [{'runtime_admission': True}]}]
                else:
                    current['runtime_admission'] = True
                with self.assertRaises(ValueError):
                    self.run_generator()

    def test_native_rule_requires_all_three_preserved_tracks(self):
        tracks = [
            'oteryn:quest-progress/quest/u11_40/cults_of_tibia/barkless/mission',
            'oteryn:quest-progress/quest/u11_40/cults_of_tibia/barkless/temp',
            'oteryn:quest-progress/quest/u11_40/cults_of_tibia/barkless/objects',
        ]
        row = callback('cults_of_tibia', 'native-symbol')
        row['trigger'] = 'use'
        row['selector'] = {'position': {'x': 32731, 'y': 31531, 'floor': -9},
                           'unique': None, 'action': 5535, 'appearance': None}
        row['teleport'] = None
        row['native_rule'] = {'kind': 'cults_symbol', 'symbol': 0}
        row['sources'] = [tool.SOURCE_PREFIX + 'data-global/scripts/quests/cults_of_tibia/actions_cult_symbol.lua:1-52']
        row['storage'] = [
            {'track': track, 'minimum': 0, 'maximum': 7 if index == 0 else 2147483647,
             'set': 0, 'preserve': True}
            for index, track in enumerate(tracks)
        ]
        self.packets['cults_of_tibia']['callbacks'] = [copy.deepcopy(row)]
        self.run_generator()
        self.assertEqual(self.output()['callbacks'][-1], row)
        for mode in ('missing_preserve', 'false_preserve', 'missing_track', 'foreign_track',
                     'malformed_native_kind', 'wrong_position', 'missing_action', 'guard_change',
                     'extra_teleport', 'wrong_source_file'):
            with self.subTest(mode=mode):
                candidate = copy.deepcopy(row)
                if mode == 'missing_preserve':
                    del candidate['storage'][0]['preserve']
                elif mode == 'false_preserve':
                    candidate['storage'][0]['preserve'] = False
                elif mode == 'missing_track':
                    candidate['storage'].pop()
                elif mode == 'foreign_track':
                    candidate['storage'][0]['track'] = 'oteryn:quest-progress/foreign'
                elif mode == 'malformed_native_kind':
                    candidate['native_rule']['kind'] = []
                elif mode == 'wrong_position':
                    candidate['selector']['position']['x'] += 1
                elif mode == 'missing_action':
                    candidate['selector']['action'] = None
                elif mode == 'guard_change':
                    candidate['storage'][0]['minimum'] = 1
                elif mode == 'extra_teleport':
                    candidate['teleport'] = {'x': 100, 'y': 200, 'floor': -7}
                else:
                    candidate['sources'] = [tool.SOURCE_PREFIX + 'data-global/scripts/quests/cults_of_tibia/fixture.lua:1']
                self.packets['cults_of_tibia']['callbacks'] = [candidate]
                with self.assertRaises(ValueError):
                    self.run_generator()

    def test_exact_twenty_one_names_and_packet_directory_are_required(self):
        path = self.root / tool.INPUT_DIRECTORY / (tool.QUEST_ORDER[0] + '.json')
        path.unlink()
        with self.assertRaises(ValueError):
            tool.generate(self.root)
        self.save_packets()
        foreign = self.root / tool.INPUT_DIRECTORY / 'foreign_quest.json'
        write_json(foreign, packet('foreign_quest'))
        with self.assertRaises(ValueError):
            tool.generate(self.root)
        foreign.unlink()
        self.packets[tool.QUEST_ORDER[0]]['quest_directory'] = 'foreign_quest'
        with self.assertRaises(ValueError):
            self.run_generator()

    def test_callback_cap_reserves_the_dynamic_altar(self):
        self.packets['cults_of_tibia']['callbacks'] = [
            callback('cults_of_tibia', f'bounded-{index}') for index in range(tool.MAX_CALLBACKS - 19)
        ]
        with self.assertRaisesRegex(ValueError, str(tool.MAX_CALLBACKS)):
            self.run_generator()
        self.packets['cults_of_tibia']['callbacks'].pop()
        self.run_generator()
        self.assertEqual(len(self.output()['callbacks']), tool.MAX_CALLBACKS)

    def test_check_reports_stale_output_without_writing(self):
        self.run_generator()
        path = self.root / tool.NPC_OUTPUT_DIRECTORY / 'cults_of_tibia.json'
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text('{}\n')
        before = {file: file.read_bytes() for file in self.root.rglob('*.json')}
        with self.assertRaisesRegex(ValueError, 'stale'):
            self.run_generator(check=True)
        self.assertEqual(before, {file: file.read_bytes() for file in self.root.rglob('*.json')})

    def test_cli_root_and_check_use_only_the_selected_offline_fixture(self):
        command = [sys.executable, str(Path(tool.__file__)), '--root', str(self.root)]
        generated = subprocess.run(command, capture_output=True, text=True, check=False)
        self.assertEqual(generated.returncode, 0, generated.stderr)
        checked = subprocess.run([*command, '--check'], capture_output=True, text=True, check=False)
        self.assertEqual(checked.returncode, 0, checked.stderr)
        self.assertEqual(len(list((self.root / tool.NPC_OUTPUT_DIRECTORY).glob('*.json'))), 23)

    def test_selected_subset_preserves_other_callbacks_packets_and_source_outputs(self):
        name = 'lions_rock'
        selected = self.packets[name]
        admitted = callback(name, 'lions-rock-inner-sanctum-entry')
        excluded = callback(name, 'lions-rock-excluded-fixture')
        selected['callbacks'] = [admitted, excluded]
        selected['runtime_exclusions'] = [{
            'key': excluded['key'], 'reason': 'Source object is absent.',
            'question': 'QUESTION: Q4-SUBSET-FIXTURE',
        }]
        self.keep_packets({name})
        other = callback('dreamers_challenge_quest', 'dreamer-retained-fixture')
        previous = copy.deepcopy(admitted)
        previous['teleport']['x'] += 1
        self.seed['callbacks'].extend([other, previous, excluded])
        write_json(self.root / tool.CALLBACK_OUTPUT, self.seed)
        other_output = self.root / tool.NPC_OUTPUT_DIRECTORY / 'dreamers_challenge_quest.json'
        other_output.parent.mkdir(parents=True, exist_ok=True)
        other_output.write_bytes(b'{"runtime_admission": false, "text": "Keep exact bytes."}\n')
        original_packet = (self.root / tool.INPUT_DIRECTORY / (name + '.json')).read_bytes()
        original_other = other_output.read_bytes()

        tool.generate(self.root, selected=[name])
        self.assertEqual(self.output()['callbacks'], sorted(
            self.seed['callbacks'][:21] + [admitted], key=lambda row: row['key'],
        ))
        self.assertEqual(
            json.loads((self.root / tool.NPC_OUTPUT_DIRECTORY / (name + '.json')).read_text()),
            selected['npc_source'],
        )
        self.assertEqual(other_output.read_bytes(), original_other)
        self.assertEqual(
            (self.root / tool.INPUT_DIRECTORY / (name + '.json')).read_bytes(), original_packet,
        )
        self.assertEqual(tool.generate(self.root, check=True, selected=[name]), [])
        with self.assertRaisesRegex(ValueError, 'exactly 23'):
            tool.generate(self.root)

        # A separately authored but unselected packet is neither read nor rewritten.
        unrelated = self.root / tool.INPUT_DIRECTORY / 'dreamers_challenge_quest.json'
        unrelated.write_bytes(b'{"unselected_packet": "Unchanged, outside this invocation."}\n')
        untouched = {path: path.read_bytes() for path in self.root.rglob('*.json')}
        self.assertEqual(tool.generate(self.root, selected=[name]), [])
        self.assertEqual(untouched, {path: path.read_bytes() for path in self.root.rglob('*.json')})

    def test_subset_order_is_canonical_and_available_selects_only_present_inputs(self):
        names = ['lions_rock', 'cults_of_tibia']
        for name in names:
            self.packets[name]['callbacks'] = [callback(name, name + '-subset-fixture')]
        self.keep_packets(names)
        tool.generate(self.root, selected=names)
        self.assertEqual(
            [row['key'] for row in self.output()['callbacks']],
            sorted([row['key'] for row in self.seed['callbacks']]
                   + [self.packets[name]['callbacks'][0]['key'] for name in names]),
        )
        self.assertEqual(tool.generate(self.root, available=True, check=True), [])
        self.assertEqual(
            {path.stem for path in (self.root / tool.NPC_OUTPUT_DIRECTORY).glob('*.json')},
            set(names),
        )

    def test_subset_cli_available_and_explicit_modes_keep_default_strict(self):
        name = 'lions_rock'
        self.packets[name]['callbacks'] = [callback(name, 'lions-subset-cli-fixture')]
        self.keep_packets({name})
        command = [sys.executable, str(Path(tool.__file__)), '--root', str(self.root)]
        generated = subprocess.run([*command, '--available'], capture_output=True, text=True, check=False)
        self.assertEqual(generated.returncode, 0, generated.stderr)
        self.assertIn('1 source packets validated', generated.stdout)
        for selection in (['--available'], ['--quests', name]):
            checked = subprocess.run([*command, *selection, '--check'], capture_output=True, text=True, check=False)
            self.assertEqual(checked.returncode, 0, checked.stderr)
        strict = subprocess.run([*command, '--check'], capture_output=True, text=True, check=False)
        self.assertNotEqual(strict.returncode, 0)
        self.assertIn('exactly 23', strict.stderr)
        conflicting = subprocess.run(
            [*command, '--available', '--quests', name], capture_output=True, text=True, check=False,
        )
        self.assertNotEqual(conflicting.returncode, 0)
        self.assertEqual(
            {path.stem for path in (self.root / tool.NPC_OUTPUT_DIRECTORY).glob('*.json')}, {name},
        )

    def test_subset_rejects_empty_missing_duplicate_foreign_and_conflicting_selection(self):
        name = 'lions_rock'
        cases = (
            ({'selected': []}, {name}, False),
            ({'selected': [name, name]}, {name}, False),
            ({'selected': ['foreign_quest']}, {name}, False),
            ({'selected': ['cults_of_tibia']}, {name}, False),
            ({'selected': name}, {name}, False),
            ({'available': True}, set(), False),
            ({'selected': [name]}, {name}, True),
            ({'available': True}, {name}, True),
            ({'selected': [name], 'available': True}, {name}, False),
        )
        for options, present, foreign in cases:
            with self.subTest(options=options, present=present, foreign=foreign):
                self.keep_packets(present)
                if foreign:
                    write_json(self.root / tool.INPUT_DIRECTORY / 'foreign_quest.json', packet('foreign_quest'))
                before = {path: path.read_bytes() for path in self.root.rglob('*.json')}
                with self.assertRaises(ValueError):
                    tool.generate(self.root, **options)
                self.assertEqual(before, {path: path.read_bytes() for path in self.root.rglob('*.json')})


class TriangleTowerSourceTests(unittest.TestCase):
    def test_triangle_wall_source_preserves_asymmetric_ids(self):
        root = Path(__file__).resolve().parents[3]
        packet = tool.load_packets(root, selected=['triangle_tower_quest'])[0]
        wall, = packet['callbacks']
        self.assertEqual(wall['selector'], {'position': {'x':32573,'y':32121,'floor':-7},
            'unique':30034,'action':None,'appearance':2772})
        self.assertEqual(wall['passages'], [{'position':{'x':32566,'y':32119,'floor':-7},
            'appearance':1270,'replacement':None,'create_on_close':1025}])


if __name__ == '__main__':
    unittest.main()
