"""The callback view must reuse the exact pinned harvest stack, without edits."""
import copy
import json
import unittest
from pathlib import Path

import questpc8_nail_custody as nails
import quest_callback_profiles as callbacks

ROOT = Path(__file__).resolve().parents[2]


class NailCustody(unittest.TestCase):
    def setUp(self):
        self.definition = next(row['definition']
                               for path in (ROOT / 'content/items/definitions').glob('items-*.json')
                               for row in json.loads(path.read_bytes())['records']
                               if row['definition']['identity']['key'] == 'oteryn:item.tibia.i953')
        self.native = next(row for row in json.loads(
            (ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']
                           if row['production_definition']['production_key'] == 'oteryn:item.tibia.i953')

    def test_exact_pinned_stack_is_accepted_without_mutating_canonical(self):
        before = copy.deepcopy(self.definition)
        view = nails.callback_definition(ROOT, self.native, self.definition)
        callbacks.complete_native_stack(ROOT, self.native, view, self.native['production_binding'])
        self.assertEqual(before, self.definition)
        self.assertEqual(view['stack_class'], 'StackCapable')

    def test_changed_stack_is_rejected(self):
        self.native['semantics']['stack']['value']['stack_max']['value'] = 99
        with self.assertRaisesRegex(ValueError, 'QUESTPC8_NAIL_SOURCE_CONFLICT'):
            nails.callback_definition(ROOT, self.native, self.definition)
