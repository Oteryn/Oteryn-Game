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
        expected = matrix.shard("reference", "node_boot",
                                matrix.pinned_sha(ROOT, matrix.NODE_BOOT_RUN, "PLATFORM_SHA"), True, {})
        expected["expected_rows"] = matrix.expected_rows(expected, matrix.row_scenarios())
        self.assertEqual(matrix.plan("reference"), [expected])

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
                log = self.write(tmp, "log", self.LOG if s["name"].startswith("g-core") else "NODE_BOOT_RESULT=NODE_BOOT_PASS\n")
                result = matrix.extract(log, name=s["name"], entry="node_boot", exit_code=0, seconds=600)
                (root / f"{prefix}{s['name']}" / "result.json").write_text(json.dumps(result))
            markdown, problems = matrix.summarize(shards, root, prefix)
        self.assertIn("| basic | NODE_BOOT_PASS | 0 | 10.0 | 0 | 0 | 0 | 0 | 0 |", markdown)
        core = next(s["name"] for s in shards if s["name"].startswith("g-core"))
        self.assertIn(f"- `{core}` W20a.full FAIL", markdown)
        self.assertIn(f"{core}: 1 MISSING scenario(s)", problems)
        self.assertIn("g-f4-f5-f6: World shard ran no scenario", problems)
        self.assertIn("rows-010-015: no result evidence", problems)
        self.assertNotIn("basic: World shard ran no scenario", problems)
        # The gates report MISSING rows without failing on them; the rest stays fatal.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for s in shards[:3]:
                (root / f"{prefix}{s['name']}").mkdir()
                log = self.write(tmp, "log", self.LOG if s["name"].startswith("g-core") else "NODE_BOOT_RESULT=NODE_BOOT_PASS\n")
                result = matrix.extract(log, name=s["name"], entry="node_boot", exit_code=0, seconds=600)
                (root / f"{prefix}{s['name']}" / "result.json").write_text(json.dumps(result))
            markdown, problems = matrix.summarize(shards, root, prefix, missing_fatal=False)
        self.assertNotIn("1 MISSING scenario(s)", problems)
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
            required = ''.join(f"SEAM_EVIDENCE e2e_all scenario={row['scenario']} status=PASS seconds=1\n"
                               for row in shards[0]["expected_rows"])
            log = self.write(tmp, "log", required + "SEAM_EVIDENCE e2e_all scenario=I01.full status=PASS seconds=1\n"
                                         "NODE_BOOT_RESULT=NODE_BOOT_PASS\n")
            result = matrix.extract(log, name="reference", entry="node_boot", exit_code=0, seconds=60)
            (root / "result.json").write_text(json.dumps(result))
            markdown, problems = matrix.summarize(shards, root, "node-boot-1-1-")
            self.assertEqual(problems, [])
            count = len(shards[0]["expected_rows"]) + 1
            self.assertIn(f"| reference | NODE_BOOT_PASS | 0 | 1.0 | {count} | {count} | 0 | 0 | 0 |", markdown)
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

    def test_redact_keeps_hotkey_event_lines(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = self.write(tmp, "in", "event=hotkey_equip_rejected stage=fence command_id=348\n"
                                           "hotkey api_key=1\nhotkey token\n")
            destination = Path(tmp) / "out"
            matrix.redact(source, destination)
            self.assertEqual(destination.read_text(), "event=hotkey_equip_rejected stage=fence command_id=348\n")


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


class RequiredRowEvidenceTests(unittest.TestCase):
    def summary(self, rows, lines, *, missing_fatal=True, result_entry=None):
        shards = matrix.plan('custom', rows=rows)
        self.assertEqual(len(shards), 1)
        shard = shards[0]
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            log = directory / 'node.log'
            log.write_text(lines + 'NODE_BOOT_RESULT=NODE_BOOT_PASS\n')
            result = matrix.extract(log, name=shard['name'], entry=shard['entry'],
                                    exit_code=0, seconds=3)
            if result_entry is not None:
                result['entry'] = result_entry
            output = directory / shard['name']
            output.mkdir()
            (output / 'result.json').write_text(json.dumps(result))
            return matrix.summarize(shards, directory, '', missing_fatal=missing_fatal)

    def test_other_scenario_cannot_replace_the_selected_row(self):
        _, problems = self.summary('206:207',
                                   'SEAM_EVIDENCE e2e_all scenario=I01.full status=PASS seconds=1\n')
        self.assertTrue(any('S09.client' in problem for problem in problems), problems)

    def test_other_entry_cannot_satisfy_the_same_named_artifact(self):
        _, problems = self.summary('206:207',
            'SEAM_EVIDENCE e2e_all scenario=S09.client status=PASS seconds=1\n',
            result_entry='game_platform')
        self.assertEqual(problems, ['rows-206-207: no result evidence'])

    def test_a_bounded_slice_or_full_row_cannot_replace_the_selected_client_row(self):
        for scenario in ('S09.logout', 'S09.full'):
            with self.subTest(scenario=scenario):
                _, problems = self.summary('206:207',
                    f'SEAM_EVIDENCE e2e_all scenario={scenario} status=PASS seconds=1\n')
                self.assertTrue(any('S09.client' in problem for problem in problems), problems)

    def test_partially_executed_batch_is_not_complete(self):
        _, problems = self.summary('1:3',
                                   'SEAM_EVIDENCE e2e_all scenario=C03.full status=PASS seconds=1\n')
        self.assertTrue(any('C12.full' in problem for problem in problems), problems)

    def test_duplicate_expected_row_is_not_one_completed_execution(self):
        line = 'SEAM_EVIDENCE e2e_all scenario=S09.client status=PASS seconds=1\n'
        _, problems = self.summary('206:207', line + line)
        self.assertTrue(any('duplicate' in problem and 'S09.client' in problem for problem in problems), problems)

    def test_capture_does_not_replace_a_terminal_verdict(self):
        for capture in (
            'SEAM_EVIDENCE e2e_capture scenario=S09.client status=CAPTURED evidence=logout\n',
            'SEAM_EVIDENCE e2e_all scenario=S09.client status=CAPTURED seconds=1\n',
        ):
            with self.subTest(capture=capture):
                _, problems = self.summary('206:207', capture +
                    'SEAM_EVIDENCE e2e_all scenario=I01.full status=PASS seconds=1\n')
                self.assertTrue(any('S09.client' in problem for problem in problems), problems)

    def test_missing_informational_does_not_waive_absent_evidence(self):
        _, problems = self.summary('206:207',
            'SEAM_EVIDENCE e2e_all scenario=I01.full status=PASS seconds=1\n', missing_fatal=False)
        self.assertTrue(any('S09.client' in problem for problem in problems), problems)

    def test_complete_terminal_evidence_can_include_extra_bounded_slices(self):
        _, problems = self.summary('206:207',
            'SEAM_EVIDENCE e2e_all scenario=S09.client status=PASS seconds=1\n'
            'SEAM_EVIDENCE e2e_all scenario=S09.logout status=PASS seconds=1\n'
            'SEAM_EVIDENCE e2e_capture scenario=S09.client status=CAPTURED evidence=logout\n')
        self.assertEqual(problems, [])

    def test_existing_failure_and_missing_status_policy_is_not_redefined(self):
        for status in ('FAIL', 'MISSING-SERVER', 'MISSING-CLIENT', 'SKIPPED'):
            with self.subTest(status=status):
                _, problems = self.summary('206:207',
                    f'SEAM_EVIDENCE e2e_all scenario=S09.client status={status} seconds=1\n',
                    missing_fatal=False)
                self.assertEqual(problems, [])

    # Terminal identities retained from run 45144642861, head aa16e94d.
    # Free-form diagnostics are omitted; FAIL acceptance remains the runner's job.
    def test_real_full_batch_keeps_existing_known_failure_accounting(self):
        scenarios = ('C03.full', 'C12.full', 'C09.full', 'C06.full', 'C02.full',
                     'C01.full', 'W22b.full', 'C05.full', 'C04.full')
        lines = ''.join(f'SEAM_EVIDENCE e2e_all scenario={sid} status=FAIL seconds=1\n'
                        for sid in scenarios)
        _, problems = self.summary('0:10', lines)
        self.assertEqual(problems, [])

    def test_real_mixed_and_legacy_batches_cannot_hide_one_silent_client(self):
        cases = (
            ('80:90', ('M10.full', 'M09.full', 'M06.full', 'C15.full', 'C08.full',
                       'S04b.full', 'C14.full', 'E02.full', 'C02.client'), 'C01.client'),
            ('220:230', ('C07.client', 'C20d.client', 'W22b.client', 'W23.client',
                         'W24.client', 'N07a.client', 'N07b.client', 'N07c.client',
                         'N07d.client'), 'C20a.client'),
        )
        for selected, scenarios, missing in cases:
            with self.subTest(selected=selected):
                lines = ''.join(f'SEAM_EVIDENCE e2e_all scenario={sid} status=FAIL seconds=1\n'
                                for sid in scenarios)
                _, problems = self.summary(selected, lines, missing_fatal=False)
                self.assertEqual(len(problems), 1, problems)
                self.assertIn(f'missing terminal verdict for {missing}', problems[0])

    def test_plan_records_ids_for_all_selected_rows_including_unwrapped_emitters(self):
        plan = matrix.plan('full')
        rows = [entry for shard in plan for entry in shard['expected_rows']]
        self.assertEqual(sorted(row['index'] for row in rows), matrix.row_indices())
        self.assertEqual(len({row['scenario'] for row in rows}), len(rows))
        by_index = {row['index']: row['scenario'] for row in rows}
        self.assertEqual({index: by_index[index] for index in (85, 86, 87, 206, 227)},
                         {85: 'S04b.full', 86: 'C14.full', 87: 'E02.full',
                          206: 'S09.client', 227: 'C07.client'})

    def test_non_row_lanes_do_not_claim_indexed_row_coverage(self):
        plan = matrix.plan('custom', groups='core;f4', basic=True, spells=True, game_platform=True)
        self.assertTrue(all(shard['expected_rows'] == [] for shard in plan))

    def test_reference_requires_its_selected_rows_as_well_as_existing_group_evidence(self):
        reference = matrix.plan('reference')[0]
        expected = {row['scenario'] for row in reference['expected_rows']}
        self.assertIn('C03.full', expected)
        self.assertIn('W23.full', expected)
        self.assertNotIn('S09.client', expected)



class RowSourceContractTests(unittest.TestCase):
    def assert_rejected(self, changes):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / matrix.CLIENT_PHASES
            source.mkdir(parents=True)
            names = ['world.rs', *(call.split('::')[0] + '.rs' for call in matrix.DIRECT_ROWS)]
            for name in names:
                text = (ROOT / matrix.CLIENT_PHASES / name).read_text()
                (source / name).write_text(changes.pop(name, lambda text: text)(text))
            for name, change in changes.items():
                (source / name).write_text(change(''))
            with self.assertRaises(matrix.PlanError):
                matrix.row_scenarios(Path(tmp))

    def test_unrecognised_dispatch_shape_cannot_disappear_from_plan(self):
        self.assert_rejected({'world.rs': lambda text: text.replace(
            'verdict::indexed("C03.full", async {', 'verdict::other("C03.full", async {', 1)})

    def test_selector_outside_the_supported_dispatcher_cannot_disappear_from_plan(self):
        self.assert_rejected({'other.rs': lambda _: 'fn extra() { if e2e_row_selected(999) {} }\n'})

    def test_duplicate_scenario_contract_is_rejected(self):
        self.assert_rejected({'world.rs': lambda text: text.replace(
            'verdict::indexed("C12.full", async {', 'verdict::indexed("C03.full", async {', 1)})

    def test_direct_emitter_contract_must_match_its_source(self):
        self.assert_rejected({'full_e2ef18.rs': lambda text: text.replace(
            'scenario=C14.full status=', 'scenario=S01.full status=')})

    def test_plan_cannot_drop_or_substitute_one_required_identity(self):
        for change in ('drop', 'replace'):
            with self.subTest(change=change):
                shard = matrix.plan('custom', rows='1:3')[0]
                if change == 'drop':
                    shard['expected_rows'].pop()
                else:
                    shard['expected_rows'][0]['scenario'] = 'I01.full'
                with self.assertRaises(matrix.PlanError):
                    matrix.validate_shard(shard)


if __name__ == "__main__":
    unittest.main()
