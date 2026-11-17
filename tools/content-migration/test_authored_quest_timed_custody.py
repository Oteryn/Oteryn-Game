"""Finite quest custody must retain exact source timing and fail closed on gaps."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import authored_quest_timed_custody as timed

ROOT = Path(__file__).resolve().parents[2]
KEY = 'oteryn:item.tibia.i169'
CLOCK = 'oteryn:quest-progress/chosen/travelling-trader/cheese-purchased-at'
PHASE = 'oteryn:quest-progress/chosen/travelling-trader/stage'


def callbacks():
    return [
        {'key': 'oteryn:quest-reward/test/buy', 'quest': 'oteryn:quest.test',
         'give': {'item': KEY, 'count': 1, 'contents': []},
         'storage': [{'track': PHASE, 'minimum': 5, 'maximum': 5, 'set': 6}],
         'state_ops': [{'track': CLOCK, 'from': {'kind': 'any'}, 'effect': {'kind': 'set_now'}}]},
        {'key': 'oteryn:quest-reward/test/handin', 'quest': 'oteryn:quest.test',
         'take': [{'item': KEY, 'count': 1}],
         'storage': [{'track': PHASE, 'minimum': 6, 'maximum': 6, 'set': 7}],
         'state_ops': [{'track': CLOCK,
                        'from': {'kind': 'elapsed_less_than', 'seconds': 1800},
                        'effect': {'kind': 'add', 'value': 0}}]},
    ]


def omitted_callbacks():
    document = callbacks()
    for callback, npc in zip(document, ['miraia', 'rashid']):
        callback['quest'] = timed.OMISSION_QUEST
        callback['npc'] = 'oteryn:npc.' + npc
        callback.pop('state_ops')
    return document


class AuthoredQuestTimedCustody(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.xml = (ROOT / 'imports/canary/items-xml/items.xml').read_bytes()
        cls.definition = next(row['definition']
                              for path in (ROOT / 'content/items/definitions').glob('items-*.json')
                              for row in json.loads(path.read_bytes())['records']
                              if row['definition']['identity']['key'] == KEY)

    def test_exact_selected_recipe_omits_decay_without_admitting_native_food(self):
        before = copy.deepcopy(self.definition)
        row, contract = timed.qualify(ROOT, self.definition, self.xml, omitted_callbacks())
        self.assertEqual(contract.mode, 'EXPLICIT_RECIPE_OMISSION')
        self.assertEqual(contract.adaptation, timed.OMISSION_ADAPTATION)
        self.assertEqual((contract.duration_seconds, contract.decay_to), (1800, 3120))
        self.assertTrue(contract.recipe_pointer.endswith('/adaptations/0'))
        for group in ('consumption', 'temporal', 'use_transform'):
            self.assertEqual(row['semantics'][group], {'state': 'UNKNOWN'})
        self.assertEqual(self.definition, before)

    def test_recipe_omission_is_not_a_callback_flag_or_unbounded_item_permission(self):
        for mutation in range(8):
            document = omitted_callbacks()
            if mutation == 0:
                document.append(copy.deepcopy(document[0]))
            elif mutation == 1:
                document[1]['quest'] = 'oteryn:foreign'
            elif mutation == 2:
                document[1]['state_ops'] = callbacks()[1]['state_ops']
            elif mutation == 3:
                document[1]['npc'] = 'oteryn:npc.miraia'
            elif mutation == 4:
                document[0]['give']['count'] = 2
            elif mutation == 5:
                document[1]['present'] = document[1].pop('take')
            elif mutation == 6:
                document[1]['storage'][0]['minimum'] = 5
            else:
                document.append({'quest': timed.OMISSION_QUEST,
                                 'take': [{'item': KEY, 'count': 1}]})
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                timed.omitted_expiry_contract(ROOT, document, KEY, (1800, 3120))
        original = json.loads((ROOT / timed.OMISSION_PATH).read_bytes())
        with tempfile.TemporaryDirectory(prefix='quest-recipe-omission-') as temporary:
            root = Path(temporary)
            path = root / timed.OMISSION_PATH
            path.parent.mkdir(parents=True)
            for mutation in range(4):
                document = copy.deepcopy(original)
                quest = document['records'][45]['definition']
                recipe = quest['oteryn_recipe']['payload']['recipe']
                if mutation == 0:
                    recipe['adaptations'] = []
                elif mutation == 1:
                    recipe['adaptations'][0] = 'Ignore source timing.'
                elif mutation == 2:
                    quest['identity']['key'] = 'oteryn:quest.foreign'
                else:
                    quest['oteryn_recipe']['profile'] = 'unreviewed'
                path.write_text(json.dumps(document))
                with self.subTest(canonical_mutation=mutation), self.assertRaisesRegex(ValueError, 'CANONICAL_OMISSION'):
                    timed.omitted_expiry_contract(root, omitted_callbacks(), KEY, (1800, 3120))

    def test_actual_source_and_custody_leave_food_and_inventory_decay_unqualified(self):
        before = copy.deepcopy(self.definition)
        self.assertEqual(timed.source_expiry(self.xml, 169), (1800, 3120))
        row, contract = timed.qualify(ROOT, self.definition, self.xml, callbacks())
        self.assertEqual(contract.duration_seconds, 1800)
        self.assertEqual(contract.decay_to, 3120)
        self.assertEqual(contract.clock_track, CLOCK)
        self.assertEqual(row['production_definition']['production_key'], KEY)
        self.assertEqual(row['admission'], {'materializable': True,
                         'stack_class': 'NonStackable',
                         'legal_destinations': ['CharacterInventory', 'Ground']})
        self.assertEqual(row['semantics']['physical']['value']['weight'],
                         {'state': 'KNOWN', 'value': 400})
        for group in ('consumption', 'temporal', 'use_transform'):
            self.assertEqual(row['semantics'][group], {'state': 'UNKNOWN'})
        self.assertEqual(self.definition, before)
        self.assertEqual(self.definition['semantics']['consumption']['value']['kind'], 'FOOD')
        self.assertEqual((ROOT / 'imports/canary/items-xml/items.xml').read_bytes(), self.xml)

    def test_every_consumer_needs_exact_source_deadline_and_original_phase(self):
        for mutation in range(9):
            document = callbacks()
            if mutation == 0:
                document[0]['state_ops'] = []
            elif mutation == 1:
                document[1]['state_ops'] = []
            elif mutation == 2:
                document[1]['state_ops'][0]['from']['seconds'] = 1801
            elif mutation == 3:
                document[1]['state_ops'][0]['from']['kind'] = 'elapsed_at_least'
            elif mutation == 4:
                document[1]['state_ops'][0]['effect']['value'] = 1
            elif mutation == 5:
                document.append({'key': 'oteryn:unguarded', 'present': [{'item': KEY, 'count': 1}]})
            elif mutation == 6:
                document[1]['quest'] = 'oteryn:foreign'
            elif mutation == 7:
                document[0]['storage'][0]['maximum'] = 6
            else:
                document[1]['storage'][0]['minimum'] = 5
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                timed.expiry_contract(document, KEY, (1800, 3120))

    def test_paid_refresh_requires_expired_same_phase_and_atomic_old_item_exchange(self):
        refresh = {'key': 'oteryn:quest-reward/test/refresh', 'quest': 'oteryn:quest.test',
                   'give': {'item': KEY, 'count': 1, 'contents': []},
                   'take': [{'item': KEY, 'count': 1}],
                   'fee': {'gold': 100, 'include_bank': True},
                   'storage': [{'track': PHASE, 'minimum': 6, 'maximum': 6,
                                'set': 6, 'preserve': True}],
                   'state_ops': [{'track': CLOCK,
                                  'from': {'kind': 'elapsed_at_least', 'seconds': 1800},
                                  'effect': {'kind': 'set_now'}}]}
        contract = timed.expiry_contract(callbacks() + [refresh], KEY, (1800, 3120))
        self.assertEqual(contract.refresh_producers, (refresh['key'],))
        for mutation in range(6):
            invalid = copy.deepcopy(refresh)
            if mutation == 0:
                invalid['state_ops'][0]['from'] = {'kind': 'any'}
            elif mutation == 1:
                invalid['state_ops'][0]['from']['seconds'] = 1799
            elif mutation == 2:
                invalid['fee']['gold'] = 0
            elif mutation == 3:
                invalid['take'] = []
            elif mutation == 4:
                invalid['storage'][0]['set'] = 5
            else:
                invalid['state_ops'][0]['effect'] = {'kind': 'add', 'value': 1800}
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                timed.expiry_contract(callbacks() + [invalid], KEY, (1800, 3120))

    def test_unrelated_callbacks_cannot_reset_or_extend_the_source_deadline(self):
        for extra in [
            {'key': 'oteryn:reset', 'state_ops': [{'track': CLOCK, 'from': {'kind': 'any'},
                                                'effect': {'kind': 'set_now'}}]},
            {'key': 'oteryn:reset', 'storage': [{'track': CLOCK, 'minimum': 0,
                                              'maximum': 10, 'set': 11}]},
        ]:
            with self.subTest(extra=extra), self.assertRaisesRegex(ValueError, 'FOREIGN_CLOCK_WRITER'):
                timed.expiry_contract(callbacks() + [extra], KEY, (1800, 3120))

    def test_cannot_mint_unbound_nested_or_multiple_duration_sources(self):
        for mutation in range(4):
            document = callbacks()
            if mutation == 0:
                document[0]['give']['count'] = 2
            elif mutation == 1:
                document.append(copy.deepcopy(document[0]))
            elif mutation == 2:
                document[0]['give'] = {'item': 'oteryn:item.tibia.i2854', 'count': 1,
                                       'contents': [{'item': KEY, 'count': 1}]}
            else:
                document[1]['take'][0]['count'] = 2
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                timed.expiry_contract(document, KEY, (1800, 3120))

    def test_pins_accepted_holds_and_equipment_owners_cannot_be_bypassed(self):
        with self.assertRaisesRegex(ValueError, 'XML_PIN'):
            timed.source_expiry(self.xml + b' ', 169)
        self.assertIsNone(timed.source_expiry(self.xml, 7397))
        with self.assertRaisesRegex(ValueError, 'UNSUPPORTED_OWNER'):
            timed.source_expiry(self.xml, 3088)
        with patch.object(timed.reward, 'load_admissions', return_value={KEY: ({'holds': ['held']},)}):
            with self.assertRaisesRegex(ValueError, 'ACCEPTED_HOLD'):
                timed.qualify(ROOT, self.definition, self.xml, callbacks())
        changed = copy.deepcopy(self.definition)
        changed['semantics']['physical']['value']['weight']['value'] = 401
        with self.assertRaisesRegex(ValueError, 'CANONICAL_PHYSICAL'):
            timed.qualify(ROOT, changed, self.xml, callbacks())


if __name__ == '__main__':
    unittest.main()
