"""No-op regeneration must not invalidate Cargo's generated-input fingerprints."""
import importlib.util
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import register_spell_families as registry

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('rooms', ROOT / 'tools/content-schema/quest-authoring/grizzly_rooms.py')
rooms = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rooms)
key_spec = importlib.util.spec_from_file_location('item_key_builder', ROOT / 'tools/content-schema/native-gameplay/build_full_spell_artifact.py')
key_builder = importlib.util.module_from_spec(key_spec)
key_spec.loader.exec_module(key_builder)
STAMP = 1_650_000_000_000_000_000


class NativeRegenerationMtime(unittest.TestCase):
    def test_unchanged_included_item_keys_preserve_mtime(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'item-keys.json'
            with patch.object(key_builder, 'item_keys', return_value=b'{"records":[]}\n'):
                expected = key_builder.refresh_item_keys(output)
                os.utime(output, ns=(STAMP, STAMP))
                self.assertEqual(key_builder.refresh_item_keys(output), expected)
                self.assertEqual(output.stat().st_mtime_ns, STAMP)

    def test_unchanged_registry_pins_preserve_mtime_and_changed_bytes_are_written(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / 'generated/pin.rs'
            generated = {'generated/pin.rs': b'const PIN: u8 = 1;\n'}
            with patch.object(registry, 'ROOT', root), patch.object(registry, 'outputs', return_value=generated), patch.object(sys, 'argv', ['register_spell_families.py']):
                self.assertEqual(registry.main(), 0)
                os.utime(output, ns=(STAMP, STAMP))
                self.assertEqual(registry.main(), 0)
                self.assertEqual(output.stat().st_mtime_ns, STAMP)
                generated['generated/pin.rs'] = b'const PIN: u8 = 2;\n'
                self.assertEqual(registry.main(), 0)
                self.assertEqual(output.read_bytes(), generated['generated/pin.rs'])
                self.assertNotEqual(output.stat().st_mtime_ns, STAMP)

    def test_unchanged_included_room_json_preserves_mtime(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'rooms.json'
            with patch.object(rooms, 'OUTPUT', output), patch.object(rooms, 'generate', return_value={'rooms': []}), patch.object(sys, 'argv', ['grizzly_rooms.py']):
                self.assertEqual(rooms.main(), 0)
                os.utime(output, ns=(STAMP, STAMP))
                self.assertEqual(rooms.main(), 0)
                self.assertEqual(output.stat().st_mtime_ns, STAMP)


if __name__ == '__main__':
    unittest.main()
