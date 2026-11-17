"""The volcano survey preserves actual eruption and Current Premium admission."""
from pathlib import Path
import unittest
import questpc_explore_bindings as explore

ROOT = Path(__file__).resolve().parents[3]


class FireEarthActivationTests(unittest.TestCase):
    def test_volcano_survey_is_bound_to_the_existing_eruption(self):
        callbacks = [c for c in explore.callbacks(ROOT)
                     if c['quest'] == 'oteryn:quest.authored.fire_from_the_earth_mini_world_change']
        self.assertEqual(len(callbacks), 1, 'one retained volcano survey entry')
        self.assertEqual(callbacks[0]['world_change'], 'fire_from_the_earth')
        self.assertEqual(callbacks[0]['admission'], {'premium': True})
