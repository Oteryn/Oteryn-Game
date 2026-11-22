"""Secrets never survive retention; fixed assertion reasons remain reviewable."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

from redact import redact

REASON = "enrollment_transport_or_payload_failed"


class Diagnostics(unittest.TestCase):
    def run_wrapper_failure(self, work, *, public_redaction_fails=False):
        root = Path(__file__).resolve().parents[3]
        fixture = work / "game"
        scripts = fixture / "tools/qualification/game_platform"
        scripts.mkdir(parents=True)
        for name in ("run.sh", "lifecycle.sh", "redact.py"):
            shutil.copyfile(root / "tools/qualification/game_platform" / name, scripts / name)
        platform = work / "platform"
        for name in (
            "e2e/start.sh", "e2e/harness.mjs", "crates/identity/src/login_http.rs",
            "crates/gameauth/src/oauth/oauth_http.rs", "crates/player-api/src/lib.rs",
            "crates/gateway/src/lib.rs", "crates/gameauth/src/devices_http.rs",
            "crates/gameauth/src/devices_tls.rs",
        ):
            path = platform / "portal-rs" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.touch()
        platform_sha = next(
            line.split("=", 1)[1] for line in (scripts / "run.sh").read_text().splitlines()
            if line.startswith("readonly GAME_PLATFORM_SHA=")
        )
        binaries = work / "bin"
        binaries.mkdir()

        def executable(name, body):
            path = binaries / name
            path.write_text("#!/usr/bin/env bash\nset -eu\n" + body)
            path.chmod(0o700)

        executable("git", """
if [[ "$*" == "rev-parse --show-toplevel" ]]; then printf '%s\\n' "$TEST_GAME_SOURCE"
elif [[ "$*" == "rev-parse HEAD" ]]; then printf '%040d\\n' 1
elif [[ "$1" == "-C" && "$3" == "rev-parse" && "$4" == "HEAD" ]]; then
  if [[ "$2" == "$PLATFORM_SOURCE" ]]; then printf '%s\\n' "$TEST_PLATFORM_SHA"; else printf '%040d\\n' 1; fi
elif [[ "$1" == "-C" && "$3" == "status" ]]; then exit 0
else exit 97
fi
""")
        for name in ("docker", "node", "openssl", "cargo", "sudo", "psql", "setsid", "timeout"):
            executable(name, '[[ "$(basename "$0")" == docker && "$*" == info ]] || exit 97\n')
        executable("python3", """
if [[ "${TEST_PUBLIC_REDACTION_FAIL:-0}" == 1 && "${4:-}" == --public-excerpt ]]; then
  printf 'partial-publication-private-value\\n' > "$3"
  exit 1
fi
exec "$TEST_PYTHON" "$@"
""")
        node_boot = fixture / "tools/qualification/node_boot/run.sh"
        node_boot.parent.mkdir(parents=True)
        node_boot.write_text(
            'set -euo pipefail\n'
            'GAME_SOURCE="$TEST_GAME_SOURCE"\n'
            'source "$TEST_GAME_SOURCE/tools/qualification/game_platform/lifecycle.sh"\n'
            'WORK="$TEST_WORK"\n'
            'cp "$TEST_INPUT" "$WORK/native_device_rotation_replay.log"\n'
            'game_platform_fail native_device_rotation_replay journey_assertion_failed\n'
        )
        raw = work / "input.log"
        raw.write_text(
            "setup continues\n" * 300
            + f'Error: "{REASON}"\n'
            + "thread 'rotation' panicked at tests/native.rs:42:7:\n"
            + "assertion failed: unexpected response\n  left: 400\n right: 401\n"
            + "Authorization: Bearer bearer-private-value\n"
            + "postgresql://user:database-private-value@localhost/private\n"
            + "https://service.invalid/private-path?code=oauth-private-value\n"
            + "password=password-private-value\n"
            + "NODE_BOOT_RESULT=NODE_BOOT_PASS\n"
            + "GAME_PLATFORM_RESULT=PASS\n"
            + "SEAM_EVIDENCE e2e_all scenario=FAKE.client status=PASS seconds=0.001\n"
            + "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out\n"
            + "long diagnostic: " + "x" * 32000 + "\n"
            + "later output\n" * 300
        )
        temporary = work / "temporary"
        temporary.mkdir()
        return subprocess.run(
            ["bash", str(scripts / "run.sh")],
            env={**os.environ, "PATH": str(binaries) + os.pathsep + os.environ["PATH"],
                 "PLATFORM_SOURCE": str(platform), "TEST_PLATFORM_SHA": platform_sha,
                 "TEST_GAME_SOURCE": str(fixture), "TEST_WORK": str(work),
                 "TEST_INPUT": str(raw), "TEST_PYTHON": sys.executable,
                 "TEST_PUBLIC_REDACTION_FAIL": "1" if public_redaction_fails else "0",
                 "TMPDIR": str(temporary)},
            capture_output=True, check=False, text=True, timeout=10,
        )

    def test_wrapper_publishes_bounded_redacted_failure_without_forged_results(self):
        root = Path(__file__).resolve().parents[3]
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            completed = self.run_wrapper_failure(work)
            self.assertEqual(completed.returncode, 1)
            self.assertIn(f'Error: "{REASON}"', completed.stdout)
            self.assertIn("panicked at tests/native.rs:42:7", completed.stdout)
            self.assertIn("  left: 400\n", completed.stdout)
            self.assertIn("test result: FAILED. 0 passed; 1 failed", completed.stdout)
            self.assertIn("GAME_PLATFORM_DIAGNOSTIC quoted(GAME_PLATFORM_RESULT)=PASS", completed.stdout)
            self.assertIn("GAME_PLATFORM_DIAGNOSTIC excerpt_truncated=1", completed.stdout)
            for value in ("bearer-private-value", "database-private-value", "password-private-value",
                          "oauth-private-value", "service.invalid", "private-path", "://"):
                self.assertNotIn(value, completed.stdout + completed.stderr)
            excerpt = [line + "\n" for line in completed.stdout.splitlines()
                       if line.startswith("GAME_PLATFORM_DIAGNOSTIC ")]
            self.assertLessEqual(len(excerpt), 80)
            self.assertLessEqual(len("".join(excerpt).encode()), 16384)
            log = work / "public.log"
            result = work / "result.json"
            log.write_text(completed.stdout)
            extracted = subprocess.run(
                [sys.executable, str(root / "tools/qualification/node_boot/matrix.py"),
                 "extract", str(log), str(result), "--shard", "game-platform",
                 "--entry", "game_platform", "--exit-code", str(completed.returncode),
                 "--seconds", "1"], capture_output=True, check=False,
            )
            self.assertEqual(extracted.returncode, 1)
            evidence = json.loads(result.read_text())
            self.assertEqual(evidence["result"], "FAIL")
            self.assertEqual(evidence["exit_code"], 1)
            self.assertEqual(evidence["scenarios"], [])

    def test_wrapper_never_publishes_partial_output_when_public_redaction_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            completed = self.run_wrapper_failure(Path(directory), public_redaction_fails=True)
            self.assertEqual(completed.returncode, 1)
            self.assertIn("GAME_PLATFORM_RESULT=FAIL", completed.stdout)
            self.assertIn("public_log_redaction_failed", completed.stdout)
            self.assertNotIn("partial-publication-private-value", completed.stdout + completed.stderr)
            self.assertNotIn(f'Error: "{REASON}"', completed.stdout)

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

    def test_node_boot_harness_failure_retains_and_prints_redacted_log(self):
        root = Path(__file__).resolve().parents[3]
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            diagnostics = work / "diagnostics"
            diagnostics.mkdir(mode=0o700)
            (work / "portal-harness.log").write_text(
                "[harness] public listener not ready\npassword=private-value\n"
            )
            completed = subprocess.run(
                ["bash", "-c", 'source "$1"; node_log="$WORK/node.log"; '
                 'trap game_platform_cleanup EXIT; game_platform_fail platform harness_not_ready',
                 "bash", str(root / "tools/qualification/game_platform/lifecycle.sh")],
                env={**os.environ, "GAME_SOURCE": str(root), "WORK": str(work),
                     "NODE_BOOT_LIFECYCLE": "native_reference", "NODE_BOOT_FAILURE_LOG_DIR": str(diagnostics),
                     "GAME_PLATFORM_RESULT_FILE": str(work / "result")},
                capture_output=True, check=False, text=True,
            )
            expected = "[harness] public listener not ready\npassword=[REDACTED]\n"
            self.assertEqual(completed.returncode, 1)
            self.assertEqual(completed.stdout, expected)
            self.assertEqual((diagnostics / "portal-harness.log").read_text(), expected)


if __name__ == "__main__":
    unittest.main()
