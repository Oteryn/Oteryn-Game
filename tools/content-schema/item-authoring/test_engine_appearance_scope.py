"""Scoped appearance parsing preserves independent mutable consumer records."""

import copy
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "content-migration"))
import qualification_inputs as engine


class AppearanceScopeTests(unittest.TestCase):
    def test_scope_reuses_bytes_without_sharing_consumer_mutations(self):
        # An object-family protobuf record with id 100 and default flags.
        raw = bytes((10, 2, 8, 100))
        with patch.object(
            engine,
            "_decode_appearance_objects",
            wraps=engine._decode_appearance_objects,
        ) as decode:
            with engine.appearance_input_scope():
                first = engine.load_appearance_objects(raw)
                second = engine.load_appearance_objects(raw)
                first[100]["flags"]["changed"] = True
                self.assertEqual(second[100]["flags"], {})
                self.assertEqual(decode.call_count, 1)
                del first[100]
                self.assertNotIn(100, first)
                self.assertIn(100, second)
                self.assertEqual(
                    copy.deepcopy(second),
                    {100: {"id": 100, "flags": {}, "frame_groups": []}},
                )
                with engine.appearance_input_scope():
                    self.assertEqual(
                        engine.load_appearance_objects(raw)[100]["flags"], {}
                    )
                self.assertEqual(decode.call_count, 1)
                engine.load_appearance_objects(raw + bytes((16, 1)))
                self.assertEqual(
                    decode.call_count, 2, "changed bytes require a fresh parse"
                )
            self.assertIsInstance(engine.load_appearance_objects(raw), dict)
            self.assertEqual(
                decode.call_count, 3, "the next qualification cannot reuse this scope"
            )

    def test_exception_does_not_leave_a_cache_active(self):
        with (
            self.assertRaisesRegex(ValueError, "qualification refused"),
            engine.appearance_input_scope(),
        ):
            raise ValueError("qualification refused")
        self.assertIsNone(engine._appearance_inputs.get())


if __name__ == "__main__":
    unittest.main()
