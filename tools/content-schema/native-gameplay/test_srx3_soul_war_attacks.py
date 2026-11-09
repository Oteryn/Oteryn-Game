import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import build_srx3_soul_war_attacks as tool


class SoulWarAttackWitnessTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        source = json.loads((tool.ROOT / 'content/creatures/definitions/spell-native-profiles.json').read_bytes())
        keys = {'oteryn:creature.' + slug for slug in tool.SPECS}
        cls.source = {'source_definitions_sha256': source['source_definitions_sha256'],
                      'records': [row for row in source['records'] if row['profile']['target']['key'] in keys]}
        cls.canonical = [row for path in (tool.ROOT / 'content/creatures/definitions').glob('creatures-*.json')
                         for row in json.loads(path.read_bytes())['records']
                         if row['definition']['identity']['key'] in keys]

    def build(self, source=None, canonical=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / 'content/creatures/definitions'
            target.mkdir(parents=True)
            (target / 'spell-native-profiles.json').write_text(json.dumps(source or self.source))
            (target / 'creatures-test.json').write_text(json.dumps({'records': canonical or self.canonical}))
            with patch.object(tool, 'ROOT', root):
                return tool.build()

    def test_exact_source_slots_preserve_actual_native_bodies_and_full_root(self):
        result = self.build()
        self.assertEqual(len(result['records']), 4)
        self.assertEqual(result['source_definitions_sha256'], self.source['source_definitions_sha256'])
        self.assertEqual({row['index'] for row in result['records']}, {1, 5, 7})
        self.assertEqual({row['schedule']['interval_ms'] for row in result['records']}, {6000, 7000})
        self.assertTrue(all(row['schedule']['ability'] == row['ability']['target'] for row in result['records']))
        self.assertEqual({row['native']['profile']['target']['key'] for row in result['records']},
                         {row['profile']['target']['key'] for row in self.source['records']})

    def test_rebound_full_source_digest_cannot_bless_altered_native_attack_clock(self):
        source = copy.deepcopy(self.source)
        source['source_definitions_sha256'] = 'a' * 64
        source['records'][0]['behavior']['data']['profile']['attacks'][0]['interval_ms'] += 1
        with self.assertRaisesRegex(ValueError, 'Native Soul War body changed'):
            self.build(source=source)

    def test_exact_creature_source_binding_is_required(self):
        canonical = copy.deepcopy(self.canonical)
        canonical[0]['source_bindings'][0]['source_revision'] = '0' * 40
        with self.assertRaisesRegex(ValueError, 'Native source binding changed'):
            self.build(canonical=canonical)


if __name__ == '__main__':
    unittest.main()
