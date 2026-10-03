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
    return {"jobs": jobs, "pull_request_types": event_types}


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


class MainJobApplicabilityTests(unittest.TestCase):
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
