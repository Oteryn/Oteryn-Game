"""The Elder terminal reuses its pre-existing canonical reference exactly."""
import copy
import json
import unittest
from pathlib import Path
import boss_corpse_terminal_reference as terminal

ROOT=Path(__file__).resolve().parents[2]

class ExistingElderReferenceTests(unittest.TestCase):
    def test_existing_canonical_reference_is_retained_without_second_allocation(self):
        index=json.loads((ROOT/'content/items/index.json').read_bytes())
        records=[row['definition'] for name in index['shards'] for row in json.loads((ROOT/name).read_bytes())['records']]
        original=copy.deepcopy(records[25377])
        self.assertEqual(original['identity']['key'],'oteryn:item.tibia.i43666')
        terminal.extend_records(records,ROOT)
        self.assertEqual([i for i,r in enumerate(records) if r['identity']['key']==terminal.KEY],[25377])
        self.assertEqual(records[25377],original,'all original UNKNOWN reference fields stay unchanged')
        self.assertEqual(len(records),34117)
