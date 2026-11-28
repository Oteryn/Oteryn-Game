"""Source inputs must never masquerade as a complete Drome admission."""
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import tibiadrome_source_profile as drome
from PIL import Image


class DromeSourceProfileTests(unittest.TestCase):
    def profile(self):
        return json.loads((drome.ROOT / drome.OUTPUT).read_bytes())

    def test_retained_profile_has_exact_source_pool_and_floor12_not_roof(self):
        profile = self.profile()
        drome.validate_profile(profile)
        self.assertEqual({r['name']: r['look_type'] for r in profile['pool']}, {
            'Domestikion': 1426, 'Hoodinion': 1424, 'Mearidion': 1425,
            'Murmillion': 1422, 'Scissorion': 1423})
        room = profile['room']
        cells = {tuple(at) for at in room['battle_cells']}
        self.assertEqual(len(cells), 252)
        self.assertEqual({z for _, _, z in cells}, {12})
        self.assertEqual((min(x for x, _, _ in cells), max(x for x, _, _ in cells)), (32283, 32298))
        self.assertNotIn((32283, 32179, 12), cells)
        self.assertNotIn((32295, 32178, 12), cells)  # Exit is not a hazard/spawn cell.
        self.assertEqual(room['lever'], [32289, 32198, 12])
        self.assertEqual(room['entry_pads'], [[32289, y, 12] for y in range(32199, 32204)])
        self.assertIs(profile['runtime_admission'], False)

    def test_unknown_fields_and_unproven_gameplay_defaults_are_rejected(self):
        mutations = [
            lambda p: p.update(runtime_admission=True),
            lambda p: p.update(unreviewed_extension={}),
            lambda p: p['pool'][0].update(speed=360),
            lambda p: p['pool'][0].update(look_type=1422),
            lambda p: p['room']['battle_cells'].append([32295, 32178, 12]),
            lambda p: p['room'].update(return_anchor=[32291, 32205, 11]),
            lambda p: p['progression'].update(skip_threshold_seconds=[30, 60]),
            lambda p: p['progression'].update(current_version_continuity='PROVEN'),
        ]
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                profile = self.profile()
                mutate(profile)
                with self.assertRaisesRegex(ValueError, 'DROME_SOURCE_PROFILE'):
                    drome.validate_profile(profile)

    def test_historical_skip_counts_do_not_manufacture_thresholds_or_score(self):
        progression = self.profile()['progression']
        self.assertEqual([(o['skipped'], o['next_displayed_wave']) for o in progression['observations']],
                         [(3, 5), (2, 8), (1, 13)])
        self.assertEqual(progression['skip_threshold_seconds'], 'UNKNOWN')
        self.assertEqual(progression['skipped_wave_score'], 'UNKNOWN')
        self.assertEqual(progression['current_version_continuity'], 'UNKNOWN')

    def test_walkability_crop_does_not_expand_room_into_portal_or_corridor(self):
        crop = Image.new('RGB', (33, 55), (255, 255, 0))
        for x, y, _ in self.profile()['room']['battle_cells']:
            crop.putpixel((x - 32275, y - 32165), (120, 120, 120))
        for x, y in ((32295, 32178), (32290, 32196), (32289, 32199)):
            crop.putpixel((x - 32275, y - 32165), (120, 120, 120))
        cells = drome.battle_cells_from_crop(crop)
        self.assertEqual(cells, self.profile()['room']['battle_cells'])
        crop.putpixel((32290 - 32275, 32185 - 32165), (255, 0, 255))
        with self.assertRaisesRegex(ValueError, 'DROME_SOURCE_ROOM_MASK'):
            drome.battle_cells_from_crop(crop)

    def test_input_byte_drift_is_refused_before_decoding(self):
        good = b'qualified witness'
        expected = hashlib.sha256(good).hexdigest()
        drome.verify_bytes(good, expected, 'fixture')
        with self.assertRaisesRegex(ValueError, 'DROME_SOURCE_HASH:fixture'):
            drome.verify_bytes(good + b'!', expected, 'fixture')

    def test_duplicate_json_keys_are_not_silently_replaced(self):
        with self.assertRaisesRegex(ValueError, 'DROME_SOURCE_DUPLICATE_KEY'):
            drome.decode_json(b'{"runtime_admission":true,"runtime_admission":false}')

    def test_catalog_must_bind_every_sprite_range_to_its_actual_sheet(self):
        rows = [{'type': 'appearances', 'file': f'appearances-{drome.CLIENT_APPEARANCE_SHA}.dat'}]
        rows += [{'type': 'sprite', 'file': f'sprites-{name}.bmp.lzma', 'spritetype': 3,
                  'firstspriteid': first, 'lastspriteid': last, 'area': 0}
                 for name, _, first, last in drome.SPRITES]
        raw = drome.canonical(rows)
        with patch.object(drome, 'CATALOG_SHA', hashlib.sha256(raw).hexdigest()):
            drome.verify_catalog(raw)
        rows[1]['firstspriteid'] += 1
        raw = drome.canonical(rows)
        with (
            patch.object(drome, 'CATALOG_SHA', hashlib.sha256(raw).hexdigest()),
            self.assertRaisesRegex(ValueError, 'DROME_SOURCE_CATALOG_BINDING'),
        ):
            drome.verify_catalog(raw)

    def test_cli_check_is_read_only_and_capture_is_idempotent(self):
        profile = self.profile()
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'profile.json'
            inputs = ['--outfits', 'outfits.json', '--client-manifest', 'manifest.json',
                      '--maps', 'maps', '--sprites', 'sprites', '--output', str(output)]
            with patch.object(drome, 'capture', return_value=profile) as capture:
                drome.main(['capture', *inputs])
                first = output.read_bytes()
                drome.main(['capture', *inputs])
                self.assertEqual(output.read_bytes(), first)
                drome.main(['check', *inputs])
                self.assertEqual(capture.call_count, 3)
                output.write_bytes(first + b' ')
                with self.assertRaisesRegex(ValueError, 'DROME_SOURCE_OUTPUT_DRIFT'):
                    drome.main(['check', *inputs])
                self.assertEqual(output.read_bytes(), first + b' ')


if __name__ == '__main__':
    unittest.main()
