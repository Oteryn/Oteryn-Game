#!/usr/bin/env python3
"""Exercise artifact custody and Node boot modes without invoking Cargo."""

import hashlib
import io
import json
from pathlib import Path
import shlex
import tarfile
import tempfile
import textwrap
import unittest
from unittest.mock import patch

import matrix
import prebuilt
from test_node_boot_tools import bash, function


class PrebuiltTest(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.identity = lambda variant: {"head": "a" * 40, "workspace": str(self.root), "variant": variant}
        identity = patch.object(prebuilt, "identity", side_effect=self.identity)
        identity.start()
        self.addCleanup(identity.stop)

    def test_reference_and_pr_cover_both_clocks_and_the_world_compiler(self):
        for set_name in ("reference", "pr", "full"):
            with self.subTest(set_name=set_name):
                self.assertEqual(prebuilt.plan(matrix.plan(set_name)), list(prebuilt.BUNDLES))
        self.assertEqual(prebuilt.select(matrix.plan("reference")[0]), ("qualification-clock", True, True))
        pr = matrix.plan("pr")
        self.assertEqual(prebuilt.select(pr[0]), ("default", False, False))
        for shard in pr[1:]:
            self.assertEqual(prebuilt.select(shard), ("qualification-clock", False, True))

    def test_custom_spells_and_portal_use_the_ordinary_clock(self):
        spells = matrix.plan("custom", spells=True)
        self.assertEqual(prebuilt.select(spells[0]), ("default", False, False))
        self.assertEqual(prebuilt.plan(spells), ["default"])
        portal = matrix.plan("custom", game_platform=True)
        self.assertFalse(portal[0]["world"])
        self.assertEqual(prebuilt.select(portal[0]), ("default", False, True))
        self.assertEqual(prebuilt.plan(portal), ["default", "world-compiler"])
        self.assertEqual(prebuilt.plan(matrix.plan("custom", rows="0:5")), ["qualification-clock", "world-compiler"])

    def test_build_and_unpack_do_not_reuse_wrong_variant_or_tampered_binary(self):
        variant = "qualification-clock"
        path = self.root / "fake"
        path.write_bytes(b"compiled executable")
        path.chmod(0o700)
        listing = ("qualification::node_boot_seam_against_running_node: test\n"
                   "qualification::node_boot_spells_against_running_node: test\n")
        binaries = {name: path for name in prebuilt.SERVER_BINS}
        with patch.object(prebuilt, "cargo_executables", side_effect=[binaries, {"oteryn_game_server": path}]) as cargo:
            with patch.object(prebuilt.subprocess, "check_output", return_value=listing):
                prebuilt.build(variant, self.root / "build")
        for invocation in cargo.call_args_list:
            self.assertEqual(invocation.args[1][-2:], ("--features", "qualification-clock"))
        bundle = self.root / "build/bundle.tar.gz"
        unpacked = self.root / "unpacked"
        with self.assertRaisesRegex(ValueError, "variant"):
            prebuilt.unpack("default", bundle, unpacked)
        prebuilt.unpack(variant, bundle, unpacked)
        prebuilt.verify(variant, unpacked)
        with self.assertRaisesRegex(ValueError, "destination"):
            prebuilt.unpack(variant, bundle, unpacked)
        (unpacked / variant / prebuilt.LIBTEST).write_bytes(b"tampered")
        with self.assertRaisesRegex(ValueError, prebuilt.LIBTEST):
            prebuilt.verify(variant, unpacked)

    def test_cargo_library_artifact_must_be_a_unique_test_executable(self):
        executable = self.root / "oteryn_game_server-test"
        executable.touch(mode=0o700)
        candidate = {
            "reason": "compiler-artifact", "target": {"kind": ["lib"], "name": "oteryn_game_server"},
            "executable": str(executable), "profile": {"test": True},
        }
        library = {**candidate, "profile": {"test": False}}
        native = {**candidate, "target": {**candidate["target"], "kind": ["bin"]}}
        args = ("oteryn-game-server", ("test", "--lib", "--no-run"), "lib", ("oteryn_game_server",))
        with patch.object(prebuilt.subprocess, "check_output", return_value="\n".join(map(json.dumps, (library, native, candidate)))):
            self.assertEqual(prebuilt.cargo_executables(*args, test=True), {"oteryn_game_server": executable})
        with patch.object(prebuilt.subprocess, "check_output", return_value="\n".join(map(json.dumps, (candidate, candidate)))):
            with self.assertRaisesRegex(ValueError, "invalid Cargo executable"):
                prebuilt.cargo_executables(*args, test=True)

    def test_prebuilt_seam_runs_exact_ignored_libtest_with_package_cwd_and_env(self):
        package = self.root / "apps/game-server"
        package.mkdir(parents=True)
        executable = self.root / prebuilt.LIBTEST
        executable.write_text('#!/bin/sh\nprintf "%s\\n" "$PWD" "$NODE_BOOT_E2E_ONLY" "$@"\n')
        executable.chmod(0o700)
        script = textwrap.dedent(f'''\
            set -Eeuo pipefail
            sleep() {{ :; }}
            GAME_SOURCE={shlex.quote(str(self.root))}
            NODE_BOOT_LIBTEST={shlex.quote(str(executable))}
            NODE_BOOT_SPELLS=0; NODE_BOOT_WORLD=0; NODE_GAMEPLAY_ENV=(); NODE_CLOCK_FEATURES=()
            BASE=/srv/x; WP5_PKI=/x; ADMIN_URL=u; WORLD_ID=w; CHANNEL_ID=c; WP5_PORT=1
            ACCOUNT_ID=a; SECOND_ACCOUNT_ID=b; FRESH_KEY_ID=f; FRESH_SEED_HEX=s
            RECOVERY_KEY_ID=r; RECOVERY_SEED_HEX=t
            NODE_BOOT_E2E_ONLY=f6,core
            ''') + function("seam_stages") + "\nseam_stages\n"
        result = bash(script)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(), [
            str(package), "f6,core", "qualification::node_boot_seam_against_running_node",
            "--ignored", "--exact", "--nocapture",
        ])

    def forge_archive(self, manifest, filename, payload, *, kind=tarfile.REGTYPE):
        archive_path = self.root / "bad.tar.gz"
        with tarfile.open(archive_path, "w:gz") as archive:
            metadata = json.dumps(manifest).encode()
            header = tarfile.TarInfo(prebuilt.MANIFEST)
            header.size = len(metadata)
            archive.addfile(header, io.BytesIO(metadata))
            member = tarfile.TarInfo(filename)
            member.type = kind
            member.mode = 0o755
            member.size = len(payload)
            archive.addfile(member, io.BytesIO(payload))
        return archive_path

    def test_unpack_rejects_escape_links_tampering_and_wrong_checkout(self):
        variant = "world-compiler"
        payload = b"executable"
        manifest = {**self.identity(variant), "sha256": {prebuilt.COMPILER: hashlib.sha256(payload).hexdigest()}}
        unpacked = self.root / "unpacked"
        for filename, content, kind, failure in (
            ("../outside", payload, tarfile.REGTYPE, "invalid Node boot archive member"),
            (prebuilt.COMPILER, b"", tarfile.SYMTYPE, "invalid Node boot archive member"),
            (prebuilt.COMPILER, b"other", tarfile.REGTYPE, "checksum mismatch"),
        ):
            with self.subTest(filename=filename, kind=kind, failure=failure):
                archive = self.forge_archive(manifest, filename, content, kind=kind)
                with self.assertRaisesRegex(ValueError, failure):
                    prebuilt.unpack(variant, archive, unpacked)
                self.assertFalse((unpacked / variant).exists())
        self.assertFalse((self.root / "outside").exists())
        for binding in ({"head": "b" * 40}, {"workspace": "/different/game"}):
            archive = self.forge_archive({**manifest, **binding}, prebuilt.COMPILER, payload)
            with self.assertRaisesRegex(ValueError, "checkout"):
                prebuilt.unpack(variant, archive, unpacked)
        link = self.root / "redirected"
        link.symlink_to(unpacked, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "not canonical"):
            prebuilt.unpack(variant, archive, link)


if __name__ == "__main__":
    unittest.main()
