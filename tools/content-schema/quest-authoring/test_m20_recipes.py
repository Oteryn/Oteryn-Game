"""M20 source recipe terms share the production quest authoring validator."""
import copy
import json
from pathlib import Path
import unittest
from jsonschema import ValidationError
import qengine_rewards


class CarriedRecipes(unittest.TestCase):
    def document(self):
        root = Path(__file__).resolve().parents[3]
        return json.loads((root / 'content/quests/npc-callbacks/m20-cursed-crystal-recipes.json').read_text())

    def test_both_source_operand_orders_are_valid(self):
        document = self.document()
        qengine_rewards.validate_callbacks(document)
        self.assertEqual({(r['carried_item'], r['carried_target']) for r in document['callbacks']}, {
            ('oteryn:item.tibia.i21505', 'oteryn:item.tibia.i21507'),
            ('oteryn:item.tibia.i21507', 'oteryn:item.tibia.i21505'),
        })

    def test_missing_or_ambiguous_selected_operands_fail_closed(self):
        for mutation in ('missing_target', 'same_operand', 'peer'):
            document = copy.deepcopy(self.document())
            row = document['callbacks'][0]
            if mutation == 'missing_target':
                row['take'].pop()
            elif mutation == 'same_operand':
                row['carried_target'] = row['carried_item']
            else:
                row['peer_requires'] = {'track': row['storage'][0]['track'], 'minimum': 0, 'maximum': 1}
            with self.assertRaises((ValueError, ValidationError)):
                qengine_rewards.validate_callbacks(document)
