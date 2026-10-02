"""Independent central-registry ownership and rejection checks on real source pins."""
from copy import deepcopy
import hashlib
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from jsonschema import Draft202012Validator, ValidationError

import native_catalog as catalog


ROOT = Path('/workspace/spell-sources')


def identities(module):
    """Declared source identities, preserving source-specific familiar aliases."""
    if module.__name__ == 'native_actor_states':
        return {name: ('instant', {source: spec['file'] for source, spec in sources.items()})
                for name, sources in module.SOURCE_SPECS.items()}
    if module.__name__ == 'native_combat':
        return {name: ('instant', {source: spec['file'] for source, spec in sources.items()})
                for name, sources in module.SPECS.items()}
    if module.__name__ == 'native_companions':
        return {name: (spec['carrier'], {source: row['path'] for source, row in spec['sources'].items()})
                for name, spec in module.SPECS.items()}
    if module.__name__ in ('native_delayed', 'native_house_movement'):
        return {name: ('instant', {source: path for source in module.REVISIONS})
                for name, path in module.FILES.items()}
    if module.__name__ == 'native_world_items':
        return {name: (carrier, {source: path for source in module.PINS})
                for name, (carrier, path) in module.PATHS.items()}
    raise AssertionError('Unreviewed source family: ' + module.__name__)


def real_sources(name, carrier, paths):
    records, texts = {}, {}
    for source, path in paths.items():
        text = (ROOT / source / path).read_text()
        raw = text.encode()
        records[source] = {'name': name, 'source': source, 'spell_type': carrier,
                           'file': path, 'source_root': ROOT / source,
                           'blob': hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()}
        texts[source, path] = text
    return records, texts


class CentralRegistryTests(unittest.TestCase):
    def test_six_source_families_have_disjoint_carrier_name_ownership(self):
        owners = {}
        expected_counts = {'native_actor_states': 12, 'native_combat': 20,
                           'native_companions': 16, 'native_delayed': 4,
                           'native_house_movement': 9, 'native_world_items': 6}
        for module in catalog.modules():
            names = identities(module)
            self.assertEqual(len(names), expected_counts[module.__name__])
            for name, (carrier, _) in names.items():
                key = carrier, name
                self.assertNotIn(key, owners, (key, owners.get(key), module.__name__))
                owners[key] = module.__name__
        self.assertEqual(len(owners), 67)
        schemas = catalog.schemas()
        self.assertEqual(len(schemas), 19)
        for schema in schemas.values():
            Draft202012Validator.check_schema(schema)

    def test_duplicate_keys_and_multiple_family_results_fail_closed(self):
        a = SimpleNamespace(__name__='a', schemas=lambda: {'duplicate': {'const': {}}})
        b = SimpleNamespace(__name__='b', schemas=lambda: {'duplicate': {'const': {}}})
        with patch.object(catalog, 'modules', return_value=[a, b]):
            with self.assertRaisesRegex(ValueError, 'duplicate native behavior schema'):
                catalog.schemas()
        a.build = b.build = lambda *args: {'key': 'duplicate', 'parameters': {}}
        with patch.object(catalog, 'modules', return_value=[a, b]):
            with self.assertRaisesRegex(ValueError, 'ambiguous native behavior ownership'):
                catalog.build('known', 'instant', {}, {})

    def test_actual_pins_qualify_every_named_profile_with_separate_evidence(self):
        if not all((ROOT / source / '.git').exists() for source in ('canary', 'crystal')):
            self.skipTest('Optional source integration requires the two pinned checkouts')
        counts = {'native': 0, 'barrier': 0}
        for module in catalog.modules():
            for name, (carrier, paths) in identities(module).items():
                with self.subTest(family=module.__name__, name=name):
                    records, texts = real_sources(name, carrier, paths)
                    before = deepcopy(records), deepcopy(texts)
                    result = catalog.build(name, carrier, records, texts)
                    if name in ('magic wall rune', 'wild growth rune'):
                        self.assertIsNone(result)
                        self.assertIsNotNone(catalog.barrier(name, carrier, records, texts))
                        self.assertTrue(catalog.barrier_evidence(name, carrier, records, texts)['observations'])
                        counts['barrier'] += 1
                    else:
                        self.assertIsNotNone(result)
                        native, evidence = result
                        self.assertEqual(set(native), {'key', 'parameters'})
                        self.assertEqual(evidence['family_module'], module.__name__)
                        self.assertTrue(evidence['observations'])
                        validator = Draft202012Validator(catalog.schemas()[native['key']])
                        invalid = deepcopy(native['parameters']); invalid['unrepresented_operation'] = True
                        with self.assertRaises(ValidationError):
                            validator.validate(invalid)
                        counts['native'] += 1
                    self.assertEqual((records, texts), before)
        self.assertEqual(counts, {'native': 65, 'barrier': 2})

    def test_changed_recognized_cast_or_census_blob_cannot_be_built(self):
        if not all((ROOT / source / '.git').exists() for source in ('canary', 'crystal')):
            self.skipTest('Optional source integration requires the two pinned checkouts')
        for module in catalog.modules():
            for name, (carrier, paths) in identities(module).items():
                with self.subTest(family=module.__name__, name=name):
                    records, texts = real_sources(name, carrier, paths)
                    source = next(iter(paths)); key = source, paths[source]
                    changed = dict(texts); changed[key] += '\n-- changed full source fixture\n'
                    try:
                        self.assertIsNone(catalog.build(name, carrier, records, changed))
                        if name in ('magic wall rune', 'wild growth rune'):
                            self.assertIsNone(catalog.barrier(name, carrier, records, changed))
                    except ValueError:
                        pass  # Recognized donor drift may explicitly refuse conversion.
                    bad = deepcopy(records); bad[source]['blob'] = '0' * 40
                    try:
                        self.assertIsNone(catalog.build(name, carrier, bad, texts))
                        if name in ('magic wall rune', 'wild growth rune'):
                            self.assertIsNone(catalog.barrier(name, carrier, bad, texts))
                    except ValueError:
                        pass


if __name__ == '__main__':
    unittest.main()
