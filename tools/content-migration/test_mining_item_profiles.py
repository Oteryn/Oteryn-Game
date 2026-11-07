import json
from pathlib import Path
import unittest
from unittest.mock import patch

import mining_item_profiles as mining

ROOT = Path(__file__).resolve().parents[2]
PROFILES = 'content/items/definitions/spell-native-profiles.json'
MANIFEST = 'content/spells.manifest.json'


class MiningItemProfilesTest(unittest.TestCase):
    def generated(self):
        document = json.loads((ROOT / PROFILES).read_bytes())
        keys = {f'oteryn:item.tibia.i{i}' for i in mining.IDS}
        document['records'] = [r for r in document['records'] if r['production_definition']['production_key'] not in keys]
        return {PROFILES: json.dumps(document).encode(), MANIFEST: (ROOT / MANIFEST).read_bytes()}

    def admit(self, generated):
        mining.admit(ROOT, generated, PROFILES, MANIFEST,
                     lambda r: (r['production_definition']['production_key'].endswith('.i40450'), int(r['production_definition']['production_key'].split('.i')[-1])),
                     lambda value: (json.dumps(value, sort_keys=True) + '\n').encode())

    def test_exact_three_profiles_have_reference_stack_and_weight_facts(self):
        generated = self.generated()
        original = json.loads(generated[PROFILES])['records']
        self.admit(generated)
        result = json.loads(generated[PROFILES])['records']
        added = [r for r in result if r not in original]
        self.assertEqual(len(added), 3)
        self.assertTrue(all(r in result for r in original))
        self.assertEqual([r['semantics']['physical']['value']['weight']['value'] for r in added], [10, 4500, 156])
        self.assertEqual([r['admission']['stack_class'] for r in added], ['StackCapable', 'NonStackable', 'StackCapable'])
        with self.assertRaisesRegex(ValueError, 'MINING_PROFILE_IDENTITY'):
            self.admit(generated)

    def test_substituted_reference_revision_is_rejected(self):
        generated = self.generated()
        read = Path.read_bytes
        def altered(path):
            raw = read(path)
            if path == ROOT / mining.FACTS:
                facts = json.loads(raw)
                facts['source']['revision'] = 'unreviewed'
                return json.dumps(facts).encode()
            return raw
        with patch.object(Path, 'read_bytes', altered):
            with self.assertRaisesRegex(ValueError, 'MINING_REFERENCE_FACTS'):
                self.admit(generated)


if __name__ == '__main__':
    unittest.main()
