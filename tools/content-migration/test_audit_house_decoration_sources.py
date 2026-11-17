"""Furniture source closure must report unavailable targets, never repair them."""
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from audit_house_decoration_sources import capture, unavailable_targets


class HouseDecorationSourceClosure(unittest.TestCase):
    def test_missing_target_is_reported_without_inventing_reciprocal_sibling(self):
        xml = b'<items><item id="34276"><attribute key="wrapableto" value="23398"/><attribute key="rotateto" value="34277"/></item><item id="34278"><attribute key="rotateto" value="34276"/></item></items>'
        rows = unavailable_targets(xml, {34276, 34278}, {34276, 34278})
        self.assertEqual(rows, [{'source': 34276, 'operation': 'rotateto', 'target': 34277,
                                 'target_in_xml': False, 'target_in_appearances': False,
                                 'target_in_canonical_references': False}])

    def test_present_appearance_does_not_grant_canonical_identity(self):
        xml = b'<items><item id="1"><attribute key="wrapableto" value="3"/><attribute key="transformonuse" value="2"/></item><item id="2"/></items>'
        rows = unavailable_targets(xml, {1, 2}, {1})
        self.assertEqual(len(rows), 1)
        self.assertTrue(rows[0]['target_in_xml'])
        self.assertTrue(rows[0]['target_in_appearances'])
        self.assertFalse(rows[0]['target_in_canonical_references'])

    def test_complete_target_closure_needs_no_unavailable_rows(self):
        xml = b'<items><item id="1"><attribute key="wrapableto" value="3"/><attribute key="rotateto" value="2"/></item><item id="2"/></items>'
        self.assertEqual(unavailable_targets(xml, {1, 2}, {1, 2}), [])

    def test_different_donor_snapshot_cannot_claim_the_pinned_revision(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'data/items').mkdir(parents=True)
            (root / 'data/items/items.xml').write_bytes(b'<items/>')
            (root / 'data/items/appearances.dat').write_bytes(b'')
            with self.assertRaisesRegex(ValueError, 'HOUSE_DECORATION_DONOR_SOURCE_DRIFT'):
                capture({'crystal': root})


if __name__ == '__main__':
    unittest.main()
