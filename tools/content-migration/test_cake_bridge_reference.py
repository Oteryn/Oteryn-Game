"""The approved Cake map identity must never acquire inventory facts."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
import cake_bridge_reference as reference
from boss_conditional_item_reference import definition

ROOT = Path(__file__).resolve().parents[2]


class CakeBridgeReferenceTests(unittest.TestCase):
    def test_exact_map_reference_is_idempotent_and_not_materializable(self):
        records = []
        reference.extend_records(records, ROOT)
        reference.extend_records(records, ROOT)
        # Crystal 201189f8 data/items/items.xml:81432-81441 skips 48227.
        # The appearance flags use src/items/items.cpp:171-181,219-251.
        # src/items/items.hpp:276-300 sets the remaining ItemType defaults.
        expected = definition(reference.KEY)
        expected['semantics'].update({
            'charges': {'state': 'NOT_APPLICABLE'},
            'classification': {'state': 'KNOWN', 'value': {
                'item_type': {'state': 'NOT_APPLICABLE'},
                'capabilities': {'state': 'UNKNOWN'},
            }},
            'container': {'state': 'NOT_APPLICABLE'},
            'physical': {'state': 'KNOWN', 'value': {
                'movable': {'state': 'KNOWN', 'value': False},
                'pickupable': {'state': 'KNOWN', 'value': False},
                'weight': {'state': 'KNOWN', 'value': 0},
            }},
            'stack': {'state': 'KNOWN', 'value': {
                'stackable': {'state': 'KNOWN', 'value': False},
                'stack_max': {'state': 'NOT_APPLICABLE'},
            }},
            'temporal': {'state': 'NOT_APPLICABLE'},
        })
        self.assertEqual(records, [expected])
        self.assertFalse(records[0]['materializable'])
        self.assertEqual(records[0]['stack_class'], 'Unknown')

    def test_inventory_admission_or_source_substitution_is_rejected(self):
        original = json.loads((ROOT / reference.PACKET).read_bytes())
        for changed in ('materializable', 'sources'):
            packet = copy.deepcopy(original)
            if changed == 'materializable':
                packet['definition']['materializable'] = True
            else:
                packet['sources'][0]['sha256'] = '0' * 64
            with self.subTest(changed=changed), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                target = root / reference.PACKET
                target.parent.mkdir(parents=True)
                target.write_text(json.dumps(packet))
                with self.assertRaisesRegex(ValueError, 'APPROVED_REFERENCE_CHANGED'):
                    reference.extend_records([], root)

    def test_existing_semantic_facts_cannot_be_overwritten(self):
        records = []
        reference.extend_records(records, ROOT)
        for facet in ('materializable', 'physical', 'presentation'):
            row = copy.deepcopy(records[0])
            if facet == 'materializable':
                row[facet] = True
            elif facet == 'physical':
                row['semantics'][facet] = {'state': 'UNKNOWN'}
            else:
                row['semantics'][facet] = {'state': 'KNOWN', 'value': {'name': 'cake tile'}}
            with self.subTest(facet=facet):
                with self.assertRaisesRegex(ValueError, 'CAKE_BRIDGE_REFERENCE_CHANGED'):
                    reference.extend_records([row], ROOT)
