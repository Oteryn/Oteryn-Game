"""Exercise the N07d verifier's pass and failure paths with disposable stubs."""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import unittest


class N07dRunnerTests(unittest.TestCase):
    def test_diagnostic_requires_fresh_capture_and_one_successful_named_replay(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            runner = root / "tools/qualification/e2e_all/run_n07d.sh"
            runner.parent.mkdir(parents=True)
            shutil.copyfile(Path(__file__).with_name(runner.name), runner)
            boot = runner.parent.parent / "node_boot/cloud_run.sh"
            boot.parent.mkdir()
            boot.write_text(
                'set -eu\n'
                'test "$OTERYN_E2E_ROW_BATCH" = 226:1\n'
                'test -z "${RUNNER_TEMP+x}"\n'
                'dir=$(dirname "$E2E_ALL_CAPTURE_DIR")\n'
                'echo "Node boot logs are in $dir"\n'
                'printf "%s\\n" "NODE_BOOT_RESULT=NODE_BOOT_PASS mode=unprivileged" > "$dir/1-rows.log"\n'
                'if [ "$CASE" != skipped ]; then\n'
                '  echo "SEAM_EVIDENCE e2e_all scenario=N07d.client status=MISSING-CLIENT seconds=0.1 evidence=exact-minimap-replay-required;" >> "$dir/1-rows.log"\n'
                'fi\n'
                'if [ "$CASE" != missing ]; then\n'
                '  echo snapshot > "$E2E_ALL_CAPTURE_DIR/n07d-snapshot.bin"\n'
                '  echo expected > "$E2E_ALL_CAPTURE_DIR/n07d-expected.txt"\n'
                'fi\n'
            )
            cargo = root / "cargo"
            cargo.write_text(
                '#!/bin/bash\n'
                'test -s "$E2E_ALL_CAPTURE_DIR/n07d-snapshot.bin" || exit 1\n'
                'case "$CASE" in\n'
                '  failed) exit 1 ;;\n'
                '  zero) echo "test result: ok. 0 passed; 0 failed; 0 ignored;" ;;\n'
                '  *) echo "test $6 ... ok"; echo "test result: ok. 1 passed; 0 failed; 0 ignored;" ;;\n'
                'esac\n'
            )
            cargo.chmod(0o700)
            captures = set()
            for case in ("success", "success", "missing", "skipped", "failed", "zero"):
                with self.subTest(case=case):
                    result = subprocess.run(
                        ["bash", str(runner)],
                        env={**os.environ, "CASE": case, "CARGO": str(cargo), "RUNNER_TEMP": "/ignored"},
                        capture_output=True,
                        text=True,
                    )
                    line = result.stderr.splitlines()[0]
                    output = Path(line.removeprefix("N07d evidence is in "))
                    self.addCleanup(shutil.rmtree, output)
                    self.assertNotIn(output, captures)
                    captures.add(output)
                    self.assertEqual(result.returncode == 0, case == "success", result)
                    self.assertNotIn("scenario=N07d.client status=PASS", result.stdout)
                    self.assertEqual("exact-terrain-replay-passed" in result.stdout, case == "success")
