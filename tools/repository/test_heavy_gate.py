#!/usr/bin/env python3
"""Exercise the actual heavy-gate script and PostgreSQL runner expressions."""
import json
import os
from pathlib import Path
import re
import subprocess
import sys
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

    def run_gate(self, *, heavy="true", real="true", build="success", pg="success", pages=None, api_exit=0,
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
                "BUILD_RESULT": build, "POSTGRES_RESULT": pg,
                "REPOSITORY": "Oteryn/Oteryn-Game", "HEAD_SHA": "head",
                "CHECK_PAGES": json.dumps(pages or [{"check_runs": []}]),
                "API_EXIT": str(api_exit), "PR_NUMBER": "123",
                "LIVE_LABELS": "\n".join(labels), "LABELS_EXIT": str(labels_exit),
                "API_CALLS": str(api_calls),
            }, text=True, capture_output=True)
            if forbid_api:
                self.assertFalse(api_calls.exists(), "Non-eligible PR called the API")
            return result

    def test_real_run_requires_build_and_postgres_and_does_not_wait_for_node_boot(self):
        for build in ("success", "failure", "cancelled", "skipped"):
            for pg in ("success", "failure", "cancelled", "skipped"):
                with self.subTest(build=build, pg=pg):
                    result = self.run_gate(build=build, pg=pg)
                    self.assertEqual(result.returncode == 0, build == pg == "success", result.stderr)
                    if result.returncode:
                        self.assertIn(build, result.stdout)
                        self.assertIn(pg, result.stdout)
        # Node boot is opt-in on PRs (owner 2026-11-18): heavy-gate neither needs nor reads it.
        self.assertNotIn("node_boot", self.job)
        self.assertNotIn("NODE_BOOT", self.job)

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
        for fragment in ("    name: heavy-gate\n", "    needs: [build, durability_postgres]\n",
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

    def test_node_boot_shards_match_protected_main_and_the_qualification_workflow(self):
        build = indented_yaml_mapping_block(self.text, "build", 2)
        heavy_if = build.split("    if: >-\n", 1)[1].split("    runs-on:", 1)[0]
        qualification = (ROOT / ".github/workflows/node-boot-qualification.yml").read_text()
        reference_steps = indented_yaml_mapping_block(qualification, "node-boot", 2).split("    steps:\n", 1)[1]
        for filename, label, ref, needs in (
            ("pr-postgres-harness.yml", "PR heavy", "github.event.pull_request.head.sha", ""),
            ("merge-group-gate.yml", "Merge Queue", "github.event.merge_group.head_sha || github.sha", "candidate"),
        ):
            with self.subTest(workflow=filename):
                text = (ROOT / ".github/workflows" / filename).read_text()
                plan = indented_yaml_mapping_block(text, "node_boot_plan", 2)
                shard = indented_yaml_mapping_block(text, "node_boot_shard", 2)
                aggregate = indented_yaml_mapping_block(text, "node_boot", 2)
                for job in (plan, shard, aggregate):
                    self.assertIsNotNone(job)
                    self.assertIn("    runs-on: ubuntu-24.04\n", job)
                    self.assertIn("    permissions:\n      contents: read\n", job)
                    self.assertEqual(len(re.findall(r"^    if: ", job, re.MULTILINE)), 1)
                self.assertIn(f"    name: {label} / Node boot / plan shards\n", plan)
                self.assertIn(f"    name: {label} / Node boot / ${{{{ matrix.shard.name }}}}\n", shard)
                self.assertIn(f"    name: {label} / Node boot against the real Platform\n", aggregate)
                self.assertIn("matrix.py plan --set pr >", plan)
                self.assertIn("    timeout-minutes: 240\n", shard)
                self.assertIn("      fail-fast: false\n", shard)
                self.assertIn("        shard: ${{ fromJSON(needs.node_boot_plan.outputs.shards) }}\n", shard)
                if needs:
                    self.assertIn(f"    needs: {needs}\n", plan)
                    self.assertIn(f"    needs: [{needs}, node_boot_plan]\n", shard)
                    self.assertIn(f"    needs: [{needs}, node_boot_plan, node_boot_shard]\n", aggregate)
                    guard = "needs.candidate.outputs.server_qualification != 'false'"
                    for job in (plan, shard):
                        self.assertIn(f"    if: {guard}\n", job)
                    self.assertIn(f"    if: always() && {guard}\n", aggregate)
                else:
                    self.assertNotIn("    needs:", plan)
                    self.assertIn("    needs: node_boot_plan\n", shard)
                    self.assertIn("    needs: [node_boot_plan, node_boot_shard]\n", aggregate)
                    # PR node boot is opt-in through its own `node-boot` label (owner 2026-11-18).
                    node_boot_if = heavy_if.replace("'heavy-tests'", "'node-boot'")
                    self.assertEqual(heavy_if.count("'heavy-tests'"), 2)
                    for job in (plan, shard):
                        self.assertEqual(job.split("    if: >-\n", 1)[1].split("    runs-on:", 1)[0], node_boot_if)
                    self.assertEqual(aggregate.split("    if: >-\n", 1)[1].split("    runs-on:", 1)[0],
                                     "      always()\n      && " + node_boot_if.lstrip())
                # Shard steps are the qualification workflow's, pinned to this gate's candidate.
                expected = reference_steps.replace(
                    "        with:\n          persist-credentials: false\n",
                    f"        with:\n          ref: ${{{{ {ref} }}}}\n          persist-credentials: false\n", 1,
                ).replace("${{ github.sha }}", f"${{{{ {ref} }}}}")
                # The extracted block runs on to the next job's leading comment.
                actual = shard.split("    steps:\n", 1)[1].split("\n  #", 1)[0]
                self.assertEqual(actual.rstrip(), expected.rstrip())
                for job in (plan, aggregate):
                    self.assertIn(f"          ref: ${{{{ {ref} }}}}\n", job)
                    self.assertIn(f"          EXPECTED_SHA: ${{{{ {ref} }}}}\n", job)
                self.assertNotIn("github.sha }}", actual.replace(ref, ""))

    def run_aggregate(self, filename, *, plan="success", shards="success", status="PASS", scenarios=True):
        sys.path.insert(0, str(ROOT / "tools/qualification/node_boot"))
        import matrix
        aggregate = indented_yaml_mapping_block((ROOT / ".github/workflows" / filename).read_text(), "node_boot", 2)
        script = textwrap.dedent(aggregate.split("        run: |\n")[-1])
        self.assertNotIn("${{", script)
        planned = matrix.plan("pr")
        with tempfile.TemporaryDirectory() as temp:
            temp = Path(temp)
            prefix = "node-boot-1-1-"
            for item in planned:
                folder = temp / "shards" / f"{prefix}{item['name']}"
                folder.mkdir(parents=True)
                rows = [{"id": f"{item['name']}-s", "status": status, "seconds": 1.0}] if item["world"] and scenarios else []
                folder.joinpath("result.json").write_text(json.dumps({
                    "shard": item["name"], "entry": item["entry"], "exit_code": 0, "seconds": 60,
                    "result": "NODE_BOOT_PASS", "incomplete": False, "scenarios": rows}))
            env = dict(os.environ, RUNNER_TEMP=str(temp), GITHUB_STEP_SUMMARY=str(temp / "summary"),
                       PLAN_RESULT=plan, SHARD_RESULT=shards, SHARDS=json.dumps(planned), PREFIX=prefix)
            return subprocess.run(["bash", "-c", script], cwd=ROOT, env=env, capture_output=True, text=True)

    def test_node_boot_aggregate_fails_closed(self):
        for filename in ("pr-postgres-harness.yml", "merge-group-gate.yml"):
            with self.subTest(workflow=filename):
                self.assertEqual(self.run_aggregate(filename).returncode, 0)
                for plan, shards in (("skipped", "skipped"), ("failure", "skipped"), ("success", "failure"),
                                     ("success", "cancelled"), ("success", "skipped"), ("", "")):
                    self.assertNotEqual(self.run_aggregate(filename, plan=plan, shards=shards).returncode, 0,
                                        (plan, shards))
                # A World shard that ran nothing fails; FAIL and MISSING rows are reported only, as in
                # the single-machine run (per-shard NODE_BOOT_PASS stays the verdict).
                self.assertNotEqual(self.run_aggregate(filename, scenarios=False).returncode, 0)
                for status in ("FAIL", "MISSING-SERVER"):
                    result = self.run_aggregate(filename, status=status)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    self.assertIn(status, result.stdout)

    def test_node_boot_job_outlives_serial_natural_clock_and_light_observations(self):
        source = (ROOT / "apps/game-server/src/qualification/client_phases/full_e2ef7.rs").read_text()
        observation_seconds = 0
        for journey in ("clock_journey", "light_journey"):
            # The deadline bounds the statement that calls the journey; no `;` lies between them.
            deadlines = re.findall(r"qualification_deadline\(([0-9_]+)\),[^;]*?\b" + journey + r"\(", source)
            self.assertEqual(len(deadlines), 1, journey)
            observation_seconds += int(deadlines[0].replace("_", ""))
        budgets = []
        for filename, job_name in (("pr-postgres-harness.yml", "node_boot_shard"),
                                   ("merge-group-gate.yml", "node_boot_shard"),
                                   ("node-boot-qualification.yml", "node-boot")):
            with self.subTest(workflow=filename):
                job = indented_yaml_mapping_block((ROOT / ".github/workflows" / filename).read_text(), job_name, 2)
                self.assertIsNotNone(job)
                timeout = re.search(r"^    timeout-minutes: ([0-9]+)$", job, re.MULTILINE)
                self.assertIsNotNone(timeout)
                seconds = int(timeout[1]) * 60
                budgets.append(seconds)
                self.assertGreater(seconds, observation_seconds,
                                   "Node boot job expires before its serial natural observations finish")
        self.assertEqual(len(set(budgets)), 1, "Every Node boot entry point needs the same job budget")

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
