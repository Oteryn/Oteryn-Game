"""Exercise the real lifecycle result writer: later failures dominate blockers."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class ResultPrecedence(unittest.TestCase):
    def test_failure_overrides_every_prior_outcome(self):
        lifecycle = Path(__file__).with_name("lifecycle.sh")
        for reason in ("node_exited_during_native", "journey_assertion_failed", "blocker_result_missing"):
            for previous in ("", "GAME_PLATFORM_RESULT=PASS\n", "GAME_PLATFORM_RESULT=BLOCKED owner=Platform what=source:1\n", "GAME_PLATFORM_RESULT=FAIL flow=earlier reason=earlier\n"):
                with self.subTest(reason=reason, previous=previous), tempfile.TemporaryDirectory() as directory:
                    result = Path(directory) / "result"
                    if previous:
                        result.write_text(previous)
                    completed = subprocess.run(
                        ["bash", "-c", 'source "$1"; game_platform_fail native_device_rotation "$2"', "bash", str(lifecycle), reason],
                        env={**os.environ, "GAME_PLATFORM_RESULT_FILE": str(result)},
                        capture_output=True, check=False,
                    )
                    self.assertEqual(completed.returncode, 1)
                    self.assertEqual(completed.stdout, b"")
                    self.assertEqual(result.read_text(), f"GAME_PLATFORM_RESULT=FAIL flow=native_device_rotation reason={reason}\n")


if __name__ == "__main__":
    unittest.main()
