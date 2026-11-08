"""A changed native Forge policy gets a new migration, retaining published bytes."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import forge_sql_admission as forge


class ForwardForgeAdmissionTests(unittest.TestCase):
    def test_explicit_forward_projection_keeps_published_migration_and_checks_latest(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            published = root / '0727_native_forge_profile_admission.sql'
            original = forge.BEGIN + '-- original immutable policy\n' + forge.END
            published.write_text(original)
            forward = root / '1305_questpc8_forge_profile_admission.sql'
            projection = forge.BEGIN + '-- newly qualified policy\n' + forge.END
            with patch.object(forge, 'MIGRATION', published), \
                    patch.object(forge, 'generated', return_value=projection), \
                    patch('sys.argv', ['forge_sql_admission.py', '--write', '--migration', str(forward)]):
                forge.main()
                self.assertEqual(published.read_text(), original)
                self.assertEqual(forge.current_migration(), forward)
                self.assertIn(projection, forward.read_text())
                forge.check()
                with patch.object(forge, 'generated', return_value=projection + '-- changed\n'):
                    with self.assertRaisesRegex(SystemExit, 'FORGE_SQL_ADMISSION_DRIFT'):
                        forge.check()
                self.assertEqual(published.read_text(), original)
                self.assertEqual(set(root.glob('*.sql')), {published, forward})


if __name__ == '__main__':
    unittest.main()
