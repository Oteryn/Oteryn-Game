"""Source control-flow fidelity, typed projections and ownership boundary controls."""
import importlib.util
from pathlib import Path
from unittest.mock import patch
from quest_component_authoring import partition
import unittest


class AssignmentTests(unittest.TestCase):
    def test_overlapping_source_id_rejected(self):
        rows = [{'source_component_id': 'same', 'source_only_mechanisms': tags} for tags in
                (['boss_lever_shared_constructor'], ['think_event'])]
        with self.assertRaises(ValueError):
            partition(rows)


def load_tests(loader, tests, pattern):
    directory = Path(__file__).resolve().parent / 'donor_sources/components'
    for lane, producer_name, test_name in (
        ('boss', 'builder', 'test_builder'), ('events', 'builder', 'test_builder'),
        ('other', 'builder', 'test_builder'), ('joins', 'component_crosswalk', 'test_component_crosswalk'),
        ('dialogue', 'dialogue_supplement', 'test_dialogue_supplement')):
        namespace = 'quest_components_' + lane
        spec = importlib.util.spec_from_file_location(namespace, directory / lane / (producer_name + '.py'))
        producer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(producer)
        spec = importlib.util.spec_from_file_location(namespace + '_tests', directory / lane / (test_name + '.py'))
        module = importlib.util.module_from_spec(spec)
        with patch.dict('sys.modules', {producer_name: producer}):
            spec.loader.exec_module(module)
        tests.addTests(loader.loadTestsFromModule(module))
    return tests
