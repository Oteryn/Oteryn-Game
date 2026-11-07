"""Pin substitution and native door admission checks for the source-key cohort."""
import json
from pathlib import Path
import tempfile
import unittest
import hidden_threats_world as source

class SourceDoors(unittest.TestCase):
    def test_exact_native_storage_and_position_cohort(self):
        data = source.facts()
        self.assertEqual([(d['position'], d['action'], d['item']) for d in data['doors']],
                         [([33025,32008,12],45974,1642),([33045,32007,12],45975,1642),([33001,32047,12],45976,1644)])
        self.assertEqual(data['source']['sha256'], '263b50458d596e4900dfa491997f184ef6c88879034a61c52aae42c983f42184')

    def test_canonical_tool_is_retained_and_repeats_are_conditional(self):
        for row in source.document()['callbacks']:
            self.assertEqual(row['item_on_target']['item'], 'oteryn:item.tibia.i27269')
            self.assertTrue(row['item_on_target']['retain'])
            self.assertIsNone(row['item_on_target']['replacement'])
            self.assertEqual(row['storage'][0]['set_if'], -1)
            self.assertEqual(row['storage'][0]['set'], 1)
            self.assertIsNone(row['selector']['appearance'])

    def test_substituted_source_cannot_emit_a_callback(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);path=root/source.CAPSULE;path.parent.mkdir(parents=True)
            data=source.facts();data['key']=3031;path.write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError,'HIDDEN_THREATS_DOORS_SOURCE_PIN'):
                source.document(root)

if __name__ == '__main__':
    unittest.main()
