"""Added source leaves retain native qualification and fail closed on stale facts."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import native_source_claims as tool
import reward_claim_authoring as author

ROOT = Path(__file__).resolve().parents[3]


class NativeSourceClaims(unittest.TestCase):
    def source(self):
        return json.loads((ROOT / tool.PATH).read_text())

    def write(self, root, value):
        path = root / tool.PATH
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(value))

    def test_existing_native_armor_is_used_without_promoting_the_old_pilot(self):
        claims, _, _ = tool.load(ROOT)
        items = author.load_items()
        original = copy.deepcopy(items['oteryn:item.tibia.i3397'])
        native = tool.item_facts(ROOT, items, claims)
        self.assertEqual(items['oteryn:item.tibia.i3397'], original)
        self.assertEqual(set(native), {'oteryn:item.tibia.i3397'})
        self.assertIsNone(author.stack_problem(native['oteryn:item.tibia.i3397'], 1))
        self.assertEqual(native['oteryn:item.tibia.i3397']['stack_class'], 'NonStackable')
        self.assertEqual(native['oteryn:item.tibia.i3397']['semantics']['physical']['value']['weight']['value'], 13000)

    def test_callback_cohort_preserves_known_facts_and_refuses_unqualified_native(self):
        import sys
        sys.path.insert(0, str(ROOT / 'tools/content-migration'))
        native = next(r for r in json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']
                      if r['production_definition']['production_key'] == 'oteryn:item.tibia.i3397')
        definition = next(r['definition'] for path in (ROOT / 'content/items/definitions').glob('items-*.json')
                          for r in json.loads(path.read_bytes())['records']
                          if r['definition']['identity']['key'] == 'oteryn:item.tibia.i3397')
        before = copy.deepcopy(definition)
        effective = tool.callback_definition(ROOT, native, definition)
        self.assertTrue(effective['materializable'])
        self.assertEqual(effective['stack_class'], 'NonStackable')
        self.assertEqual(effective['semantics']['equipment'], before['semantics']['equipment'])
        self.assertEqual(definition, before)
        bad = copy.deepcopy(native)
        bad['admission']['materializable'] = False
        with self.assertRaisesRegex(ValueError, 'callback native admission'):
            tool.callback_definition(ROOT, bad, definition)
        outside = copy.deepcopy(definition)
        outside['identity']['key'] = 'oteryn:item.tibia.i3398'
        self.assertIs(tool.callback_definition(ROOT, None, outside), outside)

    def test_duplicate_cells_and_unpinned_witnesses_are_rejected(self):
        for mutation in ('cell', 'witness', 'revision'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                source = self.source()
                if mutation == 'cell':
                    other = copy.deepcopy(source['claims'][0])
                    other['key'] += '-other'
                    source['claims'].append(other)
                elif mutation == 'witness':
                    source['claims'][0]['source']['url'] += '-changed'
                else:
                    source['claims'][0]['placement_source']['revision'] = '0' * 40
                self.write(root, source)
                with self.assertRaises(ValueError):
                    tool.load(root)

    def test_stale_native_profile_hash_cannot_supply_admission_facts(self):
        claims, _, _ = tool.load(ROOT)
        items = author.load_items()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            profile = root / 'content/items/definitions/profiles.json'
            profile.parent.mkdir(parents=True)
            raw = b'{"records": []}'
            profile.write_bytes(raw + b' ')
            manifest = root / 'content/spells.manifest.json'
            manifest.write_text(json.dumps({'item_profiles': {
                'path': 'items/definitions/profiles.json', 'sha256': hashlib.sha256(raw).hexdigest()}}))
            with self.assertRaisesRegex(ValueError, 'stale native profile'):
                tool.item_facts(root, items, claims)

    def test_runtime_include_regeneration_preserves_handwritten_code(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = 'crates/gameplay/content/src/content/world_reward_claims.rs'
            target = root / path
            target.parent.mkdir(parents=True)
            target.write_text('const BEFORE: u8 = 1;\npub const REWARD_CLAIM_SHARDS: [&str; 1] = [\n    "old"\n];\nconst AFTER: u8 = 2;\n')
            result = author.runtime_shard_source(root, ['content/a.json', 'content/b.json'])[path]
            self.assertTrue(result.startswith('const BEFORE: u8 = 1;\n'))
            self.assertTrue(result.endswith('const AFTER: u8 = 2;\n'))
            self.assertIn('REWARD_CLAIM_SHARDS: [&str; 2]', result)
            self.assertNotIn('"old"', result)
            self.assertIn('../../../../../content/a.json', result)
            self.assertIn('../../../../../content/b.json', result)


if __name__ == '__main__':
    unittest.main()
