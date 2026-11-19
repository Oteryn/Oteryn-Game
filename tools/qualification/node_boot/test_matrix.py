#!/usr/bin/env python3
"""Coverage, knob and summary tests for the node boot shard planner."""
from pathlib import Path
import json
import re
import tempfile
import unittest

import matrix

ROOT = matrix.ROOT
WORKFLOW = ROOT / ".github/workflows/node-boot-qualification.yml"


def harness_text():
    return (ROOT / matrix.NODE_BOOT_RUN).read_text()


def rust_text():
    return "\n".join(p.read_text() for p in sorted((ROOT / matrix.CLIENT_PHASES).glob("*.rs")))


def selected_rows(shard, indices):
    start, length = map(int, shard["env"]["OTERYN_E2E_ROW_BATCH"].split(":"))
    return [i for i in indices if start <= i < start + length]


class CoverageTests(unittest.TestCase):
    def setUp(self):
        self.groups = matrix.journey_groups()
        self.indices = matrix.row_indices()

    def assert_groups_once(self, shards):
        planned = [name for s in shards if s["env"].get("NODE_BOOT_E2E_ONLY", "rows") != "rows"
                   for name in s["env"]["NODE_BOOT_E2E_ONLY"].split(",")]
        self.assertEqual(sorted(planned), sorted(g for g in self.groups if g != "rows"))

    def test_full_set_runs_every_group_row_and_mode_once(self):
        shards = matrix.plan("full")
        self.assert_groups_once(shards)
        rows = [s for s in shards if s["env"].get("NODE_BOOT_E2E_ONLY") == "rows"]
        covered = [i for s in rows for i in selected_rows(s, self.indices)]
        self.assertEqual(sorted(covered), self.indices, "every row index in exactly one batch")
        self.assertTrue(all(selected_rows(s, self.indices) for s in rows), "no empty row batch")
        self.assertEqual(rows[-1]["env"]["OTERYN_E2E_ROW_BATCH"].split(":")[1], str(matrix.OPEN_END),
                         "the last batch is open-ended so new rows still run")
        names = [s["name"] for s in shards]
        for mode in ("basic", "spells", "game-platform"):
            self.assertEqual(names.count(mode), 1)

    def test_basic_pass_runs_on_exactly_one_shard(self):
        for set_name in ("pr", "full"):
            shards = matrix.plan(set_name)
            basic = [s for s in shards if s["env"].get("NODE_BOOT_WORLD") == "0"]
            self.assertEqual([s["name"] for s in basic], ["basic"])
            for s in shards:
                if s["entry"] == "node_boot" and s["world"]:
                    self.assertEqual(s["env"].get("NODE_BOOT_SKIP_BASIC"), "1", s["name"])

    def test_pr_set_matches_the_single_machine_row_batch(self):
        shards = matrix.plan("pr")
        self.assert_groups_once(shards)
        source = (ROOT / matrix.CLIENT_PHASES / "e2e_row_batch.rs").read_text()
        start, length = map(int, re.search(r"E2E_ROW_BATCH_DEFAULT: \(usize, usize\) = \((\d+), (\d+)\);", source).groups())
        rows = [s for s in shards if s["env"].get("NODE_BOOT_E2E_ONLY") == "rows"]
        covered = [i for s in rows for i in selected_rows(s, self.indices)]
        self.assertEqual(sorted(covered), [i for i in self.indices if start <= i < start + length])
        self.assertEqual(len(shards), 7)

    def test_reference_is_one_unchanged_machine(self):
        self.assertEqual(matrix.plan("reference"), [matrix.shard(
            "reference", "node_boot", matrix.pinned_sha(ROOT, matrix.NODE_BOOT_RUN, "PLATFORM_SHA"), True, {})])

    def test_custom_set_splits_groups_and_row_ranges(self):
        shards = matrix.plan("custom", groups="core;f7clock", rows="20:45", batch="10", spells=True)
        self.assertEqual([s["name"] for s in shards],
                         ["g-core", "g-f7clock", "rows-020-030", "rows-030-040", "rows-040-045", "spells"])
        self.assertEqual(matrix.plan("custom", rows="all", batch="50")[-1]["env"]["OTERYN_E2E_ROW_BATCH"],
                         f"300:{matrix.OPEN_END}")

    def test_invalid_inputs_fail_the_plan(self):
        for kwargs in ({"groups": "corr"}, {"groups": "rows"}, {"groups": "core;core"},
                       {"rows": "45:20"}, {"rows": "5"}, {"batch": "0"}, {"batch": "x"}, {}):
            with self.subTest(kwargs=kwargs), self.assertRaises(matrix.PlanError):
                matrix.plan("custom", **kwargs)
        with self.assertRaises(matrix.PlanError):
            matrix.plan("pr", groups="core")
        with self.assertRaises(matrix.PlanError):
            matrix.plan("nightly")


class KnobTests(unittest.TestCase):
    def test_every_emitted_knob_is_read_by_the_harness(self):
        harness, rust = harness_text(), rust_text()
        readers = {
            "NODE_BOOT_WORLD": harness, "NODE_BOOT_SKIP_BASIC": harness, "NODE_BOOT_SPELLS": harness,
            "NODE_BOOT_E2E_ONLY": rust, "OTERYN_E2E_ROW_BATCH": rust,
        }
        emitted = {key for set_name in ("reference", "pr", "full") for s in matrix.plan(set_name) for key in s["env"]}
        self.assertLessEqual(emitted, set(readers))
        for key in emitted:
            with self.subTest(key=key):
                pattern = rf'std::env::var\("{key}"\)' if readers[key] is rust else rf"\$\{{{key}[:}}]"
                self.assertRegex(readers[key], pattern)

    def test_typos_and_unknown_knobs_are_rejected(self):
        good = matrix.plan("pr")[1]
        for env in ({"NODE_BOOT_E2E_ONLY": "corr"}, {"NODE_BOOT_E2E_ONLY": "core,core"},
                    {"OTERYN_E2E_ROW_BATCH": "5"}, {"OTERYN_E2E_ROW_BATCH": "5:0"},
                    {"NODE_BOOT_SKIP_BASIC": "0"}, {"NODE_BOOT_E2E_ONLYY": "core"},
                    {"NODE_BOOT_E2E_ONLY": "core\nGITHUB_PATH=/tmp"}):
            with self.subTest(env=env), self.assertRaises(matrix.PlanError):
                matrix.validate_shard({**good, "env": env})
        for name in ("Bad", "a\nb", "-x", "x" * 64):
            with self.subTest(name=name), self.assertRaises(matrix.PlanError):
                matrix.export_env({**good, "name": name})

    def test_pins_match_the_lanes_that_check_them(self):
        lifecycle = (ROOT / "tools/qualification/game_platform/lifecycle.sh").read_text()
        portal = [s for s in matrix.plan("full") if s["entry"] == "game_platform"][0]
        self.assertIn(f"readonly GAME_PLATFORM_SHA={portal['platform_ref']}\n", lifecycle)

    def test_export_env_writes_only_validated_lines(self):
        text = matrix.export_env(matrix.plan("pr")[4])
        self.assertEqual(text, "SHARD_NAME=rows-000-005\nSHARD_ENTRY=node_boot\nSHARD_WORLD=1\n"
                               "NODE_BOOT_E2E_ONLY=rows\nNODE_BOOT_SKIP_BASIC=1\nOTERYN_E2E_ROW_BATCH=0:5\n")


class EvidenceTests(unittest.TestCase):
    LOG = (
        "SEAM_EVIDENCE e2e_all scenario=C19a.full status=PASS seconds=3.832\n"
        "SEAM_EVIDENCE e2e_all scenario=S03.full status=MISSING-SERVER seconds=0.167 evidence=source:x.rs:17\n"
        "SEAM_EVIDENCE e2e_all scenario=W20a.full status=FAIL seconds=50.692 field=x\n"
        "SEAM_EVIDENCE e2e_all scenario=$(bad) status=PASS seconds=1\n"
        "SEAM_EVIDENCE E2EF1_INCOMPLETE full_row_failures_reported_to_e2e_all\n"
        "NODE_BOOT_RESULT=NODE_BOOT_FAIL\n"
    )

    def write(self, directory, name, text):
        path = Path(directory) / name
        path.write_text(text)
        return path

    def test_extract_keeps_only_the_closed_vocabulary(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = matrix.extract(self.write(tmp, "log", self.LOG), name="g-core", entry="node_boot",
                                    exit_code=1, seconds=60)
        self.assertEqual([(s["id"], s["status"]) for s in result["scenarios"]],
                         [("C19a.full", "PASS"), ("S03.full", "MISSING-SERVER"), ("W20a.full", "FAIL")])
        self.assertEqual(result["result"], "NODE_BOOT_FAIL")
        self.assertTrue(result["incomplete"])
        self.assertFalse(matrix.shard_passed(result))
        self.assertTrue(matrix.shard_passed({**result, "exit_code": 0, "result": "NODE_BOOT_PASS"}))
        self.assertFalse(matrix.shard_passed({**result, "exit_code": 1, "result": "NODE_BOOT_PASS"}))

    def test_summary_fails_on_missing_evidence_empty_world_shard_and_missing_status(self):
        shards = matrix.plan("pr")
        prefix = "node-boot-1-1-"
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for s in shards[:3]:
                (root / f"{prefix}{s['name']}").mkdir()
                log = self.write(tmp, "log", self.LOG if s["name"] == "g-core" else "NODE_BOOT_RESULT=NODE_BOOT_PASS\n")
                result = matrix.extract(log, name=s["name"], entry="node_boot", exit_code=0, seconds=600)
                (root / f"{prefix}{s['name']}" / "result.json").write_text(json.dumps(result))
            markdown, problems = matrix.summarize(shards, root, prefix)
        self.assertIn("| basic | NODE_BOOT_PASS | 0 | 10.0 | 0 | 0 | 0 | 0 | 0 |", markdown)
        self.assertIn("- `g-core` W20a.full FAIL", markdown)
        self.assertIn("g-core: 1 MISSING scenario(s)", problems)
        self.assertIn("g-f4-f5-f6: World shard ran no scenario", problems)
        self.assertIn("rows-010-015: no result evidence", problems)
        self.assertNotIn("basic: World shard ran no scenario", problems)
        # The gates report MISSING rows without failing on them; the rest stays fatal.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for s in shards[:3]:
                (root / f"{prefix}{s['name']}").mkdir()
                log = self.write(tmp, "log", self.LOG if s["name"] == "g-core" else "NODE_BOOT_RESULT=NODE_BOOT_PASS\n")
                result = matrix.extract(log, name=s["name"], entry="node_boot", exit_code=0, seconds=600)
                (root / f"{prefix}{s['name']}" / "result.json").write_text(json.dumps(result))
            markdown, problems = matrix.summarize(shards, root, prefix, missing_fatal=False)
        self.assertNotIn("g-core: 1 MISSING scenario(s)", problems)
        self.assertIn("g-f4-f5-f6: World shard ran no scenario", problems)
        self.assertIn("rows-010-015: no result evidence", problems)

    def test_summary_passes_complete_evidence_and_reports_overlap(self):
        shards = matrix.plan("custom", groups="core;f4")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for s in shards:
                (root / s["name"]).mkdir()
                log = self.write(tmp, "log", "SEAM_EVIDENCE e2e_all scenario=I01.full status=PASS seconds=1\n"
                                             "NODE_BOOT_RESULT=NODE_BOOT_PASS\n")
                result = matrix.extract(log, name=s["name"], entry="node_boot", exit_code=0, seconds=60)
                (root / s["name"] / "result.json").write_text(json.dumps(result))
            markdown, problems = matrix.summarize(shards, root, "")
        self.assertEqual(problems, [])
        self.assertIn("I01.full (g-core, g-f4)", markdown)

    def test_summary_reads_a_lone_artifact_extracted_without_its_directory(self):
        # download-artifact puts a single pattern match straight into its path (the reference set).
        shards = matrix.plan("reference")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "shards"
            root.mkdir()
            log = self.write(tmp, "log", "SEAM_EVIDENCE e2e_all scenario=I01.full status=PASS seconds=1\n"
                                         "NODE_BOOT_RESULT=NODE_BOOT_PASS\n")
            result = matrix.extract(log, name="reference", entry="node_boot", exit_code=0, seconds=60)
            (root / "result.json").write_text(json.dumps(result))
            markdown, problems = matrix.summarize(shards, root, "node-boot-1-1-")
            self.assertEqual(problems, [])
            self.assertIn("| reference | NODE_BOOT_PASS | 0 | 1.0 | 1 | 1 | 0 | 0 | 0 |", markdown)
            result["shard"] = "other"
            (root / "result.json").write_text(json.dumps(result))
            _, problems = matrix.summarize(shards, root, "node-boot-1-1-")
            self.assertEqual(problems, ["reference: no result evidence"])

    def test_redact_drops_secret_bearing_lines(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = self.write(tmp, "in", "keep\nPASSWORD=x\npostgres://u@h\nmysql://a\nSecret\nauth token\n"
                                           "api_key=1\nGRANT ALL\nkeep too\n")
            destination = Path(tmp) / "out"
            matrix.redact(source, destination)
            self.assertEqual(destination.read_text(), "keep\nkeep too\n")


class WorkflowTests(unittest.TestCase):
    def test_workflow_plans_with_this_tool_and_uploads_every_shard(self):
        text = WORKFLOW.read_text()
        for fragment in (
            "python3 tools/qualification/node_boot/test_matrix.py\n",
            "python3 tools/qualification/node_boot/matrix.py plan ",
            "      fail-fast: false\n",
            "        shard: ${{ fromJSON(needs.plan.outputs.shards) }}\n",
            "          ref: ${{ matrix.shard.platform_ref }}\n",
            "      - name: Upload redacted shard evidence\n        if: always()\n",
            "          name: node-boot-${{ github.run_id }}-${{ matrix.shard.name }}\n",
            "          overwrite: true\n",
            "          pattern: node-boot-${{ github.run_id }}-*\n",
            "          PREFIX: node-boot-${{ github.run_id }}-\n",
            "          test \"$SHARD_RESULT\" = success\n",
            "python3 tools/qualification/node_boot/matrix.py summarize ",
        ):
            self.assertIn(fragment, text)
        self.assertNotIn("redact_ci_log.py", text, "shard evidence uses the wider matrix.py filter")


if __name__ == "__main__":
    unittest.main()
