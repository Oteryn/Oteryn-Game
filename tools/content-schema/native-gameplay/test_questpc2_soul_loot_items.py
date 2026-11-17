"""The selected Soul War loot closure keeps canonical and native custody aligned."""
import hashlib
import json
import unittest
from pathlib import Path
import questpc2_soul_loot_items as tool

ROOT = Path(__file__).resolve().parents[3]
EXISTING = frozenset({'oteryn:item.tibia.i3035', 'oteryn:item.tibia.i7439', 'oteryn:item.tibia.i7440', 'oteryn:item.tibia.i7443', 'oteryn:item.tibia.i9058', 'oteryn:item.tibia.i23374', 'oteryn:item.tibia.i49271', 'oteryn:item.tibia.i3041', 'oteryn:item.tibia.i3043', 'oteryn:item.tibia.i3039', 'oteryn:item.tibia.i23375', 'oteryn:item.tibia.i3037', 'oteryn:item.tibia.i23373'})
EXISTING_SHA256 = 'cfadd635e050604a0b654fe660bc652b81f9a1bf0034df0cb1eb8a714b422481'
NATIVE_PHYSICAL = frozenset({'oteryn:item.tibia.i34075', 'oteryn:item.tibia.i30061', 'oteryn:item.tibia.i30060', 'oteryn:item.tibia.i33922', 'oteryn:item.tibia.i3039', 'oteryn:item.tibia.i3036', 'oteryn:item.tibia.i34021', 'oteryn:item.tibia.i281', 'oteryn:item.tibia.i32622', 'oteryn:item.tibia.i34073', 'oteryn:item.tibia.i3038', 'oteryn:item.tibia.i9058', 'oteryn:item.tibia.i34076', 'oteryn:item.tibia.i30053', 'oteryn:item.tibia.i3043', 'oteryn:item.tibia.i34018', 'oteryn:item.tibia.i23375', 'oteryn:item.tibia.i34074', 'oteryn:item.tibia.i3037', 'oteryn:item.tibia.i34072', 'oteryn:item.tibia.i33924', 'oteryn:item.tibia.i30059', 'oteryn:item.tibia.i32623', 'oteryn:item.tibia.i3035', 'oteryn:item.tibia.i7440', 'oteryn:item.tibia.i33928', 'oteryn:item.tibia.i33952', 'oteryn:item.tibia.i3041', 'oteryn:item.tibia.i34109', 'oteryn:item.tibia.i32769', 'oteryn:item.tibia.i23373', 'oteryn:item.tibia.i34020', 'oteryn:item.tibia.i34019', 'oteryn:item.tibia.i7439', 'oteryn:item.tibia.i7443', 'oteryn:item.tibia.i33927', 'oteryn:item.tibia.i23374', 'oteryn:item.tibia.i49271', 'oteryn:item.tibia.i33926', 'oteryn:item.tibia.i33953', 'oteryn:item.tibia.i33923', 'oteryn:item.tibia.i33925', 'oteryn:item.tibia.i33920', 'oteryn:item.tibia.i33921'})
NATIVE_PHYSICAL_SHA256 = 'f928889642e1e23ab055d6ff1423a5dd142ce69e0de0e4f1063bb50675931a63'


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
                    if key not in NATIVE_PHYSICAL:
                        self.assertEqual(profile['semantics']['physical'], definition['semantics']['physical'])
                    for group in ('stack', 'container', 'charges', 'temporal'):
                        self.assertEqual(profile['semantics'][group], definition['semantics'][group])
        self.assertFalse(definitions['oteryn:item.tibia.i49271']['materializable'])
        self.assertTrue(profiles['oteryn:item.tibia.i49271']['admission']['materializable'])
        frozen = {key: definitions[key] for key in EXISTING}
        actual = hashlib.sha256(json.dumps(frozen, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
        self.assertEqual(actual, EXISTING_SHA256, 'already-qualified canonical records must remain byte-identical')
        native_physics = {key: profiles[key]['semantics']['physical'] for key in NATIVE_PHYSICAL}
        actual_physics = hashlib.sha256(json.dumps(native_physics, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
        self.assertEqual(actual_physics, NATIVE_PHYSICAL_SHA256, 'qualified native physical facts must remain byte-identical')
