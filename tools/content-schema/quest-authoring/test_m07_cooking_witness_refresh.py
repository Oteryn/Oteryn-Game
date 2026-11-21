import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-migration'))
import m42_cooking_canonical as witness

class CookingWitnessRefreshTests(unittest.TestCase):
    def test_unrelated_shard_changes_do_not_redefine_cookbook_records(self):
        selected = {'definition': {'identity': {'key': 'oteryn:item.cookbook'}, 'value': 7}}
        predecessor = json.dumps({'records': [selected]}).encode()
        for changed, stale in [(False, False), (True, False), (False, True)]:
            with self.subTest(changed=changed, stale=stale), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                record = json.loads(json.dumps(selected))
                if changed:
                    record['definition']['value'] = 8
                current = json.dumps({'records': [record, {'definition': {'identity': {'key': 'oteryn:item.other'}}}]}).encode()
                (root / 'items.json').write_bytes(current)
                for path, data in [
                    (witness.PACKET, {'canonical_inputs': [{'path': 'items.json', 'sha256': hashlib.sha256(predecessor if stale else current).hexdigest()}]}),
                    (witness.RECIPE, {'recipes': [{'ingredients': [{'item': 'oteryn:item.cookbook'}], 'reward': {'item': 'oteryn:item.cookbook'}}]}),
                ]:
                    target = root / path
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_text(json.dumps(data))
                with patch.object(witness.subprocess, 'check_output', return_value=predecessor):
                    if changed or stale:
                        with self.assertRaises(ValueError):
                            witness.capture(root)
                    else:
                        captured = witness.capture(root)
                        self.assertEqual(captured['shards']['items.json'], witness.selected(predecessor, {'oteryn:item.cookbook'}))

if __name__ == '__main__':
    unittest.main()
