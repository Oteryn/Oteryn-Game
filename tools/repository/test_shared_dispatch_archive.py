import copy
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from urllib.error import HTTPError
import zipfile


MODULE = Path(__file__).resolve().parents[1] / "ci" / "shared_dispatch_archive.py"
SPEC = importlib.util.spec_from_file_location("shared_dispatch_archive", MODULE)
shared = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(shared)


class GitHubExecutionSelectionTests(unittest.TestCase):
    @staticmethod
    def actual_worker_pair():
        # Exact execution fields from GitHub run44945072128: the second API
        # job is a copy created after the first had already finished. URLs and
        # node IDs are omitted; all execution/runner fields are retained.
        rows = (
            (1, "Set up job", "success", "20:28:03", "20:28:06"),
            (2, "Check out exact head", "success", "20:28:06", "20:28:47"),
            (3, "Verify checked-out SHA", "success", "20:28:47", "20:28:47"),
            (4, "Load worker helper for an older candidate", "skipped", "20:28:47", "20:28:47"),
            (5, "Select the exact-source worker helper", "success", "20:28:47", "20:28:47"),
            (6, "Select crates without a passed result", "success", "20:28:47", "20:28:47"),
            (7, "Install Rust toolchain", "success", "20:28:47", "20:28:57"),
            (8, "Free hosted runner disk for archived artifacts", "success", "20:28:57", "20:28:57"),
            (9, "Download this unit-test archive by immutable ID", "success", "20:28:57", "20:29:08"),
            (10, "Verify canonical Dispatch archive", "success", "20:29:08", "20:29:09"),
            (11, "Run this archive and record its current result", "success", "20:29:09", "20:30:22"),
            (12, "Upload current archive-shard result", "success", "20:30:22", "20:30:24"),
            (24, "Post Check out exact head", "success", "20:30:24", "20:30:24"),
            (25, "Complete job", "success", "20:30:24", "20:30:24"),
        )
        original = {
            "id": 136974618869, "run_id": 44945072128, "run_attempt": 1,
            "head_sha": "cbdd2e2e3f5c0e9c85994009325ee24007c970bc",
            "name": "Merge gate / Changed-crate archive (dispatch-1) / Unit-test archive (dispatch-1, 1/5)",
            "status": "completed", "conclusion": "success",
            "created_at": "2026-11-21T20:28:00Z", "started_at": "2026-11-21T20:28:02Z",
            "completed_at": "2026-11-21T20:30:28Z", "runner_id": 1000366837,
            "runner_name": "GitHub Actions 1000366837", "runner_group_id": 0,
            "runner_group_name": "GitHub Actions", "labels": ["ubuntu-24.04"],
            "steps": [{"number": number, "name": name, "status": "completed", "conclusion": conclusion,
                       "started_at": f"2026-11-21T{start}Z", "completed_at": f"2026-11-21T{end}Z"}
                      for number, name, conclusion, start, end in rows],
        }
        copied = {**copy.deepcopy(original), "id": 136978455906, "run_attempt": 2,
                  "created_at": "2026-11-21T20:42:11Z", "runner_group_id": None}
        return original, copied

    def select(self, jobs):
        return shared.select_actual_execution(jobs, run_id=44945072128,
                    head_sha="cbdd2e2e3f5c0e9c85994009325ee24007c970bc", max_attempt=3)

    def test_real_api_copy_resolves_to_original_job_and_attempt(self):
        original, copied = self.actual_worker_pair()
        self.assertEqual(self.select([copied, original]), original)
        third = {**copy.deepcopy(copied), "id": 136978455907, "run_attempt": 3,
                 "created_at": "2026-11-21T20:50:00Z"}
        self.assertEqual(self.select([original, copied, third]), original)

    def test_latest_actual_execution_always_wins(self):
        for status, conclusion in (("completed", "success"), ("completed", "failure"),
                                   ("completed", "skipped"), ("completed", "cancelled"),
                                   ("in_progress", None), ("queued", None)):
            with self.subTest(status=status, conclusion=conclusion):
                original, latest = self.actual_worker_pair()
                latest.update(created_at="2026-11-21T20:42:11Z", started_at="2026-11-21T20:42:12Z",
                              completed_at="2026-11-21T20:43:00Z", status=status, conclusion=conclusion)
                for step in latest["steps"]:
                    step.update(started_at=latest["started_at"], completed_at=latest["completed_at"])
                self.assertEqual(self.select([original, latest]), latest)

    def test_copy_cannot_jump_over_new_failure_incomplete_skipped_or_execution(self):
        for status, conclusion in (("completed", "failure"), ("in_progress", None),
                                   ("completed", "skipped"), ("completed", "success")):
            with self.subTest(status=status, conclusion=conclusion):
                original, newer = self.actual_worker_pair()
                newer.update(status=status, conclusion=conclusion, started_at="2026-11-21T20:42:12Z",
                             completed_at="2026-11-21T20:43:00Z")
                stale_copy = {**copy.deepcopy(original), "id": 136978455907, "run_attempt": 3,
                              "created_at": "2026-11-21T20:50:00Z"}
                with self.assertRaises(ValueError):
                    self.select([original, newer, stale_copy])

    def test_copies_require_complete_matching_execution_and_runner_evidence(self):
        for field, value in (("head_sha", "a" * 40), ("run_id", 1), ("name", "different"),
                             ("runner_id", 123), ("runner_name", "different"),
                             ("runner_group_id", 7), ("runner_group_name", "different"),
                             ("labels", ["self-hosted"]), ("conclusion", "failure"),
                             ("started_at", "2026-11-21T20:28:04Z")):
            with self.subTest(field=field):
                original, copied = self.actual_worker_pair()
                copied[field] = value
                with self.assertRaises(ValueError):
                    self.select([original, copied])
        for field in ("created_at", "started_at", "completed_at", "steps", "runner_id",
                      "runner_name", "runner_group_id", "runner_group_name", "labels"):
            for index in (0, 1):
                with self.subTest(missing=field, record=index):
                    jobs = list(self.actual_worker_pair())
                    del jobs[index][field]
                    with self.assertRaises(ValueError):
                        self.select(jobs)
        for change in ("remove", "change", "duplicate-number"):
            original, copied = self.actual_worker_pair()
            if change == "remove": del copied["steps"][0]["started_at"]
            if change == "change": copied["steps"][0]["conclusion"] = "failure"
            if change == "duplicate-number": copied["steps"][1]["number"] = 1
            with self.subTest(step_change=change), self.assertRaises(ValueError):
                self.select([original, copied])

    def test_ambiguous_or_unproven_copy_history_is_rejected(self):
        original, copied = self.actual_worker_pair()
        for jobs in ([copied], [original, copied, copy.deepcopy(copied)],
                     [original, copied, {**copied, "id": 99}],
                     [original, {**copied, "run_attempt": 3}],
                     [original, {**copied, "created_at": copied["completed_at"]}]):
            with self.subTest(jobs=[job["id"] for job in jobs]), self.assertRaises(ValueError):
                self.select(jobs)
        self.assertIsNone(self.select([]))

    def test_positive_and_self_hosted_runner_groups_are_not_normalized(self):
        original, copied = self.actual_worker_pair()
        original["runner_group_id"] = copied["runner_group_id"] = 7
        self.assertEqual(self.select([original, copied]), original)
        copied["runner_group_id"] = 8
        with self.assertRaises(ValueError): self.select([original, copied])
        for job in (original, copied):
            job.update(runner_group_id=0, labels=["self-hosted"], runner_name="Molehill")
        with self.assertRaises(ValueError): self.select([original, copied])

    def test_real_completed_execution_also_requires_complete_terminal_steps(self):
        for field in ("runner_id", "runner_name", "steps"):
            original, _ = self.actual_worker_pair()
            del original[field]
            with self.subTest(missing=field), self.assertRaises(ValueError):
                self.select([original])
        for update in ({"started_at": "not-a-date"}, {"status": "in_progress", "conclusion": None},
                       {"completed_at": "2026-11-21T20:28:02Z"},
                       {"started_at": "2026-11-21T20:27:00Z"},
                       {"completed_at": "2026-11-21T20:31:00Z"}):
            for copies in (False, True):
                original, copied = self.actual_worker_pair()
                original["steps"][0].update(update)
                copied["steps"][0].update(update)
                with self.subTest(update=update, copies=copies), self.assertRaises(ValueError):
                    self.select([original, copied] if copies else [original])
class FakeGitHub:
    def __init__(self, run, artifact, payload):
        self.run, self.artifact, self.payload = run, artifact, payload
        self.jobs = [{"id": 91, "run_id": run["id"], "run_attempt": run["run_attempt"],
                      "head_sha": run["head_sha"],
                      "name": "caller / Build unit-test archive (dispatch-2)", "status": "in_progress"}]
        self.downloads = 0

    def json(self, path):
        if "/workflows/merge-gate.yml/runs?" in path:
            return {"workflow_runs": [copy.deepcopy(self.run)]}
        if "/artifacts?" in path:
            artifacts = [] if self.artifact is None else [copy.deepcopy(self.artifact)]
            return {"total_count": len(artifacts), "artifacts": artifacts}
        if "/jobs?" in path:
            return {"total_count": len(self.jobs), "jobs": copy.deepcopy(self.jobs)}
        return copy.deepcopy(self.run)

    def download(self, artifact_id, destination):
        self.downloads += 1
        if artifact_id != self.artifact["id"]:
            raise AssertionError("download was not bound to the selected artifact ID")
        destination.write_bytes(self.payload)


class SharedDispatchArchiveTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.role = "dispatch-2"
        self.env = {
            "EXPECTED_SHA": "a" * 40, "GITHUB_REPOSITORY": "Oteryn/Oteryn-Game",
            "GITHUB_RUN_ID": "71", "GITHUB_RUN_ATTEMPT": "2",
            "ImageVersion": "20261111.1", "RUNNER_OS": "Linux", "RUNNER_ARCH": "X64",
        }
        self.identity = {
            "schema": shared.SCHEMA,
            "source": {"repository": self.env["GITHUB_REPOSITORY"], "head": "a" * 40,
                       "cargo_lock_sha256": "b" * 64, "workspace": str(self.root)},
            "recipe": shared.recipe(self.role), "role": self.role,
            "runtime": {"image": self.env["ImageVersion"], "platform": "Linux-X64"},
        }
        self.run = {
            "id": 71, "run_attempt": 2, "event": "pull_request", "head_sha": "a" * 40,
            "path": shared.WORKFLOW, "status": "in_progress", "conclusion": None,
            "repository": {"full_name": self.env["GITHUB_REPOSITORY"], "id": 9},
            "head_repository": {"full_name": self.env["GITHUB_REPOSITORY"], "id": 9},
        }
        self.directory = self.root / self.role
        self.directory.mkdir()
        (self.directory / "unit-tests.tar.zst").write_bytes(b"the nextest archive")
        (self.directory / "cargo-nextest").write_bytes(b"the pinned nextest executable")
        (self.directory / "runner-image").write_text(self.env["ImageVersion"] + "\n")
        (self.directory / "runner-platform").write_text("Linux-X64\n")
        listing = {"test-count": 2, "rust-suites": {"dispatch": {
            "binary-id": shared.BINARY_ID,
            "testcases": {name: {"ignored": False, "filter-match": {"status": "matches"}}
                          for name in ("ordinary::case", "gameplay_transport::monk_save::postgres_tests::case")},
        }}}
        (self.directory / "inventory.json").write_text(json.dumps(listing))
        self.manifest = shared.manifest(self.directory, self.identity, 71, 2)
        self.write_manifest()
        patcher = patch.object(shared, "context", return_value=copy.deepcopy(self.identity))
        self.context_mock = patcher.start()
        self.addCleanup(patcher.stop)

    def write_manifest(self):
        (self.directory / shared.MANIFEST).write_text(json.dumps(self.manifest))

    def api(self, *, extra=None, omitted=()):
        stream = io.BytesIO()
        with zipfile.ZipFile(stream, "w") as bundle:
            for path in self.directory.iterdir():
                if path.name not in omitted:
                    bundle.writestr(f"{self.role}/{path.name}", path.read_bytes())
            if extra is not None:
                bundle.writestr(extra, b"not allowed")
        payload = stream.getvalue()
        artifact = {
            "id": 83, "name": shared.artifact_name(self.role, 2), "expired": False,
            "size_in_bytes": len(payload), "digest": "sha256:" + hashlib.sha256(payload).hexdigest(),
            "workflow_run": {"id": 71, "head_sha": "a" * 40,
                             "repository_id": 9, "head_repository_id": 9},
        }
        return FakeGitHub(copy.deepcopy(self.run), artifact, payload)

    def restore(self, api, **kwargs):
        output = self.root / "restored"
        shared.restore(self.role, output, root=self.root, env=self.env, api=api, **kwargs)
        return output

    def test_exact_producer_archive_is_verified_before_exposure(self):
        api = self.api()
        output = self.restore(api)
        self.assertEqual(api.downloads, 1)
        self.assertEqual({path.name for path in output.iterdir()}, set(shared.FILES) | {shared.MANIFEST})
        self.assertEqual((output / "unit-tests.tar.zst").read_bytes(), b"the nextest archive")

    def test_complete_inventory_keeps_nextest_ignored_cases_and_their_identity(self):
        # This is the default nextest-list wire format, including existing
        # #[ignore] tests. Listing preserves them without selecting execution.
        path = self.directory / "inventory.json"
        listing = json.loads(path.read_text())
        cases = listing["rust-suites"]["dispatch"]["testcases"]
        ignored = "canonical_monster_successor_tests::canonical_monster_successor_reseal_and_preservation"
        cases[ignored] = {"kind": "test", "ignored": True,
                          "filter-match": {"status": "mismatch", "reason": "ignored"}}
        listing["test-count"] = len(cases)
        path.write_text(json.dumps(listing))
        observed = shared.inventory(path)
        expected_names = sorted((name, case["ignored"]) for name, case in cases.items())
        self.assertEqual(observed["tests"], 3)
        self.assertEqual(observed["identity"], hashlib.sha256(
            json.dumps(expected_names, separators=(",", ":")).encode()).hexdigest())
        self.manifest = shared.manifest(self.directory, self.identity, 71, 2)
        self.write_manifest()
        output = self.restore(self.api())
        self.assertEqual(json.loads((output / "inventory.json").read_text()), listing)

    def test_inventory_rejects_filter_exclusions_other_than_existing_ignored_cases(self):
        path = self.directory / "inventory.json"
        original = json.loads(path.read_text())
        mutations = (
            {"ignored": False, "filter-match": {"status": "mismatch", "reason": "ignored"}},
            {"ignored": False, "filter-match": {"status": "mismatch", "reason": "expression"}},
            {"ignored": True, "filter-match": {"status": "mismatch", "reason": "expression"}},
            {"ignored": True, "filter-match": {"status": "mismatch", "reason": "partition"}},
            {"ignored": True, "filter-match": {"status": "mismatch"}},
            {"ignored": "true", "filter-match": {"status": "mismatch", "reason": "ignored"}},
        )
        for case in mutations:
            listing = copy.deepcopy(original)
            listing["rust-suites"]["dispatch"]["testcases"]["ordinary::case"] = case
            path.write_text(json.dumps(listing))
            with self.subTest(case=case), self.assertRaisesRegex(ValueError, "inventory"):
                shared.inventory(path)

    def test_record_requires_a_matching_build_receipt(self):
        with self.assertRaisesRegex(ValueError, "receipt"):
            shared.record(self.directory, self.role, root=self.root, env=self.env)
        (self.directory / shared.RECEIPT).write_text(json.dumps(self.manifest))
        shared.record(self.directory, self.role, root=self.root, env=self.env)
        self.assertFalse((self.directory / shared.RECEIPT).exists())
        (self.directory / shared.RECEIPT).write_text(json.dumps(self.manifest))
        (self.directory / "unit-tests.tar.zst").write_bytes(b"changed after compilation")
        with self.assertRaisesRegex(ValueError, "digest"):
            shared.record(self.directory, self.role, root=self.root, env=self.env)

    def test_wrong_head_recipe_role_run_attempt_and_inventory_fail_closed(self):
        mutations = (
            lambda doc: doc["source"].update(head="c" * 40),
            lambda doc: doc["recipe"].update(version=999),
            lambda doc: doc.update(role="dispatch-1"),
            lambda doc: doc["producer"].update(run_id=999),
            lambda doc: doc["producer"].update(run_attempt=1),
            lambda doc: doc["inventory"].update(tests=999),
        )
        original = copy.deepcopy(self.manifest)
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                self.manifest = copy.deepcopy(original)
                mutate(self.manifest)
                self.write_manifest()
                with self.assertRaises(ValueError):
                    self.restore(self.api())
                self.assertFalse((self.root / "restored").exists())

    def test_wrong_selected_artifact_identity_or_outer_digest_fails_closed(self):
        for field, value in (("name", "wrong-name"), ("digest", "sha256:" + "0" * 64)):
            api = self.api()
            api.artifact[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                shared.validate_artifact(api.artifact, self.run, self.role)
                self.restore(api)
        api = self.api()
        api.artifact["workflow_run"]["id"] = 99
        with self.assertRaises(ValueError):
            self.restore(api)

    def test_wrong_run_source_or_repository_is_rejected(self):
        for field, value in (("head_sha", "c" * 40), ("path", ".github/workflows/other.yml"),
                             ("event", "pull_request_target")):
            run = copy.deepcopy(self.run)
            run[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                shared.validate_run(run, self.identity)
        run = copy.deepcopy(self.run)
        run["head_repository"]["full_name"] = "outsider/fork"
        with self.assertRaises(ValueError):
            shared.validate_run(run, self.identity)

    def test_zip_traversal_unexpected_and_missing_files_fail_closed(self):
        for extra in ("../escape", "dispatch-2/../../escape", "dispatch-1/inventory.json",
                      "dispatch-2/unexpected"):
            with self.subTest(extra=extra), self.assertRaises(ValueError):
                self.restore(self.api(extra=extra))
        with self.assertRaises(ValueError):
            self.restore(self.api(omitted=("inventory.json",)))
        self.assertFalse((self.root / "restored").exists())

    def test_inner_file_digest_mismatch_is_rejected(self):
        (self.directory / "unit-tests.tar.zst").write_bytes(b"tampered archive")
        with self.assertRaisesRegex(ValueError, "digest"):
            self.restore(self.api())

    def test_waiting_requires_this_role_build_to_be_running_and_is_bounded(self):
        api = self.api()
        api.artifact = None
        api.jobs = [{"id": 91, "run_id": 71, "run_attempt": 2,
                     "head_sha": "a" * 40,
                     "name": "caller / Build unit-test archive (dispatch-2)", "status": "queued"}]
        with self.assertRaises(shared.Unavailable):
            self.restore(api, discovery_grace=0, sleep=lambda _: self.fail("no grace requested"))
        api.jobs[0]["status"] = "in_progress"
        elapsed = [0]

        def sleep(seconds):
            self.assertLessEqual(seconds, 30)
            elapsed[0] += seconds

        with self.assertRaises(shared.Unavailable):
            self.restore(api, sleep=sleep, clock=lambda: elapsed[0], timeout=61)
        self.assertEqual(elapsed[0], 61)
        self.assertEqual(api.downloads, 0)

    def test_queued_run_with_active_producer_waits_for_its_verified_archive(self):
        # GitHub reported run 45004445109 queued while its Dispatch producers
        # were running. The selected job, not the aggregate run state, is active.
        api = self.api()
        api.run["status"] = "queued"
        artifact, api.artifact = api.artifact, None
        elapsed = [0]

        def sleep(seconds):
            self.assertEqual(seconds, 30)
            elapsed[0] += seconds
            api.artifact = artifact

        output = self.restore(api, sleep=sleep, clock=lambda: elapsed[0], timeout=61)
        self.assertEqual(elapsed[0], 30)
        self.assertEqual(api.downloads, 1)
        self.assertEqual((output / "unit-tests.tar.zst").read_bytes(), b"the nextest archive")

    def test_queued_run_keeps_active_and_queued_producer_wait_bounds(self):
        for status, expected_seconds in (("in_progress", 61), ("queued", 60)):
            with self.subTest(producer_status=status):
                api = self.api()
                api.run["status"] = "queued"
                api.jobs[0]["status"] = status
                api.artifact = None
                elapsed = [0]

                def sleep(seconds):
                    self.assertLessEqual(seconds, 5 if status == "queued" else 30)
                    elapsed[0] += seconds

                with self.assertRaises(shared.Unavailable):
                    self.restore(api, sleep=sleep, clock=lambda: elapsed[0], timeout=61)
                self.assertEqual(elapsed[0], expected_seconds)
                self.assertEqual(api.downloads, 0)

    def test_terminal_or_unknown_run_does_not_wait_for_missing_artifact(self):
        for status, conclusion in (("completed", "cancelled"), ("completed", "failure"),
                                   ("unknown", None), (None, None)):
            with self.subTest(status=status, conclusion=conclusion):
                api = self.api()
                api.run.update(status=status, conclusion=conclusion)
                api.artifact = None
                with self.assertRaises(shared.Unavailable):
                    self.restore(api, sleep=lambda _: self.fail("terminal run cannot be awaited"))
                self.assertEqual(api.downloads, 0)

    def test_legacy_producer_is_unavailable_without_reading_its_old_archive(self):
        api = self.api()
        api.jobs = [{"id": 91, "run_id": 71, "run_attempt": 2,
                     "name": "Old unspecialized archive producer", "status": "completed", "conclusion": "success"}]
        with self.assertRaises(shared.Unavailable):
            self.restore(api, discovery_grace=0)
        self.assertEqual(api.downloads, 0)

    def test_fast_verify_binds_producer_image_separately_from_consumer(self):
        self.env["ImageVersion"] = "newer-consumer-image"
        shared.verify(self.directory, self.role, root=self.root, env=self.env,
                      producer_image=self.identity["runtime"]["image"])
        with self.assertRaises(ValueError):
            shared.verify(self.directory, self.role, root=self.root, env=self.env,
                          producer_image="wrong-producer-image")

    def test_partial_rerun_verifies_the_actual_earlier_builder_attempt_in_the_same_run(self):
        self.env["GITHUB_RUN_ATTEMPT"] = "3"
        shared.verify(self.directory, self.role, root=self.root, env=self.env, producer_attempt=2)
        with self.assertRaises(ValueError):
            shared.verify(self.directory, self.role, root=self.root, env=self.env)
        for attempt in (0, -1, 4, True, "2"):
            with self.subTest(attempt=attempt), self.assertRaises(ValueError):
                shared.verify(self.directory, self.role, root=self.root, env=self.env, producer_attempt=attempt)
        self.env["GITHUB_RUN_ID"] = "72"
        with self.assertRaises(ValueError):
            shared.verify(self.directory, self.role, root=self.root, env=self.env, producer_attempt=2)

    def copied_producer_api(self):
        self.manifest["producer"]["run_attempt"] = 1
        self.write_manifest()
        api = self.api()
        # Adapt only the role/run/head of the captured worker execution to this
        # isolated producer fixture; preserve its exact copy chronology/steps.
        api.jobs = list(GitHubExecutionSelectionTests.actual_worker_pair())
        for job in api.jobs:
            job.update(name=f"caller / Build unit-test archive ({self.role})", run_id=71, head_sha="a" * 40)
        api.artifact["name"] = shared.artifact_name(self.role, 1)
        return api

    def test_restore_binds_a_copied_success_to_its_actual_producer_attempt(self):
        api = self.copied_producer_api()
        output = self.restore(api)
        self.assertEqual(api.downloads, 1)
        self.assertEqual(json.loads((output / shared.MANIFEST).read_text())["producer"]["run_attempt"], 1)

    def test_restore_never_uses_old_archive_after_real_newer_failure_or_incomplete(self):
        for status, conclusion in (("completed", "failure"), ("completed", "skipped"),
                                   ("in_progress", None), ("queued", None)):
            with self.subTest(status=status, conclusion=conclusion):
                api = self.copied_producer_api()
                job = api.jobs[-1]
                job.update(status=status, conclusion=conclusion, started_at="2026-11-21T20:42:12Z",
                           completed_at="2026-11-21T20:43:00Z")
                for step in job["steps"]:
                    step.update(started_at=job["started_at"], completed_at=job["completed_at"])
                with self.assertRaises(shared.Unavailable):
                    self.restore(api, discovery_grace=0, timeout=0)
                self.assertEqual(api.downloads, 0)
                self.assertFalse((self.root / "restored").exists())

    def test_restore_rechecks_the_actual_producer_before_exposing_download(self):
        api = self.copied_producer_api()
        download = api.download
        def changed(*args):
            download(*args)
            api.jobs[-1].update(status="completed", conclusion="failure")
        api.download = changed
        with self.assertRaises(ValueError):
            self.restore(api)
        self.assertEqual(api.downloads, 1)
        self.assertFalse((self.root / "restored").exists())

    def test_complete_job_history_pagination_rejects_truncation_and_duplicates(self):
        class Pages:
            def json(self, path):
                page = int(path.rsplit("page=", 1)[-1])
                return {"total_count": 101, "jobs": [{"id": n + 1} for n in
                    range((page - 1) * 100, min(page * 100, 101))]}
        self.assertEqual(len(shared.github_items(Pages(), "/jobs?filter=all", "jobs")), 101)
        for document in ({"total_count": 101, "jobs": [{"id": 1}]},
                         {"total_count": 2, "jobs": [{"id": 1}, {"id": 1}]},
                         {"total_count": 1001, "jobs": []}):
            class Invalid:
                def json(self, path): return document
            with self.subTest(document=document), self.assertRaises(ValueError):
                shared.github_items(Invalid(), "/jobs?filter=all", "jobs")

    def test_conflicting_compile_flags_or_wrappers_are_rejected(self):
        for name in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER",
                     "RUSTC_WORKSPACE_WRAPPER", "CARGO_PROFILE_DEV_DEBUG",
                     "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS", "CARGO_ALIAS_NEXTEST", "CFLAGS"):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "configuration"):
                shared.build_environment({name: "conflicting"}, self.role, self.root)

    def test_build_invokes_the_fixed_recipe_and_records_only_its_result(self):
        output = self.root / "built"
        nextest = self.directory / "cargo-nextest"
        mold, preload = self.root / "mold", self.root / "mold-wrapper.so"
        mold.write_bytes(b"pinned mold")
        preload.write_bytes(b"pinned mold wrapper")
        env = {**self.env, "MOLD_PATH": str(mold), "MOLD_PRELOAD": str(preload),
               "LD_PRELOAD": str(preload), "CARGO_HOME": str(self.root / "cargo-home"),
               "CARGO_TARGET_DIR": "target", "RUSTFLAGS": "", "GH_TOKEN": "test-token"}

        def command_output(command, **kwargs):
            if command[0] == str(nextest):
                return "cargo-nextest 0.9.146 (pinned)\n"
            if command[0] == str(mold):
                return "mold 2.42.1 (compatible with GNU ld)\n"
            if command[0] == "readelf":
                self.assertEqual(command[1:3], ["-p", ".comment"])
                self.assertEqual(Path(command[-1]).read_bytes(), b"the archived executable")
                return "String dump of section '.comment':\n  [0] mold 2.42.1 (pinned)\n"
            self.assertEqual(command[:4], ["cargo", "+1.100.0", "nextest", "list"])
            self.assertIn(str(output / "unit-tests.tar.zst"), command)
            extracted = Path(command[command.index("--extract-to") + 1])
            binary = extracted / "target/debug/deps/oteryn_gameplay_dispatch"
            binary.parent.mkdir(parents=True)
            binary.write_bytes(b"the archived executable")
            binary.chmod(0o755)
            listing = json.loads((self.directory / "inventory.json").read_text())
            listing["rust-suites"]["dispatch"]["binary-path"] = str(binary)
            return json.dumps(listing).encode()

        def compile_archive(command, **kwargs):
            self.assertEqual(command, ["cargo", "+1.100.0", *shared.CARGO_ARGS,
                                       "--archive-file", str(output / "unit-tests.tar.zst")])
            expected = {"OTERYN_DISPATCH_TEST_ARCHIVE": "dedicated", "OTERYN_DISPATCH_PG_SHARD": "2/3",
                        "CARGO_BUILD_JOBS": "2", "CARGO_INCREMENTAL": "0"}
            for name, value in expected.items():
                self.assertEqual(kwargs["env"][name], value)
            self.assertNotIn("GH_TOKEN", kwargs["env"])
            self.assertNotIn("RUSTFLAGS", kwargs["env"])
            Path(command[-1]).write_bytes(b"the nextest archive")

        with patch.object(shared.shutil, "which", return_value=str(nextest)), \
                patch.object(shared.subprocess, "check_output", side_effect=command_output), \
                patch.object(shared.subprocess, "run", side_effect=compile_archive) as run:
            shared.build(output, self.role, root=self.root, env=env)
        self.assertEqual(run.call_count, 1)
        self.assertFalse((output / shared.RECEIPT).exists())
        shared.verify(output, self.role, root=self.root, env=self.env)

    def test_archived_binary_requires_the_actual_pinned_mold_tag(self):
        extracted = self.root / "extracted"
        extracted.mkdir()
        binary = extracted / "dispatch"
        binary.write_bytes(b"archived ELF fixture")
        binary.chmod(0o755)
        listing = json.loads((self.directory / "inventory.json").read_text())
        listing["rust-suites"]["dispatch"]["binary-path"] = str(binary)
        inventory = self.directory / "inventory.json"
        inventory.write_text(json.dumps(listing))
        for comment in ("GCC: (GNU) 14.2.0\n", "mold 2.41.0 (older)\n"):
            with self.subTest(comment=comment), \
                    patch.object(shared.subprocess, "check_output", return_value=comment), \
                    self.assertRaisesRegex(ValueError, "mold"):
                shared.verify_archived_linker(inventory, extracted, self.env)
        with patch.object(shared.subprocess, "check_output", return_value="  [0] mold 2.42.1 (pinned)\n") as reader:
            shared.verify_archived_linker(inventory, extracted, self.env)
        self.assertEqual(reader.call_args.args[0], ["readelf", "-p", ".comment", str(binary)])

    def test_linker_proof_rejects_a_missing_symlink_or_outside_binary(self):
        extracted = self.root / "extracted"
        extracted.mkdir()
        outside = self.root / "outside"
        outside.write_bytes(b"unrelated executable")
        outside.chmod(0o755)
        link = extracted / "link"
        link.symlink_to(outside)
        listing = json.loads((self.directory / "inventory.json").read_text())
        inventory = self.directory / "inventory.json"
        for binary in (extracted / "missing", outside, link):
            listing["rust-suites"]["dispatch"]["binary-path"] = str(binary)
            inventory.write_text(json.dumps(listing))
            with self.subTest(binary=binary), \
                    patch.object(shared.subprocess, "check_output") as reader, \
                    self.assertRaises((ValueError, OSError)):
                shared.verify_archived_linker(inventory, extracted, self.env)
            reader.assert_not_called()

    def test_entrypoint_parity_is_required(self):
        dispatch = self.root / "crates/gameplay/dispatch"
        (dispatch / "src").mkdir(parents=True)
        (dispatch / "Cargo.toml").write_text('[[test]]\nname = "oteryn-gameplay-dispatch"\n'
                                            'path = "src/ci_unit_archive.rs"\ntest = false\n')
        body = 'pub mod gameplay_transport;\npub mod observability;\ninclude!("dispatch.rs");\n'
        (dispatch / "src/lib.rs").write_text(
            '#![cfg(not(all(test, oteryn_dispatch_test_archive = "common")))]\n' + body)
        target = dispatch / "src/ci_unit_archive.rs"
        target.write_text('#![cfg(all(test, oteryn_dispatch_test_archive = "dedicated"))]\n' + body)
        shared.entrypoints(self.root)
        target.write_text(target.read_text() + 'mod another_module;\n')
        with self.assertRaises(shared.Unavailable):
            shared.entrypoints(self.root)

    def test_artifact_storage_redirect_never_receives_the_github_token(self):
        api = shared.GitHub("Oteryn/Oteryn-Game", "test-token")
        redirect = HTTPError("https://api.github.com", 302, "Found", {
            "Location": "https://productionresultssa1.blob.core.windows.net/artifact?signature=test",
        }, None)
        with patch.object(api.opener, "open", side_effect=[redirect, io.BytesIO(b"zip")]) as opened:
            api.download(83, self.root / "download.zip")
        self.assertEqual(opened.call_args_list[0].args[0].get_header("Authorization"), "Bearer test-token")
        self.assertIsNone(opened.call_args_list[1].args[0].get_header("Authorization"))

    def test_unsafe_storage_redirect_is_rejected(self):
        api = shared.GitHub("Oteryn/Oteryn-Game", "test-token")
        for url in ("http://internal/artifact", "https://unexpected.example/artifact",
                    "https://user:password@productionresultssa1.blob.core.windows.net/artifact"):
            redirect = HTTPError("https://api.github.com", 302, "Found", {"Location": url}, None)
            with self.subTest(url=url), patch.object(api.opener, "open", side_effect=redirect) as opened:
                with self.assertRaises(ValueError):
                    api.download(83, self.root / "download.zip")
                self.assertEqual(opened.call_count, 1)

    def test_symlink_zip_entry_is_rejected(self):
        stream = io.BytesIO()
        with zipfile.ZipFile(stream, "w") as bundle:
            for path in self.directory.iterdir():
                entry = zipfile.ZipInfo(f"{self.role}/{path.name}")
                if path.name == "cargo-nextest":
                    entry.external_attr = 0o120777 << 16
                bundle.writestr(entry, path.read_bytes())
        payload = stream.getvalue()
        api = self.api()
        api.payload = payload
        api.artifact.update(size_in_bytes=len(payload), digest="sha256:" + hashlib.sha256(payload).hexdigest())
        with self.assertRaisesRegex(ValueError, "file type"):
            self.restore(api)

    def test_different_verified_image_is_unavailable_without_exposing_files(self):
        self.identity["runtime"]["image"] = "different-image"
        self.context_mock.return_value = copy.deepcopy(self.identity)
        with self.assertRaises(shared.Unavailable):
            self.restore(self.api())
        self.assertFalse((self.root / "restored").exists())

    def test_changed_attempt_during_wait_fails_closed(self):
        api = self.api()
        api.artifact = None

        def restart(_):
            api.run["run_attempt"] += 1

        with self.assertRaisesRegex(ValueError, "attempt changed"):
            self.restore(api, sleep=restart)
        self.assertEqual(api.downloads, 0)

    def test_download_limit_is_enforced_before_exposure(self):
        api = shared.GitHub("Oteryn/Oteryn-Game", "test-token")
        with patch.object(api, "open", return_value=io.BytesIO(b"too much data")), \
                patch.object(shared, "MAX_ZIP_BYTES", 4):
            with self.assertRaisesRegex(ValueError, "download limit"):
                api.download(83, self.root / "oversized.zip")

    def test_queued_or_unplanned_role_gets_only_the_short_discovery_grace(self):
        for queued in (True, False):
            api = self.api()
            api.artifact = None
            if queued:
                api.jobs[0]["status"] = "queued"
            else:
                api.jobs = []
            elapsed = [0]

            def sleep(seconds):
                self.assertLessEqual(seconds, 5)
                elapsed[0] += seconds

            with self.subTest(queued=queued), self.assertRaises(shared.Unavailable):
                self.restore(api, clock=lambda: elapsed[0], sleep=sleep)
            self.assertEqual(elapsed[0], 60)
            self.assertEqual(api.downloads, 0)


if __name__ == "__main__":
    unittest.main()
