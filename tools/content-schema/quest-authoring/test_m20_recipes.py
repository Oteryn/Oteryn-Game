"""M20 source recipe terms share the production quest authoring validator."""
import copy
import json
from pathlib import Path
import unittest
from jsonschema import ValidationError
import qengine_rewards


class CarriedRecipes(unittest.TestCase):
    def test_molten_wax_is_exact_self_consumption(self):
        root = Path(__file__).resolve().parents[3]
        document = json.loads((root / 'content/quests/npc-callbacks/m20-cursed-crystal-wax.json').read_text())
        qengine_rewards.validate_callbacks(document)
        row, = document['callbacks']
        self.assertEqual(row['take'], [{'item': 'oteryn:item.tibia.i9106', 'count': 1}])
        self.assertIsNone(row['give'])
        self.assertEqual(row['self_target'], {
            'minimum': {'x': 31938, 'y': 32932, 'floor': -10},
            'maximum': {'x': 31946, 'y': 32940, 'floor': -10},
        })
        self.assertEqual((row['storage'][0]['minimum'], row['storage'][0]['maximum'], row['storage'][0]['set']), (-1, 1, 2))
        for mutation in ('inverted', 'different_floor', 'too_wide', 'no_source', 'recipe'):
            invalid = copy.deepcopy(document)
            entry = invalid['callbacks'][0]
            if mutation == 'inverted':
                entry['self_target']['maximum']['x'] = 31937
            elif mutation == 'different_floor':
                entry['self_target']['maximum']['floor'] = -9
            elif mutation == 'too_wide':
                entry['self_target']['maximum']['x'] = 31955
            elif mutation == 'no_source':
                entry.pop('carried_item')
            else:
                entry['carried_target'] = 'oteryn:item.tibia.i21504'
            with self.assertRaises((ValueError, ValidationError)):
                qengine_rewards.validate_callbacks(invalid)

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
