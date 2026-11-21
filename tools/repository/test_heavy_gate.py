#!/usr/bin/env python3
"""Exercise the actual heavy-gate script and PostgreSQL runner expressions."""
from collections import Counter
import hashlib
import itertools
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
PARTITIONS = ROOT / ".github/workflows/postgres-test-partitions.yml"


def step_script(workflow, name):
    step = workflow.read_text().split(f"      - name: {name}\n", 1)[1]
    lines = step.split("        run: |\n", 1)[1].splitlines(keepends=True)
    script = itertools.takewhile(lambda line: not line.strip() or line.startswith("          "), lines)
    return textwrap.dedent("".join(script))


def inventory(suites):
    return {"rust-suites": {
        binary: {"binary-id": binary, "testcases": {
            name: {"filter-match": {"status": "matches"}} for name in names
        }} for binary, names in suites.items()
    }}


class PostgresLibraryArchiveTests(unittest.TestCase):
    def test_library_builds_archive_every_selected_package_in_both_modes(self):
        select = step_script(WORKFLOW, "Select PostgreSQL archive builds")
        build = step_script(PARTITIONS, "Archive PostgreSQL-backed library tests")
        cargo = (
            'cargo() { printf "%s\\n" "${OTERYN_DISPATCH_PG_SHARD-unset}" "$CARGO_BUILD_JOBS" "$@"; }\n'
        )
        dispatch = "oteryn-gameplay-dispatch"
        server = "oteryn-game-server"
        durability = "oteryn-gameplay-durability"
        for mode in ("unsharded", "legacy-shards", "canonical"):
            split = mode != "unsharded"
            for packages in ([dispatch], [durability], [server, dispatch, durability]):
                with self.subTest(mode=mode, packages=packages), tempfile.TemporaryDirectory() as directory:
                    root = Path(directory)
                    if split:
                        checker = root / "tools/repository/check_dispatch_pg_shards.py"
                        checker.parent.mkdir(parents=True)
                        checker.touch()
                    if mode == "canonical":
                        helper = root / "tools/ci/shared_dispatch_archive.py"
                        helper.parent.mkdir(parents=True)
                        helper.write_text(textwrap.dedent('''\
                            import json, os, sys
                            from pathlib import Path
                            assert sys.argv[1] == 'build'
                            directory, role = Path(sys.argv[2]), sys.argv[3]
                            assert role in ('dispatch-1', 'dispatch-2', 'dispatch-3')
                            with Path(os.environ['RUNNER_TEMP'], 'helper-calls.jsonl').open('a') as calls:
                                print(json.dumps(dict(directory=str(directory), role=role)), file=calls)
                            if os.environ.get('REJECT_CANONICAL') == '1':
                                raise SystemExit(17)
                            directory.mkdir(parents=True)
                            for name in ('unit-tests.tar.zst', 'inventory.json', 'shared-dispatch.json'):
                                (directory / name).write_text(role + ':' + name)
                            '''))
                    output = root / "outputs"
                    subprocess.run(
                        ["bash", "-c", select], cwd=root, check=True, capture_output=True, text=True,
                        env=dict(os.environ, LIB_PACKAGES=json.dumps(packages), GITHUB_OUTPUT=str(output)),
                    )
                    outputs = dict(line.split("=", 1) for line in output.read_text().splitlines())
                    library = sorted(package for package in packages if not split or package != dispatch)
                    shards = ["lib-1", "lib-2", "lib-3"] if split and dispatch in packages else []
                    library_archives = (["lib"] if library else []) + shards
                    self.assertEqual(json.loads(outputs["library"]), library)
                    self.assertEqual(json.loads(outputs["matrix"]), ["e2e", *library_archives])
                    self.assertEqual(
                        json.loads(outputs["archives"]), (["e2e"] if server in packages else []) + library_archives,
                    )
                    self.assertEqual(outputs["multiple"], str(len(json.loads(outputs["archives"])) > 1).lower())
                    for archive in library_archives:
                        env = dict(
                            os.environ, RUNNER_TEMP=directory, ARCHIVE=archive, LIB_PACKAGES=outputs["library"],
                            OTERYN_DISPATCH_PG_SHARD="2/3", CARGO_BUILD_JOBS="2",
                        )
                        if mode == "canonical" and archive != "lib":
                            env.pop("OTERYN_DISPATCH_PG_SHARD")
                        actual = subprocess.check_output(["bash", "-c", cargo + build], cwd=root, env=env, text=True)
                        if mode == "canonical" and archive != "lib":
                            role = "dispatch-" + archive[-1]
                            self.assertEqual(actual, "", "canonical archives must not invoke the legacy Cargo path")
                            call = json.loads((root / "helper-calls.jsonl").read_text().splitlines()[-1])
                            self.assertEqual(call, {"directory": str(root / "shared-dispatch-local" / role), "role": role})
                            for source, target in (("unit-tests.tar.zst", f"pg-{archive}.tar.zst"),
                                                   ("inventory.json", f"pg-{archive}.json"),
                                                   ("shared-dispatch.json", f"shared-dispatch-{archive}.json")):
                                self.assertEqual((root / "pg-archives" / target).read_text(), role + ":" + source)
                            rejected = subprocess.run(["bash", "-c", cargo + build], cwd=root,
                                                      env=dict(env, REJECT_CANONICAL="1"), capture_output=True, text=True)
                            self.assertEqual(rejected.returncode, 17, "a rejected canonical build must not fall back")
                            continue
                        if archive == "lib":
                            shard = "unset"
                            jobs = "1" if dispatch in library else "2"
                            selected = library
                        else:
                            shard = f"{archive[-1]}/3"
                            jobs = "2"
                            selected = [dispatch]
                        actual_shard, actual_jobs, *args = actual.splitlines()
                        self.assertEqual((actual_shard, actual_jobs), (shard, jobs))
                        self.assertEqual(args[:3], ["+1.100.0", "nextest", "archive"])
                        self.assertEqual([args[i + 1] for i, arg in enumerate(args) if arg == "-p"], selected)
                        for flag in ("--locked", "--lib", "--config", "--archive-file"):
                            self.assertEqual(args.count(flag), 1)
                        self.assertEqual(args[args.index("--config") + 1], "profile.dev.package.oteryn-gameplay-dispatch.debug=0")
                        self.assertEqual(args[args.index("--archive-file") + 1], f"{directory}/pg-archives/pg-{archive}.tar.zst")

    def worker_outputs(self, archives, calibrated=True):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            planner_dir = root / "pg-planner"
            planner_dir.mkdir()
            (planner_dir / "pg_archive_affinity.py").write_bytes(
                (ROOT / "tools/ci/pg_archive_affinity.py").read_bytes()
            )
            output = root / "outputs"
            subprocess.run(
                ["bash", "-c", step_script(WORKFLOW, "Size PostgreSQL partitions for the selected archives")],
                cwd=root, check=True, env=dict(os.environ, RUNNER_TEMP=directory,
                    GITHUB_OUTPUT=str(output), ARCHIVES=json.dumps(archives),
                    BUILDS=json.dumps(["e2e", *(name for name in archives if name != "e2e")]),
                    CALIBRATED=str(calibrated).lower()),
            )
            return {name: json.loads(value) for name, value in
                    (line.split("=", 1) for line in output.read_text().splitlines())}

    def worker_matrix(self, archives, calibrated=True):
        return self.worker_outputs(archives, calibrated)["matrix"]["include"]

    def test_archive_lanes_keep_global_labels_and_e2e_custody(self):
        complete = ["e2e", "lib", "lib-1", "lib-2", "lib-3"]
        for archives in (complete, complete[1:], ["lib-1", "lib-2", "lib-3"]):
            with self.subTest(archives=archives):
                matrices = self.worker_outputs(archives)
                lanes = matrices["archive_lanes"]["include"]
                self.assertEqual([lane["archive"] for lane in lanes],
                                 ["e2e", *(name for name in archives if name != "e2e")])
                for lane in lanes:
                    archive = lane["archive"]
                    self.assertEqual(lane["selected"], archive in archives)
                    self.assertEqual(lane["partitions"]["include"],
                                     [row for row in matrices["matrix"]["include"] if row["archive"] == archive])

    def test_each_selected_archive_keeps_every_calibrated_partition_once(self):
        counts = {"lib-1": 15, "lib-2": 7, "lib-3": 7, "e2e": 5, "lib": 2}
        for size in range(1, len(counts) + 1):
            for archives in itertools.combinations(counts, size):
                with self.subTest(archives=archives):
                    rows = self.worker_matrix(archives)
                    self.assertEqual([row["partition"] for row in rows], list(range(1, len(rows) + 1)))
                    self.assertCountEqual(
                        [(row["archive"], row["archive_index"], row["archive_total"]) for row in rows],
                        [(name, i, counts[name]) for name in archives for i in range(counts[name])],
                    )

    def test_every_library_test_is_planned_once_in_both_archive_modes(self):
        partition = indented_yaml_mapping_block(PARTITIONS.read_text(), "durability_postgres", 2)
        self.assertIn("matrix: ${{ fromJSON(inputs.partitions) }}", partition)
        legacy = self.worker_matrix(["lib"], calibrated=False)
        self.assertEqual(legacy, [{"partition": i} for i in range(1, 37)])
        total = len(legacy)
        script = step_script(PARTITIONS, "Plan partition")
        dispatch = "oteryn-gameplay-dispatch"
        mounted_path = "gameplay_transport::monk_save::postgres_tests::"
        ordinary = "unit::smoke"
        explicit = mounted_path + "explicit"
        mounted = [f"{mounted_path}mounted::case_{index}" for index in range(total)]
        others = {"oteryn-gameplay-durability": ["postgres::durability"], "oteryn-game-server": ["postgres::server"]}
        combined = {**others, dispatch: [ordinary, explicit, *mounted]}
        expected = Counter(f"--filterset=binary_id(={binary}) & test(={name})"
                           for binary, names in combined.items() for name in names)

        for split in (False, True):
            with self.subTest(split=split), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                plan = root / "pg-plan"
                plan.mkdir()
                planner = root / "pg-planner/plan_pg_partitions.py"
                planner.parent.mkdir()
                planner.write_bytes((ROOT / "tools/ci/plan_pg_partitions.py").read_bytes())
                inventories = {"pg-lib": others if split else combined}
                if split:
                    for shard in range(3):
                        # Unsharded unit tests are compiled in every dispatch archive and assigned once.
                        dispatch_names = [ordinary, *mounted[shard::3]]
                        if shard == 0:
                            dispatch_names.append(explicit)
                        inventories[f"pg-lib-{shard + 1}"] = {dispatch: dispatch_names}
                for archive, suites in inventories.items():
                    (plan / f"{archive}.json").write_text(json.dumps(inventory(suites)))
                actual = Counter()
                for index in range(total):
                    env = dict(os.environ, RUNNER_TEMP=directory, LABEL=str(index + 1),
                               JOB_INDEX=str(index), JOB_TOTAL=str(total), PG_AFFINITY="false")
                    subprocess.run(["bash", "-c", script], cwd=ROOT, env=env, check=True,
                                   capture_output=True, text=True)
                    for filters in plan.glob("*.filters"):
                        actual.update(filters.read_text().splitlines())
                self.assertEqual(actual, expected)

    def test_affine_workers_plan_disjoint_local_shares_with_global_labels(self):
        dispatch = "oteryn-gameplay-dispatch"
        mount = "gameplay_transport::monk_save::postgres_tests::"
        common = "unit::shared"
        lanes = self.worker_outputs(["lib-1", "lib-2", "lib-3"])["archive_lanes"]["include"]
        script = step_script(PARTITIONS, "Plan partition")
        actual = Counter()
        expected = Counter()
        for lane in lanes:
            if not lane["selected"]:
                continue
            archive = lane["archive"]
            tests = [f"{mount}{archive.replace('-', '_')}::{index}" for index in range(20)]
            expected.update(f"--filterset=binary_id(={dispatch}) & test(={test})"
                            for test in tests + ([common] if archive == "lib-1" else []))
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                plan = root / "pg-plan"
                plan.mkdir()
                planner = root / "pg-planner/plan_pg_partitions.py"
                planner.parent.mkdir()
                planner.write_bytes((ROOT / "tools/ci/plan_pg_partitions.py").read_bytes())
                (plan / f"pg-{archive}.json").write_text(json.dumps(inventory({dispatch: [common, *tests]})))
                rows = lane["partitions"]["include"]
                for index, row in enumerate(rows):
                    env = dict(os.environ, RUNNER_TEMP=directory, LABEL=str(row["partition"]),
                               JOB_INDEX=str(index), JOB_TOTAL=str(len(rows)), PG_AFFINITY="true",
                               ARCHIVE_INDEX=str(row["archive_index"]), ARCHIVE_TOTAL=str(row["archive_total"]))
                    subprocess.run(["bash", "-c", script], cwd=ROOT, env=env, check=True,
                                   capture_output=True, text=True)
                    for filters in plan.glob("*.filters"):
                        actual.update(filters.read_text().splitlines())
                for key, invalid in (("ARCHIVE_INDEX", "0"), ("ARCHIVE_TOTAL", "99")):
                    result = subprocess.run(["bash", "-c", script], cwd=ROOT, env=env | {key: invalid},
                                            capture_output=True, text=True)
                    self.assertNotEqual(result.returncode, 0)
        self.assertEqual(actual, expected)

    def test_archive_call_has_only_its_own_build_dependency_and_artifacts(self):
        caller = indented_yaml_mapping_block(WORKFLOW.read_text(), "archive_lanes", 2)
        for fragment in ('needs: prepare', 'uses: ./.github/workflows/postgres-test-partitions.yml',
                         'matrix: ${{ fromJSON(needs.prepare.outputs.archive_lanes) }}',
                         '''builds: ${{ format('["{0}"]', matrix.archive) }}''',
                         'partitions: ${{ toJSON(matrix.partitions) }}',
                         'run_partitions: ${{ matrix.selected }}'):
            self.assertIn(fragment, caller)
        text = PARTITIONS.read_text()
        build = indented_yaml_mapping_block(text, "build", 2)
        self.assertIn("if: inputs.affinity || matrix.archive == 'e2e'", build)
        partition = indented_yaml_mapping_block(text, "durability_postgres", 2)
        self.assertIn("    needs: build\n", partition)
        self.assertIn("    if: inputs.run_partitions\n", partition)
        self.assertIn("name: pr-postgres-test-inventory-${{ matrix.archive }}", partition)
        self.assertIn("name: pr-postgres-test-archives-${{ matrix.archive }}", partition)
        coverage = indented_yaml_mapping_block(WORKFLOW.read_text(), "heavy_gate", 2)
        for archive in ("e2e", "lib", "lib_1", "lib_2", "lib_3"):
            self.assertIn(f"value: ${{{{ jobs.durability_postgres.outputs.{archive}_sha256 }}}}", text)
            self.assertIn(f"{archive}_sha256: ${{{{ steps.receipt.outputs.{archive}_sha256 }}}}", partition)
            self.assertIn(f"needs.archive_lanes.outputs.{archive}_sha256", coverage)
        self.assertEqual(coverage.count("if: steps.harness.outputs.coverage == 'true'"), 3)

    def test_worker_receipt_is_success_only_and_routes_one_archive(self):
        text = PARTITIONS.read_text()
        cleanup = "      - name: Stop ephemeral PostgreSQL 18\n        if: always() && runner.environment == 'self-hosted'\n"
        marker = "      - name: Record verified PostgreSQL archive receipt\n"
        self.assertLess(text.index(cleanup), text.index(marker))
        receipt = text.split(marker, 1)[1]
        condition = re.search(r"^        if: (.+)$", receipt, re.MULTILINE)[1]
        validator = ("const [affinity,status]=JSON.parse(process.argv[1]);const inputs={affinity};"
                     "const success=()=>status==='success';process.stdout.write(JSON.stringify(" + condition + "));")
        for affinity, status in itertools.product((False, True), ("success", "failure", "cancelled", "skipped")):
            with self.subTest(affinity=affinity, status=status):
                result = subprocess.check_output(["node", "-e", validator, json.dumps((affinity, status))], text=True)
                self.assertEqual(json.loads(result), affinity and status == "success")
        self.assertIn("ARCHIVE_DIGEST: ${{ steps.verify_archive.outputs.digest }}", receipt)
        script = step_script(PARTITIONS, "Record verified PostgreSQL archive receipt")
        digest = "0123456789abcdef" * 4
        for archive in ("e2e", "lib", "lib-1", "lib-2", "lib-3"):
            with self.subTest(archive=archive), tempfile.TemporaryDirectory() as directory:
                output = Path(directory) / "output"
                env = dict(os.environ, SELECTED_ARCHIVE=archive, ARCHIVE_DIGEST=digest, GITHUB_OUTPUT=str(output))
                subprocess.run(["bash", "-c", script], env=env, check=True, capture_output=True, text=True)
                self.assertEqual(output.read_text(), f"{archive.replace('-', '_')}_sha256={digest}\n")
        for archive, value in (("lib-4", digest), ("../lib-2", digest), ("lib-2", ""),
                               ("lib-2", "z" * 64), ("lib-2", digest + "0")):
            with self.subTest(archive=archive, value=value), tempfile.TemporaryDirectory() as directory:
                output = Path(directory) / "output"
                env = dict(os.environ, SELECTED_ARCHIVE=archive, ARCHIVE_DIGEST=value, GITHUB_OUTPUT=str(output))
                result = subprocess.run(["bash", "-c", script], env=env, capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(output.exists())

    def test_worker_verifier_exports_only_a_successfully_verified_manifest_digest(self):
        verify = step_script(PARTITIONS, "Verify the selected PostgreSQL test archive and inventory")
        head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
        workspace = str(ROOT)
        with tempfile.TemporaryDirectory() as directory:
            temp = Path(directory)
            planner = temp / "pg-planner"
            planner.mkdir()
            for helper in ("plan_pg_partitions.py", "pg_archive_affinity.py"):
                (planner / helper).write_bytes((ROOT / "tools/ci" / helper).read_bytes())
            selected = temp / "pg-selected"
            selected.mkdir()
            inv = temp / "pg-inventories"
            inv.mkdir()
            payload, listing = "pg-lib-2.tar.zst", "pg-lib-2.json"
            (selected / "head-sha").write_text(head + "\n")
            (selected / "workspace").write_text(workspace + "\n")
            (selected / payload).write_bytes(b"one library archive")
            cases = inventory({"dispatch": ["gameplay_transport::monk_save::postgres_tests::case"]})
            (selected / listing).write_text(json.dumps(cases))
            names = ("head-sha", "workspace", payload, listing)
            sums = subprocess.check_output(["sha256sum", *names], cwd=selected)
            (selected / "SHA256SUMS").write_bytes(sums)
            for name in ("head-sha", "workspace", listing, "SHA256SUMS"):
                (inv / name).write_bytes((selected / name).read_bytes())
            digest = hashlib.sha256(sums).hexdigest()
            output = temp / "verify-output"
            env = dict(os.environ, RUNNER_TEMP=directory, BUILD_WORKSPACE=workspace, EXPECTED_SHA=head,
                       EXPECTED_ARCHIVES='["lib-2"]', SELECTED_ARCHIVE="lib-2", GITHUB_WORKSPACE=workspace,
                       PG_WORKSPACE=workspace, PG_SOURCE=workspace, PG_RUN="env", GITHUB_OUTPUT=str(output))
            rejected = subprocess.run(["bash", "-c", verify], env=env | {"LIB_2_SHA256": "0" * 64},
                                      capture_output=True, text=True)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("archive digest mismatch", rejected.stderr)
            self.assertFalse(output.exists())
            subprocess.run(["bash", "-c", verify], env=env | {"LIB_2_SHA256": digest},
                           check=True, capture_output=True, text=True)
            self.assertEqual(output.read_text(), f"digest={digest}\n")

    def test_final_coverage_rejects_incomplete_workers_and_inconsistent_inventories(self):
        archives = ["lib-1", "lib-2", "lib-3"]
        coverage = step_script(WORKFLOW, "Require the selected archive inventories and every partition")
        lanes = self.worker_outputs(archives)["archive_lanes"]
        head, workspace = "1" * 40, "/home/runner/work/Oteryn-Game/Oteryn-Game"
        failures = {"missing lane": "expected partition matrix", "missing worker": "expected partition matrix",
                    "duplicate worker": "expected partition matrix", "wrong label": "expected partition matrix",
                    "missing inventory": "missing or unexpected", "extra inventory": "missing or unexpected",
                    "duplicate test": "duplicate test cases", "wrong head": "head mismatch",
                    "wrong workspace": "workspace mismatch", "missing digest": "archive digest mismatch",
                    "modified listing": "bad checksum"}
        for mutation, error in {"valid": None, **failures}.items():
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                planner = root / "pg-planner"
                planner.mkdir()
                for helper in ("plan_pg_partitions.py", "pg_archive_affinity.py"):
                    (planner / helper).write_bytes((ROOT / "tools/ci" / helper).read_bytes())
                matrix = json.loads(json.dumps(lanes))
                rows = matrix["include"][1]["partitions"]["include"]
                if mutation == "missing lane":
                    matrix["include"].pop()
                elif mutation == "missing worker":
                    rows.pop()
                elif mutation == "duplicate worker":
                    rows.append(rows[0])
                elif mutation == "wrong label":
                    rows[0]["partition"] = 99
                env = dict(os.environ, RUNNER_TEMP=directory, GITHUB_WORKSPACE=workspace, EXPECTED_SHA=head,
                           ARCHIVES=json.dumps(archives), ARCHIVE_LANES=json.dumps(matrix))
                downloads = root / "pg-inventories"
                for name in archives:
                    if mutation == "missing inventory" and name == "lib-1":
                        continue
                    bundle = downloads / f"pr-postgres-test-inventory-{name}"
                    bundle.mkdir(parents=True)
                    payload, listing = f"pg-{name}.tar.zst", f"pg-{name}.json"
                    (bundle / "head-sha").write_text(("2" * 40 if mutation == "wrong head" else head) + "\n")
                    (bundle / "workspace").write_text(("/wrong" if mutation == "wrong workspace" else workspace) + "\n")
                    (bundle / payload).touch()
                    case = "common" if mutation == "duplicate test" else name.replace("-", "_")
                    mounted = f"gameplay_transport::monk_save::postgres_tests::{case}"
                    (bundle / listing).write_text(json.dumps(inventory({"dispatch": ["unit::shared", mounted]})))
                    sums = subprocess.check_output(["sha256sum", "head-sha", "workspace", payload, listing], cwd=bundle)
                    (bundle / "SHA256SUMS").write_bytes(sums)
                    (bundle / payload).unlink()
                    if mutation == "modified listing":
                        (bundle / listing).write_text("{}")
                    env[f"{name.upper().replace('-', '_')}_SHA256"] = (
                        "" if mutation == "missing digest" else hashlib.sha256(sums).hexdigest())
                if mutation == "extra inventory":
                    (downloads / "pr-postgres-test-inventory-e2e").mkdir()
                result = subprocess.run(["bash", "-c", coverage], env=env, capture_output=True, text=True)
                if error:
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(error, result.stderr)
                else:
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertIn("4 PostgreSQL tests across 29 partitions", result.stdout)


class HeavyGateTests(unittest.TestCase):
    def setUp(self):
        self.text = WORKFLOW.read_text()
        self.job = indented_yaml_mapping_block(self.text, "heavy_gate", 2)

    def run_gate(self, *, heavy="true", real="true", archive="success", legacy="skipped",
                 prepare="success", empty="false", affinity="true", pages=None, api_exit=0,
                 eligible="true", labels=(), labels_exit=0, forbid_api=False):
        self.assertIsNotNone(self.job)
        script = step_script(WORKFLOW, "Require the heavy PostgreSQL harness")
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
            output = Path(directory) / "output"
            result = subprocess.run(["bash", "-e", "-o", "pipefail", "-c", script], env={
                **os.environ, "PATH": directory + os.pathsep + os.environ["PATH"],
                "IS_ELIGIBLE": eligible, "IS_HEAVY": heavy, "IS_REAL_RUN": real,
                "PREPARE_RESULT": prepare, "EMPTY_SCOPE": empty,
                "ARCHIVE_RESULT": archive, "LEGACY_RESULT": legacy, "PG_AFFINITY": affinity,
                "REPOSITORY": "Oteryn/Oteryn-Game", "HEAD_SHA": "head",
                "CHECK_PAGES": json.dumps(pages or [{"check_runs": []}]),
                "API_EXIT": str(api_exit), "PR_NUMBER": "123",
                "LIVE_LABELS": "\n".join(labels), "LABELS_EXIT": str(labels_exit),
                "API_CALLS": str(api_calls), "GITHUB_OUTPUT": str(output),
            }, text=True, capture_output=True)
            if forbid_api:
                self.assertFalse(api_calls.exists(), "Non-eligible PR called the API")
            should_verify = (result.returncode == 0 and eligible == heavy == real == affinity == "true"
                             and empty == "false")
            self.assertEqual(output.exists(), should_verify)
            if should_verify:
                self.assertEqual(output.read_text(), "coverage=true\n")
            return result

    def test_real_run_requires_active_path_and_does_not_wait_for_node_boot(self):
        statuses = ("success", "failure", "cancelled", "skipped")
        for affinity, archive, legacy in itertools.product(("true", "false", ""), statuses, statuses):
            with self.subTest(affinity=affinity, archive=archive, legacy=legacy):
                result = self.run_gate(affinity=affinity, archive=archive, legacy=legacy)
                self.assertEqual(result.returncode == 0, (affinity, archive, legacy) in
                                 (("true", "success", "skipped"), ("false", "skipped", "success")), result.stderr)
                if result.returncode:
                    self.assertIn(archive, result.stdout)
                    self.assertIn(legacy, result.stdout)
        # Node boot is opt-in on PRs (owner 2026-11-18): heavy-gate neither needs nor reads it.
        self.assertNotIn("node_boot", self.job)
        self.assertNotIn("NODE_BOOT", self.job)

    def test_heavy_gate_requires_preparation_even_when_scope_is_empty(self):
        for prepare in ("failure", "cancelled", "skipped", ""):
            with self.subTest(prepare=prepare):
                self.assertNotEqual(self.run_gate(prepare=prepare).returncode, 0)
        for empty in ("", "invalid"):
            self.assertNotEqual(self.run_gate(empty=empty).returncode, 0)
        self.assertEqual(self.run_gate(empty="true", affinity="false", archive="skipped").returncode, 0)
        self.assertNotEqual(self.run_gate(empty="true", affinity="false", archive="failure").returncode, 0)
        self.assertNotEqual(self.run_gate(empty="true", affinity="false", archive="success").returncode, 0)

    def test_non_eligible_succeeds_without_api_or_harness(self):
        for reason in ("draft", "dependabot"):
            with self.subTest(reason=reason):
                result = self.run_gate(eligible="false", heavy="false", real="false",
                                       archive="skipped", legacy="skipped", api_exit=1,
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
                result = self.run_gate(real="false", archive="skipped", legacy="skipped", pages=pages)
                self.assertEqual(result.returncode == 0, conclusion == "success", result.stderr)

    def test_mirror_fails_closed_without_completed_check_or_on_api_error(self):
        for pages, api_exit in ((None, 0), ([{"check_runs": [{"status": "in_progress"}]}], 0), (None, 1)):
            result = self.run_gate(real="false", pages=pages, api_exit=api_exit)
            self.assertNotEqual(result.returncode, 0)
            if api_exit == 0:
                self.assertIn("still running", result.stdout)

    def test_job_contract_and_event_classification(self):
        self.assertIsNotNone(self.job)
        for fragment in ("    name: heavy-gate\n", "    needs: [prepare, archive_lanes, legacy_harness]\n",
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
                variants = "parallel_variants" if needs else "build_variants"
                self.assertIn(f"      variants: ${{{{ needs.node_boot_plan.outputs.{variants} }}}}\n", build)
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
                    dependencies = [needs, "node_boot_plan", "node_boot_build", "node_boot_default_build",
                                    "node_boot_platform_build", "world_bundle"]
                    for job, required in ((shard, dependencies), (aggregate, [*dependencies, "node_boot_shard"])):
                        actual_needs = re.search(r"^    needs: \[([^\]]+)\]$", job, re.M)[1].split(", ")
                        self.assertCountEqual(actual_needs, required)
                    guard = "needs.candidate.outputs.server_qualification != 'false'"
                    self.assertIn(f"    if: {guard}\n", plan)
                    self.assert_main_node_worker_eligibility(shard.split("    if: >-\n", 1)[1].split("    runs-on:", 1)[0])
                    self.assertIn(f"    if: always() && {guard}\n", aggregate)
                else:
                    self.assertNotIn("    needs:", plan)
                    self.assertIn("    needs: [node_boot_plan, node_boot_build, node_boot_platform_build]\n", shard)
                    self.assertIn("    needs: [node_boot_plan, node_boot_build, node_boot_platform_build, node_boot_shard]\n", aggregate)
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
                        "      - name: Download the shared Platform binary\n",
                        "      - name: Verify and unpack exact-candidate inputs\n",
                    ):
                        expected = expected.replace(marker, f"{marker}        if: {shared}\n")
                    expected = expected.replace("        if: steps.build_inputs.outputs.", f"        if: {shared} && steps.build_inputs.outputs.")
                    for variable in ("NODE_BOOT_PREBUILT_DIR", "GAME_PLATFORM_PREBUILT_DIR"):
                        expected = expected.replace(
                            f"          {variable}: ${{{{ runner.temp }}}}/node-boot-prebuilt\n",
                            f"          {variable}: ${{{{ {shared} && format('{{0}}/node-boot-prebuilt', runner.temp) || '' }}}}\n",
                        )
                else:
                    expected = expected.replace(
                        "          name: node-boot-build-world-compiler\n",
                        "          artifact-ids: ${{ needs.world_bundle.outputs.world_compiler_artifact_id }}\n",
                    ).replace(
                        "          MAIN_SHA256: ${{ needs.node_boot_build.outputs[steps.build_inputs.outputs.main_variant] }}\n",
                        "          MAIN_SHA256: ${{ steps.build_inputs.outputs.main_variant == 'default' && needs.node_boot_default_build.outputs.default || needs.node_boot_build.outputs[steps.build_inputs.outputs.main_variant] }}\n",
                    ).replace(
                        "          WORLD_SHA256: ${{ needs.node_boot_build.outputs.world-compiler }}\n",
                        "          WORLD_SHA256: ${{ needs.world_bundle.outputs.world_compiler_sha256 }}\n",
                    )
                # The extracted block runs on to the next job's leading comment.
                actual = shard.split("    steps:\n", 1)[1].split("\n  #", 1)[0]
                if needs:
                    marker = "      - name: Verify the shared World compiler producer identity\n"
                    before, remainder = actual.split(marker, 1)
                    guard_step, after = remainder.split("\n      - name:", 1)
                    self.assert_world_compiler_identity_guard(guard_step)
                    actual = before + "      - name:" + after
                self.assertEqual(actual.rstrip(), expected.rstrip())
                for job in (plan, aggregate):
                    self.assertIn(f"          ref: ${{{{ {ref} }}}}\n", job)
                    self.assertIn(f"          EXPECTED_SHA: ${{{{ {ref} }}}}\n", job)
                self.assertNotIn("github.sha }}", actual.replace(ref, ""))

    def assert_artifact_pin_order(self, plan):
        test = plan.index("python3 tools/qualification/node_boot/test_prebuilt.py")
        self.assertEqual(plan[:test].count("| sha256sum -c -"), 2)
        self.assertLess(test, plan.index("python3 tools/qualification/node_boot/prebuilt.py plan"))

    def assert_world_compiler_identity_guard(self, step):
        self.assertIn("        if: steps.build_inputs.outputs.needs_world == 'true'\n", step)
        self.assertIn("COMPILER_ARTIFACT_ID: ${{ needs.world_bundle.outputs.world_compiler_artifact_id }}", step)
        self.assertIn("COMPILER_PRODUCER_ATTEMPT: ${{ needs.world_bundle.outputs.world_compiler_producer_attempt }}", step)
        script = textwrap.dedent(step.split("        run: |\n", 1)[1])
        for artifact, producer, consumer, valid in (
            ("201", "1", "2", True), ("201", "2", "2", True),
            ("201", "3", "2", False), ("201", "0", "2", False),
            ("201", "", "2", False), ("invalid", "1", "2", False), ("0", "1", "2", False),
        ):
            with tempfile.TemporaryDirectory() as temporary:
                env = dict(os.environ, COMPILER_ARTIFACT_ID=artifact, COMPILER_PRODUCER_ATTEMPT=producer,
                           GITHUB_RUN_ATTEMPT=consumer, GITHUB_STEP_SUMMARY=str(Path(temporary) / "summary"))
                result = subprocess.run(["bash", "-c", script], env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, valid, (artifact, producer, consumer, result.stderr))

    def assert_node_worker_eligibility(self, eligibility):
        expression = eligibility.replace("labels.*.name", "labels.map(label=>label.name)")
        script = (
            "const [needs,isCancelled]=JSON.parse(process.argv[1]);"
            "const always=()=>true,cancelled=()=>isCancelled,contains=(a,b)=>a.includes(b);"
            "const github={event:{action:'synchronize',pull_request:{draft:false,user:{login:'owner'},labels:[{name:'node-boot'}]}}};"
            "process.stdout.write(JSON.stringify(" + expression + "));"
        )
        for plan, mode, build, platform, cancelled, expected in (
            ("success", "shared", "success", "success", False, True),
            ("success", "legacy", "skipped", "skipped", False, True),
            ("success", "legacy", "success", "skipped", False, False),
            ("success", "shared", "skipped", "success", False, False),
            ("success", "shared", "failure", "success", False, False),
            ("success", "", "skipped", "skipped", False, False),
            ("failure", "legacy", "skipped", "skipped", False, False),
            ("success", "legacy", "skipped", "skipped", True, False),
            *(("success", "shared", "success", status, False, False) for status in ("failure", "cancelled", "skipped", "")),
            *(("success", "legacy", "skipped", status, False, False) for status in ("success", "failure", "cancelled", "")),
        ):
            context = {"node_boot_plan": {"result": plan, "outputs": {"artifact_mode": mode}},
                       "node_boot_build": {"result": build}, "node_boot_platform_build": {"result": platform}}
            output = subprocess.check_output(["node", "-e", script, json.dumps([context, cancelled])], text=True)
            self.assertEqual(json.loads(output), expected, (plan, mode, build, platform, cancelled))

    def assert_main_node_worker_eligibility(self, expression):
        script = (
            "const [needs,isCancelled]=JSON.parse(process.argv[1]);"
            "const cancelled=()=>isCancelled,contains=(a,b)=>a.includes(b),fromJSON=JSON.parse;"
            "process.stdout.write(JSON.stringify(" + expression + "));"
        )
        baseline = {"candidate": {"outputs": {"server_qualification": "true"}},
                    "node_boot_plan": {"result": "success", "outputs": {"build_variants": '[]'}},
                    "node_boot_build": {"result": "success"}, "node_boot_platform_build": {"result": "success"},
                    "world_bundle": {"result": "success"}, "node_boot_default_build": {"result": "skipped"}}
        cases = [(baseline, False, True), (baseline, True, False)]
        for dependency in ("node_boot_plan", "node_boot_build", "node_boot_platform_build", "world_bundle"):
            for status in ("failure", "cancelled", "skipped", ""):
                changed = json.loads(json.dumps(baseline))
                changed[dependency]["result"] = status
                cases.append((changed, False, False))
        for required in (False, True):
            for status in ("success", "failure", "cancelled", "skipped", ""):
                changed = json.loads(json.dumps(baseline))
                changed["node_boot_plan"]["outputs"]["build_variants"] = '["default"]' if required else '[]'
                changed["node_boot_default_build"]["result"] = status
                cases.append((changed, False, status == "success" or (not required and status == "skipped")))
        changed = json.loads(json.dumps(baseline))
        changed["candidate"]["outputs"]["server_qualification"] = "false"
        cases.append((changed, False, False))
        for needs, cancelled, expected in cases:
            output = subprocess.check_output(["node", "-e", script, json.dumps([needs, cancelled])], text=True)
            self.assertEqual(json.loads(output), expected, (needs, cancelled))

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

    def run_aggregate(self, filename, *, plan="success", mode="shared", build="success", shards="success",
                      status="PASS", scenarios=True, platform=None, world="success", default_required=False,
                      default="skipped"):
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
                       PLATFORM_RESULT=("skipped" if mode == "legacy" else "success") if platform is None else platform,
                       WORLD_RESULT=world, DEFAULT_REQUIRED=str(default_required).lower(), DEFAULT_RESULT=default,
                       SHARDS=json.dumps(planned), PREFIX=prefix)
            return subprocess.run(["bash", "-c", script], cwd=ROOT, env=env, capture_output=True, text=True)

    def test_node_boot_aggregate_fails_closed(self):
        for filename in ("pr-postgres-harness.yml", "merge-group-gate.yml"):
            with self.subTest(workflow=filename):
                self.assertEqual(self.run_aggregate(filename).returncode, 0)
                for build in ("failure", "cancelled", "skipped", ""):
                    self.assertNotEqual(self.run_aggregate(filename, build=build).returncode, 0, build)
                for platform in ("failure", "cancelled", "skipped", ""):
                    self.assertNotEqual(self.run_aggregate(filename, platform=platform).returncode, 0, platform)
                if filename == "pr-postgres-harness.yml":
                    self.assertEqual(self.run_aggregate(filename, mode="legacy", build="skipped").returncode, 0)
                    for platform in ("success", "failure", "cancelled", ""):
                        self.assertNotEqual(self.run_aggregate(filename, mode="legacy", build="skipped",
                                                               platform=platform).returncode, 0, platform)
                    for mode, build in (("legacy", "success"), ("legacy", "failure"), ("legacy", "cancelled"), ("", "skipped")):
                        self.assertNotEqual(self.run_aggregate(filename, mode=mode, build=build).returncode, 0, (mode, build))
                else:
                    for world in ("failure", "cancelled", "skipped", ""):
                        self.assertNotEqual(self.run_aggregate(filename, world=world).returncode, 0, world)
                    self.assertEqual(self.run_aggregate(filename, default_required=True, default="success").returncode, 0)
                    for required, statuses in ((True, ("failure", "cancelled", "skipped", "")),
                                               (False, ("success", "failure", "cancelled", ""))):
                        for default in statuses:
                            self.assertNotEqual(self.run_aggregate(filename, default_required=required,
                                                                   default=default).returncode, 0, (required, default))
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
        affinity_relative = relative.with_name("pg_archive_affinity.py")
        expected = (ROOT / relative).read_bytes()
        expected_affinity = (ROOT / affinity_relative).read_bytes()
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
            helper = checkout / affinity_relative
            helper.write_bytes(expected_affinity)
            git("add", str(affinity_relative))
            git("commit", "-qm", "reviewed affinity helper")
            modern = git("rev-parse", "HEAD")
            helper.write_text("print('altered helper')")
            git("commit", "-qam", "tampered affinity helper")
            tampered_helper = git("rev-parse", "HEAD")
            git("checkout", "--detach", "-q", modern)
            planner.write_text("print('altered')")
            git("commit", "-qam", "tampered candidate")
            tampered_planner = git("rev-parse", "HEAD")
            for revision, wanted in ((legacy, "current_ci=false"), (base, "current_ci=true"),
                                     (modern, "current_ci=true\naffinity=true"),
                                     (tampered_helper, None), (tampered_planner, None)):
                with self.subTest(revision=revision, mode=wanted):
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
                        self.assertEqual(output.read_text().strip(), wanted)
                        self.assertEqual((temp / "pg-planner/plan_pg_partitions.py").read_bytes(), expected)
                        archived_helper = temp / "pg-planner" / affinity_relative.name
                        if revision == modern:
                            self.assertEqual(archived_helper.read_bytes(), expected_affinity)
                        else:
                            self.assertFalse(archived_helper.exists())

    def test_single_archive_downloads_verify_in_either_supported_layout(self):
        run = step_script(PARTITIONS, "Verify archives were built for this head")
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
        job = indented_yaml_mapping_block(PARTITIONS.read_text(), "durability_postgres", 2)
        runner = re.search(r"^    runs-on: \$\{\{ (.*?) \}\}$", job, re.MULTILINE)[1]
        image = re.search(r"^        image: \$\{\{ (.*?) \}\}$", job, re.MULTILINE)[1]
        script = (
            "const [inputs,github,matrix]=JSON.parse(process.argv[1]);const fromJSON=JSON.parse;"
            "process.stdout.write(JSON.stringify([" + runner + "," + image + "]));"
        )
        for mode in ("on", "split", "off", ""):
            for partition in (1, 4, 5, 12):
                for fork, bot in ((False, False), (True, False), (False, True)):
                    github = {"repository": "repo", "actor": "dependabot[bot]" if bot else "owner",
                              "event": {"pull_request": {"head": {"repo": {"full_name": "fork" if fork else "repo"}}}}}
                    context = [{"pg_runner_mode": mode}, github, {"partition": partition}]
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
