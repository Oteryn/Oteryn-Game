"""A complete older donor population must not qualify current dispenser facts."""
import tarfile
import tempfile
import unittest
from pathlib import Path

import dispenser_item_profiles as profiles


class DispenserSourceTests(unittest.TestCase):
    def test_capture_rejects_older_xml_even_with_all_dispenser_ids(self):
        archive = profiles.ROOT / ('tools/content-schema/quest-authoring/samples/'
                                   'donor-source/corpus.tar.gz')
        with tarfile.open(archive) as corpus:
            member = corpus.extractfile('blobs/5530bb76d896d31fb75d6cad23969e1cc463b1c5')
            self.assertIsNotNone(member)
            raw = member.read()
        self.assertTrue(set(profiles.IDS) <= profiles.source.xml_index(raw).keys())
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'items.xml'
            path.write_bytes(raw)
            with self.assertRaisesRegex(ValueError, 'DISPENSER_CURRENT_XML_DIGEST'):
                profiles.capture(path)


if __name__ == '__main__':
    unittest.main()
