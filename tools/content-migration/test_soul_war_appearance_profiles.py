"""W11 callback appearances survive the registered native regeneration stage."""
import hashlib
import json
from pathlib import Path
import unittest
import soul_war_appearance_profiles as source

ROOT = Path(__file__).resolve().parents[2]

class SoulWarAppearancePipelineTests(unittest.TestCase):
    def test_registered_stage_retains_base_order_and_pins_exact_two_source_records(self):
        path = 'content/presentations/bindings/spell-appearances.json'
        document = json.loads((ROOT / path).read_bytes())
        expected = json.loads((ROOT / source.SOURCE).read_bytes())['records']
        keys = {r['outfit_key'] for r in expected}
        base = [r for r in document['records'] if r['outfit_key'] not in keys]
        document['records'] = base
        generated = {path: json.dumps(document).encode(), 'manifest': b'{}'}
        source.admit(ROOT, generated, path, 'manifest')
        self.assertEqual(json.loads(generated[path])['records'], base + expected)
        self.assertEqual(json.loads(generated['manifest'])['spell_appearances']['sha256'], hashlib.sha256(generated[path]).hexdigest())
        before = dict(generated)
        source.admit(ROOT, generated, path, 'manifest')
        self.assertEqual(generated, before)

if __name__ == '__main__':
    unittest.main()
