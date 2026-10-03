#!/usr/bin/env python3
"""Main qualification stays applicable when stack events ignore branch filters."""
from pathlib import Path
import json
import re
import subprocess
import unittest

from validate_repository_policy_core import indented_yaml_mapping_block

ROOT = Path(__file__).resolve().parents[2]
MAIN_GUARD = "github.event.pull_request.base.ref == 'main' || !github.event.pull_request.base.ref"
ARCHITECTURE_GUARD = (
    "github.event.pull_request.base.ref == 'main' && "
    "(github.event.action != 'edited' || github.event.changes.base != null) "
    "|| !github.event.pull_request.base.ref"
)


def workflow(name):
    text = (ROOT / ".github/workflows" / name).read_text()
    jobs_text = indented_yaml_mapping_block(text, "jobs", 0)
    if jobs_text is None:
        raise AssertionError(f"Missing jobs in {name}")
    jobs = {}
    for key in re.findall(r"^  ([a-z_]+):$", jobs_text, re.MULTILINE):
        block = indented_yaml_mapping_block(jobs_text, key, 2)
        jobs[key] = dict(re.findall(r"^    (if|needs): (.+)$", block, re.MULTILINE))
    events = indented_yaml_mapping_block(text, "on", 0)
    pull_request = indented_yaml_mapping_block(events or "", "pull_request", 2)
    types = indented_yaml_mapping_block(pull_request or "", "types", 4)
    event_types = re.findall(r"^      - ([a-z_]+)$", types or "", re.MULTILINE)
    concurrency = indented_yaml_mapping_block(text, "concurrency", 0)
    concurrency_fields = dict(re.findall(
        r"^  (group|cancel-in-progress): (.+)$", concurrency or "", re.MULTILINE,
    ))
    return {"jobs": jobs, "pull_request_types": event_types, "concurrency": concurrency_fields}


def evaluate_guard(expression, base, *, action="opened", base_change=None):
    # Execute the actual workflow boolean expression with string/null operands.
    # The supported operators have the same semantics in JS and Actions here.
    allowed = (MAIN_GUARD, f"always() && ({MAIN_GUARD})", ARCHITECTURE_GUARD)
    if expression not in allowed:
        raise AssertionError(f"Unexpected applicability expression: {expression!r}")
    script = (
        "const github={event:JSON.parse(process.argv[1])};"
        "const always=()=>true;process.stdout.write(JSON.stringify(Boolean("
        + expression
        + ")));"
    )
    event = {
        "pull_request": {"base": {"ref": base}},
        "action": action,
        "changes": {"base": base_change},
    }
    return json.loads(subprocess.check_output(["node", "-e", script, json.dumps(event)]))


def evaluate_group(template, *, action, number, run_id, base_change=None):
    """Expand the actual concurrency template with Actions string/null inputs."""
    context = {
        "event": {
            "action": action,
            "changes": {"base": base_change},
            "pull_request": {"number": number},
        },
        "run_id": run_id,
    }

    def expand(match):
        script = (
            "const github=JSON.parse(process.argv[1]);"
            "const format=(pattern,...values)=>pattern.replace(/\\{(\\d+)\\}/g,"
            "(_,index)=>String(values[Number(index)]));"
            "process.stdout.write(JSON.stringify(String("
            + match.group(1).strip()
            + ")));"
        )
        return json.loads(subprocess.check_output([
            "node", "-e", script, json.dumps(context),
        ]))

    return re.sub(r"\$\{\{(.*?)\}\}", expand, template)


class MainJobApplicabilityTests(unittest.TestCase):
    def test_architecture_metadata_runs_cannot_cancel_active_audit(self):
        concurrency = workflow("architecture-semantic-audit.yml")["concurrency"]
        group = concurrency["group"]
        ordinary = evaluate_group(group, action="synchronize", number=88, run_id=100)
        first_edit = evaluate_group(group, action="edited", number=88, run_id=101)
        second_edit = evaluate_group(group, action="edited", number=88, run_id=102)
        # Even a skipped job still starts a workflow-level concurrency run.
        # None of those metadata runs may share the active qualification group.
        self.assertNotEqual(first_edit, ordinary)
        self.assertNotEqual(second_edit, ordinary)
        self.assertNotEqual(first_edit, second_edit)

    def test_architecture_source_and_retarget_runs_supersede_old_qualification(self):
        concurrency = workflow("architecture-semantic-audit.yml")["concurrency"]
        self.assertEqual(concurrency["cancel-in-progress"], "true")
        group = concurrency["group"]
        ordinary = evaluate_group(group, action="synchronize", number=88, run_id=100)
        next_head = evaluate_group(group, action="synchronize", number=88, run_id=101)
        retarget = evaluate_group(
            group, action="edited", number=88, run_id=102,
            base_change={"ref": {"from": "codex/prepared-parent"}},
        )
        self.assertEqual(ordinary, "architecture-semantic-audit-88")
        self.assertEqual(next_head, ordinary)
        self.assertEqual(retarget, ordinary)

    def test_architecture_concurrency_separates_prs(self):
        group = workflow("architecture-semantic-audit.yml")["concurrency"]["group"]
        for action in ("synchronize", "edited"):
            with self.subTest(action=action):
                self.assertNotEqual(
                    evaluate_group(group, action=action, number=88, run_id=100),
                    evaluate_group(group, action=action, number=89, run_id=100),
                )

    def test_architecture_qualifies_after_stack_ready_then_main_base_edit(self):
        audit = workflow("architecture-semantic-audit.yml")
        expression = audit["jobs"]["audit"]["if"]
        self.assertIn("ready_for_review", audit["pull_request_types"])
        self.assertFalse(evaluate_guard(expression, "codex/prepared-parent", action="ready_for_review"))
        # Retargeting an already-ready PR, including automatic retargeting after
        # its parent merges, keeps the head and emits edited rather than synchronize.
        self.assertIn("edited", audit["pull_request_types"])
        self.assertTrue(evaluate_guard(
            expression, "main", action="edited",
            base_change={"ref": {"from": "codex/prepared-parent"}},
        ))
        self.assertFalse(evaluate_guard(
            expression, "codex/prepared-parent", action="edited",
            base_change={"ref": {"from": "main"}},
        ))

    def test_architecture_metadata_edits_do_not_repeat_semantic_audit(self):
        audit = workflow("architecture-semantic-audit.yml")
        self.assertIn("edited", audit["pull_request_types"])
        expression = audit["jobs"]["audit"]["if"]
        self.assertFalse(evaluate_guard(expression, "main", action="edited"))

    def test_architecture_main_and_missing_base_keep_existing_admission(self):
        audit = workflow("architecture-semantic-audit.yml")
        expression = audit["jobs"]["audit"]["if"]
        for action in ("opened", "reopened", "synchronize", "ready_for_review"):
            self.assertIn(action, audit["pull_request_types"])
            for base in ("main", None, ""):
                with self.subTest(action=action, base=base):
                    self.assertTrue(evaluate_guard(expression, base, action=action))
        # An incomplete edited event must reach the existing admission check.
        for base in (None, ""):
            with self.subTest(edited_missing_base=base):
                self.assertTrue(evaluate_guard(expression, base, action="edited"))

    def test_main_audits_execute_even_if_branch_filter_admits_stack_event(self):
        for name in ("merge-authority-audit.yml", "architecture-semantic-audit.yml"):
            expression = workflow(name)["jobs"]["audit"]["if"]
            with self.subTest(workflow=name):
                self.assertTrue(evaluate_guard(expression, "main"))
                self.assertFalse(evaluate_guard(expression, "codex/prepared-parent"))
                # Missing base metadata continues to the existing admission checks;
                # it must not bypass main qualification merely by skipping it.
                self.assertTrue(evaluate_guard(expression, None))
                self.assertTrue(evaluate_guard(expression, ""))

    def test_stack_skips_scope_and_both_always_aggregates(self):
        jobs = workflow("merge-gate.yml")["jobs"]
        for key in ("scope", "validate", "game_gate"):
            expression = jobs[key]["if"]
            with self.subTest(job=key):
                self.assertTrue(evaluate_guard(expression, "main"))
                self.assertFalse(evaluate_guard(expression, "codex/prepared-parent"))
                self.assertTrue(evaluate_guard(expression, None))
        self.assertTrue(jobs["validate"]["if"].startswith("always() &&"))
        self.assertTrue(jobs["game_gate"]["if"].startswith("always() &&"))

    def test_no_unguarded_root_can_launch_main_product_jobs_for_stack(self):
        jobs = workflow("merge-gate.yml")["jobs"]
        for key, job in jobs.items():
            if not job.get("needs"):
                with self.subTest(job=key):
                    self.assertFalse(evaluate_guard(job["if"], "codex/prepared-parent"))
            if "always()" in job.get("if", ""):
                with self.subTest(always_job=key):
                    self.assertFalse(evaluate_guard(job["if"], "codex/prepared-parent"))

    def test_queue_qualification_has_no_pr_base_guard(self):
        jobs = workflow("merge-group-gate.yml")["jobs"]
        for key, job in jobs.items():
            with self.subTest(job=key):
                self.assertNotIn("pull_request.base", job.get("if", ""))
        self.assertEqual(jobs["game_gate"]["if"], "always()")


if __name__ == "__main__":
    unittest.main()
