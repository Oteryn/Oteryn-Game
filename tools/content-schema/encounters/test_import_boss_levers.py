"""Literal cooldown policy must preserve unsupported configurations as blockers."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("boss_levers", Path(__file__).with_name("import_boss_levers.py"))
IMPORTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORTER)


class CooldownCapture(unittest.TestCase):
    def test_only_literal_false_preserves_default_cooldown(self):
        # Crystal 201189f actions/bosses_levers/the_primal_manace.lua:19.
        for expression, blocked in [("false", False), ("true", True), ("policy()", True)]:
            with self.subTest(expression=expression), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                source = root / "lever.lua"
                source.write_text("local config = { disableCooldown = " + expression + " }\nBossLever(config)")
                row = IMPORTER.capture(root, source, {}, {"cooldown": 72000, "timeout": 1200})
                self.assertEqual("config.disableCooldown" in row["blockers"], blocked)
                self.assertEqual(row["cooldown_seconds"], 72000)


if __name__ == "__main__":
    unittest.main()
