"""Artifact consumers execute verified binaries without rebuilding or sharing state."""

import json
from pathlib import Path
import re
import shlex
import subprocess
import tempfile
import unittest

from prepare import portal_binary

HERE = Path(__file__).resolve().parent


def function(path, name):
    return re.search(rf"(?ms)^{name}\(\) \{{.*?^\}}", path.read_text()).group(0)


class ArtifactConsumers(unittest.TestCase):
    def test_platform_uses_the_verified_binary_and_rejects_failed_verification(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            binary = root / "prebuilt/platform/oteryn-portal"
            binary.parent.mkdir(parents=True)
            binary.write_text("#!/bin/sh\nexit 0\n")
            binary.chmod(0o700)
            body = function(HERE / "lifecycle.sh", "game_platform_binary")
            script = f'''set -Eeuo pipefail
WORK={shlex.quote(str(root))}
GAME_SOURCE=/game
PLATFORM_SOURCE=/game/_platform
GAME_PLATFORM_PREBUILT_DIR={shlex.quote(str(root / 'prebuilt'))}
cargo() {{ echo unexpected-cargo >&2; return 91; }}
game_platform_fail() {{ echo "$*" >&2; exit 92; }}
python3() {{
  if [[ "$1" == /game/tools/qualification/node_boot/prebuilt.py ]]; then
    [[ "$2 $3" == 'verify platform' && "$5" == --platform-source && "$6" == "$PLATFORM_SOURCE" ]]
    return "$VERIFY_RESULT"
  fi
  command python3 "$@"
}}
''' + body + "\ngame_platform_binary\n"
            for result in (0, 1):
                with self.subTest(verification=result):
                    metadata = root / "portal-meta.json"
                    metadata.unlink(missing_ok=True)
                    executed = subprocess.run(["bash", "-c", f"VERIFY_RESULT={result}\n" + script],
                                              capture_output=True, text=True, timeout=10, check=False)
                    self.assertNotIn("unexpected-cargo", executed.stderr)
                    if result == 0:
                        self.assertEqual(executed.returncode, 0, executed.stderr)
                        self.assertEqual(portal_binary(root), binary)
                    else:
                        self.assertEqual(executed.returncode, 92)
                        self.assertFalse(metadata.exists())

    def test_platform_missing_or_nonexecutable_artifact_cannot_be_launched(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / "portal"
            (root / "portal-meta.json").write_text(json.dumps({"executable": str(binary)}))
            for exists in (False, True):
                if exists:
                    binary.write_text("not executable")
                with self.assertRaisesRegex(ValueError, "real_portal_binary_missing"):
                    portal_binary(root)

    def test_seam_uses_exact_libtest_with_cargo_package_working_directory(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            package = root / "apps/game-server"
            package.mkdir(parents=True)
            binary = root / "prebuilt/server-seam/node-boot-libtest"
            binary.parent.mkdir(parents=True)
            binary.write_text('#!/bin/sh\nprintf "%s\\n" "$PWD" "$@"\n')
            binary.chmod(0o700)
            body = function(HERE.parent / "wp5_s3b/run.sh", "prepare_seam_test")
            setup = f'''set -Eeuo pipefail
GAME_SOURCE={shlex.quote(str(root))}
SERVER_SEAM_PREBUILT_DIR={shlex.quote(str(root / 'prebuilt'))}
TEST_TARGET=(--lib)
TEST_NAME=qualification::server_seam_real_owners_over_tcp_tls
cargo() {{ echo unexpected-cargo >&2; return 91; }}
python3() {{ [[ "$2 $3" == 'verify server-seam' ]]; return "$VERIFY_RESULT"; }}
''' + body + '\nprepare_seam_test\ncd "$SEAM_TEST_CWD"\n"${SEAM_TEST_COMMAND[@]}"\n'
            for mode, verify in (("seam", 0), ("spell-seam", 0), ("s3b", 0), ("seam", 1)):
                with self.subTest(mode=mode, verification=verify):
                    executed = subprocess.run(["bash", "-c", f"WP5_QUALIFICATION={mode}\nVERIFY_RESULT={verify}\n" + setup],
                                              capture_output=True, text=True, timeout=10, check=False)
                    self.assertNotIn("unexpected-cargo", executed.stderr)
                    if mode != "s3b" and verify == 0:
                        self.assertEqual(executed.returncode, 0, executed.stderr)
                        self.assertEqual(executed.stdout.splitlines(), [str(package),
                            "qualification::server_seam_real_owners_over_tcp_tls", "--ignored", "--exact", "--nocapture"])
                    else:
                        self.assertEqual(executed.returncode, 2)
                        self.assertNotIn(str(package), executed.stdout)


if __name__ == "__main__":
    unittest.main()
