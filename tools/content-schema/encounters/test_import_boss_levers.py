"""Literal cooldown policy must preserve unsupported configurations as blockers."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

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


class PrimalHookCapture(unittest.TestCase):
    def test_missing_source_keeps_hooks_unknown(self):
        with tempfile.TemporaryDirectory() as directory:
            self.assertEqual(IMPORTER.capture_primal_hooks(Path(directory)), ([], False))

    def test_verified_source_retains_death_and_think_hooks_and_rejects_substitution(self):
        # Test-owned Lua fixtures; no donor callback body is redistributed.
        sources = {
            "monster.lua": 'monster.events = {"ThePrimalMenaceDeath", "ThePrimeOrdealBossDeath"}\nmType.onThink = function(monster, interval) end',
            "hazard.lua": 'local event = CreatureEvent("ThePrimalMenaceDeath")',
            "quest.lua": 'local event = CreatureEvent("ThePrimeOrdealBossDeath")',
        }
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            proofs = {}
            for name, text in sources.items():
                raw = text.encode()
                (root / name).write_bytes(raw)
                proofs[name] = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
            with patch.object(IMPORTER, "PRIMAL_HOOK_BLOBS", proofs):
                self.assertEqual(IMPORTER.capture_primal_hooks(root),
                    (["ThePrimalMenaceDeath", "ThePrimeOrdealBossDeath", "mType.onThink"], True))
                (root / "quest.lua").write_text(sources["quest.lua"] + " -- substituted")
                self.assertEqual(IMPORTER.capture_primal_hooks(root), ([], False))


if __name__ == "__main__":
    unittest.main()
