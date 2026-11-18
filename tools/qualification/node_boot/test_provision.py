"""Exercise the provisioning client against a bounded local control socket."""

from concurrent.futures import ThreadPoolExecutor
import importlib.util
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
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
                        request = bytearray()
                        while chunk := connection.recv(128):
                            request.extend(chunk)
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
                            self.assertIn(b"qualify", control.recv(128))
                            child.stdin.close()
                            self.assertEqual(child.wait(timeout=2), 125)
                    finally:
                        if child.poll() is None:
                            child.kill()

    def test_qualified_reply_is_accepted(self):
        self.exchange(b"qualified\n")

    def test_coin_converter_fixture_uses_control_request(self):
        self.exchange(b"qualified\n", "coin-converters")

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
                        self.assertEqual(connection.recv(1), b"!")
                        connection.sendall(b"!")
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


if __name__ == "__main__":
    unittest.main()
