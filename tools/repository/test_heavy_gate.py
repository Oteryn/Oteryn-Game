#!/usr/bin/env python3
"""Exercise the actual heavy-gate script and PostgreSQL runner expressions."""
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import textwrap
import unittest

from validate_repository_policy_core import indented_yaml_mapping_block

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/pr-postgres-harness.yml"


class HeavyGateTests(unittest.TestCase):
    def setUp(self):
        self.text = WORKFLOW.read_text()
        self.job = indented_yaml_mapping_block(self.text, "heavy_gate", 2)

    def run_gate(self, *, heavy="true", real="true", build="success", pg="success", node_boot="success", pages=None, api_exit=0,
                 eligible="true", labels=(), labels_exit=0, forbid_api=False):
        self.assertIsNotNone(self.job)
        script = textwrap.dedent(self.job.split("        run: |\n", 1)[1])
        self.assertNotIn("${{", script)
        with tempfile.TemporaryDirectory(dir=ROOT) as directory:
            gh = Path(directory) / "gh"
            gh.write_text(
                '#!/bin/bash\nset -eu\n'
                'printf "called\\n" >> "$API_CALLS"\n'
                'if [[ "$*" == "api --paginate --slurp repos/Oteryn/Oteryn-Game/commits/head/check-runs?check_name=heavy-gate&filter=all&per_page=100" ]]; then\n'
                '  printf "%s" "$CHECK_PAGES"\nexit "$API_EXIT"\n'
                'fi\n'
                'test "$#" = 5 && test "$1" = api && test "$2" = repos/Oteryn/Oteryn-Game/issues/123/labels\n'
                'test "$3" = --paginate && test "$4" = --jq && test "$5" = ".[].name"\n'
                'printf "%s" "$LIVE_LABELS"\nexit "$LABELS_EXIT"\n'
            )
            gh.chmod(0o755)
            api_calls = Path(directory) / "api_calls"
            result = subprocess.run(["bash", "-e", "-o", "pipefail", "-c", script], env={
                **os.environ, "PATH": directory + os.pathsep + os.environ["PATH"],
                "IS_ELIGIBLE": eligible, "IS_HEAVY": heavy, "IS_REAL_RUN": real,
                "BUILD_RESULT": build, "POSTGRES_RESULT": pg, "NODE_BOOT_RESULT": node_boot,
                "REPOSITORY": "Oteryn/Oteryn-Game", "HEAD_SHA": "head",
                "CHECK_PAGES": json.dumps(pages or [{"check_runs": []}]),
                "API_EXIT": str(api_exit), "PR_NUMBER": "123",
                "LIVE_LABELS": "\n".join(labels), "LABELS_EXIT": str(labels_exit),
                "API_CALLS": str(api_calls),
            }, text=True, capture_output=True)
            if forbid_api:
                self.assertFalse(api_calls.exists(), "Non-eligible PR called the API")
            return result

    def test_real_run_requires_all_three_jobs_successful(self):
        for build in ("success", "failure", "cancelled", "skipped"):
            for pg in ("success", "failure", "cancelled", "skipped"):
                for node_boot in ("success", "failure", "cancelled", "skipped"):
                    with self.subTest(build=build, pg=pg, node_boot=node_boot):
                        result = self.run_gate(build=build, pg=pg, node_boot=node_boot)
                        self.assertEqual(result.returncode == 0, build == pg == node_boot == "success", result.stderr)
                        if result.returncode:
                            self.assertIn(build, result.stdout)
                            self.assertIn(pg, result.stdout)
                            self.assertIn(node_boot, result.stdout)

    def test_non_eligible_succeeds_without_api_or_harness(self):
        for reason in ("draft", "dependabot"):
            with self.subTest(reason=reason):
                result = self.run_gate(eligible="false", heavy="false", real="false",
                                       build="skipped", pg="skipped", api_exit=1,
                                       labels_exit=1, forbid_api=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(len(result.stdout.splitlines()), 1)

    def test_unlabelled_payload_with_live_heavy_label_fails(self):
        result = self.run_gate(heavy="false", labels=("bug", "heavy-tests", "enhancement"))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("heavy-tests was added after this event; the heavy-tests run publishes heavy-gate",
                      result.stdout)

    def test_unlabelled_payload_without_live_heavy_label_passes(self):
        for labels in ((), ("bug",), ("heavy-tests-extra",)):
            with self.subTest(labels=labels):
                result = self.run_gate(heavy="false", labels=labels)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("Not a heavy-tests PR", result.stdout)

    def test_unlabelled_payload_fails_closed_on_labels_api_error(self):
        result = self.run_gate(heavy="false", labels_exit=1)
        self.assertNotEqual(result.returncode, 0)

    def test_mirror_uses_latest_completed_across_pages(self):
        for conclusion in ("success", "failure", "cancelled", "neutral", "skipped", "timed_out"):
            with self.subTest(conclusion=conclusion):
                pages = [{"check_runs": [
                    {"id": 4, "status": "in_progress", "conclusion": None},
                    {"id": 1, "status": "completed", "completed_at": "2026-11-04T01:00:00Z", "conclusion": "success"},
                ]}, {"check_runs": [
                    {"id": 3, "status": "completed", "completed_at": "2026-11-04T02:00:00Z", "conclusion": conclusion},
                ]}]
                result = self.run_gate(real="false", build="skipped", pg="skipped", pages=pages)
                self.assertEqual(result.returncode == 0, conclusion == "success", result.stderr)

    def test_mirror_fails_closed_without_completed_check_or_on_api_error(self):
        for pages, api_exit in ((None, 0), ([{"check_runs": [{"status": "in_progress"}]}], 0), (None, 1)):
            result = self.run_gate(real="false", pages=pages, api_exit=api_exit)
            self.assertNotEqual(result.returncode, 0)
            if api_exit == 0:
                self.assertIn("still running", result.stdout)

    def test_job_contract_and_event_classification(self):
        self.assertIsNotNone(self.job)
        for fragment in ("    name: heavy-gate\n", "    needs: [build, durability_postgres, node_boot]\n",
                         "    if: always()\n", "    runs-on: ubuntu-24.04\n", "    timeout-minutes: 10\n",
                         "      checks: read\n", "      pull-requests: read\n",
                         "          PR_NUMBER: ${{ github.event.pull_request.number }}\n", "          GH_TOKEN: ${{ github.token }}\n"):
            self.assertIn(fragment, self.job)
        self.assertNotIn("contents:", self.job)
        expressions = dict(re.findall(r"^          (IS_ELIGIBLE|IS_HEAVY|IS_REAL_RUN): \$\{\{ (.*?) \}\}$", self.job, re.MULTILINE))
        for draft, author, labels, action, label, heavy, real in (
            (False, "owner", ["heavy-tests"], "synchronize", "", True, True),
            (False, "owner", ["heavy-tests"], "labeled", "heavy-tests", True, True),
            (False, "owner", ["heavy-tests"], "labeled", "bug", True, False),
            (True, "owner", ["heavy-tests"], "opened", "", False, True),
            (False, "dependabot[bot]", ["heavy-tests"], "opened", "", False, True),
            (False, "owner", [], "opened", "", False, True),
        ):
            context = {"event": {"pull_request": {"draft": draft, "user": {"login": author},
                       "labels": [{"name": name} for name in labels]}, "action": action, "label": {"name": label}}}
            for key, expected in (("IS_ELIGIBLE", not draft and author != "dependabot[bot]"),
                                  ("IS_HEAVY", heavy), ("IS_REAL_RUN", real)):
                expression = expressions[key].replace("labels.*.name", "labels.map(x=>x.name)")
                script = "const github=JSON.parse(process.argv[1]);const contains=(a,b)=>a.includes(b);process.stdout.write(JSON.stringify(" + expression + "));"
                actual = subprocess.check_output(["node", "-e", script, json.dumps(context)], text=True)
                self.assertEqual(json.loads(actual), expected)

    def test_node_boot_matches_protected_main_steps_and_build_guard(self):
        node_boot = indented_yaml_mapping_block(self.text, "node_boot", 2)
        build = indented_yaml_mapping_block(self.text, "build", 2)
        protected = indented_yaml_mapping_block(
            (ROOT / ".github/workflows/merge-group-gate.yml").read_text(), "node_boot", 2)
        self.assertIsNotNone(node_boot)
        self.assertEqual(node_boot.split("    if: >-\n", 1)[1].split("    runs-on:", 1)[0],
                         build.split("    if: >-\n", 1)[1].split("    runs-on:", 1)[0])
        self.assertNotIn("    needs:", node_boot)
        for fragment in ("    runs-on: ubuntu-24.04\n", "    timeout-minutes: 60\n",
                         "    permissions:\n      contents: read\n",
                         "    name: PR heavy / Node boot against the real Platform\n"):
            self.assertIn(fragment, node_boot)
        expected_steps = protected.split("    steps:\n", 1)[1].replace(
            "exact merge-group candidate", "exact PR head").replace(
            "exact merge-group SHA", "exact PR head SHA").replace(
            "github.event.merge_group.head_sha || github.sha", "github.event.pull_request.head.sha").replace(
            "${{ github.sha }}", "${{ github.event.pull_request.head.sha }}")
        self.assertEqual(node_boot.split("    steps:\n", 1)[1].split("\n  #", 1)[0].rstrip(),
                         expected_steps.rstrip())
        self.assertIn("          NODE_BOOT_RESULT: ${{ needs.node_boot.result }}\n", self.job)

    def test_runner_and_service_have_identical_routing(self):
        job = indented_yaml_mapping_block(self.text, "durability_postgres", 2)
        runner = re.search(r"^    runs-on: \$\{\{ (.*?) \}\}$", job, re.MULTILINE)[1]
        image = re.search(r"^        image: \$\{\{ (.*?) \}\}$", job, re.MULTILINE)[1]
        script = (
            "const [needs,github,matrix]=JSON.parse(process.argv[1]);const fromJSON=JSON.parse;"
            "process.stdout.write(JSON.stringify([" + runner + "," + image + "]));"
        )
        for mode in ("on", "split", "off", ""):
            for partition in (1, 4, 5, 12):
                for fork, bot in ((False, False), (True, False), (False, True)):
                    github = {"repository": "repo", "actor": "dependabot[bot]" if bot else "owner",
                              "event": {"pull_request": {"head": {"repo": {"full_name": "fork" if fork else "repo"}}}}}
                    context = [{"build": {"outputs": {"pg_runner_mode": mode}}}, github, {"partition": partition}]
                    output = subprocess.check_output(["node", "-e", script, json.dumps(context)], text=True)
                    actual_runner, actual_image = json.loads(output)
                    native = not fork and not bot and (mode == "on" or mode == "split" and partition <= 4)
                    self.assertEqual(isinstance(actual_runner, list), native)
                    self.assertEqual(actual_image == "", native)
                    if not native:
                        self.assertEqual(actual_runner, "ubuntu-24.04")
                        self.assertTrue(actual_image.startswith("postgres:18.6-bookworm@sha256:"))

    def test_unrelated_label_keeps_a_separate_concurrency_group(self):
        concurrency = indented_yaml_mapping_block(self.text, "concurrency", 0)
        self.assertIn("  cancel-in-progress: true\n", concurrency)
        self.assertIn(
            "  group: pr-postgres-harness-${{ github.event.pull_request.number }}"
            "${{ (github.event.action == 'labeled' && github.event.label.name != 'heavy-tests')"
            " && format('-label-{0}', github.run_id) || '' }}\n", concurrency,
        )


if __name__ == "__main__":
    unittest.main()
