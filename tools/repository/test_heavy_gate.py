#!/usr/bin/env python3
"""Exercise the actual heavy-gate script and PostgreSQL runner expressions."""
import hashlib
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

    def run_gate(self, *, heavy="true", real="true", build="success", pg="success",
                 prepare="success", empty="false", pages=None, api_exit=0,
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
                "PREPARE_RESULT": prepare, "EMPTY_SCOPE": empty,
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

    def test_heavy_gate_requires_preparation_even_when_scope_is_empty(self):
        for prepare in ("failure", "cancelled", "skipped", ""):
            with self.subTest(prepare=prepare):
                self.assertNotEqual(self.run_gate(prepare=prepare).returncode, 0)
        for empty in ("", "invalid"):
            self.assertNotEqual(self.run_gate(empty=empty).returncode, 0)
        self.assertEqual(self.run_gate(empty="true", build="skipped", pg="skipped").returncode, 0)
        self.assertNotEqual(self.run_gate(empty="true", build="skipped", pg="failure").returncode, 0)
        self.assertNotEqual(self.run_gate(empty="true", build="success", pg="skipped").returncode, 0)

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
        for fragment in ("    name: heavy-gate\n", "    needs: [prepare, build, durability_postgres]\n",
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
        build = indented_yaml_mapping_block(self.text, "prepare", 2)
        heavy_if = build.split("    if: >-\n", 1)[1].split("    runs-on:", 1)[0]
        qualification = (ROOT / ".github/workflows/node-boot-qualification.yml").read_text()
        reference_steps = indented_yaml_mapping_block(qualification, "node-boot", 2).split("    steps:\n", 1)[1]
        artifact_mode = "needs.node_boot_plan.outputs.artifact_mode"
        shared = f"{artifact_mode} == 'shared'"
        legacy = f"{artifact_mode} == 'legacy'"
        qualified = "matrix.shard.entry == 'game_platform' || matrix.shard.env.NODE_BOOT_SPELLS == '1'"
        self.assert_artifact_pin_order(indented_yaml_mapping_block(qualification, "plan", 2))
        for filename, label, ref, needs in (
            ("pr-postgres-harness.yml", "PR heavy", "github.event.pull_request.head.sha", ""),
            ("merge-group-gate.yml", "Merge Queue", "github.event.merge_group.head_sha || github.sha", "candidate"),
        ):
            with self.subTest(workflow=filename):
                text = (ROOT / ".github/workflows" / filename).read_text()
                plan = indented_yaml_mapping_block(text, "node_boot_plan", 2)
                shard = indented_yaml_mapping_block(text, "node_boot_shard", 2)
                build = indented_yaml_mapping_block(text, "node_boot_build", 2)
                aggregate = indented_yaml_mapping_block(text, "node_boot", 2)
                self.assertIn("    needs: node_boot_plan\n", build)
                self.assertIn("    uses: ./.github/workflows/node-boot-build.yml\n", build)
                self.assertIn(f"      revision: ${{{{ {ref} }}}}\n", build)
                self.assertIn("      variants: ${{ needs.node_boot_plan.outputs.build_variants }}\n", build)
                self.assert_artifact_pin_order(plan)
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
                    self.assertIn(f"    needs: [{needs}, node_boot_plan, node_boot_build]\n", shard)
                    self.assertIn(f"    needs: [{needs}, node_boot_plan, node_boot_build, node_boot_shard]\n", aggregate)
                    guard = "needs.candidate.outputs.server_qualification != 'false'"
                    for job in (plan, shard):
                        self.assertIn(f"    if: {guard}\n", job)
                    self.assertIn(f"    if: always() && {guard}\n", aggregate)
                else:
                    self.assertNotIn("    needs:", plan)
                    self.assertIn("    needs: [node_boot_plan, node_boot_build]\n", shard)
                    self.assertIn("    needs: [node_boot_plan, node_boot_build, node_boot_shard]\n", aggregate)
                    self.assertIn("      artifact_mode: ${{ steps.artifact_support.outputs.mode }}\n", plan)
                    self.assertIn(f"    if: needs.node_boot_plan.result == 'success' && {shared}\n", build)
                    # PR node boot is opt-in through its own `node-boot` label (owner 2026-11-18).
                    node_boot_if = heavy_if.replace("'heavy-tests'", "'node-boot'")
                    self.assertEqual(heavy_if.count("'heavy-tests'"), 2)
                    self.assertEqual(plan.split("    if: >-\n", 1)[1].split("    runs-on:", 1)[0], node_boot_if)
                    eligibility = shard.split("    if: >-\n", 1)[1].split("    runs-on:", 1)[0]
                    self.assertTrue(eligibility.endswith("      && " + node_boot_if.lstrip()))
                    self.assert_node_worker_eligibility(eligibility)
                    self.assertEqual(aggregate.split("    if: >-\n", 1)[1].split("    runs-on:", 1)[0],
                                     "      always()\n      && " + node_boot_if.lstrip())
                # Shard steps are the qualification workflow's, pinned to this gate's candidate.
                expected = reference_steps.replace(
                    "        with:\n          persist-credentials: false\n",
                    f"        with:\n          ref: ${{{{ {ref} }}}}\n          persist-credentials: false\n", 1,
                ).replace("${{ github.sha }}", f"${{{{ {ref} }}}}")
                if not needs:
                    expected = expected.replace(
                        f"        if: {qualified}\n", f"        if: {shared} && ({qualified})\n",
                    ).replace(
                        f"      - name: Install pinned Rust toolchain\n        if: {shared} && ({qualified})\n",
                        f"      - name: Install pinned Rust toolchain\n        if: {legacy} || {qualified}\n",
                    )
                    for marker in (
                        "        id: build_inputs\n",
                        "      - name: Download Game qualification binaries\n",
                        "      - name: Verify and unpack exact-candidate inputs\n",
                    ):
                        expected = expected.replace(marker, f"{marker}        if: {shared}\n")
                    expected = expected.replace("        if: steps.build_inputs.outputs.", f"        if: {shared} && steps.build_inputs.outputs.")
                    expected = expected.replace(
                        "          NODE_BOOT_PREBUILT_DIR: ${{ runner.temp }}/node-boot-prebuilt\n",
                        f"          NODE_BOOT_PREBUILT_DIR: ${{{{ {shared} && format('{{0}}/node-boot-prebuilt', runner.temp) || '' }}}}\n",
                    )
                # The extracted block runs on to the next job's leading comment.
                actual = shard.split("    steps:\n", 1)[1].split("\n  #", 1)[0]
                self.assertEqual(actual.rstrip(), expected.rstrip())
                for job in (plan, aggregate):
                    self.assertIn(f"          ref: ${{{{ {ref} }}}}\n", job)
                    self.assertIn(f"          EXPECTED_SHA: ${{{{ {ref} }}}}\n", job)
                self.assertNotIn("github.sha }}", actual.replace(ref, ""))

    def assert_artifact_pin_order(self, plan):
        test = plan.index("python3 tools/qualification/node_boot/test_prebuilt.py")
        self.assertEqual(plan[:test].count("| sha256sum -c -"), 2)
        self.assertLess(test, plan.index("python3 tools/qualification/node_boot/prebuilt.py plan"))

    def assert_node_worker_eligibility(self, eligibility):
        expression = eligibility.replace("labels.*.name", "labels.map(label=>label.name)")
        script = (
            "const [needs,isCancelled]=JSON.parse(process.argv[1]);"
            "const always=()=>true,cancelled=()=>isCancelled,contains=(a,b)=>a.includes(b);"
            "const github={event:{action:'synchronize',pull_request:{draft:false,user:{login:'owner'},labels:[{name:'node-boot'}]}}};"
            "process.stdout.write(JSON.stringify(" + expression + "));"
        )
        for plan, mode, build, cancelled, expected in (
            ("success", "shared", "success", False, True),
            ("success", "legacy", "skipped", False, True),
            ("success", "legacy", "success", False, False),
            ("success", "shared", "skipped", False, False),
            ("success", "shared", "failure", False, False),
            ("success", "", "skipped", False, False),
            ("failure", "legacy", "skipped", False, False),
            ("success", "legacy", "skipped", True, False),
        ):
            context = {"node_boot_plan": {"result": plan, "outputs": {"artifact_mode": mode}},
                       "node_boot_build": {"result": build}}
            output = subprocess.check_output(["node", "-e", script, json.dumps([context, cancelled])], text=True)
            self.assertEqual(json.loads(output), expected, (plan, mode, build, cancelled))

    def test_pr_node_boot_accepts_only_complete_artifact_support_or_legacy(self):
        job = indented_yaml_mapping_block(self.text, "node_boot_plan", 2)
        step = job.split("      - name: Verify qualification artifact support\n", 1)[1].split("\n      - name:", 1)[0]
        script = textwrap.dedent(step.split("        run: |\n", 1)[1])
        harness = Path("tools/qualification/node_boot/run.sh")
        helper = Path("tools/qualification/node_boot/prebuilt.py")
        workflow = Path(".github/workflows/node-boot-build.yml")
        for name, marker, has_helper, has_workflow, corrupt, expected in (
            ("legacy", False, False, False, False, "legacy"),
            ("shared", True, True, True, False, "shared"),
            ("missing helper", True, False, True, False, None),
            ("missing workflow", True, True, False, False, None),
            ("missing marker", False, True, True, False, None),
            ("workflow only", False, False, True, False, None),
            ("corrupt helper", True, True, True, True, None),
        ):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temp:
                temp = Path(temp)
                (temp / harness).parent.mkdir(parents=True)
                (temp / workflow).parent.mkdir(parents=True)
                (temp / harness).write_text("NODE_BOOT_PREBUILT_DIR\n" if marker else "cargo test\n")
                if has_helper:
                    (temp / helper).write_bytes(b"unknown helper" if corrupt else (ROOT / helper).read_bytes())
                if has_workflow:
                    (temp / workflow).write_bytes((ROOT / workflow).read_bytes())
                outputs = temp / "outputs"
                env = dict(os.environ, GITHUB_OUTPUT=str(outputs))
                result = subprocess.run(["bash", "-c", script], cwd=temp, env=env, capture_output=True, text=True)
                if expected is None:
                    self.assertNotEqual(result.returncode, 0)
                    self.assertFalse(outputs.exists())
                else:
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    self.assertEqual(outputs.read_text(), f"mode={expected}\n")

    def run_aggregate(self, filename, *, plan="success", mode="shared", build="success", shards="success", status="PASS", scenarios=True):
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
                       PLAN_RESULT=plan, ARTIFACT_MODE=mode, BUILD_RESULT=build, SHARD_RESULT=shards,
                       SHARDS=json.dumps(planned), PREFIX=prefix)
            return subprocess.run(["bash", "-c", script], cwd=ROOT, env=env, capture_output=True, text=True)

    def test_node_boot_aggregate_fails_closed(self):
        for filename in ("pr-postgres-harness.yml", "merge-group-gate.yml"):
            with self.subTest(workflow=filename):
                self.assertEqual(self.run_aggregate(filename).returncode, 0)
                for build in ("failure", "cancelled", "skipped", ""):
                    self.assertNotEqual(self.run_aggregate(filename, build=build).returncode, 0, build)
                if filename == "pr-postgres-harness.yml":
                    self.assertEqual(self.run_aggregate(filename, mode="legacy", build="skipped").returncode, 0)
                    for mode, build in (("legacy", "success"), ("legacy", "failure"), ("legacy", "cancelled"), ("", "skipped")):
                        self.assertNotEqual(self.run_aggregate(filename, mode=mode, build=build).returncode, 0, (mode, build))
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

    def test_older_postgres_head_uses_only_the_reviewed_base_planner(self):
        job = indented_yaml_mapping_block(self.text, "prepare", 2)
        step = job.split("      - name: Stage the reviewed PostgreSQL planner\n", 1)[1]
        run = textwrap.dedent(step.split("        run: |\n", 1)[1].split("\n      -", 1)[0])
        relative = Path("tools/ci/plan_pg_partitions.py")
        expected = (ROOT / relative).read_bytes()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            checkout = root / "checkout"
            checkout.mkdir()

            def git(*args):
                return subprocess.check_output(
                    ["git", "-c", "user.name=Test", "-c", "user.email=test@example.invalid", *args],
                    cwd=checkout, text=True, stderr=subprocess.DEVNULL,
                ).strip()

            git("init", "-q")
            git("commit", "--allow-empty", "-qm", "legacy head")
            legacy = git("rev-parse", "HEAD")
            planner = checkout / relative
            planner.parent.mkdir(parents=True)
            planner.write_bytes(expected)
            git("add", str(relative))
            git("commit", "-qm", "protected planner")
            base = git("rev-parse", "HEAD")
            planner.write_text("print('altered')")
            git("commit", "-qam", "tampered candidate")
            tampered = git("rev-parse", "HEAD")
            for revision, wanted in ((legacy, "false"), (base, "true"), (tampered, None)):
                with self.subTest(mode=wanted):
                    git("checkout", "--detach", "-q", revision)
                    temp = root / revision
                    temp.mkdir()
                    output = temp / "env"
                    result = subprocess.run(
                        ["bash", "-c", run], cwd=checkout, capture_output=True, text=True,
                        env=dict(os.environ, BASE_SHA=base, RUNNER_TEMP=str(temp),
                                 GITHUB_OUTPUT=str(output)),
                    )
                    self.assertEqual(git("rev-parse", "HEAD"), revision)
                    self.assertEqual(result.returncode == 0, wanted is not None, result.stdout + result.stderr)
                    if wanted is not None:
                        self.assertEqual(output.read_text().strip(), f"current_ci={wanted}")
                        self.assertEqual((temp / "pg-planner/plan_pg_partitions.py").read_bytes(), expected)

    def test_single_archive_downloads_verify_in_either_supported_layout(self):
        job = indented_yaml_mapping_block(self.text, "durability_postgres", 2)
        verification = job.split("      - name: Verify archives were built for this head\n", 1)[1]
        run = verification.split("        run: |\n", 1)[1].split("\n      #", 1)[0]
        head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
        workspace = str(ROOT)
        for nested in (False, True):
            with self.subTest(nested=nested), tempfile.TemporaryDirectory() as temporary:
                download = Path(temporary) / "pg-download"
                if nested:
                    download /= "pr-postgres-test-archives-lib"
                download.mkdir(parents=True)
                (download / "head-sha").write_text(head + "\n")
                (download / "workspace").write_text(workspace + "\n")
                archive = download / "pg-lib.tar.zst"
                archive.write_bytes(b"one library archive")
                (download / "pg-lib.json").write_text("{}")
                sums = subprocess.check_output(
                    ["sha256sum", "head-sha", "workspace", archive.name, "pg-lib.json"], cwd=download,
                )
                (download / "SHA256SUMS").write_bytes(sums)
                env = dict(
                    os.environ, EXPECTED_SHA=head, EXPECTED_ARCHIVES='["lib"]',
                    BUILD_WORKSPACE=workspace, GITHUB_WORKSPACE=workspace,
                    PG_WORKSPACE=workspace, PG_SOURCE=workspace, PG_RUN="env",
                    RUNNER_TEMP=temporary, LIB_SHA256=hashlib.sha256(sums).hexdigest(),
                )
                listing = download / "pg-lib.json"
                listing.write_text('{"altered": true}')
                corrupted = subprocess.run(
                    ["bash", "-c", textwrap.dedent(run)], env=env, text=True, capture_output=True,
                )
                self.assertNotEqual(corrupted.returncode, 0)
                listing.write_text("{}")
                result = subprocess.run(
                    ["bash", "-c", textwrap.dedent(run)], env=env, text=True, capture_output=True,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertTrue((Path(temporary) / "pg-archives" / archive.name).is_file())
                self.assertEqual((Path(temporary) / "pg-plan/pg-lib.json").read_text(), "{}")

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
                    context = [{"prepare": {"outputs": {"pg_runner_mode": mode}}}, github, {"partition": partition}]
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
