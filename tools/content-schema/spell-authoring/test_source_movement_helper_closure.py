import copy
from pathlib import Path
import unittest

import source_movement_helper_closure as adapter


class MovementHelperClosureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof, cls.current, cls.old = adapter.build(Path('/workspace/spell-sources/canary'), 'magic rope')

    def test_exact_delta_does_not_qualify_engine_execution(self):
        self.assertFalse(self.proof['engine_and_transitive_API_execution_qualified'])
        self.assertFalse(self.proof['runtime_activation'])
        self.assertTrue(self.proof['global_delta']['remaining_global_bytes_equal'])
        self.assertEqual([False, True, True], [row['full_bytes_equal'] for row in self.proof['support_files']])

    def test_other_revisions_and_other_native_spells_refused(self):
        for name, revision in [('find person', adapter.REVISION), ('magic rope', adapter.OLD_REVISION)]:
            with self.subTest(name=name, revision=revision), self.assertRaises(ValueError):
                adapter.qualify(name, self.current, self.old, revision)

    def test_changed_rope_ids_helper_flags_and_cast_refused(self):
        for path, before, after in [
            ('data/global.lua', b'386', b'387'),
            ('data/libs/functions/tile.lua', b'CONST_PROP_BLOCKSOLID', b'CONST_PROP_BLOCKPATH'),
            ('data/libs/functions/position.lua', b'DIRECTION_SOUTH', b'DIRECTION_NORTH'),
            (adapter.native.FILES['magic rope'], b'return true', b'return false')]:
            changed = copy.deepcopy(self.current)
            self.assertIn(before, changed[path])
            changed[path] = changed[path].replace(before, after, 1)
            with self.subTest(path=path), self.assertRaises(ValueError):
                adapter.qualify('magic rope', changed, self.old)

    def test_pvp_block_and_unrelated_addition_fail_closed(self):
        for changed_bytes in [self.current[adapter.GLOBAL_PATH].replace(b'expert-pvp', b'expert-pvpx'),
                              self.current[adapter.GLOBAL_PATH] + b'\nropeSpots = {}\n']:
            changed = {**self.current, adapter.GLOBAL_PATH: changed_bytes}
            with self.assertRaisesRegex(ValueError, 'global delta'):
                adapter.qualify('magic rope', changed, self.old)

    def test_missing_support_and_changed_old_reference_refused(self):
        changed = copy.deepcopy(self.current)
        del changed['data/libs/functions/tile.lua']
        with self.assertRaisesRegex(ValueError, 'file set'):
            adapter.qualify('magic rope', changed, self.old)
        old = {**self.old, adapter.GLOBAL_PATH: self.old[adapter.GLOBAL_PATH] + b'\n'}
        with self.assertRaisesRegex(ValueError, 'qualified support'):
            adapter.qualify('magic rope', self.current, old)


if __name__ == '__main__':
    unittest.main()
