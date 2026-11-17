"""Failure diagnostics for the disposable spell-character bootstrap."""
import contextlib
import io
import re
import socket
import subprocess
import tempfile
import threading
import unittest
import uuid
from pathlib import Path
from unittest.mock import patch

import provision


class ProvisionTests(unittest.TestCase):
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
                   subprocess.CompletedProcess([], 1, "Artisan: issue refused\n", "safe PHP error\n")]
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
