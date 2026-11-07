"""Native Wrap policy is derived only from the qualified portable source roster."""
from pathlib import Path
import copy
import importlib.util
import json
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('furniture_wrap_owner', Path(__file__).with_name('furniture_item_profiles.py'))
OWNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(OWNER)


class NativeWrapSource(unittest.TestCase):
    def setUp(self):
        self.rules = json.loads((ROOT / OWNER.RULES).read_bytes())
        self.profiles = json.loads((ROOT / OWNER.PROFILE).read_bytes())

    def test_exact_source_roster_and_quantity_policy(self):
        [(name, sql)] = OWNER.generated_wrap_admission(self.rules, self.profiles)
        self.assertEqual(name, 'tools/content-schema/native-gameplay/furniture-native-wrap.generated.sql')
        self.assertNotIn(b'__PORTABLE_WRAP_ROWS__', sql)
        self.assertIn(b'(14687,1)', sql)
        self.assertIn(b'(36951,100)', sql)
        self.assertNotIn(b'(25879,', sql)

    def test_admission_identity_and_stack_drift_fail_closed(self):
        for drift in ('unknown_stack', 'excess_stack', 'nonmaterializable', 'wrong_revision', 'missing_source'):
            with self.subTest(drift=drift):
                profiles = copy.deepcopy(self.profiles)
                source = next(row for row in profiles['records'] if row['production_definition']['production_key'] == 'oteryn:item.tibia.i36951')
                if drift == 'unknown_stack':
                    source['semantics']['stack'] = {'state': 'UNKNOWN'}
                elif drift == 'excess_stack':
                    source['semantics']['stack']['value']['stack_max']['value'] = 101
                elif drift == 'nonmaterializable':
                    source['admission']['materializable'] = False
                elif drift == 'wrong_revision':
                    source['production_definition']['revision_ref'] = 'definition-r2'
                else:
                    profiles['records'].remove(source)
                with self.assertRaises(ValueError):
                    OWNER.generated_wrap_admission(self.rules, profiles)


if __name__ == '__main__':
    unittest.main()
