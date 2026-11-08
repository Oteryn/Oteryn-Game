"""Exact M41 operands preserve real source identities, timers and container sizes."""
import json
import unittest

import m41_quest_item_profiles as tool


class M41ItemProfileTests(unittest.TestCase):
    def test_exact_set_preserves_corpse_timers_and_beak_stack(self):
        packet = json.loads((tool.ROOT / tool.PATH).read_bytes())
        rows = {r['production_definition']['production_key']: r for r in packet['records']}
        self.assertEqual(set(rows), {f'oteryn:item.tibia.i{i}' for i in tool.IDS})
        for item in [52677, 52678, 52679, 52680]:
            row = rows[f'oteryn:item.tibia.i{item}']
            self.assertEqual(row['admission']['legal_destinations'], ['Ground'])
            self.assertEqual(row['semantics']['container']['value']['capacity']['value'], 10)
            self.assertEqual(row['semantics']['temporal']['value']['duration']['value'], 300000)
        self.assertEqual(rows['oteryn:item.tibia.i52632']['admission']['stack_class'], 'StackCapable')
        self.assertEqual(rows['oteryn:item.tibia.i31574']['semantics']['physical']['value']['weight']['value'], 850)
        planned = {'profiles': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
        tool.admit(tool.ROOT, planned, 'profiles', 'manifest',
                   lambda r: r['production_definition']['production_key'], lambda v: json.dumps(v).encode())
        self.assertEqual(len(json.loads(planned['profiles'])['records']), 6)
        with self.assertRaisesRegex(ValueError, 'DUPLICATE'):
            tool.admit(tool.ROOT, planned, 'profiles', 'manifest',
                       lambda r: r['production_definition']['production_key'], lambda v: json.dumps(v).encode())
