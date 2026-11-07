"""Real decoder round trips and Source cache admission boundary controls."""
import importlib.util
from pathlib import Path
from unittest.mock import patch
from donor_sources.maps import verify


def load_tests(loader, tests, pattern):
    directory = Path(__file__).resolve().parent / 'donor_sources/maps'
    for name in ('test_nodes', 'test_portable'):
        spec = importlib.util.spec_from_file_location('quest_maps_' + name, directory / (name + '.py'))
        module = importlib.util.module_from_spec(spec)
        with patch.dict('sys.modules', {'verify': verify}):
            spec.loader.exec_module(module)
        tests.addTests(loader.loadTestsFromModule(module))
    return tests
