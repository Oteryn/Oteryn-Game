import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import refresh_season_creature_inventory as inventory


class SeasonalInventoryMergeTests(unittest.TestCase):
    def test_combined_cake_and_primal_inventory_retains_exact_loot_membership(self):
        outputs = inventory.generated(inventory.ROOT)
        manifest = json.loads(outputs['content/manifest.json'])
        index = json.loads((inventory.ROOT / 'content/loot/index.json').read_bytes())
        self.assertEqual(manifest['families']['Loot']['records'], index['record_count'])

    def test_unqualified_extra_loot_is_still_rejected(self):
        index_path = inventory.ROOT / 'content/loot/index.json'
        index = json.loads(index_path.read_bytes())
        shard_path = inventory.ROOT / index['shards'][0]
        read_bytes = Path.read_bytes

        def injected(path):
            raw = read_bytes(path)
            if path == index_path:
                document = json.loads(raw)
                document['record_count'] += 1
                return json.dumps(document).encode()
            if path == shard_path:
                document = json.loads(raw)
                extra = copy.deepcopy(document['records'][0])
                extra['definition']['identity']['key'] = 'oteryn:loot.unqualified_inventory_probe'
                document['records'].append(extra)
                return json.dumps(document).encode()
            return raw

        with patch.object(Path, 'read_bytes', injected):
            with self.assertRaisesRegex(ValueError, 'Canonical family summary mismatch: Loot'):
                inventory.generated(inventory.ROOT)


if __name__ == '__main__':
    unittest.main()
