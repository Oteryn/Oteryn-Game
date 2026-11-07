"""No-network tests of the SPEED-1 step-speed table (CONDITIONS-0 §4.2)."""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import generate_step_speed_table as generator  # noqa: E402

# Canary `04b83b51` step speeds (`creature.hpp:76-78, 1061-1067`), computed with Canary's double
# formula: the floor speed, paralysis' floor 40, a level 1 player (110), level 111 (220) and the
# clamp ceiling.
CANARY_SAMPLES = {10: 9, 40: 99, 110: 278, 111: 280, 150: 366, 220: 500, 1000: 1326, 65535: 4717}


class StepSpeedTableTest(unittest.TestCase):
    def setUp(self) -> None:
        self.committed = json.loads(generator.OUTPUT.read_text(encoding="utf-8"))

    def test_committed_file_is_the_generated_table(self) -> None:
        self.assertEqual(generator.OUTPUT.read_text(encoding="utf-8"), generator.render(generator.table()))

    def test_range_digest_and_samples(self) -> None:
        values = self.committed["step_speed"]
        self.assertEqual(self.committed["schema"], generator.SCHEMA)
        self.assertEqual((self.committed["speed_min"], self.committed["speed_max"]), (10, 65_535))
        self.assertEqual(len(values), 65_535 - 10 + 1)
        self.assertEqual(self.committed["sha256_u16le"], generator.digest(values))
        self.assertEqual(
            self.committed["sha256_u16le"],
            "323b70ceb76edc53ed341a31d67fea5543149689ead9ad51856286e792bfe436",
        )
        for speed, step_speed in CANARY_SAMPLES.items():
            self.assertEqual(values[speed - 10], step_speed, speed)

    def test_table_is_positive_and_non_decreasing(self) -> None:
        values = self.committed["step_speed"]
        self.assertTrue(all(1 <= value <= 0xFFFF for value in values))
        self.assertTrue(all(a <= b for a, b in zip(values, values[1:])))

    def test_no_value_sits_on_a_rounding_boundary(self) -> None:
        # Another libm could round differently only within ~1e-9 of .5; no speed comes near it.
        import math

        for speed in range(10, 65_536):
            raw = generator.SPEED_A * math.log(speed + generator.SPEED_B) + generator.SPEED_C + 0.5
            self.assertGreater(abs(raw - round(raw)), 1e-7, speed)

    def test_a_speed_outside_the_formula_range_is_refused(self) -> None:
        with self.assertRaises(ValueError):
            generator.step_speed(7)


if __name__ == "__main__":
    unittest.main()
