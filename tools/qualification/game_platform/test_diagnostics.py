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
        for line, secret in [
            ("Authorization: Bearer bearer-secret", "bearer-secret"),
            ("Authorization: OterynDevice device-secret", "device-secret"),
            ("otd1." + "x" * 43, "otd1." + "x" * 43),
            ('{"code":"oauth-code"}', "oauth-code"),
            ("?code=oauth-code&state=oauth-state", "oauth-code"),
            ("?state=oauth-state", "oauth-state"),
            ('{"access_token":"oauth-token"}', "oauth-token"),
            ('{"refresh_token":"refresh-value"}', "refresh-value"),
            ("Cookie: session=cookie-value; other=other-value", "cookie-value"),
            ("Set-Cookie: session=cookie-value; HttpOnly", "cookie-value"),
            ("password=password-value", "password-value"),
            ('"password":\n"multiline-value"', "multiline-value"),
            ('{"secret":"private-value"}', "private-value"),
            ("secret=private-value", "private-value"),
            ('{"Cookie":"session=cookie-json-value"}', "cookie-json-value"),
            ("postgresql://user:database-value@localhost/private", "database-value"),
            ("postgres://user:database-value@localhost/private", "database-value"),
            ("9ad3" * 8, "9ad3" * 8),
            ("xY2_gH3-lK4_mN5-pQ6_rS7-tU8_vW9-zA0", "xY2_gH3-lK4_mN5-pQ6_rS7-tU8_vW9-zA0"),
        ]:
            with self.subTest(secret_class=len(secret)):
                result = redact(f'Error: "{REASON}"\n{line}\n')
                self.assertNotIn(secret, result)
                self.assertIn(f'Error: "{REASON}"', result)

    def test_review_leaks_removed_through_failure_retention(self):
        root = Path(__file__).resolve().parents[3]
        cases = [
            ("https://alice:private-password@example.test/path", ["alice", "private-password"]),
            (r'\"password\":\"escaped-password-value\"', ["escaped-password-value"]),
            (r'\"access_token\":\"escaped-token-value\"', ["escaped-token-value"]),
            ('{"ticket":"ticket-private-value"}', ["ticket-private-value"]),
            ('{"native_ticket":"native-private-value"}', ["native-private-value"]),
            ('{"session_ticket":"session-private-value"}', ["session-private-value"]),
            ("custom+tls://alice:custom-password@example.test/path", ["alice", "custom-password"]),
            ('{"secret":["first-private-value","later-private-value",["nested-private-value"]]}', ["first-private-value", "later-private-value", "nested-private-value"]),
            (r'\"secret\":[\"escaped-first-value\",\"escaped-later-value\"]', ["escaped-first-value", "escaped-later-value"]),
        ]
        cases += [
            ("https://alice:private'password@example.test/path", ['alice', "private'password"]),
            ('https://alice:private"password@example.test/path', ['alice', 'private"password']),
            ('https://alice:private%27password@example.test/path', ['alice', 'private%27password']),
            ('https://alice:private:password@example.test/path', ['alice', 'private:password']),
            ("custom+tls://alice:private'part@later-private@example.test/path", ['alice', "private'part", 'later-private']),
            ("password=prefix'private-suffix", ['prefix', 'private-suffix']),
            ('password=prefix"private-suffix', ['prefix', 'private-suffix']),
            ('password=prefix%27private-suffix', ['prefix', 'private-suffix']),
            ('password=prefix:private-suffix', ['prefix', 'private-suffix']),
        ]
        for index, (payload, secrets) in enumerate(cases):
            with self.subTest(case=index), tempfile.TemporaryDirectory() as directory:
                work = Path(directory)
                destination = work / "diagnostics"
                destination.mkdir(mode=0o700)
                (work / "identity.log").write_text(payload + "\nerror[E0308]: mismatched types\n")
                completed = subprocess.run(
                    ["bash", "-c", 'source "$1"; game_platform_fail web_identity journey_assertion_failed', "bash", str(root / "tools/qualification/game_platform/lifecycle.sh")],
                    env={**os.environ, "GAME_SOURCE": str(root), "WORK": str(work), "NODE_BOOT_FAILURE_LOG_DIR": str(destination), "GAME_PLATFORM_RESULT_FILE": str(work / "result")},
                    capture_output=True, check=False,
                )
                self.assertEqual(completed.returncode, 1)
                sanitized = (destination / "identity.log").read_text()
                for secret in secrets:
                    self.assertNotIn(secret, sanitized)
                self.assertIn("error[E0308]: mismatched types", sanitized)

    def test_authority_and_secret_value_boundaries(self):
        for boundary in ["/path", "?query=value", "#fragment", " trailing text"]:
            value = "https://alice:private'part@later-part@example.test" + boundary
            self.assertEqual(redact(value), "https://[REDACTED]@example.test" + boundary)
        for delimiter in [" ", ",", ";", "&", ")", "}", "]"]:
            for quote in ["'", '\"']:
                self.assertEqual(redact("password=prefix" + quote + "suffix" + delimiter + "after"), "password=[REDACTED]" + delimiter + "after")
        self.assertEqual(redact('password="prefix\\\"suffix" after'), "password=[REDACTED] after")
        self.assertEqual(redact("password='quoted value' after"), "password=[REDACTED] after")

    def test_diagnostics_survive_unchanged(self):
        text = ("   Compiling oteryn-platform-client v0.1.0\n"
                "thread 'journey' panicked at tests/flow.rs:42:7:\n"
                "assertion `left == right` failed: unexpected response\n"
                "  left: 400\n right: 401\n"
                "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s\n"
                "error[E0308]: mismatched types\n"
                "test blocker_requires_byte_exact_shipping_request ... FAILED\n")
        self.assertEqual(redact(text), text)

    def test_no_test_name_exemption_and_explicit_evidence_only(self):
        digest = "b57a092c" * 8
        line = "test " + digest + " ... FAILED"
        self.assertEqual(redact(line), "test [REDACTED] ... FAILED")
        self.assertEqual(redact(line, [digest]), line)
        self.assertNotIn(digest, redact("test_prefix_" + digest + " ... FAILED"))
        sha = "09a7" * 10
        self.assertEqual(redact("pins game=" + sha), "pins game=[REDACTED]")
        self.assertEqual(redact("pins game=" + sha, [sha]), "pins game=" + sha)
        self.assertNotIn(digest, redact("password=" + digest, [digest]))

    def test_fixed_sqlstate_reason_survives(self):
        line = 'Error: "postgres_assertion_query_failed:device_state:42P01"'
        self.assertEqual(redact(line + "\n"), line + "\n")

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
                self.assertEqual((diagnostics / name).read_text(), f'Error: "{REASON}"\nAuthorization: Bearer [REDACTED]\n')
                self.assertEqual((diagnostics / name).stat().st_mode & 0o777, 0o600)


if __name__ == "__main__":
    unittest.main()
