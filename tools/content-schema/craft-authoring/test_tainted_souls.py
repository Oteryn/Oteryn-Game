"""Source and count-policy regressions for the M22 carried transformation."""
import hashlib
import json
import copy
import jsonschema
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import craft_authoring as craft
import tainted_souls as source


class TaintedSouls(unittest.TestCase):
    def test_recipe_is_registered_once_in_the_production_craft_catalogue(self):
        rows = [row for row in craft.build() if row['identity']['key'] == source.KEY]
        self.assertEqual(len(rows), 1, 'the qualified transform must be playable, not only a source packet')
        self.assertEqual(rows[0], source.row(craft.item, craft.operand, craft.fixed, craft.MESSAGES))

    def test_schema_accepts_only_explicit_target_capture(self):
        schema = json.loads(Path(craft.__file__).with_name('schema.json').read_bytes())
        validator = jsonschema.Draft202012Validator(schema)
        row = source.row(craft.item, craft.operand, craft.fixed, craft.MESSAGES)
        validator.validate([row])
        for field, value in [('role', 'USED'), ('role', 'INGREDIENT'), ('count', 2),
                             ('effect', 'KEEP'), ('sub_type', 9), ('output_sub_type', 0),
                             ('capture_target_stack', 'true')]:
            with self.subTest(field=field, value=value):
                invalid = copy.deepcopy(row)
                invalid['recipe']['operands'][1][field] = value
                self.assertFalse(validator.is_valid([invalid]))

    def test_source_script_and_nonstackable_output_are_independently_qualified(self):
        packet = json.loads((source.ROOT / source.PACKET).read_bytes())
        script = source.ROOT / 'docs/reference/content-prep/qmech-m22/actions_star_herb.lua'
        self.assertEqual(hashlib.sha256(script.read_bytes()).hexdigest(), packet['source']['sha256'])
        self.assertEqual(packet['target_quantity'], 'CAPTURED_WHOLE_STACK')
        self.assertEqual(packet['output_quantity'], 1)
        items = {r['item']: r for r in packet['items']}
        self.assertTrue(items[3736]['stackable'])
        self.assertTrue(items[10297]['stackable'])
        self.assertFalse(items[22008]['stackable'])
        self.assertEqual(items[22008]['weight_centi_oz'], 240)

    def test_invalid_whole_stack_or_output_count_never_generates_a_recipe(self):
        original = json.loads((source.ROOT / source.PACKET).read_bytes())
        for field, value in [('target_quantity', 'ONE_UNIT'), ('output_quantity', 37), ('used_count', 37)]:
            with self.subTest(field=field), tempfile.TemporaryDirectory() as name:
                root = Path(name)
                path = root / source.PACKET
                path.parent.mkdir(parents=True)
                packet = dict(original, **{field: value})
                path.write_text(json.dumps(packet))
                with patch.object(source, 'ROOT', root), self.assertRaisesRegex(ValueError, 'QUANTITY_POLICY'):
                    source.row(craft.item, craft.operand, craft.fixed, craft.MESSAGES)

    def test_source_recipe_uses_qm22_capture_target_stack_policy(self):
        rows = [source.row(craft.item, craft.operand, craft.fixed, craft.MESSAGES)]
        self.assertEqual(len(rows), 1)
        operands = rows[0]['recipe']['operands']
        self.assertEqual([(r['role'], r['count'], r['effect']) for r in operands],
                         [('USED', 1, 'CONSUME'), ('TARGET', 1, 'TRANSFORM_TO')])
        self.assertEqual(operands[1]['output_item_id'], craft.item(22008))
        self.assertTrue(operands[1].get('capture_target_stack', False))


if __name__ == '__main__':
    unittest.main()
