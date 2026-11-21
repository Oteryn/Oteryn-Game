"""Process isolation, complete execution and failure propagation for migration checks."""
import importlib.util
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import textwrap
import unittest

from parallel_checks import Check, run_checks


class ParallelChecks(unittest.TestCase):
    def test_independent_processes_overlap_without_sharing_state(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            script = root / "rendezvous.py"
            script.write_text(
                "import os, pathlib, sys, time\n"
                "root = pathlib.Path(sys.argv[1])\n"
                "(root / sys.argv[2]).write_text(str(os.getpid()))\n"
                "deadline = time.monotonic() + 5\n"
                "while not (root / sys.argv[3]).exists():\n"
                "    if time.monotonic() > deadline: raise SystemExit(7)\n"
                "    time.sleep(0.01)\n"
            )
            checks = [Check(name, (sys.executable, str(script), str(root), name, other))
                      for name, other in (("one", "two"), ("two", "one"))]
            run_checks(checks, jobs=2)
            self.assertNotEqual((root / "one").read_text(), (root / "two").read_text())

    def test_failure_does_not_skip_the_remaining_inventory(self):
        with tempfile.TemporaryDirectory() as directory:
            completed = Path(directory) / "completed"
            checks = [
                Check("fails", (sys.executable, "-c", "raise SystemExit(7)")),
                Check("still-runs", (sys.executable, "-c",
                      "import pathlib, sys; pathlib.Path(sys.argv[1]).write_text('ran')",
                      str(completed))),
            ]
            with self.assertRaisesRegex(RuntimeError, "fails=7"):
                run_checks(checks, jobs=1)
            self.assertEqual(completed.read_text(), "ran")

    def test_invalid_plan_fails_before_launching_a_process(self):
        with tempfile.TemporaryDirectory() as directory:
            absent = str(Path(directory) / "must-not-launch")
            check = Check("duplicate", (absent,))
            for checks, jobs in (([], 2), ([check, check], 2), ([check], 0), ([check], 5)):
                with self.subTest(checks=checks, jobs=jobs), self.assertRaises(ValueError):
                    run_checks(checks, jobs=jobs)

    def test_workflow_aggregate_requires_every_qualification_to_succeed(self):
        workflow = (Path(__file__).resolve().parents[2]
                    / ".github/workflows/content-tree-migration.yml").read_text()
        lines = workflow.splitlines()
        start = lines.index("  validate:")
        end = next((index for index in range(start + 1, len(lines))
                    if re.fullmatch(r"  [a-z_]+:", lines[index])), len(lines))
        aggregate = lines[start:end]
        bindings = dict((dependency, variable) for variable, dependency in re.findall(
            r"^\s+([A-Z_]+): \$\{\{ needs\.([a-z_]+)\.result \}\}$",
            "\n".join(aggregate), re.MULTILINE))
        self.assertEqual(set(bindings), {
            "source_contracts", "migration_checks", "content_integrity",
        })
        self.assertEqual(len(set(bindings.values())), len(bindings))
        run_steps = [index for index, line in enumerate(aggregate)
                     if line.strip() == "run: |"]
        self.assertEqual(len(run_steps), 1)
        body = []
        for line in aggregate[run_steps[0] + 1:]:
            if line.strip() and not line.startswith("          "):
                break
            body.append(line)
        script = textwrap.dedent("\n".join(body))
        self.assertTrue(script.strip())

        def execute(results):
            return subprocess.run(["bash", "-c", script], env=results,
                                  capture_output=True, text=True, timeout=5)

        success = {variable: "success" for variable in bindings.values()}
        result = execute(success)
        self.assertEqual(result.returncode, 0, result.stderr)
        for dependency, variable in bindings.items():
            for state in ("failure", "cancelled", "skipped"):
                with self.subTest(dependency=dependency, state=state):
                    result = execute(success | {variable: state})
                    self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_inventory_has_every_existing_check_once(self):
        path = Path(__file__).with_name("test_world_project_v2_to_tree.py")
        spec = importlib.util.spec_from_file_location("world_tree_checks", path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        checks = module.check_inventory()
        names = [check.name for check in checks]
        expected_internal = {
            "package-tree-contract", "document-roundtrip-negatives",
            "closed-weapon-metadata", "relation-row-ownership", "closed-bank-admission",
        }
        expected_external = {
            "test_monster_seven_spell_overlay.py", "test_quest_registration_preservation.py",
            "test_boss_corpse_profiles", "test_authored_npc_item_profiles",
            "test_quest_corpse_profiles", "test_boss_reward_yield_profiles",
            "test_equipment_source_overlaps", "test_equipment_known_speed_pin",
            "test_refresh_equipment_profile_pin", "test_refresh_season_creature_inventory",
            "test_m42_cooking_canonical", "test_quest_batch3_item_profiles",
            "test_questpc7_merge_audit_callbacks", "test_full_spell_artifact.CorpseSourceCohortTests",
            "test_questpc2_rotten_loot_items", "summer_corpse_items.py",
            "test_quest_reward_item_semantics.py", "test_amber_item_semantics.py",
            "test_lion_trial_item_profiles.py", "test_engnpc_m04_item_profiles.py",
            "test_world_change_item_profiles.py", "quest_reward_item_semantics.py",
            "test_reward_claim_authoring.py", "test_reward_claim_variant_authoring.py",
            "test_starter_kit_authoring.py", "test_item_taxonomy.py",
            "test_item_official_navigation.py",
        }
        self.assertEqual(set(names), expected_internal | expected_external)
        self.assertEqual(len(names), len(set(names)))
        workflow_names = [check.name for check in module.check_inventory(workflow_checks=True)]
        outer = {
            "test_source_relocations.py", "test_regenerate_content.py",
            "test_oteryn_additive_driver.py", "test_npc_presence_stage.py",
            "test_item_navigation_source_supplement.py", "test_item_engine_navigation.py",
            "test_item_external_family_refinement.py", "test_item_bounded7_navigation.py",
        }
        self.assertEqual(set(workflow_names), set(names) | outer)
        self.assertEqual(len(workflow_names), len(set(workflow_names)))
        self.assertEqual(workflow_names.count("test_item_taxonomy.py"), 1)


if __name__ == "__main__":
    unittest.main()
