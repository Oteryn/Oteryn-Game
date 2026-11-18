import contextlib
import os
import sys
import time
import io
import re
import subprocess
import threading
import uuid
import provision

"""Exercise the provisioning client against a bounded local control socket."""

from concurrent.futures import ThreadPoolExecutor
import importlib.util
from pathlib import Path
import socket
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "node_boot_provision", Path(__file__).with_name("provision.py")
)
PROVISION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROVISION)
SESSION = "01890f4c-3b2a-7c01-8d11-9a321b7c0004"


class ProvisionTests(unittest.TestCase):
    def exchange(self, answer):
        with tempfile.TemporaryDirectory() as directory:
            path = str(Path(directory) / "control.sock")
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
                listener.bind(path)
                listener.listen(1)
                listener.settimeout(3)

                def serve():
                    with listener.accept()[0] as connection:
                        connection.settimeout(3)
                        request = bytearray()
                        while chunk := connection.recv(128):
                            request.extend(chunk)
                            self.assertLessEqual(len(request), 128)
                        self.assertEqual(request, f"qualify {SESSION} knight\n".encode())
                        connection.sendall(answer)

                with ThreadPoolExecutor(max_workers=1) as executor:
                    served = executor.submit(serve)
                    try:
                        PROVISION.provision(path, SESSION, "knight")
                    finally:
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
                        self.assertIn(b"qualify", connection.recv(128))
                        child.stdin.close()
                        self.assertEqual(child.wait(timeout=2), 124)
                finally:
                    if child.poll() is None:
                        child.kill()
                    child.wait(timeout=3)
                    child.stdout.close()
                    child.stderr.close()

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

    def exchange(self, answer):
        with tempfile.TemporaryDirectory() as directory:
            path = str(Path(directory) / "control.sock")
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
                listener.bind(path)
                listener.listen(1)
                listener.settimeout(3)

                def serve():
                    with listener.accept()[0] as connection:
                        connection.settimeout(3)
                        request = bytearray()
                        while chunk := connection.recv(128):
                            request.extend(chunk)
                            self.assertLessEqual(len(request), 256)
                        self.assertEqual(request, f"spawn {SESSION} {self.KEY}\n".encode())
                        connection.sendall(answer)

                with ThreadPoolExecutor(max_workers=1) as executor:
                    served = executor.submit(serve)
                    try:
                        return PROVISION.spawn(path, SESSION, self.KEY)
                    finally:
                        served.result(timeout=3)

    def test_spawned_reply_is_returned(self):
        reply = f"spawned {self.IDENTITY} 7\n".encode()
        self.assertEqual(self.exchange(reply), f"spawned {self.IDENTITY} 7")

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


class C17BootstrapTests(unittest.TestCase):
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
