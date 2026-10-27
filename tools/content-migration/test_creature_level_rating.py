import importlib.util
import json
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location(
    "rating", HERE / "creature_level_rating.py"
)
rating = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(rating)


class CreatureLevelRatingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.report = rating.build_report()

    def test_calibration_within_ten_percent(self):
        for row in self.report["calibration"]:
            self.assertLessEqual(
                abs(row["oteryn"] - row["observed_tibia"]), row["observed_tibia"] * 0.1
            )

    def test_deterministic(self):
        self.assertEqual(rating.render(), rating.render())

    def test_every_creature_is_rated(self):
        expected = 0
        for path in sorted(
            (rating.ROOT / "content/creatures/definitions").glob("creatures-*.json")
        ):
            expected += len(json.loads(path.read_text(encoding="utf-8"))["records"])
        self.assertEqual(len(self.report["creatures"]), expected)
        self.assertEqual(
            len({row["key"] for row in self.report["creatures"]}), expected
        )

    def test_missing_attacks_are_null_safe(self):
        burst, dps = rating.attack_metrics(None)
        self.assertEqual((burst, dps), (0, 0))
        row = rating.rate_creature(
            "oteryn:creature.fixture",
            {"health": 20, "experience": 5, "details": {}},
            None,
        )
        self.assertEqual(row["burst"], 0)


if __name__ == "__main__":
    unittest.main()
