"""World compiler reuse is bound to this run/attempt and the exact compiled candidate."""
import copy
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch
from urllib.parse import parse_qs, urlparse
import zipfile

import world_compiler_artifact as compiler


class WorldCompilerArtifact(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.shared, self.prebuilt = compiler.dependencies()
        self.binding = {"head": "1" * 40, "workspace": str(self.root), "variant": "world-compiler",
                        "toolchain": "1.100.0", "features": [], "profile": "release"}
        identity = patch.object(self.prebuilt, "identity", return_value=self.binding)
        identity.start()
        self.addCleanup(identity.stop)
        head = patch.object(compiler.subprocess, "check_output", return_value=self.binding["head"] + "\n")
        head.start()
        self.addCleanup(head.stop)
        self.env = {"GITHUB_REPOSITORY": "Oteryn/Oteryn-Game", "GITHUB_RUN_ID": "101",
                    "GITHUB_RUN_ATTEMPT": "2", "EXPECTED_SHA": self.binding["head"],
                    "GITHUB_SHA": self.binding["head"]}
        repository = {"id": 51, "full_name": "Oteryn/Oteryn-Game"}
        self.run = {"id": 101, "run_attempt": 2, "head_sha": self.env["GITHUB_SHA"],
                    "event": "push", "path": ".github/workflows/merge-group-gate.yml",
                    "repository": repository, "head_repository": repository, "status": "in_progress"}

    def api(self, *, manifest_mutation=None, extra_member=None, wrong_binary=False):
        raw = b"#!/bin/sh\nexit 0\n"
        executable = self.root / "compiler"
        executable.write_bytes(raw)
        manifest = dict(self.binding, sha256={compiler.COMPILER: self.shared.sha256(executable)})
        if manifest_mutation:
            manifest_mutation(manifest)
        tar_bytes = io.BytesIO()
        with tarfile.open(fileobj=tar_bytes, mode="w:gz") as archive:
            entries = [("manifest.json", json.dumps(manifest).encode()),
                       (compiler.COMPILER, b"different" if wrong_binary else raw)]
            if extra_member is not None:
                entries.append((extra_member, b"x"))
            for name, contents in entries:
                info = tarfile.TarInfo(name)
                info.size, info.mode = len(contents), 0o755
                archive.addfile(info, io.BytesIO(contents))
        download = self.root / "artifact.zip"
        with zipfile.ZipFile(download, "w") as archive:
            archive.writestr("bundle.tar.gz", tar_bytes.getvalue())
        artifact = {"id": 201, "name": "world-compiler-2", "expired": False,
                    "size_in_bytes": download.stat().st_size,
                    "digest": "sha256:" + self.shared.sha256(download),
                    "workflow_run": {"id": 101, "head_sha": self.run["head_sha"],
                                     "repository_id": 51, "head_repository_id": 51}}
        case = self
        job = {"id": 301, "name": ("Integration shards / World compiler"
                                  if self.run["event"] == "workflow_dispatch" else "Merge Queue / World bundle"),
               "run_id": 101, "run_attempt": 2, "head_sha": self.run["head_sha"],
               "status": "completed", "conclusion": "success"}

        class API:
            artifacts = [artifact]
            jobs = [job]
            calls = []
            downloaded = []

            def json(self, path):
                self.calls.append(path)
                page = int(parse_qs(urlparse(path).query).get("page", ["1"])[0])
                start = 100 * (page - 1)
                if "/artifacts?" in path:
                    return {"total_count": len(self.artifacts), "artifacts": self.artifacts[start:start + 100]}
                if "/jobs?" in path:
                    return {"total_count": len(self.jobs), "jobs": self.jobs[start:start + 100]}
                return case.run

            def download(self, artifact_id, path):
                if artifact_id not in {item["id"] for item in self.artifacts}:
                    raise AssertionError("wrong immutable artifact requested")
                self.downloaded.append(artifact_id)
                path.write_bytes(download.read_bytes())

        return API(), artifact

    def restore(self, api, **kwargs):
        return compiler.restore(self.root / "restored", repo_root=self.root, env=self.env,
                                api=api, timeout=0, **kwargs)

    def test_current_run_archive_reuses_the_existing_source_and_checksum_verifier(self):
        api, _ = self.api()
        path = self.restore(api)
        self.assertEqual(path, self.root / "restored/world-compiler" / compiler.COMPILER)
        self.assertTrue(path.is_file())
        self.assertFalse(any("/workflows/" in path for path in api.calls), "must never search latest runs")

    def test_manual_workflow_binds_resolved_candidate_separately_from_workflow_source_head(self):
        self.env["GITHUB_SHA"] = "2" * 40
        self.run.update(head_sha=self.env["GITHUB_SHA"], event="workflow_dispatch",
                        path=".github/workflows/integration-shards-dispatch.yml")
        api, _ = self.api()
        path = self.restore(api, workflow="integration-shards-dispatch.yml")
        self.assertTrue(path.is_file())

    def test_wrong_run_attempt_repo_or_workflow_is_rejected_before_download(self):
        for key, value in (("id", 999), ("run_attempt", 3), ("head_sha", "2" * 40),
                           ("path", ".github/workflows/unrelated.yml"),
                           ("head_repository", {"id": 88, "full_name": "other/repo"})):
            with self.subTest(key=key):
                original = copy.deepcopy(self.run)
                self.run[key] = value
                api, _ = self.api()
                with self.assertRaises(ValueError):
                    self.restore(api)
                self.assertFalse((self.root / "restored").exists())
                self.run = original

    def test_corrupt_digest_source_profile_features_or_binary_cannot_be_exposed(self):
        for mutation in ("api-digest", "head", "profile", "features", "binary"):
            with self.subTest(mutation=mutation):
                def change(manifest):
                    if mutation == "head": manifest["head"] = "2" * 40
                    if mutation == "profile": manifest["profile"] = "dev"
                    if mutation == "features": manifest["features"] = ["test-support"]
                api, artifact = self.api(manifest_mutation=change, wrong_binary=mutation == "binary")
                if mutation == "api-digest": artifact["digest"] = "sha256:" + "0" * 64
                with self.assertRaises(ValueError):
                    self.restore(api)
                self.assertFalse((self.root / "restored").exists())

    def test_missing_producer_is_a_bounded_failure_without_another_release_build(self):
        api, _ = self.api()
        api.artifacts = []
        with self.assertRaisesRegex(RuntimeError, "wait bound"):
            self.restore(api)
        self.assertFalse((self.root / "restored").exists())

    def test_bad_archive_inventory_is_refused_by_the_shared_unpacker(self):
        for member in ("../escape", "manifest.json", "unrequested"):
            with self.subTest(member=member):
                api, _ = self.api(extra_member=member)
                with self.assertRaises(ValueError):
                    self.restore(api)
                self.assertFalse((self.root / "escape").exists())
                self.assertFalse((self.root / "restored").exists())

    def test_wait_budget_is_bounded_and_does_not_wait_for_a_failed_producer(self):
        api, _ = self.api()
        api.artifacts = []
        job = api.jobs[0]
        job["status"] = "in_progress"
        elapsed = 0
        waits = []

        def sleep(seconds):
            nonlocal elapsed
            self.assertGreater(seconds, 0)
            self.assertLessEqual(seconds, 15)
            waits.append(seconds)
            elapsed += seconds

        with self.assertRaisesRegex(RuntimeError, "wait bound"):
            compiler.restore(self.root / "restored", repo_root=self.root, env=self.env, api=api,
                             timeout=31, clock=lambda: elapsed, sleep=sleep)
        self.assertEqual(waits, [15, 15, 1])
        job["status"] = "completed"
        waits.clear()
        with self.assertRaisesRegex(RuntimeError, "wait bound"):
            compiler.restore(self.root / "restored", repo_root=self.root, env=self.env, api=api,
                             timeout=600, clock=lambda: elapsed, sleep=sleep)
        self.assertEqual(waits, [])

    def test_run_attempt_change_while_waiting_is_not_rebound(self):
        api, _ = self.api()
        api.artifacts = []
        api.jobs[0]["status"] = "in_progress"

        def change_attempt(seconds):
            self.run["run_attempt"] = 3

        with self.assertRaisesRegex(ValueError, "current-run identity"):
            compiler.restore(self.root / "restored", repo_root=self.root, env=self.env, api=api,
                             timeout=1, clock=lambda: 0, sleep=change_attempt)
        self.assertFalse((self.root / "restored").exists())

    def test_partial_rerun_keeps_the_successful_producers_actual_attempt(self):
        api, artifact = self.api()
        api.jobs[0]["run_attempt"] = 1
        artifact["name"] = "world-compiler-1"
        path = self.restore(api)
        self.assertTrue(path.is_file())
        self.assertEqual(api.downloaded, [artifact["id"]])
        self.assertTrue(any("jobs?filter=all" in path for path in api.calls))

    def test_manual_partial_rerun_preserves_both_source_identities_and_producer_attempt(self):
        self.env["GITHUB_SHA"] = "2" * 40
        self.run.update(head_sha=self.env["GITHUB_SHA"], event="workflow_dispatch",
                        path=".github/workflows/integration-shards-dispatch.yml")
        api, artifact = self.api()
        api.jobs[0]["run_attempt"] = 1
        artifact["name"] = "world-compiler-1"
        self.assertTrue(self.restore(api, workflow="integration-shards-dispatch.yml").is_file())
        self.assertEqual(api.downloaded, [artifact["id"]])

    def test_newer_failed_producer_blocks_fallback_to_an_older_success(self):
        api, artifact = self.api()
        old = dict(api.jobs[0], id=300, run_attempt=1)
        latest = dict(api.jobs[0], conclusion="failure")
        api.jobs = [old, latest]
        artifact["name"] = "world-compiler-1"
        with self.assertRaisesRegex(RuntimeError, "wait bound"):
            self.restore(api)
        self.assertEqual(api.downloaded, [])
        self.assertFalse((self.root / "restored").exists())

    def test_latest_producer_is_selected_from_all_bounded_job_pages(self):
        api, artifact = self.api()
        old = dict(api.jobs[0], id=300, run_attempt=1)
        latest = dict(api.jobs[0], conclusion="failure")
        other = [dict(latest, id=400 + i, name=f"Unrelated worker {i}") for i in range(99)]
        api.jobs = [old, *other, latest]
        artifact["name"] = "world-compiler-1"
        with self.assertRaisesRegex(RuntimeError, "wait bound"):
            self.restore(api)
        self.assertTrue(any("jobs?filter=all&per_page=100&page=2" in path for path in api.calls))
        self.assertEqual(api.downloaded, [])

    def test_published_bytes_remain_usable_if_pin_check_fails_after_publication(self):
        api, artifact = self.api()
        api.jobs[0]["conclusion"] = "failure"
        self.assertTrue(self.restore(api).is_file())
        self.assertEqual(api.downloaded, [artifact["id"]])

    def test_retained_producer_artifact_is_found_beyond_the_first_artifact_page(self):
        api, artifact = self.api()
        api.jobs[0]["run_attempt"] = 1
        artifact["name"] = "world-compiler-1"
        api.artifacts = [dict(artifact, id=500 + i, name=f"Unrelated artifact {i}") for i in range(100)] + [artifact]
        self.assertTrue(self.restore(api).is_file())
        self.assertTrue(any("artifacts?per_page=100&page=2" in path for path in api.calls))
        self.assertEqual(api.downloaded, [artifact["id"]])

    def test_incomplete_or_unbounded_discovery_cannot_select_an_older_producer(self):
        api, artifact = self.api()
        artifact["name"] = "world-compiler-1"
        api.jobs[0]["run_attempt"] = 1
        original = api.json

        def oversized(path):
            response = original(path)
            if "/jobs?" in path:
                response["total_count"] = 100 * compiler.MAX_DISCOVERY_PAGES + 1
            return response

        api.json = oversized
        with self.assertRaisesRegex(ValueError, "discovery bound"):
            self.restore(api)
        self.assertEqual(api.downloaded, [])
        api.json = original
        latest = dict(api.jobs[0], id=999, run_attempt=2, conclusion="failure")
        api.jobs = [*api.jobs, *[dict(latest, id=400 + i, name=f"Other {i}") for i in range(99)], latest]

        def changed(path):
            response = original(path)
            if "jobs?filter=all&per_page=100&page=2" in path:
                response["total_count"] += 1
            return response

        api.json = changed
        with self.assertRaisesRegex(RuntimeError, "stabilize"):
            self.restore(api)
        self.assertEqual(api.downloaded, [])


if __name__ == "__main__":
    unittest.main()
