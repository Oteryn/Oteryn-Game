"""A reused native-entry libtest must match source, build variant and test inventory."""
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import shutil
import tempfile
import tarfile
import textwrap
import unittest
from unittest.mock import patch
from urllib.parse import parse_qs, urlparse
import zipfile

import prebuilt


class NativeEntryPrebuilt(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.bundle = self.root / "bundle"
        self.bundle.mkdir()
        self.binary = self.bundle / prebuilt.TARGET
        self.write_stub()
        self.binding = {"head": "1" * 40, "workspace": str(self.root),
                        "target": prebuilt.TARGET, "profile": "test", "features": [],
                        "toolchain": "1.100.0"}
        self.manifest = dict(self.binding, sha256=self.digest(), tests=sorted(prebuilt.TESTS),
                             ignored=sorted(prebuilt.IGNORED),
                             selection={"mode": "packages", "packages": ["oteryn-game-server"]},
                             libraries=False)
        self.manifest_path = self.bundle / "manifest.json"
        self.save()
        identity = patch.object(prebuilt, "identity", return_value=self.binding)
        identity.start()
        self.addCleanup(identity.stop)

    def write_stub(self, tests=None, ignored=None):
        tests = sorted(prebuilt.TESTS if tests is None else tests)
        ignored = sorted(prebuilt.IGNORED if ignored is None else ignored)
        self.binary.write_text(
            "#!/usr/bin/env python3\n"
            "import sys\n"
            f"tests = {tests!r}\nignored = {ignored!r}\n"
            "if '--list' not in sys.argv: raise SystemExit(19)\n"
            "for name in (ignored if '--ignored' in sys.argv else tests):\n"
            "    print(name + ': test')\n"
        )
        self.binary.chmod(0o700)

    def digest(self):
        return hashlib.sha256(self.binary.read_bytes()).hexdigest()

    def save(self):
        self.manifest_path.write_text(json.dumps(self.manifest))

    def test_valid_prebuilt_preserves_all_three_tests_and_issuer_requirement(self):
        self.assertEqual(prebuilt.executable(self.bundle, self.root), self.binary)
        self.assertEqual(prebuilt.list_tests(self.binary, self.root, ignored=True), prebuilt.IGNORED)

    def test_stale_source_wrong_profile_or_features_are_refused(self):
        for key, value in (("head", "2" * 40), ("workspace", "/other"),
                           ("profile", "release"), ("features", ["pg_integration_tests"]),
                           ("tests", []), ("ignored", [])):
            with self.subTest(key=key):
                original = copy.deepcopy(self.manifest)
                self.manifest[key] = value
                self.save()
                with self.assertRaises(ValueError):
                    prebuilt.executable(self.bundle, self.root)
                self.manifest = original

    def test_changed_bytes_or_symlink_cannot_reuse_the_receipt(self):
        self.binary.write_bytes(b"modified")
        with self.assertRaisesRegex(ValueError, "checksum"):
            prebuilt.executable(self.bundle, self.root)
        other = self.root / "other"
        self.binary.rename(other)
        self.binary.symlink_to(other)
        with self.assertRaisesRegex(ValueError, "canonical"):
            prebuilt.executable(self.bundle, self.root)

    def test_missing_extra_or_no_longer_ignored_test_is_rejected_even_with_matching_hash(self):
        for tests, ignored in ((prebuilt.TESTS - prebuilt.IGNORED, prebuilt.IGNORED),
                               (prebuilt.TESTS | {"unexpected"}, prebuilt.IGNORED),
                               (prebuilt.TESTS, set())):
            with self.subTest(tests=tests, ignored=ignored):
                self.write_stub(tests, ignored)
                self.manifest["sha256"] = self.digest()
                self.save()
                with self.assertRaisesRegex(ValueError, "inventory"):
                    prebuilt.executable(self.bundle, self.root)

    def test_failed_test_listing_is_not_an_empty_success(self):
        self.binary.write_text("#!/bin/sh\nexit 8\n")
        self.manifest["sha256"] = self.digest()
        self.save()
        with self.assertRaises(subprocess.CalledProcessError):
            prebuilt.executable(self.bundle, self.root)

    def cargo_message(self):
        return {"reason": "compiler-artifact", "executable": str(self.binary),
                "target": {"name": prebuilt.TARGET, "kind": ["test"]},
                "features": [], "profile": {"test": True}}

    def cargo_output(self, messages):
        original = subprocess.check_output

        def output(command, **kwargs):
            if command[0] == "cargo":
                return "\n".join(json.dumps(message) for message in messages)
            return original(command, **kwargs)
        return patch.object(prebuilt.subprocess, "check_output", side_effect=output)

    def test_build_packages_only_the_compiled_and_enumerated_test_executable(self):
        output = self.root / "built"
        with self.cargo_output([self.cargo_message()]):
            prebuilt.build(output, self.root)
        self.assertEqual(prebuilt.executable(output, self.root), output / prebuilt.TARGET)

    def test_wrong_feature_or_duplicate_cargo_artifact_cannot_be_published(self):
        message = self.cargo_message()
        other = copy.deepcopy(message)
        other["features"] = ["qualification-clock"]
        for messages in ([], [other], [message, message]):
            with self.subTest(messages=messages), self.cargo_output(messages):
                with self.assertRaises(ValueError):
                    prebuilt.build(self.root / "refused", self.root)
            self.assertFalse((self.root / "refused/manifest.json").exists())

    def test_head_change_during_build_cannot_issue_an_artifact_receipt(self):
        moved = dict(self.binding, head="2" * 40)
        with self.cargo_output([self.cargo_message()]), \
                patch.object(prebuilt, "identity", side_effect=[self.binding, moved]):
            with self.assertRaisesRegex(ValueError, "source changed"):
                prebuilt.build(self.root / "refused", self.root)
        self.assertFalse((self.root / "refused/manifest.json").exists())

    def test_shared_producer_preserves_its_workspace_library_feature_closure(self):
        output = self.root / "shared"
        with self.cargo_output([self.cargo_message()]) as cargo:
            prebuilt.build(output, self.root, rust_selection={"mode": "workspace"})
        command = next(call.args[0] for call in cargo.call_args_list if call.args[0][0] == "cargo")
        self.assertIn("--workspace", command)
        self.assertIn("--lib", command)
        manifest = json.loads((output / "manifest.json").read_text())
        self.assertEqual(manifest["selection"], {"mode": "workspace"})
        self.assertIs(manifest["libraries"], True)
        self.assertEqual(prebuilt.executable(output, self.root), output / prebuilt.TARGET)

    def test_package_selection_must_include_the_server_and_remain_exact(self):
        valid = {"mode": "packages", "packages": ["oteryn-gameplay-content", "oteryn-game-server"]}
        self.assertEqual(prebuilt.package_selection(valid)["packages"], sorted(valid["packages"]))
        for invalid in ({"mode": "none"}, {"mode": "packages", "packages": []},
                        {"mode": "packages", "packages": ["other"]},
                        {"mode": "packages", "packages": ["oteryn-game-server", "oteryn-game-server"]}):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                prebuilt.package_selection(invalid)

    def artifact_api(self, *, malformed_member=None, libraries=True):
        self.manifest["libraries"] = libraries
        self.save()
        tar_bytes = io.BytesIO()
        with tarfile.open(fileobj=tar_bytes, mode="w:gz") as archive:
            for name in ("manifest.json", prebuilt.TARGET):
                raw = (self.bundle / name).read_bytes()
                member = tarfile.TarInfo(name)
                member.size = len(raw)
                member.mode = 0o755
                archive.addfile(member, io.BytesIO(raw))
            if malformed_member is not None:
                member = tarfile.TarInfo(malformed_member)
                member.size = 1
                archive.addfile(member, io.BytesIO(b"x"))
        bundle = self.root / "download.zip"
        with zipfile.ZipFile(bundle, "w") as archive:
            archive.writestr("bundle.tar.gz", tar_bytes.getvalue())
        repository = {"id": 51, "full_name": "Oteryn/Oteryn-Game"}
        run = {"id": 101, "run_attempt": 2, "head_sha": self.binding["head"],
               "event": "pull_request", "path": ".github/workflows/merge-gate.yml",
               "repository": repository, "head_repository": repository, "status": "completed"}
        artifact = {"id": 201, "name": "native-entry-room-2", "expired": False,
                    "size_in_bytes": bundle.stat().st_size,
                    "digest": "sha256:" + prebuilt.sha256(bundle),
                    "workflow_run": {"id": 101, "head_sha": self.binding["head"],
                                     "repository_id": 51, "head_repository_id": 51}}
        job = {"id": 301, "name": "Build unit-test archive (lib)", "run_id": 101,
               "run_attempt": 2, "head_sha": self.binding["head"],
               "status": "completed", "conclusion": "success",
               "created_at": "2026-11-21T20:16:47Z", "started_at": "2026-11-21T20:16:49Z",
               "completed_at": "2026-11-21T20:28:00Z", "runner_id": 1000366763,
               "runner_name": "GitHub Actions 1000366763", "runner_group_id": 0,
               "runner_group_name": "GitHub Actions", "labels": ["ubuntu-24.04"],
               "steps": [{"name": "Set up job", "status": "completed", "conclusion": "success",
                          "number": 1, "started_at": "2026-11-21T20:16:51Z",
                          "completed_at": "2026-11-21T20:16:54Z"}]}

        class API:
            runs = [run]
            artifacts = [artifact]
            jobs = [job]
            calls = []
            downloaded = []

            def json(self, path):
                self.calls.append(path)
                page = int(parse_qs(urlparse(path).query).get("page", ["1"])[0])
                start = 100 * (page - 1)
                if "/workflows/" in path:
                    return {"workflow_runs": self.runs}
                if "/artifacts?" in path:
                    return {"total_count": len(self.artifacts), "artifacts": self.artifacts[start:start + 100]}
                if "/jobs?" in path:
                    return {"total_count": len(self.jobs), "jobs": self.jobs[start:start + 100]}
                return run

            def download(self, artifact_id, target):
                if artifact_id != artifact["id"]:
                    raise AssertionError("wrong immutable artifact requested")
                self.downloaded.append(artifact_id)
                target.write_bytes(bundle.read_bytes())

        return API(), run, artifact

    def restore(self, api):
        output = self.root / "restored"
        prebuilt.restore(output, repo_root=self.root, api=api, timeout=0,
                         env={"GITHUB_REPOSITORY": "Oteryn/Oteryn-Game", "EXPECTED_SHA": self.binding["head"]})
        return output

    def test_restores_only_the_exact_immutable_common_producer_artifact(self):
        api, _, _ = self.artifact_api()
        output = self.restore(api)
        self.assertEqual(prebuilt.executable(output, self.root), output / prebuilt.TARGET)

    def test_selected_wrong_head_attempt_digest_or_nonshared_recipe_is_not_a_cache_miss(self):
        for mutation in ("head", "attempt", "digest", "recipe"):
            with self.subTest(mutation=mutation):
                api, run, artifact = self.artifact_api(libraries=mutation != "recipe")
                if mutation == "head":
                    artifact["workflow_run"]["head_sha"] = "2" * 40
                elif mutation == "attempt":
                    run["run_attempt"] = 3
                    api.jobs[0]["run_attempt"] = 3
                    artifact["name"] = "native-entry-room-3"
                    artifact["workflow_run"]["id"] = 999
                elif mutation == "digest":
                    artifact["digest"] = "sha256:" + "0" * 64
                with self.assertRaises(ValueError):
                    self.restore(api)
                self.assertFalse((self.root / "restored").exists())

    def test_no_producer_is_distinct_from_an_invalid_selected_artifact(self):
        api, _, _ = self.artifact_api()
        api.runs = []
        with self.assertRaises(prebuilt.transport().Unavailable):
            self.restore(api)
        self.assertFalse((self.root / "restored").exists())

    def test_unexpected_duplicate_or_traversing_archive_member_is_rejected(self):
        for member in ("../escape", "unexpected", "manifest.json"):
            with self.subTest(member=member):
                api, _, _ = self.artifact_api(malformed_member=member)
                with self.assertRaises(ValueError):
                    self.restore(api)
                self.assertFalse((self.root / "escape").exists())
                self.assertFalse((self.root / "restored").exists())

    def test_changed_producer_attempt_during_wait_is_rejected(self):
        api, run, _ = self.artifact_api()
        api.artifacts = []
        run["status"] = "in_progress"
        api.jobs[0].update(status="in_progress", conclusion=None, completed_at=None)
        original = api.json
        api.json = lambda path: dict(run, run_attempt=3) if path.endswith("/runs/101") else original(path)
        with self.assertRaisesRegex(ValueError, "attempt changed"):
            prebuilt.restore(self.root / "restored", repo_root=self.root, api=api,
                             clock=lambda: 0, sleep=lambda seconds: None, timeout=1,
                             env={"GITHUB_REPOSITORY": "Oteryn/Oteryn-Game",
                                  "EXPECTED_SHA": self.binding["head"]})
        self.assertFalse((self.root / "restored").exists())

    def copied_jobs(self, api):
        # PR #3367's observed partial-rerun chronology and hosted runner sentinel;
        # the logical role/source stay scoped to this isolated executable fixture.
        original = copy.deepcopy(api.jobs[0])
        original.update(id=136971564220, run_attempt=1)
        alias = copy.deepcopy(original)
        alias.update(id=136978455148, run_attempt=2, created_at="2026-11-21T20:42:10Z",
                     runner_group_id=None)
        api.jobs = [original, alias]

    def test_copied_success_uses_the_original_producer_attempt_and_immutable_artifact(self):
        api, _, artifact = self.artifact_api()
        self.copied_jobs(api)
        artifact["name"] = "native-entry-room-1"
        output = self.restore(api)
        self.assertEqual(prebuilt.executable(output, self.root), output / prebuilt.TARGET)
        self.assertEqual(api.downloaded, [artifact["id"]])
        self.assertTrue(any("jobs?filter=all" in path for path in api.calls))

    def test_copied_producer_history_is_resolved_beyond_the_first_job_page(self):
        api, _, artifact = self.artifact_api()
        self.copied_jobs(api)
        original, alias = api.jobs
        api.jobs = [original, *[dict(original, id=400 + i, name=f"Other job {i}")
                                for i in range(99)], alias]
        artifact["name"] = "native-entry-room-1"
        self.assertTrue((self.restore(api) / prebuilt.TARGET).is_file())
        self.assertTrue(any("jobs?filter=all&per_page=100&page=2" in path for path in api.calls))

    def test_new_attempt_during_download_cannot_expose_the_prior_artifact(self):
        api, run, _ = self.artifact_api()
        download = api.download

        def changed(artifact_id, path):
            download(artifact_id, path)
            run["run_attempt"] = 3

        api.download = changed
        with self.assertRaisesRegex(ValueError, "attempt changed during restore"):
            self.restore(api)
        self.assertFalse((self.root / "restored").exists())

    def test_failed_current_producer_cannot_reuse_its_own_published_artifact(self):
        api, _, _ = self.artifact_api()
        api.jobs[0]["conclusion"] = "failure"
        api.jobs[0]["steps"][0]["conclusion"] = "failure"
        with self.assertRaisesRegex(prebuilt.transport().Unavailable, "not running or successful"):
            self.restore(api)
        self.assertFalse(any("/artifacts?" in path for path in api.calls))
        self.assertEqual(api.downloaded, [])
        self.assertFalse((self.root / "restored").exists())

    def test_failure_of_the_same_producer_during_download_cannot_expose_the_artifact(self):
        api, _, artifact = self.artifact_api()
        download = api.download

        def failed(artifact_id, path):
            download(artifact_id, path)
            api.jobs[0]["conclusion"] = "failure"
            api.jobs[0]["steps"][0]["conclusion"] = "failure"

        api.download = failed
        with self.assertRaisesRegex(ValueError, "no longer running or successful"):
            self.restore(api)
        self.assertEqual(api.downloaded, [artifact["id"]])
        self.assertFalse((self.root / "restored").exists())

    def test_unproved_copy_is_rejected_instead_of_reusing_or_silently_falling_back(self):
        for mutation in ("missing-original", "changed-runner"):
            with self.subTest(mutation=mutation):
                api, _, artifact = self.artifact_api()
                self.copied_jobs(api)
                artifact["name"] = "native-entry-room-1"
                if mutation == "missing-original":
                    api.jobs = api.jobs[1:]
                else:
                    api.jobs[-1]["runner_name"] = "different-runner"
                with self.assertRaises(ValueError):
                    self.restore(api)
                self.assertEqual(api.downloaded, [])
                self.assertFalse((self.root / "restored").exists())

    def test_new_real_failed_or_incomplete_producer_never_reuses_an_older_archive(self):
        for status in ("completed", "in_progress", "queued"):
            with self.subTest(status=status):
                api, run, artifact = self.artifact_api()
                old = dict(api.jobs[0], id=300, run_attempt=1)
                latest = dict(api.jobs[0], status=status, created_at="2026-11-21T20:42:10Z",
                              started_at=None if status == "queued" else "2026-11-21T20:42:13Z",
                              completed_at="2026-11-21T20:43:00Z" if status == "completed" else None,
                              conclusion="failure" if status == "completed" else None)
                latest["steps"] = [dict(step, started_at=latest["started_at"],
                                        completed_at=latest["completed_at"], conclusion="failure")
                                   for step in latest["steps"]] if status == "completed" else []
                api.jobs = [old, latest]
                run["status"] = "in_progress"
                artifact["name"] = "native-entry-room-1"
                with self.assertRaises(prebuilt.transport().Unavailable):
                    self.restore(api)
                self.assertEqual(api.downloaded, [])
                self.assertFalse((self.root / "restored").exists())


class NativeEntryProcessControl(unittest.TestCase):
    """Exercise Bash ordering and cleanup with local stub commands; no Docker/Rust."""

    def run_fixture(self, issuer_fails):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        scripts = root / "tools/qualification/native_entry_room"
        scripts.mkdir(parents=True)
        (root / "apps/game-server").mkdir(parents=True)
        (root / "_platform").mkdir()
        (root / "tmp").mkdir()
        shutil.copyfile(Path(__file__).with_name("run.sh"), scripts / "run.sh")
        commands = root / "commands"
        commands.mkdir()

        def script(path, source):
            path.write_text("#!/usr/bin/env python3\n" + textwrap.dedent(source))
            path.chmod(0o700)

        prefix = f"from pathlib import Path\nroot = Path({str(root)!r})\n"
        script(commands / "git", 'print("eb65ce453949a352fdc0e3ed0132d2cde0084fb7")\n')
        script(commands / "docker", prefix + textwrap.dedent(f"""
            import sys, time
            deadline = time.monotonic() + 5
            while not (root / 'build-started').exists():
                if time.monotonic() > deadline: raise SystemExit(20)
                time.sleep(0.01)
            if sys.argv[1] == 'build':
                print('fixture-image')
            else:
                if {issuer_fails!r}: raise SystemExit(9)
                (root / 'issued').write_text('issued')
                print('fixture-issued-receipt')
        """))
        binary = textwrap.dedent(f"""
            #!/usr/bin/env python3
            import json, os
            from pathlib import Path
            assert os.environ['OTERYN_NATIVE_ENTRY_TOPOLOGY_RECEIPT'] == 'fixture-issued-receipt'
            Path({str(root / 'executed')!r}).write_text(str(Path.cwd()))
        """).lstrip()
        script(scripts / "prebuilt.py", prefix + textwrap.dedent(f"""
            import signal, sys, time
            def stopped(signum, frame):
                (root / 'build-stopped').write_text('stopped')
                raise SystemExit(143)
            if sys.argv[1] == 'build':
                signal.signal(signal.SIGTERM, stopped)
                (root / 'build-started').write_text('started')
                deadline = time.monotonic() + 5
                while not (root / 'issued').exists():
                    if time.monotonic() > deadline: raise SystemExit(21)
                    time.sleep(0.01)
                directory = Path(sys.argv[2]); directory.mkdir()
                target = directory / 'fixture-test'
                target.write_text({binary!r}); target.chmod(0o700)
            else:
                print(Path(sys.argv[2]) / 'fixture-test')
        """))
        environment = dict(os.environ, PATH=str(commands) + os.pathsep + os.environ["PATH"],
                           RUNNER_TEMP=str(root / "tmp"), PLATFORM_SOURCE=str(root / "_platform"),
                           OTERYN_NATIVE_ENTRY_PARALLEL_BUILD="1")
        environment.pop("OTERYN_NATIVE_ENTRY_PREBUILT", None)
        result = subprocess.run(["bash", str(scripts / "run.sh")], cwd=root, env=environment,
                                capture_output=True, text=True, timeout=10)
        return root, result

    def test_fallback_build_overlaps_issuer_and_retains_receipt_and_cargo_working_directory(self):
        root, result = self.run_fixture(False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("NATIVE_ENTRY_ROOM_RESULT=PASS", result.stdout)
        self.assertEqual((root / "executed").read_text(), str(root / "apps/game-server"))
        self.assertEqual(list((root / "tmp").iterdir()), [])

    def test_issuer_failure_stops_the_background_build_and_removes_its_directory(self):
        root, result = self.run_fixture(True)
        self.assertEqual(result.returncode, 9, result.stderr)
        self.assertNotIn("NATIVE_ENTRY_ROOM_RESULT=PASS", result.stdout)
        self.assertEqual((root / "build-stopped").read_text(), "stopped")
        self.assertEqual(list((root / "tmp").iterdir()), [])


if __name__ == "__main__":
    unittest.main()
