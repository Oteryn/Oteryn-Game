"""Explicit Oteryn constructors cannot impersonate donor identities or mutate known facts."""
import copy
import json
from pathlib import Path
import unittest
import authored_static_item_constructors as producer

ROOT = Path(__file__).resolve().parents[2]

class StaticConstructors(unittest.TestCase):
    def test_house_defaults_preserve_exact_constructor_admissions(self):
        import house_decoration_actions as house
        profiles, admissions = producer.prepare(
            ROOT, json.loads((ROOT / producer.INTENT).read_bytes()))
        before = copy.deepcopy(profiles)
        overlay = json.loads((ROOT / house.PROFILES).read_bytes())
        house.apply_top_order(profiles, overlay)
        self.assertEqual(profiles, before)
        for profile in profiles:
            self.assertTrue(producer.matches_admission(profile, admissions))
        identifier = profiles[0]['production_binding']['external_id']
        self.assertEqual(overlay['always_on_top'][identifier], 0)
        overlay['always_on_top'][identifier] = 1
        with self.assertRaisesRegex(ValueError, 'HOUSE_AUTHORED_TOP_ORDER_CONFLICT'):
            house.apply_top_order(profiles, overlay)
        self.assertEqual(profiles, before)

    def test_declared_four_plain_custody_records_keep_nonphysical_semantics(self):
        intent = json.loads((ROOT / producer.INTENT).read_bytes())
        profiles, admissions = producer.prepare(ROOT, intent)
        self.assertEqual(len(profiles), 4)
        self.assertEqual(len(admissions['records']), 4)
        for row in profiles:
            key = row['production_definition']['production_key']
            canonical = producer.definitions(ROOT)[key]
            self.assertEqual(row['admission'], {'materializable': True, 'stack_class': 'NonStackable', 'legal_destinations': ['CharacterInventory', 'Ground']})
            self.assertEqual(row['production_binding']['source_key'], 'oteryn:source.oteryn_authored')
            for group, value in canonical['semantics'].items():
                if group not in ('physical', 'stack', 'container'):
                    self.assertEqual(row['semantics'][group], value)
            self.assertEqual(row['semantics']['physical']['value']['weight']['value'], 1000)
            self.assertTrue(producer.matches_admission(row, admissions))
            changed = copy.deepcopy(row)
            changed['admission']['stack_class'] = 'StackCapable'
            self.assertFalse(producer.matches_admission(changed, admissions))
            changed = copy.deepcopy(row)
            changed['production_binding']['source_key'] = 'oteryn:source.crystalserver'
            self.assertFalse(producer.matches_admission(changed, admissions))

    def test_callback_reuse_keeps_exact_first_party_constructor(self):
        import authored_quest_item_custody
        intent = json.loads((ROOT / producer.INTENT).read_bytes())
        profiles, admissions = producer.prepare(ROOT, intent)
        bindings = [row['binding'] for row in admissions['records']]
        for profile in profiles:
            definition = producer.definitions(ROOT)[profile['production_definition']['production_key']]
            before = copy.deepcopy(profile)
            authored_quest_item_custody.reuse_existing(definition, profile, bindings)
            self.assertEqual(profile, before)
            changed = copy.deepcopy(profile)
            changed['production_binding']['source_revision'] = '0' * 64
            with self.assertRaises(ValueError):
                authored_quest_item_custody.reuse_existing(definition, changed, bindings)

    def test_rejects_unpinned_unrelated_duplicate_and_invalid_policies(self):
        original = json.loads((ROOT / producer.INTENT).read_bytes())
        for case in range(8):
            intent = copy.deepcopy(original)
            if case == 0: intent['recipes']['sha256'] = '0' * 64
            elif case == 1: intent['constructors'][0]['role'] = 'Unrelated reward'
            elif case == 2: intent['constructors'].append(copy.deepcopy(intent['constructors'][0]))
            elif case == 3: intent['constructors'][0]['weight'] = -1
            elif case == 4: intent['constructors'][0]['item'] = 'oteryn:item.tibia.i3031'
            elif case == 5: intent['constructors'][0]['recipe_title'] = 'Unknown quest'
            elif case == 6: intent['basis'] = 'SOURCE_EQUIVALENT'
            else: intent['constructors'][0]['quest'] = 'oteryn:quest.authored.unrelated'
            with self.subTest(case=case), self.assertRaises(ValueError):
                producer.prepare(ROOT, intent)

if __name__ == '__main__':
    unittest.main()
