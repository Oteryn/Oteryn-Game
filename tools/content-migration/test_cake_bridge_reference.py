"""The approved Cake map identity must never acquire inventory facts."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
import cake_bridge_reference as reference

ROOT = Path(__file__).resolve().parents[2]


class CakeBridgeReferenceTests(unittest.TestCase):
    def test_exact_map_reference_is_idempotent_and_not_materializable(self):
        records = []
        reference.extend_records(records, ROOT)
        reference.extend_records(records, ROOT)
        self.assertEqual(records, [reference.definition(reference.KEY)])
        self.assertFalse(records[0]['materializable'])
        self.assertEqual(records[0]['stack_class'], 'Unknown')
        self.assertTrue(all(value == {'state': 'UNKNOWN'} for value in records[0]['semantics'].values()))

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
        row = reference.definition(reference.KEY)
        row['materializable'] = True
        with self.assertRaisesRegex(ValueError, 'REFERENCE_CHANGED'):
            reference.extend_records([row], ROOT)
