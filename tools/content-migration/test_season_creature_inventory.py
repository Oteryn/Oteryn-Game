"""A refreshed registry may not hide a substituted canonical Creature."""
import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

from refresh_season_creature_inventory import ROOT, generated


class RegistrySourceBoundaryTests(unittest.TestCase):
    def test_same_count_substituted_creature_cannot_refresh_the_registry(self):
        index = json.loads((ROOT / 'content/creatures/definitions/index.json').read_bytes())
        shard = ROOT / index['shards'][0]
        document = json.loads(shard.read_bytes())
        forged = copy.deepcopy(document)
        row = next(row for row in forged['records']
                   if row['definition']['identity']['key'] != 'oteryn:creature.cake_golem')
        row['definition']['identity']['key'] = 'oteryn:creature.unqualified_substitution'
        original = Path.read_bytes

        def read(path):
            return json.dumps(forged).encode() if path == shard else original(path)

        with patch.object(Path, 'read_bytes', read):
            with self.assertRaisesRegex(ValueError, 'Canonical family summary mismatch: Creature'):
                generated(ROOT)


if __name__ == '__main__':
    unittest.main()
