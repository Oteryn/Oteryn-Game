import copy
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import zipfile


SCRIPT = Path(__file__).with_name("collect_rust_fast_results.py")
SPEC = importlib.util.spec_from_file_location("collect_rust_fast_results", SCRIPT)
collector = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(collector)


class CollectRustFastResultsTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.reports, self.archives, self.extract = (self.root / name for name in ("reports", "archives", "extract"))
        self.reports.mkdir()
        self.output = self.root / "output.txt"
        self.env = dict(os.environ, EXPECTED_SHA="a" * 40, BASE_SHA="b" * 40,
                        ARCHIVE_ROLES='["lib", "dispatch-1"]', SHARD="2/5", SLUG="tests-2-5",
                        GITHUB_RUN_ID="71", GITHUB_RUN_ATTEMPT="2", PLAN_IMAGE="20261111.1",
                        PLAN_PLATFORM="Linux-X64", RUST_SELECTION=json.dumps({
                            "mode": "packages", "packages": ["a", "b"]}),
                        TEST_SHARDS=json.dumps({"tests-2-5": ["a"]}), GITHUB_OUTPUT=str(self.output))
        self.results = {}
        for role in ("lib", "dispatch-1"):
            directory = self.reports / f"rust-fast-results-2-{role}-tests-2-5" / role / "tests-2-5"
            directory.mkdir(parents=True)
            self.results[role] = directory / "result.json"
            self.results[role].write_text(json.dumps({
                "head": "a" * 40, "base_sha": "b" * 40, "role": role, "shard": "2/5", "slug": "tests-2-5",
                "run_id": "71", "run_attempt": "2", "image": "20261111.1", "platform": "Linux-X64",
                "build_image": "20261111.1", "build_platform": "Linux-X64",
                "pending_packages": ["a"], "exit_code": 0, "completed": True,
            }))
            self.junit(role)

    def junit(self, role, *, failure=False, declared=1):
        case = '<testcase name="case"><failure/></testcase>' if failure else '<testcase name="case"/>'
        (self.results[role].parent / "junit.xml").write_text(
            f'<testsuites><testsuite name="a" tests="{declared}" failures="{int(failure)}" errors="0">'
            + case + '</testsuite></testsuites>')

    def change(self, archive="lib", **updates):
        path = self.results[archive]
        document = json.loads(path.read_text())
        document.update(updates)
        path.write_text(json.dumps(document))

    def invoke(self):
        return subprocess.run([sys.executable, str(SCRIPT), "--reports", str(self.reports),
                               "--archive-root", str(self.archives), "--extract-root", str(self.extract)],
                              env=self.env, capture_output=True, text=True)

    def assert_rejected(self):
        result = self.invoke()
        self.assertNotIn(result.returncode, (0, 100), result.stdout + result.stderr)
        self.assertFalse(self.output.exists(), "invalid evidence must not claim completion")
        self.assertFalse(self.archives.exists(), "invalid evidence must not be staged")
        self.assertFalse(self.extract.exists(), "invalid evidence must not be staged")

    def test_complete_current_role_union_is_staged_and_cacheable(self):
        result = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.output.read_text(), "completed=true\ncode=0\ncacheable=true\n")
        for role in self.results:
            self.assertEqual((self.archives / role / "runner-image").read_text(), "20261111.1\n")
            self.assertEqual((self.archives / role / "runner-platform").read_text(), "Linux-X64\n")
            self.assertEqual((self.extract / role / "junit.xml").read_bytes(),
                             (self.results[role].parent / "junit.xml").read_bytes())

    def test_missing_archive_or_partial_result_is_rejected(self):
        self.results["dispatch-1"].unlink()
        self.assert_rejected()

    def test_stale_identity_wrong_attempt_code_and_incomplete_results_are_rejected(self):
        original = self.results["lib"].read_bytes()
        for update in ({"head": "c" * 40}, {"base_sha": "c" * 40}, {"run_id": "72"},
                       {"run_attempt": "1"}, {"role": "bins"}, {"shard": "1/5"},
                       {"slug": "tests-1-5"}, {"exit_code": 1}, {"exit_code": False},
                       {"completed": False}, {"completed": 1}, {"unexpected": True}):
            with self.subTest(update=update):
                self.results["lib"].write_bytes(original)
                self.change(**update)
                self.assert_rejected()

    def test_completed_failures_return100_with_current_witness(self):
        self.change(exit_code=100)
        self.junit("lib", failure=True)
        result = self.invoke()
        self.assertEqual(result.returncode, 100, result.stderr)
        self.assertEqual(self.output.read_text(), "completed=true\ncode=100\ncacheable=true\n")

    def test_success_with_failed_junit_and_failure_without_witness_are_rejected(self):
        self.junit("lib", failure=True)
        self.assert_rejected()
        self.junit("lib")
        self.change(exit_code=100)
        self.assert_rejected()
        (self.results["lib"].parent / "junit.xml").unlink()
        self.assert_rejected()

    def test_malformed_incomplete_or_unexpected_package_junit_is_rejected(self):
        path = self.results["lib"].parent / "junit.xml"
        for source in ('<testsuites>', '<!DOCTYPE x [<!ENTITY a "secret">]><testsuites/>',
                       '<testsuites><testsuite name="a" tests="2"><testcase name="one"/></testsuite></testsuites>',
                       '<testsuites><testsuite name="not-selected" tests="0"/></testsuites>',
                       '<testsuites><testsuite name="a" tests="1"><testcase name="one">'
                       '<x:failure xmlns:x="unexpected"/></testcase></testsuite></testsuites>'):
            with self.subTest(source=source):
                path.write_text(source)
                self.assert_rejected()

    def test_identity_drift_requires_all_packages_and_disables_cache(self):
        self.change(image="different-image")
        self.assert_rejected()
        self.change(pending_packages=["a", "b"])
        result = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("cacheable=false\n", self.output.read_text())

    def test_unknown_builder_requires_full_selection(self):
        self.change(build_image="unknown")
        self.assert_rejected()
        self.change(pending_packages=["a", "b"])
        result = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("cacheable=false\n", self.output.read_text())

    def test_empty_pending_hit_and_zero_matching_tests_do_not_invent_junit(self):
        self.env["TEST_SHARDS"] = '{"tests-2-5": []}'
        for role in self.results:
            self.change(role, pending_packages=[])
            (self.results[role].parent / "junit.xml").unlink()
        result = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(any(self.extract.rglob("junit.xml")))

    def test_missing_optional_junit_on_zero_matches_is_not_synthesized(self):
        (self.results["lib"].parent / "junit.xml").unlink()
        result = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.extract / "lib/junit.xml").exists())

    def test_duplicate_unexpected_roles_and_symlinks_are_rejected(self):
        self.env["ARCHIVE_ROLES"] = '["lib", "lib"]'
        self.assert_rejected()
        self.env["ARCHIVE_ROLES"] = '["lib", "dispatch-1"]'
        unexpected = self.reports / "rust-fast-results-2-bins-tests-2-5"
        unexpected.mkdir()
        self.assert_rejected()
        unexpected.rmdir()
        path = self.results["lib"]
        stored = self.root / "elsewhere.json"
        path.rename(stored)
        path.symlink_to(stored)
        self.assert_rejected()

    def test_nested_duplicate_report_is_rejected(self):
        duplicate = self.results["lib"].parent / "duplicate"
        duplicate.mkdir()
        (duplicate / "result.json").write_bytes(self.results["lib"].read_bytes())
        self.assert_rejected()

    def test_successful_retry_preserves_exit_and_disables_cache(self):
        (self.results["lib"].parent / "junit.xml").write_text(
            '<testsuites><testsuite name="a" tests="1" failures="0" errors="0">'
            '<testcase name="case"><flakyFailure/><flakyError/></testcase>'
            '</testsuite></testsuites>')
        result = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.output.read_text(), "completed=true\ncode=0\ncacheable=false\n")

    def test_failed_retry_requires_a_final_failure_witness(self):
        self.change(exit_code=100)
        path = self.results["lib"].parent / "junit.xml"
        prefix = '<testsuites><testsuite name="a" tests="1" failures="0" errors="0"><testcase name="case">'
        suffix = '</testcase></testsuite></testsuites>'
        path.write_text(prefix + '<rerunFailure/><rerunError/>' + suffix)
        self.assert_rejected()
        path.write_text(prefix + '<failure/><rerunFailure/>' + suffix)
        result = self.invoke()
        self.assertEqual(result.returncode, 100, result.stderr)
        self.assertEqual(self.output.read_text(), "completed=true\ncode=100\ncacheable=false\n")

    def api(self, *, attempts=None, failed=(), extra=None):
        attempts = attempts or {"lib": 1, "dispatch-1": 2}
        repository = {"id": 9, "full_name": "Oteryn/Oteryn-Game"}
        self.env["GITHUB_REPOSITORY"] = repository["full_name"]
        run = {"id": 71, "run_attempt": 2, "head_sha": self.env["EXPECTED_SHA"],
               "event": "pull_request", "path": ".github/workflows/merge-gate.yml",
               "repository": repository, "head_repository": repository}
        jobs, artifacts, payloads = [], [], {}
        for index, (role, attempt) in enumerate(attempts.items(), 1):
            jobs.append({"id": 100 + index, "name": f"caller / Unit-test archive ({role}, 2/5)",
                         "run_id": 71, "run_attempt": attempt, "status": "completed",
                         "head_sha": self.env["EXPECTED_SHA"],
                         "created_at": f"2026-11-21T20:{attempt}0:00Z",
                         "started_at": f"2026-11-21T20:{attempt}0:01Z",
                         "completed_at": f"2026-11-21T20:{attempt}1:00Z",
                         "runner_id": 123, "runner_name": "GitHub Actions 123",
                         "runner_group_id": None, "runner_group_name": "GitHub Actions",
                         "labels": ["ubuntu-24.04"],
                         "steps": [{"name": "Run tests", "number": 1, "status": "completed",
                                    "conclusion": "success", "started_at": f"2026-11-21T20:{attempt}0:01Z",
                                    "completed_at": f"2026-11-21T20:{attempt}1:00Z"}],
                         "conclusion": "failure" if role in failed else "success"})
            result = json.loads(self.results[role].read_text())
            result["run_attempt"] = str(attempt)
            stream = io.BytesIO()
            with zipfile.ZipFile(stream, "w") as bundle:
                bundle.writestr(f"{role}/tests-2-5/result.json", json.dumps(result))
                junit = self.results[role].with_name("junit.xml")
                if junit.exists():
                    bundle.writestr(f"{role}/tests-2-5/junit.xml", junit.read_bytes())
                if extra is not None:
                    bundle.writestr(extra, b"unexpected")
            payload = stream.getvalue()
            payloads[200 + index] = payload
            artifacts.append({"id": 200 + index,
                              "name": f"rust-fast-results-{attempt}-{role}-tests-2-5", "expired": False,
                              "size_in_bytes": len(payload), "digest": "sha256:" + hashlib.sha256(payload).hexdigest(),
                              "workflow_run": {"id": 71, "head_sha": self.env["EXPECTED_SHA"],
                                               "repository_id": 9, "head_repository_id": 9}})

        class API:
            def __init__(self):
                self.run, self.jobs, self.artifacts, self.payloads = run, jobs, artifacts, payloads
                self.calls, self.downloads = [], []

            def json(self, path):
                self.calls.append(path)
                if "/jobs?" in path:
                    if "filter=all" not in path:
                        raise AssertionError("partial reruns require all actual job attempts")
                    return {"total_count": len(self.jobs), "jobs": copy.deepcopy(self.jobs)}
                if "/artifacts?" in path:
                    return {"total_count": len(self.artifacts), "artifacts": copy.deepcopy(self.artifacts)}
                return copy.deepcopy(self.run)

            def download(self, artifact_id, destination, *, max_bytes):
                self.downloads.append(artifact_id)
                if len(self.payloads[artifact_id]) > max_bytes:
                    raise ValueError("oversized test bundle")
                destination.write_bytes(self.payloads[artifact_id])

        return API()

    def downloaded(self, api):
        directory = self.root / "downloaded"
        attempts = collector.download_reports(directory, self.env, api=api)
        return collector.collect(directory, self.archives, self.extract, self.env, attempts=attempts)

    def test_partial_rerun_joins_prior_success_and_latest_completed_failure(self):
        self.change("dispatch-1", exit_code=100)
        self.junit("dispatch-1", failure=True)
        api = self.api(failed=("dispatch-1",))
        api.jobs.append({**api.jobs[1], "id": 90, "run_attempt": 1, "conclusion": "success"})
        code, cacheable = self.downloaded(api)
        self.assertEqual((code, cacheable), (100, True))
        self.assertEqual(api.downloads, [201, 202])
        self.assertTrue((self.extract / "lib/junit.xml").is_file())

    def test_copied_success_record_uses_the_actual_original_artifact_attempt(self):
        # GitHub partial reruns create new job IDs/run_attempt values while
        # retaining the original execution, including all step timestamps.
        api = self.api()
        original = api.jobs[0]
        api.jobs.append({**copy.deepcopy(original), "id": 103, "run_attempt": 2,
                         "created_at": "2026-11-21T20:22:00Z"})
        self.assertEqual(self.downloaded(api), (0, True))
        self.assertEqual(api.downloads, [201, 202])

    def test_newest_failed_job_without_result_never_falls_back_to_prior_success(self):
        api = self.api()
        api.jobs.append({**api.jobs[0], "id": 103, "run_attempt": 2, "conclusion": "failure"})
        with self.assertRaisesRegex(ValueError, "artifact"):
            self.downloaded(api)
        self.assertFalse(self.archives.exists())
        self.assertFalse((self.root / "downloaded").exists())

    def test_latest_skipped_incomplete_or_wrong_job_identity_is_rejected(self):
        for update in ({"conclusion": "skipped"}, {"status": "in_progress"},
                       {"run_id": 72}, {"run_attempt": 3}, {"run_attempt": 0}):
            with self.subTest(update=update):
                api = self.api()
                api.jobs[0].update(update)
                with self.assertRaises(ValueError):
                    self.downloaded(api)
                self.assertEqual(api.downloads, [])

    def test_download_rejects_wrong_head_attempt_digest_and_role_inventory(self):
        for mutation in ("head", "attempt", "digest", "inventory", "result-attempt"):
            with self.subTest(mutation=mutation):
                api = self.api(extra="../escape" if mutation == "inventory" else None)
                if mutation == "head": api.artifacts[0]["workflow_run"]["head_sha"] = "c" * 40
                if mutation == "attempt": api.artifacts[0]["name"] = "rust-fast-results-2-lib-tests-2-5"
                if mutation == "digest": api.artifacts[0]["digest"] = "sha256:" + "0" * 64
                if mutation == "result-attempt":
                    source = io.BytesIO(api.payloads[201])
                    changed = io.BytesIO()
                    with zipfile.ZipFile(source) as old, zipfile.ZipFile(changed, "w") as new:
                        for name in old.namelist():
                            data = old.read(name)
                            if name.endswith("result.json"):
                                document = json.loads(data)
                                document["run_attempt"] = "2"
                                data = json.dumps(document).encode()
                            new.writestr(name, data)
                    api.payloads[201] = changed.getvalue()
                    api.artifacts[0].update(size_in_bytes=len(api.payloads[201]),
                                            digest="sha256:" + hashlib.sha256(api.payloads[201]).hexdigest())
                with self.assertRaises(ValueError):
                    self.downloaded(api)
                self.assertFalse((self.root / "downloaded").exists())

    def test_failed_job_cannot_be_turned_into_success_by_a_passed_report(self):
        api = self.api(failed=("lib",))
        with self.assertRaises(ValueError):
            self.downloaded(api)

    def test_api_pagination_requires_complete_bounded_inventory(self):
        class API:
            def json(self, path):
                page = int(path.rsplit("page=", 1)[-1])
                return {"total_count": 101, "jobs": [{"id": n} for n in range((page - 1) * 100, min(page * 100, 101))]}
        self.assertEqual(len(collector.api_pages(API(), "/jobs?filter=all", "jobs")), 101)

        class TooMany:
            def json(self, path):
                return {"total_count": 1001, "jobs": []}
        with self.assertRaises(ValueError):
            collector.api_pages(TooMany(), "/jobs?filter=all", "jobs")


if __name__ == "__main__":
    unittest.main()
