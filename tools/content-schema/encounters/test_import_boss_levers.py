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


class GalthenHookCapture(unittest.TestCase):
    def test_exact_source_and_native_profile_are_required_for_typed_exit(self):
        import copy
        # Test-owned source bytes exercise the exact proof protocol; production
        # constants remain pinned to the complete donor bodies.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            key = "oteryn:creature.ahau"
            lever = root / "lever.lua"
            lever.write_text('local config = { boss = {name = "Ahau"}, exitTeleporter = Position(34002,31706,10) }\nBossLever(config)')
            monster = root / "monster.lua"
            monster.write_text('monster.events = {}')
            profile = {"profile": {"target": {"key": key}}}
            digest = lambda value: hashlib.sha256(value).hexdigest()
            proof = ("lever.lua", digest(lever.read_bytes()), "monster.lua",
                     digest(monster.read_bytes()), digest(IMPORTER.canonical(profile)),
                     [34002, 31706, 10])
            with patch.object(IMPORTER, "GALTHEN_CLOSURES", {key: proof}):
                actual = IMPORTER.capture(root, lever, {key: profile}, {"cooldown": 72000, "timeout": 1200})
                self.assertTrue(actual["callbacks_known"])
                self.assertEqual(actual["required_callbacks"], ["BossLeverOnDeath"])
                self.assertNotIn("config.exitTeleporter", actual["blockers"])
                self.assertEqual(actual["source_exit_teleporter"], [34002, 31706, 10])
                changed = copy.deepcopy(profile)
                changed["new_callback"] = "unsupported"
                for profiles in ({key: changed}, {}):
                    rejected = IMPORTER.capture(root, lever, profiles, {"cooldown": 72000, "timeout": 1200})
                    self.assertFalse(rejected["callbacks_known"])
                    self.assertIn("config.exitTeleporter", rejected["blockers"])
                monster.write_text(monster.read_text() + " -- changed")
                self.assertIsNone(IMPORTER.galthen_callback_closure(root, lever, key, {key: profile}))
                monster.write_text('monster.events = {}')
                lever.write_text(lever.read_text().replace("34002", "34003"))
                self.assertIsNone(IMPORTER.galthen_callback_closure(root, lever, key, {key: profile}))

    def test_unknown_boss_cannot_borrow_native_closure(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "lever.lua"
            path.write_text("BossLever({})")
            self.assertIsNone(IMPORTER.galthen_callback_closure(root, path, "oteryn:creature.other", {}))


if __name__ == "__main__":
    unittest.main()
