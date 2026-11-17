"""Control refusal evidence stays bounded and preserves the node response."""
import unittest
from unittest.mock import MagicMock, patch

import provision


class ProvisionResponseTests(unittest.TestCase):
    def test_refusal_preserves_node_answer(self):
        control = MagicMock()
        control.__enter__.return_value = control
        control.recv.side_effect = [
            b"rejected reason=spell_fixture_requires_a_fresh_disposabl\n",
            b"",
        ]
        with patch.object(provision.socket, "socket", return_value=control):
            with self.assertRaisesRegex(RuntimeError, "provisioning: rejected reason=spell_fixture_requires_a_fresh_disposabl\\n"):
                provision.provision("unused", "01890f4c-3b2a-7c01-8d11-9a321b7c0004", "knight")
