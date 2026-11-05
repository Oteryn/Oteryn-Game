import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('assets', Path(__file__).with_name('import_assets.py'))
assets = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(assets)


def number(field, value):
    result = bytes([field << 3])
    while value > 127:
        result += bytes([(value & 127) | 128])
        value >>= 7
    return result + bytes([value])


def message(field, value):
    return bytes([(field << 3) | 2, len(value)]) + value


class ImportTests(unittest.TestCase):
    def test_b24_ambient_ids_have_a_separate_namespace(self):
        data = message(1, number(1, 7) + message(2, b'ambient.ogg'))
        data += message(3, number(1, 25) + number(2, 7))
        self.assertEqual(assets.decode_ambient(data), {'25': ['ambient.ogg']})
        with self.assertRaises(ValueError):
            assets.decode_ambient(data + message(3, number(1, 25) + number(2, 7)))

    def test_official_indirection_and_random_variants(self):
        # Synthetic protobuf records; no official audio bytes or guessed real cue IDs.
        sounds = b''.join(message(1, number(1, i) + message(2, name))
                          for i, name in [(7, b'a.ogg'), (8, b'b.ogg')])
        simple = number(1, 41) + message(5, number(1, 7))
        random = number(1, 42) + message(6, number(1, 7) + number(1, 8))
        data = sounds + message(2, simple) + message(2, random)
        self.assertEqual(assets.decode_cues(data), {'41': ['a.ogg'], '42': ['a.ogg', 'b.ogg']})
        with self.assertRaises(ValueError):
            assets.decode_cues(data + message(2, simple))
        with self.assertRaises(ValueError):
            assets.decode_cues(sounds + message(2, number(1, 43) + message(5, number(1, 99))))
        with self.assertRaises(ValueError):
            assets.decode_cues(data[:-1])

    def test_inventory_pins_metadata_and_audio_and_rejects_missing_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'a.ogg').write_bytes(b'synthetic audio placeholder')
            data = message(1, number(1, 7) + message(2, b'a.ogg'))
            data += message(2, number(1, 41) + message(5, number(1, 7)))
            (root / 'sounds.dat').write_bytes(data)
            result = assets.inventory(root, 'a' * 64, 'sounds.dat')
            self.assertEqual(result['sound_cues'], {'41': ['a.ogg']})
            self.assertEqual([r['name'] for r in result['files']], ['a.ogg', 'sounds.dat'])
            self.assertEqual(result['files'][0]['sha256'], assets.digest((root / 'a.ogg').read_bytes()))
            (root / 'a.ogg').unlink()
            with self.assertRaises(ValueError):
                assets.inventory(root, 'a' * 64, 'sounds.dat')


if __name__ == '__main__':
    unittest.main()
