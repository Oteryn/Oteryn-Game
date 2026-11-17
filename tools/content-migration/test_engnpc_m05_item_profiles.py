"""M05 freshness checks must never rewrite a stale publication."""
import tempfile
import unittest
from pathlib import Path
import engnpc_m05_item_profiles as profiles

class PublicationTests(unittest.TestCase):
    def test_check_accepts_current_and_preserves_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / 'packet.json'
            profiles.publish(target, {'schema': 'fixture'})
            before = target.read_bytes()
            profiles.publish(target, {'schema': 'fixture'}, check=True)
            self.assertEqual(target.read_bytes(), before)

    def test_check_rejects_missing_and_stale_without_writing(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / 'packet.json'
            with self.assertRaisesRegex(ValueError, 'ENGNPC_M05_ITEM_PROFILE_DRIFT'):
                profiles.publish(target, {}, check=True)
            self.assertFalse(target.exists())
            target.write_bytes(b'stale')
            with self.assertRaisesRegex(ValueError, 'ENGNPC_M05_ITEM_PROFILE_DRIFT'):
                profiles.publish(target, {}, check=True)
            self.assertEqual(target.read_bytes(), b'stale')

if __name__ == '__main__':
    unittest.main()
