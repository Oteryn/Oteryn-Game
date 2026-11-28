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


class ForgeCensusTests(unittest.TestCase):
    def test_inspect_forge_requests_the_exact_session_and_returns_the_census(self):
        with patch.object(PROVISION, "_request", return_value=b'[{"stacks":3}]') as exchange:
            self.assertEqual(PROVISION.inspect_forge("/control.sock", SESSION), [{"stacks": 3}])
            exchange.assert_called_once_with("/control.sock", f"inspect-forge {SESSION}\n", 128 * 1024)

    def test_inspect_forge_rejects_invalid_sessions_refusals_and_oversize_census(self):
        with patch.object(PROVISION, "_request") as exchange:
            with self.assertRaises(ValueError):
                PROVISION.inspect_forge("/control.sock", SESSION.upper())
            exchange.assert_not_called()
            for reply in (b"rejected reason=forge_census_unavailable", b"{}", b" " * (128 * 1024 + 1)):
                with self.subTest(length=len(reply)), self.assertRaises(RuntimeError):
                    exchange.return_value = reply
                    PROVISION.inspect_forge("/control.sock", SESSION)


if __name__ == "__main__":
    unittest.main()
