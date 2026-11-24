"""Regression checks for Crystal-backed appearances of legacy mount declarations."""
import json
import unittest

import qengine_rewards as q


class MountAppearances(unittest.TestCase):
    def test_finished_percht_sleighs_use_final_crystal_looks(self):
        generated = json.loads(q.build()['content/cosmetics/outfits/authoring.json'])
        mounts = {row['key']: row for row in generated['document']['mounts']}
        for colour, look in (('cold', 1232), ('bright', 1233), ('dark', 1234)):
            with self.subTest(colour=colour):
                legacy = mounts[f'oteryn:content.mount.finished_{colour}_percht_sleigh']
                current = mounts[f'oteryn:content.mount.{colour}_percht_sleigh_final']
                self.assertEqual(legacy['look'], current['look'])
                self.assertEqual(legacy['look']['id'], look)
                self.assertEqual(legacy['speed_bonus'], current['speed_bonus'])
                self.assertEqual(legacy['premium'], current['premium'])
