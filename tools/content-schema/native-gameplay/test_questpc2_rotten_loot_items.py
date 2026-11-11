"""Six source admissions preserve every accepted record outside their explicit scope."""
import copy
import hashlib
import json
import unittest
from pathlib import Path
import questpc2_rotten_loot_items as tool

ROOT = Path(__file__).resolve().parents[3]


def outside_snapshot(records, keys):
    # Compare every complete outside record, including JSON types, across this
    # owner's call. Other qualified producers can advance the repository corpus.
    return json.dumps({key: row for key, row in records.items() if key not in keys},
                      sort_keys=True, separators=(',', ':')).encode()


class RottenItemAdmissionTests(unittest.TestCase):
    def test_exact_six_canonical_and_native_admissions_preserve_other_records(self):
        from qualification_inputs import appearance_input_scope

        keys = {f'oteryn:item.tibia.i{i}' for i in tool.selected()}
        self.assertEqual(len(keys), 6)
        definitions = {}
        for path in (ROOT / 'content/items/definitions').glob('items-*.json'):
            for row in json.loads(path.read_bytes())['records']:
                definition = row['definition']
                self.assertNotIn(definition['identity']['key'], definitions)
                definitions[definition['identity']['key']] = definition
        profile_path = 'content/items/definitions/spell-native-profiles.json'
        document = json.loads((ROOT / profile_path).read_bytes())
        profiles = {r['production_definition']['production_key']: r
                    for r in document['records']}
        self.assertEqual(len(profiles), len(document['records']))
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
                    if group == 'physical' and key in {
                            'oteryn:item.tibia.i43501', 'oteryn:item.tibia.i43502',
                            'oteryn:item.tibia.i43503', 'oteryn:item.tibia.i43504'}:
                        # The earlier BossYield owner retains its source weight;
                        # canonical reference weights remain independently fixed.
                        expected = copy.deepcopy(definitions[key]['semantics'][group])
                        self.assertEqual(expected['value']['weight'],
                                         {'state': 'KNOWN', 'value': 150})
                        expected['value']['weight'] = {'state': 'KNOWN', 'value': 0}
                        self.assertEqual(profiles[key]['semantics'][group], expected)
                        continue
                    self.assertEqual(definitions[key]['semantics'][group], profiles[key]['semantics'][group])
        canonical_outside = outside_snapshot(definitions, keys)
        native_outside = outside_snapshot(profiles, keys)
        accepted = {key: copy.deepcopy(definitions[key]) for key in keys}
        native_selected = {key: copy.deepcopy(profiles[key]) for key in keys}
        # Exercise the six actual admissions while retaining every accepted
        # fact. The reset is only an in-memory pre-admission fixture.
        for key in keys:
            definitions[key]['materializable'] = False
        manifest_path = 'manifest'
        generated = {profile_path: json.dumps(document).encode(),
                     manifest_path: b'{"item_profiles":{}}'}
        canonical = lambda value: (json.dumps(value, sort_keys=True) + '\n').encode()
        with appearance_input_scope():
            tool.apply_admissions(list(definitions.values()), ROOT)
            tool.admit_native(ROOT, generated, profile_path, manifest_path,
                              lambda row: row['production_definition']['production_key'],
                              canonical)
        self.assertEqual({key: definitions[key] for key in keys}, accepted)
        self.assertEqual(outside_snapshot(definitions, keys), canonical_outside)
        actual_document = json.loads(generated[profile_path])
        actual = {row['production_definition']['production_key']: row
                  for row in actual_document['records']}
        self.assertEqual(len(actual), len(actual_document['records']))
        self.assertEqual(set(actual), set(profiles))
        self.assertEqual({key: actual[key] for key in keys}, native_selected)
        self.assertEqual(outside_snapshot(actual, keys), native_outside)
        self.assertEqual(json.loads(generated[manifest_path])['item_profiles']['sha256'],
                         hashlib.sha256(generated[profile_path]).hexdigest())

    def test_boss_yield_custody_survives_real_rotten_capture(self):
        import boss_reward_yield_profiles as boss
        from qualification_inputs import appearance_input_scope

        canonical_inputs = {path: hashlib.sha256(path.read_bytes()).hexdigest() for path in
                            (ROOT / 'content/items/definitions').glob('items-*.json')}
        selected = {f'oteryn:item.tibia.i{i}' for i in (43501, 43502, 43503, 43504)}
        generated = {'profiles': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
        order = lambda row: row['production_definition']['production_key']
        with appearance_input_scope():
            boss.admit(ROOT, generated, 'profiles', 'manifest', order, boss.corpses.encode)
            before = {row['production_definition']['production_key']: row
                      for row in json.loads(generated['profiles'])['records']}
            self.assertEqual(len(before), 46)
            self.assertTrue(selected <= before.keys())
            tool.admit_native(ROOT, generated, 'profiles', 'manifest', order, boss.corpses.encode)
            records = json.loads(generated['profiles'])['records']
            actual = {row['production_definition']['production_key']: row for row in records}
            self.assertEqual(len(records), 48)
            self.assertEqual(len(actual), len(records))
            self.assertEqual(set(actual) - set(before), {
                'oteryn:item.tibia.i43899', 'oteryn:item.tibia.i43900'})
            for key, row in before.items():
                self.assertEqual(actual[key], row, key)
            self.assertEqual(json.loads(generated['manifest'])['item_profiles']['sha256'],
                             hashlib.sha256(generated['profiles']).hexdigest())
        definitions = {row['definition']['identity']['key']: row['definition']
                       for path in canonical_inputs
                       for row in json.loads(path.read_bytes())['records']
                       if row['definition']['identity']['key'] in selected}
        self.assertEqual(set(definitions), selected)
        for key in selected:
            expected = copy.deepcopy(definitions[key]['semantics']['physical'])
            self.assertEqual(expected['value']['weight'], {'state': 'KNOWN', 'value': 150})
            expected['value']['weight'] = {'state': 'KNOWN', 'value': 0}
            self.assertEqual(actual[key]['semantics']['physical'], expected, key)
        for path, expected_hash in canonical_inputs.items():
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), expected_hash, str(path))
