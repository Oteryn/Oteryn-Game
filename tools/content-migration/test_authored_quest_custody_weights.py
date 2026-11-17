"""Source conflicts retain only an exact, unchanged accepted physical fact."""
import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET

import authored_quest_item_custody as custody

ROOT = Path(__file__).resolve().parents[2]


class CustodyWeights(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw = (ROOT / 'imports/canary/items-xml/items.xml').read_bytes()
        cls.node = next(n for n in ET.fromstring(cls.raw) if n.get('id') == '30226')
        for path in (ROOT / 'content/items/definitions').glob('items-*.json'):
            for row in json.loads(path.read_bytes())['records']:
                if row['definition']['identity']['key'] == 'oteryn:item.tibia.i30226':
                    cls.definition = row['definition']
                if row['definition']['identity']['key'] == 'oteryn:item.tibia.i30277':
                    cls.icicle = row['definition']

    def test_real_branch_custody_preserves_every_accepted_fact_and_four_ounce_weight(self):
        before = copy.deepcopy(self.definition)
        row = custody.qualify(ROOT, self.definition, self.raw)
        self.assertEqual(self.definition, before)
        self.assertEqual(row['semantics']['physical']['value']['weight'], custody.grizzly.source.K(400))
        self.assertEqual(row['admission']['legal_destinations'], ['CharacterInventory', 'Ground'])

    def test_batch_reuses_proofs_and_still_refuses_a_changed_member(self):
        expected = custody.qualify(ROOT, self.definition, self.raw)
        definitions = [self.definition, self.icicle]
        before = copy.deepcopy(definitions)
        with patch.object(custody.reward, 'load_admissions', wraps=custody.reward.load_admissions) as admissions:
            actual = custody.qualify_many(ROOT, definitions, self.raw, terminal_expiry=True)
        self.assertEqual(admissions.call_count, 1)
        self.assertEqual(actual[0], expected)
        self.assertEqual(definitions, before)
        changed = copy.deepcopy(self.definition)
        changed['semantics']['physical']['value']['weight']['value'] = 401
        with self.assertRaisesRegex(ValueError, 'WEIGHT_CORRECTION_BINDING'):
            custody.qualify_many(ROOT, [self.icicle, changed], self.raw, terminal_expiry=True)
        with self.assertRaisesRegex(ValueError, 'INSTANCE_OWNER_REQUIRED'):
            custody.qualify_many(ROOT, definitions, self.raw)

    def test_changed_definition_or_xml_refuses_before_publishing_a_correction(self):
        for changed in ('definition', 'xml'):
            definition, node = copy.deepcopy(self.definition), copy.deepcopy(self.node)
            if changed == 'definition':
                definition['semantics']['physical']['value']['weight']['value'] = 401
            else:
                next(a for a in node if a.get('key') == 'weight').set('value', '99999')
            row = {'semantics': {'physical': {'value': {'weight': custody.grizzly.source.K(100000)}}}}
            before = copy.deepcopy(row)
            with self.assertRaisesRegex(ValueError, 'WEIGHT_CORRECTION_BINDING'):
                custody.correct_weight(ROOT, definition, node, row)
            self.assertEqual(row, before)

    def test_terminal_loot_expiry_requires_the_existing_explicit_timed_owner(self):
        with self.assertRaisesRegex(ValueError, 'INSTANCE_OWNER_REQUIRED'):
            custody.qualify(ROOT, self.icicle, self.raw)
        before = copy.deepcopy(self.icicle)
        row = custody.qualify(ROOT, self.icicle, self.raw, terminal_expiry=True)
        self.assertEqual(self.icicle, before)
        self.assertEqual(row['semantics']['charges'], custody.grizzly.source.NA)
        temporal = row['semantics']['temporal']['value']
        self.assertEqual(temporal['duration'], custody.grizzly.source.K(600000))
        self.assertEqual(temporal['decay_target'], custody.grizzly.source.NA)
        self.assertEqual(temporal['consumption_mode'], custody.grizzly.source.K('DURABLE_ABSOLUTE_DEADLINE'))


if __name__ == '__main__':
    unittest.main()
