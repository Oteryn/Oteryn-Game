"""Native registration preserves unchanged compiler inputs while applying updates."""
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import register_spell_families as registration


class RegistrationWrites(unittest.TestCase):
    def test_unchanged_inputs_keep_mtime_and_changed_or_new_outputs_are_written(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            unchanged = root / 'unchanged.json'
            changed = root / 'changed.json'
            unchanged.write_bytes(b'preserved')
            changed.write_bytes(b'old')
            for path in [unchanged, changed]:
                os.utime(path, ns=(1, 1))
            generated = {'unchanged.json': b'preserved', 'changed.json': b'new',
                         'nested/created.json': b'created'}
            with patch.object(registration, 'ROOT', root), \
                 patch.object(registration, 'outputs', return_value=generated), \
                 patch('sys.argv', ['register_spell_families.py']):
                self.assertEqual(registration.main(), 0)
            self.assertEqual(unchanged.read_bytes(), b'preserved')
            self.assertEqual(unchanged.stat().st_mtime_ns, 1)
            self.assertEqual(changed.read_bytes(), b'new')
            self.assertNotEqual(changed.stat().st_mtime_ns, 1)
            self.assertEqual((root / 'nested/created.json').read_bytes(), b'created')
