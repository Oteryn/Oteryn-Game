"""The selected Soul War loot closure keeps canonical and native custody aligned."""
import hashlib
import json
import unittest
from pathlib import Path
import questpc2_soul_loot_items as tool

ROOT = Path(__file__).resolve().parents[3]
EXISTING = frozenset({'oteryn:item.tibia.i3035', 'oteryn:item.tibia.i3041', 'oteryn:item.tibia.i3043', 'oteryn:item.tibia.i3038', 'oteryn:item.tibia.i9058', 'oteryn:item.tibia.i3039', 'oteryn:item.tibia.i49271', 'oteryn:item.tibia.i23374', 'oteryn:item.tibia.i23373', 'oteryn:item.tibia.i23375', 'oteryn:item.tibia.i7443', 'oteryn:item.tibia.i3037', 'oteryn:item.tibia.i7440', 'oteryn:item.tibia.i7439'})
EXISTING_SHA256 = '0e108a17544784b30674cb651d4adaf58512801dd8eae1b836f1ec3ce2c87296'


class SoulLootClosureTests(unittest.TestCase):
    def test_every_selected_loot_item_has_canonical_and_native_custody(self):
        keys = {f'oteryn:item.tibia.i{i}' for i in tool.selected(ROOT)}
        source_ids = tool.shared.selected(ROOT, tool.BOSSES)
        self.assertEqual(tool.selected(ROOT) | tool.PROTECTED, source_ids)
        self.assertEqual(len(keys), 43)
        self.assertNotIn('oteryn:item.tibia.i49271', keys)
        self.assertIn('oteryn:item.tibia.i34109', keys)
        self.assertIn('oteryn:item.tibia.i33952', keys)
        definitions = {r['definition']['identity']['key']: r['definition']
                       for p in (ROOT / 'content/items/definitions').glob('items-*.json')
                       for r in json.loads(p.read_bytes())['records']}
        profiles = {r['production_definition']['production_key']: r for r in json.loads(
            (ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']}
        for key in sorted(keys):
            with self.subTest(item=key):
                definition, profile = definitions[key], profiles[key]
                self.assertTrue(definition['materializable'], key)
                self.assertTrue(profile['admission']['materializable'], key)
                self.assertEqual(profile['admission']['stack_class'], definition['stack_class'])
                if key not in EXISTING:
                    for group in ('physical', 'stack', 'container', 'charges', 'temporal'):
                        self.assertEqual(profile['semantics'][group], definition['semantics'][group])
        self.assertFalse(definitions['oteryn:item.tibia.i49271']['materializable'])
        self.assertTrue(profiles['oteryn:item.tibia.i49271']['admission']['materializable'])
        frozen = {key: definitions[key] for key in EXISTING}
        actual = hashlib.sha256(json.dumps(frozen, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
        self.assertEqual(actual, EXISTING_SHA256, 'already-qualified canonical records must remain byte-identical')
