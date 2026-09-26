"""Regression checks for pinned source identity through Windows line-ending conversion."""
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import canary_batch as cb


class CanaryProvenance(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        self.git('config', 'core.autocrlf', 'false')
        (self.root / 'monster.lua').write_bytes(b'monster.health = 20\n')
        self.git('add', 'monster.lua')
        self.git('-c', 'user.name=Source test', '-c', 'user.email=test@example.invalid',
                 'commit', '-qm', 'pinned test source')
        self.pin = self.git('rev-parse', 'HEAD').strip()

    def tearDown(self):
        self.tmp.cleanup()

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.root), *args], text=True)

    def test_crlf_checkout_keeps_canonical_git_identity(self):
        expected = self.git('rev-parse', self.pin + ':monster.lua').strip()
        self.git('config', 'core.autocrlf', 'true')
        (self.root / 'monster.lua').write_bytes(b'monster.health = 20\r\n')
        with patch.object(cb, 'REVISION', self.pin):
            cb.require_pinned_checkout(self.root)
            self.assertEqual(cb.source_blob(self.root, 'monster.lua'), expected)

    def test_modified_source_is_rejected_before_conversion(self):
        (self.root / 'monster.lua').write_bytes(b'monster.health = 999\n')
        with patch.object(cb, 'REVISION', self.pin):
            with self.assertRaises(subprocess.CalledProcessError):
                cb.require_pinned_checkout(self.root)

    def test_different_head_is_rejected(self):
        with patch.object(cb, 'REVISION', '0' * 40):
            with self.assertRaises(ValueError):
                cb.require_pinned_checkout(self.root)


if __name__ == '__main__':
    unittest.main()
