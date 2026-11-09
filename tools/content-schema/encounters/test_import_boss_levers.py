"""Literal cooldown policy must preserve unsupported configurations as blockers."""
import hashlib
import importlib.util
import json
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


class CommittedBossCapture(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        root = Path(__file__).resolve().parents[3]
        cls.rows = json.loads((root / "content/encounters/bosses/crystal-lever-candidates.json").read_bytes())["records"]

    def test_shipped_capture_has_strict_source_constructor_order(self):
        keys = [(row["path"], row["line"]) for row in self.rows]
        self.assertEqual(len(keys), len(set(keys)), "one record per source constructor")
        self.assertEqual(keys, sorted(keys), "runtime requires increasing source keys")

    def test_shipped_capture_preserves_qualified_and_unavailable_source_facts(self):
        soul = [row for row in self.rows if row["path"] == IMPORTER.m26_endgame_capture.SOUL_PATH]
        self.assertEqual(len(soul), 6, "six registered constructors, no obsolete generic capture")
        self.assertEqual([row["line"] for row in soul], [296, 332, 355, 394, 422, 450])
        self.assertEqual([row["cooldown_seconds"] for row in soul], [72000] * 5 + [259200])
        self.assertEqual([row["callbacks_known"] for row in soul], [True] + [False] * 5)
        for key, proof in IMPORTER.GALTHEN_CLOSURES.items():
            with self.subTest(creature=key):
                rows = [row for row in self.rows if row["path"] == proof[0]]
                self.assertEqual(len(rows), 1)
                row = rows[0]
                self.assertEqual(row["sha256"], proof[1])
                self.assertEqual(row["native_profile_sha256"], proof[4])
                self.assertEqual(row["source_exit_teleporter"], proof[5])
                self.assertEqual(row["required_callbacks"], ["BossLeverOnDeath"])
                self.assertTrue(row["callbacks_known"])
                self.assertEqual(row["blockers"], [])
        for filename, cooldown in [("rotten_bakragore.lua", 244800), ("rotten_bakragore1.lua", 72000)]:
            with self.subTest(source=filename):
                rows = [row for row in self.rows if row["path"].endswith("/" + filename)]
                self.assertEqual(len(rows), 1)
                self.assertEqual(rows[0]["cooldown_seconds"], cooldown)
                self.assertFalse(rows[0]["callbacks_known"], "capture does not admit missing native callbacks")


class SelectiveGalthenCapture(unittest.TestCase):
    def test_recapture_preserves_other_sources_and_refuses_substitution_or_duplicates(self):
        import copy
        import contextlib
        import io
        import sys

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "config.lua.dist").write_text("bossDefaultTimeToFightAgain = 20 * 60 * 60\nbossDefaultTimeToDefeat = 20 * 60\n")
            lever = root / "lever.lua"
            lever.write_text('local config = { boss = {name = "Ahau"}, exitTeleporter = Position(34002,31706,10) }\nBossLever(config)')
            monster = root / "monster.lua"
            monster.write_text("monster.events = {}")
            key = "oteryn:creature.ahau"
            profile = {"profile": {"target": {"key": key}}}
            proof = ("lever.lua", IMPORTER.sha(lever.read_bytes()), "monster.lua",
                     IMPORTER.sha(monster.read_bytes()), IMPORTER.sha(IMPORTER.canonical(profile)),
                     [34002, 31706, 10])
            profiles = root / "profiles.json"
            profiles.write_text(json.dumps({"records": [profile]}))
            output = root / "candidates.json"
            arguments = ["import_boss_levers.py", "--source-root", str(root),
                         "--native-profiles", str(profiles), "--output", str(output), "--update-galthen"]
            with patch.object(IMPORTER, "GALTHEN_CLOSURES", {key: proof}), patch.object(sys, "argv", arguments):
                row = IMPORTER.capture(root, lever, {key: profile}, {"cooldown": 72000, "timeout": 1200})
                other = copy.deepcopy(row)
                other.update(path="unselected.lua", line=7, callbacks_known=False)
                document = {"schema": "OTERYN_CRYSTAL_BOSS_LEVER_CANDIDATES/v1",
                            "source_repository": "zimbadev/crystalserver", "source_revision": IMPORTER.PIN,
                            "records": [row, other, copy.deepcopy(row)]}
                output.write_text(json.dumps(document))
                with contextlib.redirect_stdout(io.StringIO()):
                    IMPORTER.main()
                    first = output.read_bytes()
                    IMPORTER.main()
                self.assertEqual(output.read_bytes(), first, "selective recapture is deterministic")
                self.assertEqual(json.loads(first)["records"], [row, other], "unselected rows are retained exactly")
                lever.write_bytes(lever.read_bytes() + b" -- source changed")
                with self.assertRaisesRegex(ValueError, "SOURCE_SUBSTITUTION"):
                    IMPORTER.main()
                self.assertEqual(output.read_bytes(), first, "failed custody cannot replace the output")
                lever.write_bytes(lever.read_bytes().removesuffix(b" -- source changed"))
                duplicated = json.loads(first)
                duplicated["records"].append(copy.deepcopy(other))
                output.write_text(json.dumps(duplicated))
                before = output.read_bytes()
                with self.assertRaisesRegex(ValueError, "DUPLICATE_SOURCE_CAPTURE"):
                    IMPORTER.main()
                self.assertEqual(output.read_bytes(), before, "unselected duplicates are not silently discarded")


if __name__ == "__main__":
    unittest.main()
