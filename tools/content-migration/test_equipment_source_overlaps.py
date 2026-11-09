"""Source identities prove every subtraction; the runtime union is never consulted."""
import re
import json
from pathlib import Path
import tempfile
import unittest

import equipment_source_overlaps as aliases
import refresh_equipment_profile_pin as pins


class SourceOverlapProofs(unittest.TestCase):
    SOURCE = '''const PROFILE_ADDITIONS: &[(&str, usize)] = &[
    ("UNCHANGED", 123),
    ("QMECH-M49", 19),
    ("QMECH-M50", 9),
    ("M26-ENDGAME", 6),
    ("QREADY1-CRAFT", 6),
    ("ENGNPC-M05", 1),
    ("M22-TAINTED-TALISMAN", 1),
    ("NPCFIX2-M22", 1),
];'''

    @classmethod
    def setUpClass(cls):
        cls.bosses = pins.keys(next(document for name, document, _
            in pins.source_cohorts(pins.ROOT) if name == 'SRX3-BOSS-CORPSES'))

    def test_exact_source_overlaps_remove_32_duplicate_contributions(self):
        result = aliases.refresh(self.SOURCE, pins.ROOT, self.bosses)
        counts = {name: int(value) for name, value in re.findall(r'\("([^"]+)", (\d+)\)', result)}
        self.assertEqual(counts, {'UNCHANGED': 123, 'QMECH-M49': 5, 'QMECH-M50': 1,
            'M26-ENDGAME': 1, 'QREADY1-CRAFT': 4, 'ENGNPC-M05': 0,
            'M22-TAINTED-TALISMAN': 0, 'NPCFIX2-M22': 0})
        self.assertEqual(sum(int(v) for v in re.findall(r', (\d+)\)', self.SOURCE)) - sum(counts.values()), 32)
        self.assertEqual(aliases.refresh(result, pins.ROOT, self.bosses), result)

    def test_unproven_boss_overlap_cannot_lower_the_cohort(self):
        with self.assertRaisesRegex(ValueError, 'SOURCE_INTERSECTION:M26-ENDGAME'):
            aliases.refresh(self.SOURCE, pins.ROOT, self.bosses - {'oteryn:item.tibia.i33864'})

    def test_arbitrary_prior_count_cannot_be_silently_repaired(self):
        with self.assertRaisesRegex(ValueError, 'COHORT_DECLARATION:QMECH-M49'):
            aliases.refresh(self.SOURCE.replace('"QMECH-M49", 19', '"QMECH-M49", 18'), pins.ROOT, self.bosses)

    def test_duplicate_or_foreign_packet_identity_refuses_overlap_proof(self):
        expected = aliases.names({21506, 21507})
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for ids in ({21506, 21508}, [21506, 21506]):
                with self.subTest(ids=ids):
                    (root / 'packet.json').write_text(json.dumps({'records': [
                        {'production_definition': {'production_key':
                            f'oteryn:item.tibia.i{item_id}'}} for item_id in ids]}))
                    with self.assertRaisesRegex(ValueError, 'SOURCE_IDENTITIES:packet.json'):
                        aliases.source_keys(root, 'packet.json', expected)


if __name__ == '__main__':
    unittest.main()
