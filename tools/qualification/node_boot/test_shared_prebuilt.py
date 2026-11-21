#!/usr/bin/env python3
"""Shared qualification builds preserve variant and producer custody."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest
from unittest.mock import patch

import matrix
import prebuilt


class SharedBuilds(unittest.TestCase):
    def test_pr_predecessor_fallback_requires_both_known_digests(self):
        workflow = (prebuilt.ROOT / ".github/workflows/pr-postgres-harness.yml").read_text()
        step = workflow.split("      - name: Verify qualification artifact support\n", 1)[1]
        body = step.split("        run: |\n", 1)[1].split("      - name: Test the shard planner\n", 1)[0]
        # The historical pair remains immutable when the current producer pins rotate.
        helper_digest = "56029b4b5706f4983c262d65982abcbe029eaff9c30c408bd583561639722ed7"
        build_digest = "7135b31f6acc0ebc2626b80823f0a67c7afed41ef509900ad3b14437dce1c373"
        stub = '''sha256sum() {
  local expected path actual
  if [[ "$1" == -c ]]; then read -r expected path; else path="$1"; fi
  case "$path" in
    tools/qualification/node_boot/prebuilt.py) actual="$FIXTURE_HELPER_DIGEST" ;;
    .github/workflows/node-boot-build.yml) actual="$FIXTURE_BUILD_DIGEST" ;;
    *) return 9 ;;
  esac
  if [[ "$1" == -c ]]; then [[ "$expected" == "$actual" ]]; else printf '%s  %s\\n' "$actual" "$path"; fi
}
'''
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name, value in (("tools/qualification/node_boot/run.sh", "NODE_BOOT_PREBUILT_DIR"),
                                ("tools/qualification/node_boot/prebuilt.py", "fixture"),
                                (".github/workflows/node-boot-build.yml", "fixture")):
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(value)
            output = root / "outputs"
            for helper, build, code in ((helper_digest, build_digest, 0),
                                        ("0" * 64, build_digest, 1), (helper_digest, "0" * 64, 1)):
                with self.subTest(helper=helper, build=build):
                    output.unlink(missing_ok=True)
                    executed = subprocess.run(["bash", "-c", stub + textwrap.dedent(body)], cwd=root,
                        env={"PATH": os.defpath, "GITHUB_OUTPUT": str(output),
                             "FIXTURE_HELPER_DIGEST": helper, "FIXTURE_BUILD_DIGEST": build},
                        capture_output=True, text=True, timeout=10, check=False)
                    self.assertEqual(executed.returncode, code, executed.stderr)
                    if code == 0:
                        self.assertEqual(output.read_text(), "mode=legacy\n")
                    else:
                        self.assertFalse(output.exists())

    def test_main_summary_accepts_only_intentionally_skipped_default_build(self):
        workflow = (prebuilt.ROOT / ".github/workflows/merge-group-gate.yml").read_text()
        step = workflow.split("      - name: Require every node boot shard\n", 1)[1]
        body = step.split("        run: |\n", 1)[1].split("\n  server_seam:", 1)[0]
        script = 'python3() { echo "current shard evidence"; return "$SUMMARY_STATUS"; }\n' + textwrap.dedent(body)
        with tempfile.TemporaryDirectory() as temporary:
            environment = {
                "PATH": os.defpath, "RUNNER_TEMP": temporary, "GITHUB_STEP_SUMMARY": temporary + "/summary",
                "PLAN_RESULT": "success", "BUILD_RESULT": "success", "PLATFORM_RESULT": "success",
                "WORLD_RESULT": "success",
                "SHARD_RESULT": "success", "DEFAULT_REQUIRED": "false", "DEFAULT_RESULT": "skipped",
                "SHARDS": "[]", "PREFIX": "current-", "SUMMARY_STATUS": "0",
            }
            for changes, expected in (({}, 0), ({"DEFAULT_REQUIRED": "true", "DEFAULT_RESULT": "success"}, 0),
                                      ({"DEFAULT_REQUIRED": "true"}, 1), ({"DEFAULT_RESULT": "failure"}, 1),
                                      ({"PLATFORM_RESULT": "failure"}, 1), ({"SHARD_RESULT": "failure"}, 1),
                                      ({"WORLD_RESULT": "failure"}, 1), ({"WORLD_RESULT": "skipped"}, 1),
                                      ({"SUMMARY_STATUS": "1"}, 1)):
                with self.subTest(changes=changes):
                    result = subprocess.run(["bash", "-c", script], env={**environment, **changes},
                                            capture_output=True, text=True, timeout=10, check=False)
                    self.assertEqual(result.returncode, expected, result.stderr)

    def test_shipping_default_reuses_only_verified_seam_library_without_rebuilding_it(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            executable = root / "binary"
            executable.write_bytes(b"shipping libtest")
            executable.chmod(0o700)
            def identity(variant, _platform=None):
                return {"head": "a" * 40, "workspace": str(root), "variant": variant,
                        "toolchain": "1.100.0", "features": [],
                        "profile": "test" if variant == "server-seam" else "dev+test"}
            listing = "\n".join(f"qualification::{name}: test" for name in (
                "server_seam_real_owners_over_tcp_tls", "node_boot_seam_against_running_node",
                "node_boot_spells_against_running_node",
            ))
            with patch.object(prebuilt, "identity", side_effect=identity), patch.object(
                prebuilt.subprocess, "check_output", return_value=listing,
            ):
                with patch.object(prebuilt, "cargo_executables", return_value={"oteryn_game_server": executable}):
                    prebuilt.build("server-seam", root / "seam-build")
                prebuilt.unpack("server-seam", root / "seam-build/bundle.tar.gz", root / "seam")
                with patch.object(prebuilt, "cargo_executables", return_value={name: executable for name in prebuilt.SERVER_BINS}) as cargo:
                    prebuilt.build("default", root / "default-build", server_seam_source=root / "seam")
                    self.assertEqual(cargo.call_count, 1)
                    self.assertEqual(cargo.call_args.args[1], ("build", "--bins"))
                prebuilt.unpack("default", root / "default-build/bundle.tar.gz", root / "result")
                self.assertEqual((root / "result/default" / prebuilt.LIBTEST).read_bytes(), executable.read_bytes())
                manifest = root / "seam/server-seam/manifest.json"
                valid = manifest.read_text()
                for mutation in ("profile", "digest", "missing"):
                    with self.subTest(mutation=mutation), patch.object(prebuilt, "cargo_executables") as cargo:
                        manifest.write_text(valid)
                        library = root / "seam/server-seam" / prebuilt.LIBTEST
                        library.write_bytes(executable.read_bytes())
                        library.chmod(0o700)
                        if mutation == "profile":
                            value = json.loads(valid)
                            value["profile"] = "release"
                            manifest.write_text(json.dumps(value))
                        elif mutation == "digest":
                            library.write_bytes(b"changed")
                        else:
                            library.unlink()
                        with self.assertRaises(ValueError):
                            prebuilt.build("default", root / "rejected", server_seam_source=root / "seam")
                        cargo.assert_not_called()

    def test_seam_library_cannot_replace_a_different_feature_variant(self):
        with patch.object(prebuilt, "cargo_executables") as cargo:
            with self.assertRaisesRegex(ValueError, "default"):
                prebuilt.build("qualification-clock", Path("unused"), server_seam_source=Path("unused"))
            cargo.assert_not_called()

    def test_native_basic_cannot_select_the_shipping_binary(self):
        self.assertEqual(prebuilt.select(matrix.plan("pr")[0]), ("qualification-spawn", False, False))
        self.assertEqual(prebuilt.select(matrix.plan("custom", game_platform=True)[0]), ("default", False, True))

    def test_world_build_contains_the_spawn_proof_before_publication(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            executable = root / "binary"
            executable.write_bytes(b"binary")
            executable.chmod(0o700)
            identity = {"head": "a" * 40, "workspace": str(root), "variant": "qualification-clock"}
            listing = "\n".join((
                "qualification::node_boot_seam_against_running_node: test",
                "qualification::node_boot_spells_against_running_node: test",
            ))
            with patch.object(prebuilt, "identity", return_value=identity), patch.object(
                prebuilt, "cargo_executables", side_effect=[
                    {name: executable for name in prebuilt.SERVER_BINS}, {"oteryn_game_server": executable},
                ],
            ), patch.object(prebuilt.subprocess, "check_output", return_value=listing):
                with self.assertRaisesRegex(ValueError, "spawn"):
                    prebuilt.build("qualification-clock", root / "output")
            self.assertFalse((root / "output/bundle.tar.gz").exists())

    def test_seam_build_is_a_lib_only_shipping_variant(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            executable = root / "binary"
            executable.write_bytes(b"seam libtest")
            executable.chmod(0o700)
            identity = {
                "head": "a" * 40, "workspace": str(root), "variant": "server-seam",
                "toolchain": "1.100.0", "profile": "test", "features": [],
            }
            with patch.object(prebuilt, "identity", return_value=identity), patch.object(
                prebuilt, "cargo_executables", return_value={"oteryn_game_server": executable},
            ) as cargo, patch.object(prebuilt.subprocess, "check_output", return_value=
                    "qualification::server_seam_real_owners_over_tcp_tls: test\n"):
                prebuilt.build("server-seam", root / "output")
                self.assertEqual(cargo.call_count, 1)
                self.assertEqual(cargo.call_args.args[1], ("test", "--lib", "--no-run"))
                prebuilt.unpack("server-seam", root / "output/bundle.tar.gz", root / "unpacked")
                prebuilt.verify("server-seam", root / "unpacked")
                manifest_path = root / "unpacked/server-seam/manifest.json"
                valid = json.loads(manifest_path.read_text())
                for changed in ({"head": "b" * 40}, {"features": ["qualification-spawn"]},
                                {"profile": "release"}, {"toolchain": "other"}):
                    manifest_path.write_text(json.dumps({**valid, **changed}))
                    with self.assertRaisesRegex(ValueError, "checkout|variant"):
                        prebuilt.verify("server-seam", root / "unpacked")
                manifest_path.write_text(json.dumps(valid))
                (root / "unpacked/server-seam" / prebuilt.LIBTEST).unlink()
                with self.assertRaisesRegex(ValueError, "executable"):
                    prebuilt.verify("server-seam", root / "unpacked")

    def test_platform_identity_rejects_another_revision_or_dirty_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            platform = root / "_platform"
            platform.mkdir()
            pin = root / "tools/qualification/game_platform/lifecycle.sh"
            pin.parent.mkdir(parents=True)
            pin.write_text("readonly GAME_PLATFORM_SHA=" + "c" * 40 + "\n")
            with patch.object(prebuilt, "ROOT", root):
                for actual, dirty in (("d" * 40, ""), ("c" * 40, "portal-rs/Cargo.toml\n")):
                    def git(command, **kwargs):
                        cwd = Path(kwargs.get("cwd", root))
                        if "--show-toplevel" in command:
                            return str(cwd) + "\n"
                        if "diff" in command:
                            return dirty
                        return (actual if cwd == platform else "a" * 40) + "\n"
                    with patch.object(prebuilt.subprocess, "check_output", side_effect=git):
                        with self.assertRaisesRegex(ValueError, "Platform"):
                            prebuilt.identity("platform", platform)

    def test_platform_archive_binds_both_sources_profile_and_executable(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            executable = root / "portal"
            executable.write_bytes(b"portal")
            executable.chmod(0o700)
            identity = {
                "head": "a" * 40, "workspace": str(root), "variant": "platform",
                "toolchain": "1.100.0", "profile": "dev", "features": [],
                "platform_head": "c" * 40, "platform_workspace": str(root / "_platform"),
            }
            with patch.object(prebuilt, "identity", return_value=identity), patch.object(
                prebuilt, "cargo_executables", return_value={prebuilt.PORTAL: executable},
            ) as cargo:
                prebuilt.build("platform", root / "output")
                self.assertEqual(cargo.call_args.args[1], ("build", "--manifest-path",
                    str(root / "_platform/portal-rs/Cargo.toml"), "--bin", "oteryn-portal"))
                prebuilt.unpack("platform", root / "output/bundle.tar.gz", root / "unpacked")
                prebuilt.verify("platform", root / "unpacked")
                path = root / "unpacked/platform/manifest.json"
                valid = json.loads(path.read_text())
                for changed in ({"head": "b" * 40}, {"platform_head": "d" * 40}, {"profile": "release"}):
                    path.write_text(json.dumps({**valid, **changed}))
                    with self.assertRaisesRegex(ValueError, "checkout|variant"):
                        prebuilt.verify("platform", root / "unpacked")
                path.write_text(json.dumps(valid))
                (path.parent / prebuilt.PORTAL).write_bytes(b"modified portal")
                with self.assertRaisesRegex(ValueError, "executable"):
                    prebuilt.verify("platform", root / "unpacked")


if __name__ == "__main__":
    unittest.main()
