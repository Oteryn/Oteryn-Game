import json
import tempfile
import unittest
from pathlib import Path

import bestiary_source_roster as roster


class BestiaryRosterTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        consumer = self.root / roster.CONSUMER
        consumer.parent.mkdir(parents=True)
        consumer.write_text((roster.ROOT / roster.CONSUMER).read_text())
        self.path = 'content/creatures/definitions/creatures-00000-00001.json'
        self.index = {'schema': 'OTERYN_FAMILY_INDEX/v1', 'family': 'Creature', 'record_count': 2, 'shards': [self.path]}
        self.shard = {'family': 'Creature', 'shard': {'count': 2}, 'records': [
            {'authoring': {'profile': {'bestiary': {'charm_points': 30}}}},
            {'authoring': {'profile': {'bestiary': None}}},
        ]}
        self.write()

    def write(self):
        (self.root / 'content/creatures/definitions').mkdir(parents=True, exist_ok=True)
        (self.root / 'content/creatures/definitions/index.json').write_text(json.dumps(self.index))
        (self.root / self.path).write_text(json.dumps(self.shard))

    def test_addition_updates_exact_roster_and_both_population_pins(self):
        source = roster.refreshed(self.root)
        self.assertIn(f'"{self.path}"', source)
        self.assertIn('const CREATURE_COUNT: usize = 2;', source)
        self.assertIn('const BESTIARY_COUNT: usize = 1;', source)
        self.assertIn('fn canonical_bestiary_rows(', source)
        self.assertNotIn('creatures-01500-', source)

    def test_refuses_index_record_count_mismatch(self):
        self.index['record_count'] = 3
        self.write()
        with self.assertRaisesRegex(ValueError, 'SOURCE_COUNT'):
            roster.refreshed(self.root)

    def test_refuses_noncanonical_shard_reference(self):
        self.index['shards'] = ['../unqualified.json']
        self.write()
        with self.assertRaisesRegex(ValueError, 'SOURCE_PATH'):
            roster.refreshed(self.root)


if __name__ == '__main__':
    unittest.main()
