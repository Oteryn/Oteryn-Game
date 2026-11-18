import contextlib
import os
import sys
import time
import io
import json
import re
import subprocess
import threading
import uuid
import provision

"""Exercise the provisioning client against a bounded local control socket."""

from concurrent.futures import ThreadPoolExecutor
import importlib.util
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import threading
import unittest
from unittest.mock import MagicMock, patch


SPEC = importlib.util.spec_from_file_location(
    "node_boot_provision", Path(__file__).with_name("provision.py")
)
PROVISION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROVISION)
SESSION = "01890f4c-3b2a-7c01-8d11-9a321b7c0004"


class ProvisionTests(unittest.TestCase):
    def test_control_reply_can_arrive_after_twenty_seconds(self):
        with patch.object(PROVISION.socket, "socket") as socket_factory, patch.object(
            PROVISION.time, "monotonic", side_effect=[0, 0, 0, 0, 0, 0, 0, 25, 25]
        ):
            control = MagicMock()
            socket_factory.return_value.__enter__.return_value = control
            control.recv.side_effect = [b"!", b"qualified\n", b""]
            PROVISION.provision("fixture.sock", SESSION, "knight")
            control.recv.assert_called()

    def test_queued_request_gets_full_reply_budget_after_readiness(self):
        now = [0]
        with patch.object(PROVISION.socket, "socket") as factory, patch.object(
            PROVISION.time, "monotonic", side_effect=lambda: now[0]
        ):
            control = MagicMock()
            factory.return_value.__enter__.return_value = control

            def recv(_limit):
                if now[0] == 0:
                    control.sendall.assert_called_once_with(b"!")
                    now[0] = 25
                    return b"!"
                if now[0] == 25:
                    control.sendall.assert_called_with(f"qualify {SESSION} knight\n".encode())
                    now[0] = 64
                    return b"qualified\n"
                return b""

            control.recv.side_effect = recv
            PROVISION.provision("fixture.sock", SESSION, "knight")

    def exchange(self, answer, profile="knight"):
        with tempfile.TemporaryDirectory() as directory:
            path = str(Path(directory) / "control.sock")
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
                listener.bind(path)
                listener.listen(1)
                listener.settimeout(3)

                def serve():
                    with listener.accept()[0] as connection:
                        connection.settimeout(3)
                        self.assertEqual(connection.recv(1), b"!")
                        connection.sendall(b"!")
                        with connection.makefile("rb", buffering=0) as reader:
                            request = reader.readline(129)
                        self.assertLessEqual(len(request), 128)
                        self.assertEqual(request, f"qualify {SESSION} {profile}\n".encode())
                        connection.sendall(answer)

                with ThreadPoolExecutor(max_workers=1) as executor:
                    served = executor.submit(serve)
                    try:
                        PROVISION.provision(path, SESSION, profile)
                    finally:
                        served.result(timeout=3)

    def test_closing_parent_pipe_stops_helper_with_pending_socket_reply(self):
        with tempfile.TemporaryDirectory() as directory:
            path = str(Path(directory) / "control.sock")
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
                listener.bind(path)
                listener.listen(1)
                listener.settimeout(2)
                with subprocess.Popen(
                    [sys.executable, str(Path(__file__).with_name("provision.py")),
                     "--watch-stdin", "--socket", path, "--session", SESSION, "--profile", "knight"],
                    stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                ) as child:
                    try:
                        with listener.accept()[0] as control:
                            control.settimeout(2)
                            self.assertEqual(control.recv(1), b"!")
                            control.sendall(b"!")
                            self.assertEqual(control.makefile("rb", buffering=0).readline(129), f"qualify {SESSION} knight\n".encode())
                            child.stdin.close()
                            self.assertEqual(child.wait(timeout=2), 125)
                    finally:
                        if child.poll() is None:
                            child.kill()

    def test_qualification_wait_keeps_the_write_half_open(self):
        with tempfile.TemporaryDirectory() as directory:
            path = str(Path(directory) / "control.sock")
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
                listener.bind(path)
                listener.listen(1)
                listener.settimeout(3)

                def serve():
                    with listener.accept()[0] as connection:
                        connection.settimeout(0.25)
                        self.assertEqual(connection.recv(1), b"!")
                        connection.sendall(b"!")
                        with connection.makefile("rb", buffering=0) as request:
                            self.assertEqual(request.readline(), f"qualify {SESSION} knight\n".encode())
                            try:
                                with self.assertRaises(TimeoutError):
                                    connection.recv(1)
                            finally:
                                connection.sendall(b"qualified\n")

                with ThreadPoolExecutor(max_workers=1) as executor:
                    served = executor.submit(serve)
                    PROVISION.provision(path, SESSION, "knight")
                    served.result(timeout=3)

    def test_closing_caller_pipe_disconnects_submitted_provisioning(self):
        with tempfile.TemporaryDirectory() as directory:
            path = str(Path(directory) / "control.sock")
            read_fd, write_fd = os.pipe()
            submitted = threading.Event()
            with os.fdopen(read_fd, "rb") as cancelled, os.fdopen(write_fd, "wb") as caller:
                with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
                    listener.bind(path)
                    listener.listen(1)
                    listener.settimeout(3)

                    def serve():
                        with listener.accept()[0] as connection:
                            connection.settimeout(3)
                            self.assertEqual(connection.recv(1), b"!")
                            connection.sendall(b"!")
                            with connection.makefile("rb", buffering=0) as request:
                                self.assertEqual(
                                    request.readline(), f"qualify {SESSION} knight\n".encode()
                                )
                                submitted.set()
                                self.assertEqual(connection.recv(1), b"")

                    with ThreadPoolExecutor(max_workers=2) as executor:
                        served = executor.submit(serve)
                        call = executor.submit(PROVISION.provision, path, SESSION, "knight", cancel=cancelled)
                        self.assertTrue(submitted.wait(3))
                        caller.close()
                        with self.assertRaisesRegex(RuntimeError, "caller disconnected"):
                            call.result(timeout=3)
                        served.result(timeout=3)

    def test_fixture_refusal_relays_only_closed_diagnostic_tokens(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "node.log"
            log.write_text('node event=qualification_fixture_refused reason=fixture_disabled\n'
                           'node event=qualification_fixture_refused session=private profile=c17a reason="C17 spell fixture requires an untouched disposable Character"\n')
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                PROVISION.report_node_failures(str(log), 0)
            self.assertIn("reason=fixture_disabled", stderr.getvalue())
            self.assertNotIn("private", stderr.getvalue())

    def test_c17_fixture_reconciles_the_same_request_after_lost_ack(self):
        calls = []
        def request(path, line, limit, **kwargs):
            calls.append(line)
            return b"" if len(calls) == 1 else b"qualified\n"
        with patch.object(PROVISION, "_request", request):
            PROVISION.provision("socket", SESSION, "c17a")
        self.assertEqual(calls, [f"qualify {SESSION} c17a-reconcile\n"] * 2)

    def test_parent_eof_stops_a_blocked_provisioning_helper(self):
        with tempfile.TemporaryDirectory() as directory:
            path = str(Path(directory) / "control.sock")
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
                listener.bind(path)
                listener.listen(1)
                listener.settimeout(3)
                child = subprocess.Popen([
                    __import__("sys").executable, str(Path(__file__).with_name("provision.py")),
                    "--watch-parent-stdin", "--socket", path, "--session", SESSION, "--profile", "knight",
                ], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                try:
                    with listener.accept()[0] as connection:
                        connection.settimeout(3)
                        self.assertEqual(connection.recv(1), b"!")
                        connection.sendall(b"!")
                        self.assertIn(b"qualify", connection.recv(128))
                        child.stdin.close()
                        self.assertEqual(child.wait(timeout=2), 124)
                finally:
                    if child.poll() is None:
                        child.kill()
                    child.wait(timeout=3)
                    child.stdout.close()
                    child.stderr.close()

    def test_r3_remote_writer_cleans_completed_and_expired_state_but_keeps_recent_fences(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            expired = root/'oteryn-c17-intent-expired'; expired.mkdir(); (expired/'cancelled').touch()
            (expired/'expires').write_text(str(int(provision.writer_clock())-1))
            recent = root/'oteryn-c17-intent-recent'; recent.mkdir(); (recent/'cancelled').touch()
            (recent/'expires').write_text(str(int(provision.writer_clock())+30))
            op = str(uuid.uuid4())
            result = subprocess.run(['sh', '-c', provision.REMOTE_INTENT.replace('/tmp', str(root)), 'test-intent', op, str(int(provision.writer_clock())+30), 'sh', '-c', 'true'], capture_output=True, timeout=3)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(expired.exists())
            self.assertTrue((recent/'cancelled').exists(), 'recent delayed-start fence was removed')
            self.assertFalse((root/('oteryn-c17-intent-'+op)).exists())

    def test_r3_delayed_remote_start_expires_even_when_cancellation_is_unavailable(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); marker = root/'committed'; php = root/'php'
            php.write_text('#!' + sys.executable + "\nimport pathlib\npathlib.Path(" + repr(str(marker)) + ").touch()\n")
            php.chmod(0o755)
            writer = provision.PlatformIntentWriter('container', str(uuid.uuid4()))
            writer.expires = int(provision.writer_clock()) + 1
            def delayed(command, stage, timeout):
                remote = [arg.replace('/tmp/oteryn-c17-intent-', str(root/'lease-')) for arg in command[5:]]
                def start():
                    time.sleep(2)
                    return subprocess.run(remote, env=dict(os.environ, PATH=str(root) + os.pathsep + os.environ['PATH']), capture_output=True, timeout=4)
                work.append(executor.submit(start))
                raise RuntimeError('Docker writer acknowledgement timed out')
            work = []
            with ThreadPoolExecutor(max_workers=1) as executor:
                with patch.object(provision, 'run_capture', side_effect=delayed), patch.object(writer, 'cancel', side_effect=RuntimeError('Docker cancellation unavailable')):
                    with self.assertRaisesRegex(RuntimeError, 'writer acknowledgement timed out'):
                        writer.run('unused', {})
                self.assertEqual(work[0].result(timeout=5).returncode, 124)
            self.assertFalse(marker.exists(), 'delayed remote exec issued an intent after expiry')
            self.assertIsNone(provision.REMOTE_WRITER)

    def test_r3_unknown_replies_have_bounded_backoff(self):
        with patch.object(PROVISION, "_request", side_effect=[b"unavailable\n"] * 3 + [b"qualified\n"]), patch.object(PROVISION.time, "sleep") as sleep:
            PROVISION.provision("socket", SESSION, "c17a")
        self.assertEqual(sleep.call_count, 3)
        self.assertTrue(all(call.args[0] >= 0.1 for call in sleep.call_args_list))

    def test_r3_failed_cancel_retains_the_writer_and_original_error(self):
        writer = provision.PlatformIntentWriter('container', '01934f10-7c04-7001-805b-3b1122334401')
        calls = []
        def cancel():
            calls.append(provision.REMOTE_WRITER)
            if len(calls) == 1:
                raise RuntimeError('cancel failed')
            writer.quiesced.set()
        with patch.object(provision, 'run_capture', side_effect=RuntimeError('original intent failure')), patch.object(writer, 'cancel', side_effect=cancel):
            with self.assertRaisesRegex(RuntimeError, 'original intent failure'):
                writer.run('php', {})
        self.assertGreaterEqual(len(calls), 2)
        self.assertTrue(all(value is writer for value in calls))
        self.assertIsNone(provision.REMOTE_WRITER)

    def test_r2_timeout_leaves_time_to_reconcile_the_same_build(self):
        now = [0.0]
        calls = []
        def request(path, line, limit, **kwargs):
            calls.append(line)
            if len(calls) == 1:
                now[0] += kwargs["timeout"]
                raise TimeoutError("lost acknowledgement after commit")
            return b"qualified\n"
        with patch.object(PROVISION.time, "monotonic", side_effect=lambda: now[0]), patch.object(PROVISION, "_request", request):
            PROVISION.provision("socket", SESSION, "c17a")
        self.assertEqual(len(calls), 2)
        self.assertEqual(calls[0], calls[1])

    def test_r2_exhausted_reconciliation_reports_an_unknown_outcome(self):
        now = [0.0]
        def request(path, line, limit, **kwargs):
            now[0] += kwargs["timeout"]
            raise TimeoutError("no commit acknowledgement")
        with patch.object(PROVISION.time, "monotonic", side_effect=lambda: now[0]), patch.object(PROVISION, "_request", request):
            with self.assertRaisesRegex(RuntimeError, "outcome=unknown"):
                PROVISION.provision("socket", SESSION, "c17a")

    def test_r2_unknown_database_commit_reply_reconciles(self):
        with patch.object(PROVISION, "_request", side_effect=[b"unavailable\n", b"qualified\n"]) as request:
            PROVISION.provision("socket", SESSION, "c17a")
        self.assertEqual(request.call_count, 2)

    def test_r2_partial_unknown_commit_reply_reconciles(self):
        with patch.object(PROVISION, "_request", side_effect=[b"unavail", b"qualified\n"]) as request:
            PROVISION.provision("socket", SESSION, "c17a")
        self.assertEqual(request.call_count, 2)

    def test_r2_parent_cancellation_stops_the_remote_platform_writer(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            docker = root / "docker"
            docker.write_text("#!" + sys.executable + "\n" + """import os,subprocess,sys
if sys.argv[1] == 'ps':
    print('container-id')
else:
    command=sys.argv[5:]
    command=[arg.replace('/tmp/oteryn-c17-intent-',os.environ['LEASE_PREFIX']) for arg in command]
    child=subprocess.Popen(command,start_new_session=True)
    sys.exit(child.wait())
""")
            php = root / "php"
            php.write_text("#!" + sys.executable + "\n" + """import os,pathlib,time
pathlib.Path(os.environ['WRITER_STARTED']).write_text(str(os.getpid()))
time.sleep(0.7)
pathlib.Path(os.environ['WRITER_COMMITTED']).write_text('intent issued')
""")
            docker.chmod(0o755)
            php.chmod(0o755)
            started, committed = root / 'started', root / 'committed'
            env = dict(os.environ, PATH=str(root) + os.pathsep + os.environ['PATH'],
                       WRITER_STARTED=str(started), WRITER_COMMITTED=str(committed),
                       LEASE_PREFIX=str(root / 'lease-'))
            child = subprocess.Popen([sys.executable, str(Path(__file__).with_name('provision.py')),
                '--watch-parent-stdin', '--socket', str(root / 'unused'), '--project', 'test', '--fresh-character', '{}'],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
            try:
                deadline = time.monotonic() + 3
                while not started.exists() and time.monotonic() < deadline:
                    time.sleep(0.01)
                self.assertTrue(started.exists(), 'Platform writer never started')
                child.stdin.close()
                self.assertEqual(child.wait(timeout=3), 124)
                time.sleep(0.9)
                self.assertFalse(committed.exists(), 'Platform writer committed after cancellation ended the row')
            finally:
                if child.poll() is None:
                    child.kill()
                child.wait(timeout=3)
                child.stdout.close()
                child.stderr.close()

    def test_qualified_reply_is_accepted(self):
        self.exchange(b"qualified\n")

    def test_coin_converter_fixture_uses_control_request(self):
        self.exchange(b"qualified\n", "coin-converters")

    def test_q02_sweep_profile_reaches_the_node(self):
        self.exchange(b"qualified\n", "sweep")
    def test_victim_profile_reaches_the_control_socket(self):
        self.exchange(b"qualified\n", "victim")

    def test_refusal_retains_the_exact_node_answer(self):
        with self.assertRaises(RuntimeError) as refused:
            self.exchange(b"rejected\n")
        self.assertIn("node_answer=b'rejected\\n'", str(refused.exception))

    def test_reply_diagnostic_escapes_control_and_non_ascii_bytes(self):
        with self.assertRaises(RuntimeError) as refused:
            self.exchange(b"rejected\n\x1b[31m\xff\r")
        detail = str(refused.exception)
        self.assertIn("node_answer=b'rejected\\n\\x1b[31m\\xff\\r'", detail)
        self.assertTrue(detail.isascii())
        self.assertFalse(any(ord(char) < 32 for char in detail))

    def test_oversized_reply_diagnostic_contains_only_64_source_bytes(self):
        with self.assertRaises(RuntimeError) as refused:
            self.exchange(b"x" * 64 + b"Y")
        detail = str(refused.exception)
        self.assertIn(f"node_answer={b'x' * 64!r}", detail)
        self.assertNotIn("Y", detail)
        self.assertIn("truncated=True", detail)

    def test_empty_reply_is_diagnosed(self):
        with self.assertRaises(RuntimeError) as refused:
            self.exchange(b"")
        self.assertIn("node_answer=b''", str(refused.exception))


class SpawnTests(unittest.TestCase):
    KEY = "oteryn:creature.rat"
    IDENTITY = "0123456789abcdef0123456789abcdef"

    def exchange(self, answer, key=None):
        key = self.KEY if key is None else key
        with tempfile.TemporaryDirectory() as directory:
            path = str(Path(directory) / "control.sock")
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
                listener.bind(path)
                listener.listen(1)
                listener.settimeout(3)

                def serve():
                    with listener.accept()[0] as connection:
                        connection.settimeout(3)
                        self.assertEqual(connection.recv(1), b"!")
                        connection.sendall(b"!")
                        request = bytearray()
                        while chunk := connection.recv(128):
                            request.extend(chunk)
                            self.assertLessEqual(len(request), 256)
                        self.assertEqual(request, f"spawn {SESSION} {key}\n".encode())
                        connection.sendall(answer)

                with ThreadPoolExecutor(max_workers=1) as executor:
                    served = executor.submit(serve)
                    try:
                        return PROVISION.spawn(path, SESSION, key)
                    finally:
                        served.result(timeout=3)

    def test_spawned_reply_is_returned(self):
        reply = f"spawned {self.IDENTITY} 7 42\n".encode()
        self.assertEqual(self.exchange(reply), f"spawned {self.IDENTITY} 7 42")

    def test_spawned_reply_requires_the_actor_local_identity(self):
        with self.assertRaises(RuntimeError):
            self.exchange(f"spawned {self.IDENTITY} 7\n".encode())

    def test_cleanup_reply_is_specific_to_the_cleanup_request(self):
        self.assertEqual(self.exchange(b"cleaned\n", "cleanup"), "cleaned")
        with self.assertRaises(RuntimeError):
            self.exchange(b"cleaned\n")

    def test_refusal_retains_the_exact_node_answer(self):
        with self.assertRaises(RuntimeError) as refused:
            self.exchange(b"rejected reason=unknown_creature\n")
        self.assertIn(
            "node_answer=b'rejected reason=unknown_creature\\n'", str(refused.exception)
        )

    def test_invalid_requests_never_reach_the_socket(self):
        for session, key in (
            ("not-a-session", self.KEY),
            (SESSION, ""),
            (SESSION, "bad key"),
            (SESSION, "key\n"),
        ):
            with self.assertRaises(ValueError):
                PROVISION.spawn("/nonexistent.sock", session, key)


class CapacityTests(unittest.TestCase):
    def test_current_session_and_capacity_are_bound_to_the_request(self):
        with patch.object(PROVISION, "_request", return_value=b"capacity 2 60000\n") as request:
            self.assertEqual(PROVISION.carry_capacity("control.sock", SESSION), "capacity 2 60000")
            request.assert_called_once_with("control.sock", f"capacity {SESSION}\n", 64)

    def test_capacity_rejects_missing_or_malformed_observation(self):
        for answer in (b"rejected\n", b"capacity 2 0\n", b"capacity 2 60000\nextra"):
            with self.subTest(answer=answer), patch.object(PROVISION, "_request", return_value=answer):
                with self.assertRaisesRegex(RuntimeError, "node refused qualification capacity"):
                    PROVISION.carry_capacity("control.sock", SESSION)


class DamageInspectionTests(unittest.TestCase):
    TARGET = "0123456789abcdef0123456789abcdef 7"
    CORPSE = "fedcba9876543210fedcba9876543210"

    def test_request_names_the_exact_actor_incarnation(self):
        with patch.object(PROVISION, "_request", return_value=f"damage 12 {self.CORPSE}\n".encode()) as exchange:
            self.assertEqual(PROVISION.inspect_damage("/control.sock", SESSION, self.TARGET),
                             f"damage 12 {self.CORPSE}")
            exchange.assert_called_once_with("/control.sock",
                                             f"inspect-damage {SESSION} {self.TARGET}\n", 128)

    def test_invalid_generation_and_incomplete_replies_are_refused(self):
        with patch.object(PROVISION, "_request", return_value=b"damage 12\n") as exchange:
            with self.assertRaises(ValueError):
                PROVISION.inspect_damage("/control.sock", SESSION, self.TARGET[:-1] + "0")
            exchange.assert_not_called()
            with self.assertRaises(RuntimeError):
                PROVISION.inspect_damage("/control.sock", SESSION, self.TARGET)


class C17BootstrapTests(unittest.TestCase):
    def test_isolated_bootstrap_refuses_before_platform_writes(self):
        with patch.object(provision.socket, "socket") as socket_factory, \
                patch.object(provision, "run_capture") as platform:
            control = socket_factory.return_value.__enter__.return_value
            control.makefile.return_value.__enter__.return_value.readline.return_value = b"rejected\n"
            with self.assertRaisesRegex(RuntimeError, "outside qualification mode"):
                provision.fresh_character("node.sock", "project", {"new_account": True})
            control.sendall.assert_called_once_with(b"qualification-project\n")
            platform.assert_not_called()

    def test_isolated_bootstrap_refuses_unrelated_platform_project(self):
        request = {"new_account": True, "account": "operator"}
        with patch.object(provision.socket, "socket") as socket_factory, \
                patch.object(provision, "run_capture", return_value="unrelated-container") as platform, \
                patch.object(provision.PlatformIntentWriter, "run") as writer, \
                patch.object(provision, "commit_character_bootstrap"):
            control = socket_factory.return_value.__enter__.return_value
            control.recv.return_value = b"!"
            control.makefile.return_value.__enter__.return_value.readline.return_value = b"other-project\n"
            with self.assertRaisesRegex(RuntimeError, "does not match"):
                provision.fresh_character("node.sock", "project", request)
            platform.assert_not_called()
            writer.assert_not_called()

    def test_isolated_bootstrap_creates_canonical_separate_account_and_passes_premium(self):
        request = {"account": "bootstrap-operator", "new_account": True, "premium": True}
        with patch.object(provision.socket, "socket") as socket_factory, \
                patch.object(provision, "run_capture", return_value="platform-container"), \
                patch.object(provision.PlatformIntentWriter, "run") as writer, \
                patch.object(provision, "commit_character_bootstrap"):
            socket_factory.return_value.__enter__.return_value.makefile.return_value.__enter__.return_value.readline.return_value = b"project\n"
            result = json.loads(provision.fresh_character("node.sock", "project", request))
        self.assertEqual(uuid.UUID(result["account"]).version, 7)
        self.assertEqual(writer.call_args.args[1]["operator_account"], "bootstrap-operator")
        self.assertEqual(writer.call_args.args[1]["account"], result["account"])
        self.assertTrue(writer.call_args.args[1]["premium"])

    def test_sanitizer_drops_sensitive_lines_case_insensitively(self):
        source = "safe\nPASSWORD=a\npostgres://a\nmysql://a\nSeCrEt=a\nToken=a\nkey=a\nlast\n"
        self.assertEqual(provision.sanitized_text(source.encode()), "safe\nlast")

    def test_failed_command_reports_sanitized_stdout_and_stderr(self):
        child = subprocess.CompletedProcess([], 7, "Artisan rejected the name\nsecret=hidden\n",
                                            "safe PHP detail\nTOKEN=hidden\n")
        log = io.StringIO()
        with patch.object(provision.subprocess, "run", return_value=child), contextlib.redirect_stderr(log):
            with self.assertRaisesRegex(RuntimeError, "Platform bootstrap intent exit status 7"):
                provision.run_capture(["docker"], "Platform bootstrap intent", 30)
        self.assertIn("Artisan rejected the name", log.getvalue())
        self.assertIn("safe PHP detail", log.getvalue())
        self.assertNotIn("hidden", log.getvalue())
        self.assertIn("stdout:", log.getvalue())
        self.assertIn("stderr:", log.getvalue())

    def test_timeout_reports_partial_output_without_command_arguments(self):
        failure = subprocess.TimeoutExpired(["private-argument"], 30,
                                            output=b"partial Artisan output\n",
                                            stderr=b"password=hidden\nsafe timeout detail\n")
        log = io.StringIO()
        with patch.object(provision.subprocess, "run", side_effect=failure), contextlib.redirect_stderr(log):
            with self.assertRaisesRegex(RuntimeError, "intent timed out after 30s"):
                provision.run_capture(["docker"], "intent", 30)
        self.assertIn("partial Artisan output", log.getvalue())
        self.assertIn("safe timeout detail", log.getvalue())
        self.assertNotIn("hidden", log.getvalue())
        self.assertNotIn("private-argument", log.getvalue())

    def test_success_returns_output_without_polluting_the_name_channel(self):
        child = subprocess.CompletedProcess([], 0, "captured output", "captured warning")
        log = io.StringIO()
        with patch.object(provision.subprocess, "run", return_value=child), contextlib.redirect_stderr(log):
            self.assertEqual(provision.run_capture(["docker"], "intent", 30), "captured output")
        self.assertEqual(log.getvalue(), "")

    def test_generated_names_match_the_pinned_shared_naming_rules(self):
        # Platform 3bc2d0e CharacterBootstrapIntentContract::assertRequestedName
        # and Game CHAR-NAME-1: total 2..29, ASCII words, single spaces.
        for value in (0, (1 << 128) - 1, 0x01934F107C047001805B3B1122334401):
            name = provision.fresh_character_name(uuid.UUID(int=value))
            self.assertEqual(len(name), 26)
            self.assertIsNotNone(re.fullmatch(r"[A-Za-z]+(?: [A-Za-z]+)*", name))
            self.assertLessEqual(len(name), 29)

    def test_php_failure_is_reported_before_connecting_to_the_node(self):
        results = [subprocess.CompletedProcess([], 0, "container-id\n", ""),
                   subprocess.CompletedProcess([], 1, "Artisan: issue refused\n", "safe PHP error\n"),
                   subprocess.CompletedProcess([], 0, "", "")]
        log = io.StringIO()
        with patch.object(provision.subprocess, "run", side_effect=results), contextlib.redirect_stderr(log):
            with self.assertRaisesRegex(RuntimeError, "Platform bootstrap intent exit status 1"):
                provision.fresh_character("unused.sock", "test-project", {})
        self.assertIn("Artisan: issue refused", log.getvalue())
        self.assertIn("safe PHP error", log.getvalue())

    def node_refusal(self, answer):
        results = [subprocess.CompletedProcess([], 0, "container-id\n", ""),
                   subprocess.CompletedProcess([], 0, "", "")]
        received = []
        with tempfile.TemporaryDirectory() as directory:
            path = str(Path(directory) / "control.sock")
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
                listener.bind(path)
                listener.listen()
                def serve():
                    connection, _ = listener.accept()
                    with connection:
                        request = bytearray()
                        while block := connection.recv(128):
                            request.extend(block)
                        received.append(bytes(request))
                        connection.sendall(answer)
                thread = threading.Thread(target=serve, daemon=True)
                thread.start()
                try:
                    with patch.object(provision.subprocess, "run", side_effect=results):
                        with self.assertRaises(RuntimeError) as failure:
                            provision.fresh_character(path, "test-project", {})
                finally:
                    thread.join(timeout=5)
                self.assertFalse(thread.is_alive())
        self.assertEqual(len(received), 1)
        self.assertEqual(uuid.UUID(received[0].decode().strip()).version, 7)
        return str(failure.exception)

    def test_mid_run_unavailable_reconciles_the_same_issued_operation(self):
        results = [subprocess.CompletedProcess([], 0, "container-id\n", ""),
                   subprocess.CompletedProcess([], 0, "", "")]
        received = []
        with tempfile.TemporaryDirectory() as directory:
            path = str(Path(directory) / "control.sock")
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
                listener.bind(path)
                listener.listen()
                listener.settimeout(2)
                def serve():
                    for answer in (b"unavailable\n", b"committed\n"):
                        try:
                            connection, _ = listener.accept()
                        except TimeoutError:
                            return
                        with connection:
                            request = bytearray()
                            while block := connection.recv(128):
                                request.extend(block)
                            received.append(bytes(request))
                            connection.sendall(answer)
                thread = threading.Thread(target=serve, daemon=True)
                thread.start()
                try:
                    with patch.object(provision.subprocess, "run", side_effect=results) as commands:
                        name = provision.fresh_character(path, "test-project", {})
                finally:
                    thread.join(timeout=5)
                self.assertFalse(thread.is_alive())
        self.assertEqual(commands.call_count, 2, "the Platform intent must be issued once")
        self.assertEqual(len(received), 2)
        self.assertEqual(received[0], received[1], "retry must reconcile the same operation")
        operation = uuid.UUID(received[0].decode().strip())
        self.assertEqual(operation.version, 7)
        self.assertEqual(name, provision.fresh_character_name(operation))

    def test_persistent_unavailable_stops_at_the_original_deadline(self):
        now = [0]
        def expire(_delay):
            now[0] = 60
        with patch.object(provision.socket, "socket") as sockets, \
                patch.object(provision.time, "monotonic", side_effect=lambda: now[0]), \
                patch.object(provision.time, "sleep", side_effect=expire), \
                contextlib.redirect_stderr(io.StringIO()):
            control = sockets.return_value.__enter__.return_value
            control.recv.side_effect = [b"unavailable\n", b""]
            with self.assertRaisesRegex(RuntimeError, "deadline exceeded; answer='unavailable'"):
                provision.commit_character_bootstrap("control.sock", uuid.uuid4())
            self.assertEqual(sockets.call_count, 1)
            self.assertEqual(control.sendall.call_count, 1)

    def test_lost_or_partial_acknowledgement_reconciles_the_same_intent(self):
        for lost in (b"", b"committed"):
            with self.subTest(reply=lost), tempfile.TemporaryDirectory() as directory:
                path = str(Path(directory) / "control.sock")
                results = [subprocess.CompletedProcess([], 0, "container-id\n", ""),
                           subprocess.CompletedProcess([], 0, "", "")]
                received = []
                with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
                    listener.bind(path)
                    listener.listen()
                    listener.settimeout(3)
                    def serve():
                        for answer in (lost, b"committed\n"):
                            try:
                                connection, _ = listener.accept()
                            except TimeoutError:
                                return
                            with connection:
                                connection.settimeout(3)
                                request = bytearray()
                                while block := connection.recv(128):
                                    request.extend(block)
                                received.append(bytes(request))
                                connection.sendall(answer)
                    thread = threading.Thread(target=serve, daemon=True)
                    thread.start()
                    try:
                        with patch.object(provision.subprocess, "run", side_effect=results) as commands, \
                                contextlib.redirect_stderr(io.StringIO()):
                            provision.fresh_character(path, "test-project", {})
                    finally:
                        thread.join(timeout=4)
                    self.assertFalse(thread.is_alive())
                self.assertEqual(commands.call_count, 2)
                self.assertEqual(len(received), 2)
                self.assertEqual(received[0], received[1])

    def test_read_timeout_reconciles_within_the_original_budget(self):
        log = io.StringIO()
        with patch.object(provision.socket, "socket") as sockets, \
                contextlib.redirect_stderr(log):
            control = sockets.return_value.__enter__.return_value
            control.recv.side_effect = [TimeoutError(), b"committed\n", b""]
            provision.commit_character_bootstrap("control.sock", uuid.uuid4())
            self.assertEqual(sockets.call_count, 2)
            self.assertEqual(control.sendall.call_args_list[0], control.sendall.call_args_list[1])
            self.assertTrue(all(call.args[0] <= 30 for call in control.settimeout.call_args_list))

    def test_unavailable_reports_only_new_node_failure_tokens(self):
        with tempfile.TemporaryDirectory() as directory:
            node_log = Path(directory) / "node.log"
            node_log.write_text("event=character_bootstrap_failure stage=commit reason=old\n")
            offset = node_log.stat().st_size
            with node_log.open("a") as log:
                log.write('node ts=1 level=info module=node event=character_bootstrap_failure trace=private '
                          'detail="stage=account_security reason=evidence step=publish cause=root_unavailable '
                          'replay=busy" account=private\n')
                log.write("event=unrelated secret=hidden\n")
                log.write('event=chat detail="payload event=character_bootstrap_failure '
                          'stage=forged reason=forged"\n')
                log.write("event=qualification_fixture_failure stage=build reason=refused character=private\n")
            log = io.StringIO()
            with contextlib.redirect_stderr(log):
                provision.report_node_failures(node_log, offset)
            self.assertEqual(log.getvalue().splitlines(), [
                "event=character_bootstrap_failure stage=account_security reason=evidence "
                "step=publish cause=root_unavailable replay=busy",
                "event=qualification_fixture_failure stage=build reason=refused",
            ])
            for forbidden in ("private", "hidden", "reason=old"):
                self.assertNotIn(forbidden, log.getvalue())

    def test_unavailable_appends_node_stage_to_the_deadline_tail(self):
        now = [0]
        with tempfile.TemporaryDirectory() as directory:
            node_log = Path(directory) / "node.log"
            node_log.write_text("")
            def receive(_size):
                with node_log.open("a") as log:
                    log.write("event=character_bootstrap_failure stage=intent_read reason=source\n")
                return b"unavailable\n"
            def expire(_delay):
                now[0] = 60
            log = io.StringIO()
            with patch.object(provision.socket, "socket") as sockets, \
                    patch.object(provision.time, "monotonic", side_effect=lambda: now[0]), \
                    patch.object(provision.time, "sleep", side_effect=expire), \
                    contextlib.redirect_stderr(log):
                control = sockets.return_value.__enter__.return_value
                control.recv.side_effect = [b"unavailable\n", b""]
                # Append at the actual exchange, after the provisioner captures
                # its log cursor, as the running node does.
                control.sendall.side_effect = lambda _: receive(65)
                with self.assertRaisesRegex(RuntimeError, "deadline exceeded"):
                    provision.commit_character_bootstrap("control.sock", uuid.uuid4(), node_log)
            self.assertIn("event=character_bootstrap_failure stage=intent_read reason=source",
                          log.getvalue())

    def test_node_refusal_includes_the_actual_answer(self):
        self.assertIn("answer='rejected'", self.node_refusal(b"rejected\n"))

    def test_node_refusal_does_not_echo_sensitive_answers(self):
        message = self.node_refusal(b"token=hidden\n")
        self.assertIn("<empty or filtered>", message)
        self.assertNotIn("hidden", message)


if __name__ == "__main__":
    unittest.main()
