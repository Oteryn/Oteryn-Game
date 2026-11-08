"""The shipped Gnomus report must publish its physical reward candidate."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]

class M05NativeRewardTests(unittest.TestCase):
    def test_gnomus_reward_is_a_reviewed_physical_qualification_input(self):
        import sys
        directory = ROOT / 'tools/content-migration'
        sys.path.insert(0, str(directory))
        try:
            import quest_batch3_item_profiles as profiles
            self.assertIn('content/dialogues/quest-rules/quest-batch3-qn3-dangerous_depths.json', profiles.EXTRA_INPUTS)
        finally:
            sys.path.remove(str(directory))

    def test_runtime_packet_is_closed_to_the_source_reward(self):
        packet = json.loads((ROOT / 'tools/content-schema/native-gameplay/engnpc-m05-item-profiles.json').read_text())
        self.assertTrue(packet['runtime_enabled'])
        self.assertEqual({r['production_definition']['production_key'] for r in packet['records']}, {'oteryn:item.tibia.i27654'})
        self.assertTrue(all(r['classification'] in ('QUALIFIED_PLAIN_PORTABLE', 'EXISTING_NATIVE') for r in packet['classification']))

    def test_runtime_adapter_refuses_an_unqualified_packet(self):
        import sys
        import tempfile
        sys.path.insert(0, str(ROOT / 'tools/content-migration'))
        try:
            import engnpc_m05_item_profiles as adapter
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                target = root / adapter.PATH
                target.parent.mkdir(parents=True)
                target.write_text(json.dumps({'schema': adapter.SCHEMA, 'records': [], 'classification': []}))
                with self.assertRaisesRegex(ValueError, 'ENGNPC_M05_ITEM_QUALIFICATION'):
                    adapter.admit(root, {}, '', '', [], lambda value: value)
        finally:
            sys.path.pop(0)

    def test_disabled_publication_cannot_reach_the_native_merger(self):
        import sys
        import tempfile
        from unittest.mock import patch
        sys.path.insert(0, str(ROOT / 'tools/content-migration'))
        try:
            import engnpc_m05_item_profiles as adapter
            packet = json.loads((ROOT / adapter.PATH).read_text())
            packet['runtime_enabled'] = False
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                target = root / adapter.PATH
                target.parent.mkdir(parents=True)
                target.write_text(json.dumps(packet))
                with patch.object(adapter.shared, 'admit_records') as merge:
                    with self.assertRaisesRegex(ValueError, 'ENGNPC_M05_ITEM_QUALIFICATION'):
                        adapter.admit(root, {}, '', '', [], lambda value: value)
                    merge.assert_not_called()
        finally:
            sys.path.pop(0)

    def test_gnomus_token_is_admitted_by_the_runtime_item_provider(self):
        document = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_text())
        records = [r for r in document['records'] if r['production_definition']['production_key'] == 'oteryn:item.tibia.i27654']
        self.assertEqual(len(records), 1)
        admission = records[0]['admission']
        self.assertTrue(admission['materializable'])
        self.assertIn('CharacterInventory', admission['legal_destinations'])

if __name__ == '__main__':
    unittest.main()
