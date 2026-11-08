"""Reliable Ram keeps its semantic mount identity across donor ordinal changes."""
import json
import unittest
import qengine_rewards


class ReliableRamSourceTest(unittest.TestCase):
    def test_named_ram_look_and_grant_are_promoted_together(self):
        outputs = qengine_rewards.build()
        author = json.loads(outputs['content/cosmetics/outfits/authoring.json'])
        doc = author['document']
        rows = [m for m in doc['mounts'] if m['key'] == 'oteryn:content.mount.reliable_ram']
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]['look']['id'], 1934)
        self.assertNotEqual(rows[0]['look']['id'], 1962, 'Jaracal is not Reliable Ram')
        self.assertEqual((rows[0]['speed_bonus'], rows[0]['premium']), (10, False))
        mapping = next(m for m in doc['grant_mappings'] if m['grant_key'] == rows[0]['key'] + '/quest-grant')
        self.assertEqual(mapping['target'], {'kind': 'Mount', 'mount_key': rows[0]['key']})
        self.assertEqual(mapping['compatible_provenance'], [qengine_rewards.PROVENANCE])
        index = doc['mounts'].index(rows[0])
        paths = {p['path'] for p in author['promotions']}
        for field in ('key', 'revision', 'look', 'speed_bonus', 'premium', 'source'):
            self.assertIn(f'/mounts/{index}/{field}', paths)


if __name__ == '__main__':
    unittest.main()
