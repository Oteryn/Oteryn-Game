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
    def test_temporary_objects_require_durable_use_and_bounded_lifetime(self):
        c = callback('tibia_tales', 'temporary')
        c.update(trigger='use', teleport=None,
                 storage=[{'track':'oteryn:quest-progress/fixture','minimum':1,'maximum':1,'set':1,'preserve':True}])
        c['effects'] = {'temporary_item': {'appearance':3133,'action':4206,'lifetime_us':60000000},
                        'present':[{'item':'oteryn:item.tibia.i3457','count':1}]}
        tool.validate_effects(c, 'temporary')
        for field,value in [('appearance',0),('action',0),('lifetime_us',0),('lifetime_us',86400000001)]:
            bad=copy.deepcopy(c);bad['effects']['temporary_item'][field]=value
            with self.assertRaises(ValueError):tool.validate_effects(bad,'temporary')
        for mutation in ('readonly','missing-state','step','duplicate-tool','zero-tool','mixed-projection'):
            bad=copy.deepcopy(c)
            if mutation=='readonly':bad['storage'][0]['read_only']=True
            elif mutation=='missing-state':bad['storage']=[]
            elif mutation=='step':bad['trigger']='step_in'
            elif mutation=='duplicate-tool':bad['effects']['present']*=2
            elif mutation=='zero-tool':bad['effects']['present'][0]['count']=0
            else:bad['effects']['item_cycle']={'position':{'x':100,'y':200,'floor':-7},'remove':103,'create':3133}
            with self.assertRaises(ValueError,msg=mutation):tool.validate_effects(bad,'temporary')

    def test_failure_teleport_requires_valid_native_position_and_returning_step(self):
        row = callback('cults_of_tibia', 'failure-landing')
        row['return_on_failure'] = True
        row['failure_teleport'] = {'x': 32818, 'y': 31599, 'floor': -9}
        tool.validate_callback(row, 'cults_of_tibia')
        for field, value in [('trigger', 'use'), ('return_on_failure', False),
                             ('failure_teleport', {'x': 1, 'y': 2, 'floor': -16})]:
            with self.assertRaises(ValueError):
                tool.validate_callback({**row, field: value}, 'cults_of_tibia')


    def test_conditional_set_is_exact_and_cannot_be_a_read_only_guard(self):
        row = callback('cults_of_tibia', 'conditional-reset')
        row['storage'] = [{'track': 'oteryn:quest-progress/conditional-reset/flag',
                           'minimum': -1, 'maximum': 2, 'set': 0, 'set_if': 1}]
        tool.validate_callback(row, 'cults_of_tibia')
        for patch in [{'set_if': 3}, {'set_if': True}, {'preserve': True}, {'read_only': True}]:
            invalid = copy.deepcopy(row)
            invalid['storage'][0].update(patch)
            with self.assertRaises(ValueError):
                tool.validate_callback(invalid, 'cults_of_tibia')

    def test_remote_teleport_requires_a_readonly_step_and_bounded_positions(self):
        source = Path(__file__).parent / 'batches/quests4/formogar_mine_hoist.json'
        row = json.loads(source.read_text())['callbacks'][0]
        row['remote_teleport'] = {
            'position': {'x': 32156, 'y': 31125, 'floor': -10},
            'appearance': 2772,
            'destination': {'x': 32157, 'y': 31125, 'floor': -11},
        }
        tool.validate_callback(row, 'formogar_mine_hoist')
        for field, value in [('appearance', 0), ('appearance', 65536),
                             ('destination', {'x': 1, 'y': 1, 'floor': -16})]:
            invalid = copy.deepcopy(row)
            invalid['remote_teleport'][field] = value
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                tool.validate_callback(invalid, 'formogar_mine_hoist')
        row['trigger'] = 'use'
        with self.assertRaises(ValueError):
            tool.validate_callback(row, 'formogar_mine_hoist')

    def test_hoist_step_reads_both_instructions_without_a_write(self):
        source = Path(__file__).parent / 'batches/quests4/formogar_mine_hoist.json'
        row = json.loads(source.read_text())['callbacks'][0]
        for storage in row['storage']:
            storage.pop('preserve', None)
            storage['read_only'] = True
        tool.validate_callback(row, 'formogar_mine_hoist')
        mixed = copy.deepcopy(row)
        mixed['storage'][1]['read_only'] = False
        with self.assertRaises(ValueError):
            tool.validate_callback(mixed, 'formogar_mine_hoist')

    def test_paired_plate_departure_is_stateless_and_cannot_teleport(self):
        row = callback('thais_lighthouse', 'thais-lighthouse-stairs-out')
        row.update(trigger='step_out', teleport=None, lever=[431, 430], reversible=True)
        tool.validate_callback(row, 'thais_lighthouse')
        for field, value in [('teleport', {'x': 100, 'y': 200, 'floor': -7}),
                             ('storage', [{'track': 'oteryn:track/fixture',
                                           'minimum': -1, 'maximum': 1, 'set': 1}])]:
            with self.subTest(field=field):
                invalid = copy.deepcopy(row)
                invalid[field] = value
                with self.assertRaises(ValueError):
                    tool.validate_callback(invalid, 'thais_lighthouse')
    def test_failure_teleport_requires_valid_native_position_and_returning_step(self):
        row = callback('cults_of_tibia', 'failure-landing')
        row['return_on_failure'] = True
        row['failure_teleport'] = {'x': 32818, 'y': 31599, 'floor': -9}
        tool.validate_callback(row, 'cults_of_tibia')
        for field, value in [('trigger', 'use'), ('return_on_failure', False),
                             ('failure_teleport', {'x': 1, 'y': 2, 'floor': -16})]:
            with self.assertRaises(ValueError):
                tool.validate_callback({**row, field: value}, 'cults_of_tibia')

    def test_conditional_set_is_exact_and_cannot_be_a_read_only_guard(self):
        row = callback('cults_of_tibia', 'conditional-reset')
        row['storage'] = [{'track': 'oteryn:quest-progress/conditional-reset/flag',
                           'minimum': -1, 'maximum': 2, 'set': 0, 'set_if': 1}]
        tool.validate_callback(row, 'cults_of_tibia')
        for patch in [{'set_if': 3}, {'set_if': True}, {'preserve': True}, {'read_only': True}]:
            invalid = copy.deepcopy(row)
            invalid['storage'][0].update(patch)
            with self.assertRaises(ValueError):
                tool.validate_callback(invalid, 'cults_of_tibia')

    def test_remote_conditions_allow_stateless_routes_and_reject_overloading(self):
        row = json.loads((Path(__file__).parent / 'batches/quests4/formogar_mine_hoist.json').read_text())['callbacks'][0]
        row['storage'] = []
        row['remote_teleport'] = {
            'position': {'x': 32802, 'y': 31584, 'floor': -1}, 'appearance': 2772,
            'destination': {'x': 32701, 'y': 31639, 'floor': -6},
            'additional': [{'position': {'x': 32803, 'y': 31584, 'floor': -1}, 'appearance': 2773}],
        }
        tool.validate_callback(row, 'formogar_mine_hoist')
        for edit in range(5):
            invalid = copy.deepcopy(row)
            condition = invalid['remote_teleport']['additional'][0]
            if edit == 0:
                condition['appearance'] = 0
            elif edit == 1:
                condition['position']['floor'] = -16
            elif edit == 2:
                condition['position'] = invalid['remote_teleport']['position']
            elif edit == 3:
                invalid['remote_teleport']['additional'] *= 8
            else:
                invalid['offerings'] = [{'position': {'x': 1, 'y': 2, 'floor': -1},
                                         'item': 'oteryn:item.tibia.i3031', 'count': 1}]
            with self.subTest(edit=edit), self.assertRaises(ValueError):
                tool.validate_callback(invalid, 'formogar_mine_hoist')

    def test_current_crystal_world_witness_requires_lines_and_sha_without_outfit(self):
        source = ('crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:'
                  'data-global/scripts/quests/draconia/movement-escape.lua:9-26:'
                  'sha256:53f3313e99fb541c3e016b5ac09a2e372c0ad8a382786ccd79db6bb40a700312')
        tool.provenance([source], 'world witness')
        for invalid in [source.replace(':9-26', ''), source.replace(':9-26', ':26-9'),
                        source.replace('9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d', 'untrusted'),
                        source[:-1] + 'g']:
            with self.subTest(source=invalid), self.assertRaises(ValueError):
                tool.provenance([invalid], 'world witness')
        tool.provenance([source.replace(':9-26', '')], 'legacy wardrobe witness', outfit=True)

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
        self.assertEqual(len(list((self.root / tool.NPC_OUTPUT_DIRECTORY).glob('*.json'))), 21)

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
        with self.assertRaisesRegex(ValueError, 'exactly 21'):
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
        self.assertIn('exactly 21', strict.stderr)
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


if __name__ == '__main__':
    unittest.main()
