import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import native_goal_loot_items as native


def definitions():
    keys = {f'oteryn:item.tibia.i{i}' for i in native.selected()}
    return {r['definition']['identity']['key']: r['definition']
            for p in (native.ROOT / 'content/items/definitions').glob('items-*.json')
            for r in json.loads(p.read_bytes())['records'] if r['definition']['identity']['key'] in keys}


class NativeGoalLootProof(unittest.TestCase):
    def test_selected_source_tables_have_complete_finite_custody(self):
        ids = native.selected()
        self.assertEqual(len(ids), 71)
        self.assertTrue({44158, 3082, 2824, 3031} <= ids)
        items = definitions()
        self.assertEqual(len(items), 71)
        self.assertTrue(all(d['materializable'] for d in items.values()))
        amulet = items['oteryn:item.tibia.i3082']
        self.assertEqual(amulet['semantics']['charges']['value']['count'], {'state': 'KNOWN', 'value': 50})
        self.assertEqual(amulet['stack_class'], 'NonStackable')

    def test_existing_admitted_native_physical_facts_are_preserved(self):
        key = 'oteryn:item.tibia.i3031'
        profile = native.profiles(native.ROOT, {3031})[key]
        profile['admission']['materializable'] = True
        profile['semantics']['physical']['value']['weight']['value'] = 10
        before = copy.deepcopy(profile['semantics']['physical'])
        canonical = lambda value: json.dumps(value, sort_keys=True).encode()
        generated = {'profiles': canonical({'records': [profile]}),
                     'manifest': canonical({'item_profiles': {'sha256': '0' * 64}})}
        with patch.object(native, 'selected', return_value={3031}):
            native.admit_native(native.ROOT, generated, 'profiles', 'manifest',
                                lambda row: row['production_definition']['production_key'], canonical)
        actual = json.loads(generated['profiles'])['records'][0]['semantics']['physical']
        self.assertEqual(actual, before)

    def test_trophy_alternate_source_is_exact_and_not_a_general_pin_bypass(self):
        source = native.importer(native.ROOT)
        raw = native.alternate_xml(native.ROOT)
        with self.assertRaisesRegex(ValueError, 'ALTERNATE_XML_SCOPE_OR_PIN'):
            source.facts(native.ROOT, {3082}, xml_override=raw)
        with self.assertRaisesRegex(ValueError, 'ALTERNATE_XML_SCOPE_OR_PIN'):
            source.facts(native.ROOT, {44158}, xml_override=raw + b' ')
        result = native.profiles(native.ROOT, {44158, 3082})
        self.assertEqual(set(result), {'oteryn:item.tibia.i44158', 'oteryn:item.tibia.i3082'})
        self.assertEqual(result['oteryn:item.tibia.i3082']['semantics']['charges']['value']['count'],
                         {'state': 'KNOWN', 'value': 50})

    def test_wrong_charge_fact_refuses_all_pending_admissions_atomically(self):
        source = definitions()
        items = [copy.deepcopy(source[f'oteryn:item.tibia.i{i}']) for i in (2824, 3082, 44158)]
        for item in items:
            item['materializable'] = False
        items[1]['semantics']['charges']['value']['count']['value'] = 49
        before = copy.deepcopy(items)
        with self.assertRaisesRegex(ValueError, 'CORE_CONFLICT'):
            native.apply_admissions(items)
        self.assertEqual(items, before)

    def test_exact_full_charge_admission_preserves_source_definition_and_unowned_data(self):
        item = copy.deepcopy(definitions()['oteryn:item.tibia.i3082'])
        item['materializable'] = False
        before = copy.deepcopy(item)
        native.apply_admissions([item])
        self.assertTrue(item['materializable'])
        for group in ('physical', 'weapon', 'charges', 'use_transform', 'temporal'):
            self.assertEqual(item['semantics'][group], before['semantics'][group])
        self.assertEqual(item['semantics']['charges']['value']['count']['value'], 50)


if __name__ == '__main__':
    unittest.main()
