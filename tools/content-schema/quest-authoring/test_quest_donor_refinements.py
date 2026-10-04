"""Portable aggregate of concrete donor refinement lane regressions."""
import importlib.util
from pathlib import Path
import sys
from unittest.mock import patch


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_tests(loader, tests, pattern):
    base = Path(__file__).parent / 'donor_sources/refinements'
    for lane in ('conditions', 'joins', 'joins_next', 'joins_next2', 'fields', 'helpers', 'composition', 'progress', 'progress_config'):
        module = load_module('donor_refinement_' + lane, base / lane / 'builder.py')
        with patch.dict(sys.modules, {'builder': module}):
            suite = load_module('test_donor_refinement_' + lane, base / lane / 'test_builder.py')
        tests.addTests(loader.loadTestsFromModule(suite))
    for lane, filename in (('entity_predicates', 'test_engine.py'),
                           ('entity_values', 'test_getter_values.py')):
        if lane == 'entity_values':
            dependency = load_module('donor_getter_dependencies', base / lane / 'dependency_model.py')
            with patch.dict(sys.modules, {'dependency_model': dependency}):
                module = load_module('donor_getter_values', base / lane / 'getter_values.py')
            with patch.dict(sys.modules, {'getter_values': module, 'dependency_model': dependency}):
                suite = load_module('test_donor_' + lane, base / lane / filename)
        else:
            suite = load_module('test_donor_' + lane, base / lane / filename)
        tests.addTests(loader.loadTestsFromModule(suite))
    suite = load_module('test_donor_dialogue_all', base / 'dialogue/test_quest_dialogue_links_all.py')
    tests.addTests(loader.loadTestsFromModule(suite))
    module = load_module('donor_dialogue_scoped', base / 'dialogue/quest_dialogue_links_all.py')
    with patch.dict(sys.modules, {'quest_dialogue_links_all': module}):
        suite = load_module('test_donor_dialogue_scoped', base / 'dialogue/test_scoped_storage_aliases.py')
    tests.addTests(loader.loadTestsFromModule(suite))
    return tests
