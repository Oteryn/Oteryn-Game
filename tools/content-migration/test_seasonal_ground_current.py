"""Current map provenance cannot bypass the captured seasonal ground rules."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location(
    'seasonal_ground_owner', Path(__file__).with_name('seasonal_ground_physics.py')
)
OWNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(OWNER)


class CurrentGround(unittest.TestCase):
    def setUp(self):
        self.current = json.loads((OWNER.ROOT / OWNER.PACKET).read_bytes())
        self.current['b3_sha256'] = hashlib.sha256(
            (OWNER.ROOT / OWNER.REGION).read_bytes()
        ).hexdigest()

    def test_current_region_keeps_the_captured_source_census(self):
        self.assertEqual(self.current['census'], OWNER.census())
        self.assertIn(self.current['b3_sha256'], OWNER.render(self.current))

    def test_stale_packet_is_rejected_with_identical_census(self):
        self.current['b3_sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'SEASONAL_GROUND_CAPTURE_DRIFT'):
            OWNER.render(self.current)

    def test_current_digest_does_not_authorize_a_changed_census(self):
        self.current['census'][0][0] += 1
        with self.assertRaisesRegex(ValueError, 'SEASONAL_GROUND_CAPTURE_DRIFT'):
            OWNER.render(self.current)

    def test_exact_source_roster_remains_required(self):
        self.current['grounds'].pop()
        with self.assertRaisesRegex(ValueError, 'SEASONAL_GROUND_EXACT_ROSTER'):
            OWNER.render(self.current)

    def test_raw_donor_binding_remains_required(self):
        self.current['sources'][0]['sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'SEASONAL_GROUND_RAW_BINDINGS'):
            OWNER.render(self.current)

    def test_known_projectile_fact_cannot_be_relaxed(self):
        changed = copy.deepcopy(self.current)
        changed['grounds'][0]['canary']['block_projectile'] = True
        with self.assertRaisesRegex(ValueError, 'SEASONAL_GROUND_QUALIFICATION'):
            OWNER.render(changed)


if __name__ == '__main__':
    unittest.main(verbosity=2)
