"""Independent rewards cannot silently become nested container contents."""
import copy
import json
from pathlib import Path
import unittest
from jsonschema import Draft202012Validator


class BackpackOutputs(unittest.TestCase):
    def test_backpack_plan_is_bounded_and_exclusive_with_nested_contents(self):
        schema = json.loads(Path(__file__).with_name('shared-reward-callbacks.schema.json').read_text())
        validator = Draft202012Validator({'$defs': schema['$defs'], '$ref': '#/$defs/give'})
        item = {'item': 'oteryn:item.tibia.i3032', 'count': 3}
        reward = {'item': 'oteryn:item.tibia.i7343', 'count': 1, 'contents': [], 'backpack_outputs': [item]}
        validator.validate(reward)
        for edit in ('nested', 'twenty_children', 'zero_quantity', 'unknown_field'):
            bad = copy.deepcopy(reward)
            if edit == 'nested':
                bad['contents'] = [item]
            elif edit == 'twenty_children':
                bad['backpack_outputs'] = [item] * 20
            elif edit == 'zero_quantity':
                bad['backpack_outputs'][0]['count'] = 0
            else:
                bad['backpack_outputs'][0]['container'] = True
            self.assertTrue(list(validator.iter_errors(bad)), edit)
        legacy = {'item': reward['item'], 'count': 1, 'contents': [item]}
        validator.validate(legacy)


if __name__ == '__main__':
    unittest.main()
