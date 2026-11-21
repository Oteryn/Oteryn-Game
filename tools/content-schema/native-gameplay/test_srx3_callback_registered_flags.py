import hashlib
from pathlib import Path
import tempfile
import unittest
import build_srx3_callback_registered_flags as tool


class RegisteredFlagsAuditTests(unittest.TestCase):
    def test_flags_keep_explicit_lines_and_cpp_defaults(self):
        flags, setters = tool.registered_flags('local spell = Spell("instant")\nspell:needTarget(true)\nspell:needDirection(false)\n')
        self.assertEqual({"needs_target": True, "needs_direction": False}, flags)
        self.assertEqual([2, 3], [row["line"] for row in setters])
        self.assertEqual(({"needs_target": False, "needs_direction": False}, []),
                         tool.registered_flags('local spell = Spell("instant")\n-- spell:needTarget(true)\n'))

    def test_dynamic_flag_is_never_silently_defaulted(self):
        with self.assertRaisesRegex(ValueError, "unresolved registered flag"):
            tool.registered_flags("spell:needTarget(config.target)")

    def test_pinned_git_blob_mismatch_refuses_evidence(self):
        raw = b'spell:needTarget(true)\n'
        sha = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / (sha + ".lua")).write_bytes(raw)
            entry = {"path": "source.lua", "sha": sha}
            self.assertEqual(raw, tool.verified_blob(root, entry))
            (root / (sha + ".lua")).write_bytes(raw + b"-- changed\n")
            with self.assertRaisesRegex(ValueError, "Git blob mismatch"):
                tool.verified_blob(root, entry)
