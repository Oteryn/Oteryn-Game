import copy
import json
import unittest
from unittest.mock import patch

import encounter_loot_item_admissions as loot


class EncounterLootAdmissionTests(unittest.TestCase):
    def test_client_only_objects_cannot_invent_stack_or_container_limits(self):
        self.assertTrue(loot.client_only_custody_supported({'flags': {'flags.take': True}}))
        for flags in ({'flags.take': True, 'flags.container': True},
                      {'flags.take': True, 'flags.cumulative': True},
                      {'flags.take': False}):
            self.assertFalse(loot.client_only_custody_supported({'flags': flags}))

    def test_source_closure_preserves_all_non_custody_fields_and_is_idempotent(self):
        owned = {f'oteryn:item.tibia.i{i}' for i in loot.IDS}
        definitions = [r['definition']
                       for p in (loot.ROOT / 'content/items/definitions').glob('items-*.json')
                       for r in json.loads(p.read_bytes())['records']
                       if r['definition']['identity']['key'] in owned]
        before = copy.deepcopy(definitions)
        loot.apply_admissions(definitions)
        for old, new in zip(before, definitions):
            self.assertTrue(new['materializable'])
            for field in ('identity', 'client_projection', 'kind'):
                self.assertEqual(old[field], new[field])
            for group in old['semantics']:
                if group not in ('stack', 'container'):
                    self.assertEqual(old['semantics'][group], new['semantics'][group])
        once = copy.deepcopy(definitions)
        loot.apply_admissions(definitions)
        self.assertEqual(once, definitions)

    def test_conflict_refuses_whole_batch_before_mutation(self):
        facts = loot.facts()
        definitions = []
        for key, profile in facts.items():
            definitions.append({'identity': {'key': key}, 'stack_class': 'Unknown',
                                'materializable': False,
                                'semantics': copy.deepcopy(profile['semantics'])})
        definitions[-1]['stack_class'] = 'Invalid'
        before = copy.deepcopy(definitions)
        with patch.object(loot, 'facts', return_value=facts):
            with self.assertRaisesRegex(ValueError, 'STACK_CONFLICT'):
                loot.apply_admissions(definitions)
        self.assertEqual(before, definitions)
        definitions.append(copy.deepcopy(definitions[0]))
        with self.assertRaisesRegex(ValueError, 'DUPLICATE_IDENTITY'):
            loot.apply_admissions(definitions)


if __name__ == '__main__':
    unittest.main()
