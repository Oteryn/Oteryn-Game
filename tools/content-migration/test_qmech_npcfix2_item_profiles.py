"""M22: narrowly qualify the source-backed nonstackable talisman output."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import qmech_npcfix2_item_profiles as tool


class TalismanQualification(unittest.TestCase):
    def test_selected_output_reproduces_its_retained_source_packet(self):
        actual = tool.capture()
        expected = json.loads((tool.shared.ROOT / tool.PATH).read_bytes())
        self.assertEqual(actual, expected)
        self.assertEqual([r['production_definition']['production_key'] for r in actual['records']], [tool.KEY])
        self.assertEqual(actual['records'][0]['admission']['stack_class'], 'NonStackable')

    def test_stackable_output_fails_qualification(self):
        packet = {'records': [{'admission': {'stack_class': 'StackCapable'}}]}
        with patch.object(tool.shared, 'capture', return_value=packet), self.assertRaisesRegex(ValueError, 'STACK_CONFLICT'):
            tool.capture()

    def test_wrong_item_never_reaches_native_admission(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            path = root / tool.PATH
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps({'schema': tool.SCHEMA, 'records': [
                {'production_definition': {'production_key': 'oteryn:item.tibia.i3031'}}]}))
            with patch.object(tool.shared, 'admit_records') as admit, self.assertRaisesRegex(ValueError, 'PROFILE_SCOPE'):
                tool.admit(root, {}, '', '', {}, lambda value: value)
            admit.assert_not_called()

    def test_default_reward_owner_packet_is_unchanged(self):
        expected = json.loads((tool.shared.ROOT / tool.shared.PROOF).read_bytes())
        self.assertEqual(tool.shared.capture(), expected)


if __name__ == '__main__':
    unittest.main()
