"""A source-backed survey must keep both current-World and Premium requirements."""
from pathlib import Path
import unittest
import questpc_explore_bindings as explore

ROOT = Path(__file__).resolve().parents[3]


class DeeplingsActivationTests(unittest.TestCase):
    def test_deeplings_survey_is_activated_with_both_guards(self):
        callbacks = [c for c in explore.callbacks(ROOT)
                     if c['quest'] == 'oteryn:quest.deeplings_worldchange' and '/s1/' in c['key']]
        self.assertEqual(len(callbacks), 1, 'one source-backed first survey entry')
        self.assertEqual(callbacks[0]['world_change'], 'deeplings')
        self.assertEqual(callbacks[0]['admission'], {'premium': True})
