"""Authoring boundary and preservation checks for the batch-one callback compiler."""

import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import batch_one_world_callbacks as compiler


class BatchOneCallbacksTest(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.inputs = (
            self.root / "tools/content-schema/quest-authoring/batch-one-world-callbacks"
        )
        self.inputs.mkdir(parents=True)
        self.output = self.root / "content/quests/world-callbacks.json"
        self.output.parent.mkdir(parents=True)
        self.chayenne = self.inputs / "chayenne_realm.json"
        self.original = {
            "schema": 1,
            "quest": "oteryn:quest.chayenne_realm",
            "callbacks": [
                {
                    "key": "oteryn:quest-transition/chayenne-realm/exit",
                    "quest": "oteryn:quest.chayenne_realm",
                    "trigger": "step_in",
                    "selector": {
                        "position": None,
                        "unique": None,
                        "action": 55024,
                        "appearance": None,
                    },
                    "lever": None,
                    "reversible": False,
                    "passages": [],
                    "teleport": {"x": 33117, "y": 32604, "floor": -6},
                    "storage": [],
                    "offerings": [],
                    "return_on_failure": False,
                    "failure_message": None,
                    "sources": [
                        (
                            f"crystal:{compiler.PIN}:data-global/scripts/quests/"
                            "chayenne_realm/movements_enter_realm.lua:19"
                        )
                    ],
                }
            ],
        }
        self.save(self.chayenne, self.original)
        # The authoring tests need no other quest's not-yet-committed input.
        # Git retrieval is mocked; source syntax, bounds, ownership and files are real.
        self.reader = patch.object(
            compiler, "read_source", return_value="-- source\n" * 100
        )
        self.reader.start()
        self.addCleanup(self.reader.stop)

    def save(self, path, value):
        path.write_text(
            json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )

    def run_compiler(self, **kwargs):
        return compiler.run(self.root, self.root / "crystal", **kwargs)

    def test_generation_preserves_foreign_bytes_order_and_check_does_not_write(self):
        # Feed one already-validated cohort to the real final merge/write path.
        cohort = patch.object(
            compiler, "load_inputs", return_value=self.original["callbacks"]
        )
        cohort.start()
        self.addCleanup(cohort.stop)
        foreign = copy.deepcopy(self.original["callbacks"][0])
        foreign.update(
            key="oteryn:quest-callback/z-foreign",
            quest="oteryn:quest.foreign",
            failure_message="Zażółć, literal \\n stays here.",
        )
        foreign_two = {**foreign, "key": "oteryn:quest-callback/a-foreign"}
        raw_one = json.dumps(foreign, ensure_ascii=True, separators=(",", ": "))
        raw_two = json.dumps(foreign_two, ensure_ascii=False, indent=1)
        baseline = (
            '{"schema": 1, "callbacks": [\n' + raw_one + ",\n" + raw_two + "\n]}\n"
        )
        self.output.write_text(baseline, encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "out of date"):
            self.run_compiler(check=True)
        self.assertEqual(self.output.read_text(encoding="utf-8"), baseline)
        self.run_compiler()
        generated = self.output.read_text(encoding="utf-8")
        self.assertIn(raw_one, generated)
        self.assertIn(raw_two, generated)
        self.assertLess(generated.index(raw_one), generated.index(raw_two))
        records = json.loads(generated)["callbacks"]
        self.assertEqual(
            [r for r in records if r["quest"] == "oteryn:quest.foreign"],
            [foreign, foreign_two],
        )
        self.assertEqual(len(records), 3)
        self.run_compiler(check=True)
        self.run_compiler(check=True, quest="chayenne_realm")
        self.run_compiler()
        self.assertEqual(self.output.read_text(encoding="utf-8"), generated)

    def test_duplicate_keys_and_new_parser_fields_fail_before_writing(self):
        self.output.write_text('{"schema":1,"callbacks":[]}\n', encoding="utf-8")
        baseline = self.output.read_bytes()
        self.chayenne.write_text(
            '{"schema":1,"schema":1,"quest":"x","callbacks":[]}', encoding="utf-8"
        )
        with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
            self.run_compiler(inputs_only=True, quest="chayenne_realm")
        duplicate = copy.deepcopy(self.original)
        duplicate["callbacks"].append(copy.deepcopy(duplicate["callbacks"][0]))
        self.save(self.chayenne, duplicate)
        with self.assertRaisesRegex(ValueError, "duplicate callback key"):
            self.run_compiler(inputs_only=True, quest="chayenne_realm")
        for field, value in (("success_message", "text"), ("failure_teleport", None)):
            invalid = copy.deepcopy(self.original)
            invalid["callbacks"][0][field] = value
            self.save(self.chayenne, invalid)
            with self.assertRaisesRegex(ValueError, "unknown field"):
                self.run_compiler(inputs_only=True, quest="chayenne_realm")
        feru = self.inputs / "ferumbras_ascension.json"
        invalid = copy.deepcopy(self.original)
        invalid["quest"] = "oteryn:quest.ferumbras_ascension_quest"
        row = invalid["callbacks"][0]
        row.update(
            quest=invalid["quest"],
            key="oteryn:quest-transition/ferumbras-ascension/tarbaz-first-note",
            storage=[
                {
                    "track": "oteryn:quest-progress/quest/u10_90/ferumbras_ascension/basin_counter",
                    "minimum": 8,
                    "maximum": 8,
                    "set": None,
                }
            ],
            sources=[
                (
                    f"crystal:{compiler.PIN}:data-global/scripts/quests/"
                    "ferumbras_ascension/actions_tarbaz_notes.lua:3"
                )
            ],
        )
        self.save(feru, invalid)
        with self.assertRaisesRegex(ValueError, "integer"):
            self.run_compiler(inputs_only=True, quest="ferumbras_ascension")
        self.assertEqual(self.output.read_bytes(), baseline)

    def test_authoring_checks_exact_sources_and_owner_without_runtime_file(self):
        self.run_compiler(inputs_only=True, quest="chayenne_realm")
        self.assertFalse(self.output.exists())
        good = self.original["callbacks"][0]["sources"][0]
        bad_sources = [
            good.replace(compiler.PIN, "0" * 40),
            good.rsplit(":", 1)[0],
            good.rsplit(":", 1)[0] + ":101",
            good.rsplit(":", 1)[0] + ":20-19",
            good.replace("chayenne_realm/", "the_dream_courts/"),
            good.replace("chayenne_realm/", "chayenne_realm/../"),
        ]
        for source in bad_sources:
            invalid = copy.deepcopy(self.original)
            invalid["callbacks"][0]["sources"] = [source]
            self.save(self.chayenne, invalid)
            with self.assertRaisesRegex(ValueError, "source"):
                self.run_compiler(inputs_only=True, quest="chayenne_realm")
        invalid = copy.deepcopy(self.original)
        invalid["callbacks"][0]["quest"] = "oteryn:quest.the_dream_courts_quest"
        self.save(self.chayenne, invalid)
        with self.assertRaisesRegex(ValueError, "foreign quest"):
            self.run_compiler(inputs_only=True, quest="chayenne_realm")
        self.save(self.chayenne, self.original)
        self.save(self.inputs / "foreign_quest.json", self.original)
        with self.assertRaisesRegex(ValueError, "foreign input"):
            self.run_compiler(inputs_only=True)
        self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
