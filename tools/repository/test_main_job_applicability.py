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


def workflow(name):
    text = (ROOT / ".github/workflows" / name).read_text()
    jobs_text = indented_yaml_mapping_block(text, "jobs", 0)
    if jobs_text is None:
        raise AssertionError(f"Missing jobs in {name}")
    jobs = {}
    for key in re.findall(r"^  ([a-z_]+):$", jobs_text, re.MULTILINE):
        block = indented_yaml_mapping_block(jobs_text, key, 2)
        jobs[key] = dict(re.findall(r"^    (if|needs): (.+)$", block, re.MULTILINE))
    return {"jobs": jobs}


def evaluate_guard(expression, base):
    # Execute the actual workflow boolean expression with string/null operands.
    # The supported operators have the same semantics in JS and Actions here.
    allowed = (MAIN_GUARD, f"always() && ({MAIN_GUARD})")
    if expression not in allowed:
        raise AssertionError(f"Unexpected applicability expression: {expression!r}")
    script = (
        "const github={event:{pull_request:{base:{ref:JSON.parse(process.argv[1])}}}};"
        "const always=()=>true;process.stdout.write(JSON.stringify(Boolean("
        + expression
        + ")));"
    )
    return json.loads(subprocess.check_output(["node", "-e", script, json.dumps(base)]))


class MainJobApplicabilityTests(unittest.TestCase):
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
