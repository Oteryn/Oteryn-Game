"""Secrets never survive retention; fixed assertion reasons remain reviewable."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from redact import redact

REASON = "enrollment_transport_or_payload_failed"


class Diagnostics(unittest.TestCase):
    def test_each_secret_class_is_removed(self):
        for index, secret_line in enumerate([
            "Authorization: Bearer bearer-secret",
            "Authorization: OterynDevice device-secret",
            "otd1." + "x" * 43,
            '{"code":"oauth-code", "access_token":"oauth-token", "refresh_token":"refresh-secret"}',
            "Cookie: session=cookie-secret; other=another-secret",
            "Set-Cookie: session=cookie-secret; HttpOnly",
            "password=password-secret",
            "postgresql://user:database-secret@localhost/private",
            "postgres://user:database-secret@localhost/private",
            '"password":\n"multiline-secret"',
            "unlabeled-opaque-secret",
            'Error: "bearer_secret"',
        ]):
            with self.subTest(secret_class=index):
                result = redact(f'Error: "{REASON}"\n{secret_line}\n', {REASON})
                self.assertEqual(result.splitlines()[0], f'Error: "{REASON}"')
                self.assertTrue(all(line == "[REDACTED]" for line in result.splitlines()[1:]))

    def test_fixed_sqlstate_reason_survives(self):
        line = 'Error: "postgres_assertion_query_failed:device_state:42P01"'
        self.assertEqual(redact(line, {"postgres_assertion_query_failed", "device_state"}), line + "\n")

    def test_failure_retains_each_step_log(self):
        root = Path(__file__).resolve().parents[3]
        for name in ("identity.log", "native_ticket_and_gateway_fail_closed.log", "native_device_rotation_replay.log"):
            with self.subTest(step=name), tempfile.TemporaryDirectory() as directory:
                work = Path(directory)
                diagnostics = work / "diagnostics"
                diagnostics.mkdir(mode=0o700)
                (work / name).write_text(f'Error: "{REASON}"\nAuthorization: Bearer private-value\n')
                completed = subprocess.run(
                    ["bash", "-c", 'source "$1"; game_platform_fail native_device_rotation journey_assertion_failed', "bash", str(root / "tools/qualification/game_platform/lifecycle.sh")],
                    env={**os.environ, "GAME_SOURCE": str(root), "WORK": str(work), "NODE_BOOT_FAILURE_LOG_DIR": str(diagnostics), "GAME_PLATFORM_RESULT_FILE": str(work / "result")},
                    capture_output=True, check=False,
                )
                self.assertEqual(completed.returncode, 1)
                self.assertEqual(completed.stdout, b"")
                self.assertEqual((diagnostics / name).read_text(), f'Error: "{REASON}"\n[REDACTED]\n')
                self.assertEqual((diagnostics / name).stat().st_mode & 0o777, 0o600)


if __name__ == "__main__":
    unittest.main()
