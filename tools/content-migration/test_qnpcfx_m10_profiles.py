import copy
import json
import unittest

import qnpcfx_item_profiles as npc


class M10Profiles(unittest.TestCase):
    def packet(self):
        return json.loads((npc.batch.ROOT / npc.PATH).read_bytes())

    def test_source_cohort_is_explicit_and_all_required_physical_items_are_qualified(self):
        expected = {f'oteryn:item.tibia.i{i}' for i in (919, 7528, 7529, 7530, 7531, 7532, 52636)}
        self.assertEqual(npc.KEYS, expected)
        packet = self.packet()
        self.assertEqual({r['production_definition']['production_key'] for r in packet['records']}, expected)
        self.assertFalse(any(r['classification'] == 'HELD' for r in packet['classification']))
        for row in packet['records']:
            self.assertEqual(row['admission']['legal_destinations'], ['CharacterInventory', 'Ground'])

    def test_generated_cohort_receipt_keeps_other_producers_and_raw_assertions(self):
        source = '''    // QNPCFX: old source proof.
    ("QNPCFX", 1),
    ("ANOTHER-OWNER", 42),
    assert_eq!(item.attributes.speed_bonus, Some(0));
'''
        result = npc.cohort_receipt(source, self.packet())
        self.assertIn('(\"QNPCFX\", 7)', result)
        self.assertIn('(\"ANOTHER-OWNER\", 42)', result)
        self.assertIn('assert_eq!(item.attributes.speed_bonus, Some(0));', result)
        self.assertEqual(npc.cohort_receipt(result, self.packet()), result)

    def test_generated_identity_witness_covers_every_item_and_preserves_venison_checks(self):
        source = '''    assert_eq!(npc_additions.len(), 1);
    assert_eq!(
        npc_additions[0]["production_definition"]["production_key"],
        "oteryn:item.tibia.i52636"
    );
    assert_eq!(venison.attributes.speed_bonus, Some(0));
'''
        result = npc.profile_identity_receipt(source, self.packet())
        self.assertIn('assert_eq!(npc_additions.len(), 7);', result)
        for key in npc.KEYS:
            self.assertIn(f'"{key}"', result)
        self.assertIn('assert_eq!(venison.attributes.speed_bonus, Some(0));', result)
        self.assertEqual(npc.profile_identity_receipt(result, self.packet()), result)

    def test_cohort_receipt_rejects_missing_foreign_or_duplicate_source_records(self):
        original = self.packet()
        source = '    // QNPCFX: source.\n    ("QNPCFX", 7),'
        for mode in ('missing', 'foreign', 'duplicate'):
            packet = copy.deepcopy(original)
            if mode == 'missing':
                packet['records'].pop()
            elif mode == 'foreign':
                packet['records'][0]['production_definition']['production_key'] = 'oteryn:item.tibia.i1'
            else:
                packet['records'].append(packet['records'][0])
            with self.assertRaises(ValueError, msg=mode):
                npc.cohort_receipt(source, packet)


if __name__ == '__main__':
    unittest.main()
