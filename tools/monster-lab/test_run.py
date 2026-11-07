import tempfile
import unittest
from pathlib import Path

from lab import LabError
from run import check_result, merge_training, output_directory, prepare_input


class RunBoundaryTests(unittest.TestCase):
    def test_output_cannot_overwrite_repository(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            config = {'repository': str(root / 'repo'), 'read_only_inputs': {'stage': str(root / 'stage.json')}}
            with self.assertRaises(LabError):
                output_directory(config, root / 'repo' / 'results')

    def test_output_cannot_overwrite_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            config = {'repository': str(root / 'repo'), 'read_only_inputs': {'bundles': str(root / 'bundles')}}
            with self.assertRaises(LabError):
                output_directory(config, root / 'bundles' / 'results')

    def test_held_or_unknown_actor_cannot_claim_native_execution(self):
        report = {'monsters': [{'monster': 'held_boss', 'native_import': 'HELD'}]}
        with self.assertRaises(LabError):
            prepare_input({}, report, ['held_boss'], 30, 9)

    def test_excessive_run_is_rejected_before_input_loading(self):
        with self.assertRaises(LabError):
            prepare_input({}, {}, [], 1001, 9)

    def test_seed_outside_native_range_is_rejected(self):
        with self.assertRaises(LabError):
            prepare_input({}, {}, [], 30, 256)

    def test_false_success_cannot_hide_a_blocked_profile(self):
        result = self.result()
        result['outcomes'][0]['status'] = 'COMPONENT_BLOCKED'
        with self.assertRaises(LabError):
            check_result(result, self.manifest(), ['rat'])

    def test_stale_result_from_another_stage_is_rejected(self):
        result = self.result()
        result['stage_sha256'] = 'old-stage'
        with self.assertRaises(LabError):
            check_result(result, self.manifest(), ['rat'])

    def test_missing_requested_profile_is_rejected(self):
        with self.assertRaises(LabError):
            check_result(self.result(), self.manifest(), ['rat', 'cat'])

    def test_complete_partial_report_is_preserved_but_not_success(self):
        result = self.result()
        result.update(passed=False, passed_profiles=0, blocked=1)
        result['outcomes'][0]['status'] = 'COMPONENT_BLOCKED'
        self.assertFalse(check_result(result, self.manifest(), ['rat']))

    def test_outcome_from_another_tick_run_is_rejected(self):
        result = self.result()
        result['outcomes'][0]['schedule']['ticks'] = 1
        with self.assertRaises(LabError):
            check_result(result, self.manifest(), ['rat'])

    def test_training_cannot_replace_a_baseline_record(self):
        original = {'identity': {'family': 'Creature', 'key': 'oteryn:creature.rat', 'revision': 'r1'}}
        baseline = {'records': [original], 'authoring_profiles': []}
        extra = {'production_admission': False, 'runtime_activated': False,
                 'records': [dict(original, unexpected='changed')], 'authoring_profiles': [],
                 'admitted_training_monsters': []}
        with self.assertRaises(LabError):
            merge_training(baseline, extra, {'monsters': []})

    def test_training_requires_an_explicit_private_namespace(self):
        extra = {'production_admission': False, 'runtime_activated': False,
                 'records': [{'identity': {'family': 'Creature', 'key': 'oteryn:creature.boss', 'revision': 'r1'}}],
                 'authoring_profiles': [], 'admitted_training_monsters': []}
        with self.assertRaises(LabError):
            merge_training({'records': [], 'authoring_profiles': []}, extra, {'monsters': []})

    def test_generic_hp_fixture_cannot_claim_authored_monster_hp(self):
        result = self.result()
        result['outcomes'][0].update(authored_hp_tested=20, authored_source_key_admitted=True,
                                     native_owner_lifecycle={'initial_hp': 20, 'status': 'PASS'})
        with self.assertRaises(LabError):
            check_result(result, self.manifest(), ['rat'], {'rat': 1000})

    @staticmethod
    def manifest():
        return {'stage_sha256': 'stage', 'map': {'source_sha256': 'map'}, 'seed': 9, 'ticks': 30}

    @staticmethod
    def result():
        return {'passed': True, 'outcomes': [{'key': 'rat', 'status': 'PASS_NATIVE_SCHEDULE_PREPARATION',
                'fresh_replay_identical': True, 'schedule': {'ticks': 30, 'retry_idempotent': True}}],
                'scope': 'headless_native_component_arena', 'creatures_tested': 1,
                'passed_profiles': 1, 'blocked': 0, 'stage_sha256': 'stage',
                'map_source_sha256': 'map', 'seed': 9, 'live_server_started': False,
                'native_owner_lifecycle': {'status': 'PASS'}}


if __name__ == '__main__':
    unittest.main()
