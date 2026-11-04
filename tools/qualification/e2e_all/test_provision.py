"""Exercise provisioning over a real Unix control socket without privileged boot."""

import importlib.util
from pathlib import Path
import socket
import tempfile
import threading
import unittest
from unittest import mock


SPEC = importlib.util.spec_from_file_location(
    "node_boot_provision", Path(__file__).resolve().parents[1] / "node_boot/provision.py"
)
provision = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(provision)
SESSION = "01890f4c-3b2a-7c01-8d11-9a321b7c0004"


class ProvisionTests(unittest.TestCase):
    def exchange(self, reply, expected_error=None):
        with tempfile.TemporaryDirectory(prefix=".provision-test-", dir=Path.cwd()) as scratch:
            path = str(Path(scratch) / "control.sock")
            requests, errors = [], []
            with socket.socket(socket.AF_UNIX) as listener:
                listener.settimeout(3)
                listener.bind(path)
                listener.listen(1)

                def serve():
                    try:
                        with listener.accept()[0] as connection:
                            connection.settimeout(3)
                            request = bytearray()
                            while chunk := connection.recv(64):
                                request.extend(chunk)
                            requests.append(bytes(request))
                            # Separate writes exercise a streamed answer, not a datagram.
                            for part in reply:
                                connection.sendall(part)
                    except Exception as error:
                        errors.append(error)

                worker = threading.Thread(target=serve, daemon=True)
                worker.start()
                try:
                    if expected_error is None:
                        provision.provision(path, SESSION, "sorcerer")
                    else:
                        with self.assertRaises(expected_error):
                            provision.provision(path, SESSION, "sorcerer")
                finally:
                    worker.join(timeout=4)
                self.assertFalse(worker.is_alive())
                self.assertEqual(errors, [])
                self.assertEqual(requests, [f"qualify {SESSION} sorcerer\n".encode()])

    def test_node_acceptance_can_arrive_in_multiple_reads(self):
        self.exchange([b"quali", b"fied\n"])

    def test_refusal_truncation_and_extra_bytes_are_not_success(self):
        for reply in [[b"rejected\n"], [b"qualified"], [b"qualified\nextra"], []]:
            with self.subTest(reply=reply):
                self.exchange(reply, RuntimeError)

    def test_response_reads_share_one_deadline(self):
        with mock.patch.object(provision.socket, "socket") as factory, mock.patch.object(
            provision.time, "monotonic", side_effect=[0, 21, 21]
        ):
            with self.assertRaises(TimeoutError):
                provision.provision("control.sock", SESSION, "sorcerer")
            factory.return_value.__enter__.return_value.recv.assert_not_called()

    def test_bad_sessions_and_profiles_are_rejected_before_connecting(self):
        for session, profile in [
            (SESSION, "sorcerer\nknight"),
            (SESSION.upper(), "knight"),
            (SESSION.replace("7c01", "4c01"), "knight"),
            ("bad", "knight"),
        ]:
            with self.subTest(session=session, profile=profile), self.assertRaises(ValueError):
                provision.provision("missing.sock", session, profile)
