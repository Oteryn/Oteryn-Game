import copy
import json
import unittest
from pathlib import Path
import npc_shopping_bag as subject


class ShoppingBagAdmission(unittest.TestCase):
    def source(self):
        path = Path(__file__).resolve().parents[2] / 'content/items/definitions/items-10500-10999.json'
        return next(copy.deepcopy(row['definition']) for row in json.loads(path.read_bytes())['records']
                    if row['definition']['identity']['key'] == subject.KEY)

    def test_only_the_selected_bag_is_admitted(self):
        bag = self.source()
        bag["materializable"] = False
        bag["stack_class"] = "Unknown"
        other = copy.deepcopy(bag)
        other['identity']['key'] = 'oteryn:item.tibia.i21412'
        other['materializable'] = False
        before = copy.deepcopy(other)
        subject.apply_admissions([other, bag])
        self.assertEqual(other, before)
        self.assertTrue(bag['materializable'])
        self.assertEqual(bag['stack_class'], 'NonStackable')

    def test_conflicting_capacity_or_stack_refuses(self):
        for field in ('capacity', 'stack'):
            bag = self.source()
            if field == 'capacity':
                bag['semantics']['container']['value']['capacity']['value'] = 21
            else:
                bag['semantics']['stack']['value']['stackable']['value'] = True
            with self.assertRaisesRegex(ValueError, 'NPC_SHOPPING_BAG_FACT_CONFLICT'):
                subject.apply_admissions([bag])


if __name__ == '__main__':
    unittest.main()
