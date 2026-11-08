"""Six source admissions preserve every accepted record outside their explicit scope."""
import hashlib
import json
import unittest
from pathlib import Path
import questpc2_rotten_loot_items as tool

ROOT = Path(__file__).resolve().parents[3]
CANONICAL_OUTSIDE = 'a36cccacd5536b476e508e0b1d2936657c80485d62c9f0df962c3865c2876130'
NATIVE_OUTSIDE = 'a64f6741e8bf27700dab39a0545dbe0b524b34f65542bf8b58a4ce63dd44a3bc'


def digest(records):
    return hashlib.sha256(json.dumps(records, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


class RottenItemAdmissionTests(unittest.TestCase):
    def test_exact_six_canonical_and_native_admissions_preserve_other_records(self):
        keys = {f'oteryn:item.tibia.i{i}' for i in tool.selected()}
        self.assertEqual(len(keys), 6)
        definitions = {}
        for path in (ROOT / 'content/items/definitions').glob('items-*.json'):
            for row in json.loads(path.read_bytes())['records']:
                definition = row['definition']
                definitions[definition['identity']['key']] = definition
        profiles = {r['production_definition']['production_key']: r for r in json.loads(
            (ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']}
        for key in sorted(keys):
            with self.subTest(item=key):
                self.assertTrue(definitions[key]['materializable'])
                self.assertTrue(profiles[key]['admission']['materializable'])
                self.assertEqual(definitions[key]['stack_class'], profiles[key]['admission']['stack_class'])
                for group in tool.source.GROUPS:
                    if group == 'physical' and key in {'oteryn:item.tibia.i43899', 'oteryn:item.tibia.i43900'}:
                        # Their admitted native zero weight remains owned by its original source.
                        self.assertEqual(profiles[key]['semantics'][group]['value']['weight'],
                                         {'state': 'KNOWN', 'value': 0})
                        self.assertEqual(definitions[key]['semantics'][group]['value']['weight'],
                                         {'state': 'KNOWN', 'value': 450})
                        continue
                    self.assertEqual(definitions[key]['semantics'][group], profiles[key]['semantics'][group])
        self.assertEqual(digest({k: v for k, v in definitions.items() if k not in keys}), CANONICAL_OUTSIDE)
        self.assertEqual(digest({k: v for k, v in profiles.items() if k not in keys}), NATIVE_OUTSIDE)
