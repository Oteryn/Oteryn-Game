"""A donor fluid argument must survive as one admitted vessel's initial state."""
import copy
import json
from pathlib import Path
import unittest

from native_container_fluids import project_fluid_child


class NativeContainerFluidTests(unittest.TestCase):
    def test_existing_native_flask_retains_source_subtype_without_a_stack(self):
        root = Path(__file__).resolve().parents[3]
        profiles = json.loads((root / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        record = next(row for row in profiles['records']
                      if row['production_definition']['production_key'] == 'oteryn:item.tibia.i2874')
        item = {'identity': record['production_binding']['target'],
                'materializable': record['admission']['materializable'],
                'stack_class': record['admission']['stack_class'], 'semantics': record['semantics']}
        quantity = {'item': item['identity'], 'count': 1}
        self.assertEqual(project_fluid_child(quantity, 1, item),
                         {**quantity, 'fluid_subtype': 1})
        for mutation in range(9):
            changed = copy.deepcopy(item)
            reward = copy.deepcopy(quantity)
            argument = 1
            if mutation == 0: changed['materializable'] = False
            elif mutation == 1: changed['stack_class'] = 'StackCapable'
            elif mutation == 2: changed['semantics']['fluid'] = {'state': 'UNKNOWN'}
            elif mutation == 3: changed['semantics']['fluid']['value']['fluid_type'] = {'state': 'UNKNOWN'}
            elif mutation == 4: argument = 2
            elif mutation == 5: argument = True
            elif mutation == 6: reward['count'] = True
            elif mutation == 7: reward['item']['key'] = 'oteryn:item.tibia.i3602'
            else: changed['semantics']['temporal'] = {'state': 'UNKNOWN'}
            with self.subTest(mutation=mutation):
                self.assertIsNone(project_fluid_child(reward, argument, changed))

    def test_source_projection_requires_prior_qualification_of_the_complete_claim(self):
        import container_native
        import reward_claim_authoring as plain
        items = plain.load_items()
        rows = [row['definition'] for path in (plain.ROOT / plain.CONTENT_DIR).glob('reward-claim-variants-*.json')
                for row in json.loads(path.read_bytes())['records']]
        wine = next(row for row in rows if row['identity']['key'] ==
                    'oteryn:reward-claim.quest.u7_9.dreamers_challenge.chests_wine')
        self.assertTrue(wine['data_holds'])
        self.assertIsNone(container_native.derive(wine, items, plain.stack_problem))
        profiles = json.loads((plain.ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        record = next(row for row in profiles['records']
                      if row['production_definition']['production_key'] == 'oteryn:item.tibia.i2874')
        # Explicit fixture precondition: an upstream owner has qualified every
        # hold and this exact existing native Item. Production holds stay intact.
        qualified = copy.deepcopy(wine)
        qualified['data_holds'] = []
        items['oteryn:item.tibia.i2874'] = {
            'identity': record['production_binding']['target'],
            'materializable': record['admission']['materializable'],
            'stack_class': record['admission']['stack_class'], 'semantics': record['semantics']}
        recipe = container_native.derive(qualified, items, plain.stack_problem)
        self.assertIsNotNone(recipe)
        self.assertEqual([row['unique_id'] for row in recipe['placements']], [6197, 6198, 6199, 6200])
        for placement in recipe['placements']:
            self.assertEqual(len(placement['contents']), 3)
            self.assertEqual(placement['contents'][1]['count'], 1)
            self.assertEqual(placement['contents'][1]['fluid_subtype'], 1)
        qualified['provenance']['pilot_key'] = 'canary:reward-claim/other-lane-owned'
        self.assertIsNone(container_native.derive(qualified, items, plain.stack_problem))
