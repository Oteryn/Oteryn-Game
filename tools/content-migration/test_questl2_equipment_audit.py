"""New pinned quest Items extend the identity union; arbitrary live keys cannot."""
import unittest
from questl2_equipment_audit import checked_source_union


class SourceUnion(unittest.TestCase):
    def test_pinned_source_extension(self):
        self.assertEqual(checked_source_union({'old'}, {'new'}, {'new'}, {'old', 'new'}),
                         {'old', 'new'})

    def test_unproven_live_extension_is_refused(self):
        with self.assertRaisesRegex(ValueError, 'SOURCE_UNION_DRIFT'):
            checked_source_union({'old'}, {'new'}, {'new'}, {'old', 'new', 'injected'})

    def test_missing_published_key_is_refused(self):
        with self.assertRaisesRegex(ValueError, 'SOURCE_UNION_DRIFT'):
            checked_source_union({'old'}, {'new'}, {'new'}, {'new'})

    def test_source_proof_must_equal_explicit_producer(self):
        with self.assertRaisesRegex(ValueError, 'UNREVIEWED_SOURCE_PROOF'):
            checked_source_union({'old'}, {'new'}, {'new', 'injected'}, {'old', 'new', 'injected'})
