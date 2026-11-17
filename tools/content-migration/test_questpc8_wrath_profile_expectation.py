"""The expectation comes from accepted identity, never from generated profiles."""
import json
import tempfile
import unittest
from pathlib import Path

from questpc8_wrath_profile_expectation import declared_keys


class WrathProfileExpectationTests(unittest.TestCase):
    def fixture(self, root, tool='oteryn:item.tibia.i11371', count=4):
        paths = {
            'content/quests/authored-world-callbacks.json': {'callbacks': [
                {'key': f'oteryn:quest-callback/authored-wrath-crystal-{i}',
                 'item_on_target': {'item': tool}} for i in range(1, 5)]},
            'tools/content-schema/quest-authoring/samples/server-completion/chosen-source-events-rewards/packet.json': {
                'quests': [{'wiki_title': 'Wrath of the Emperor Quest', 'stages': [
                    {'stage_key': 's8', 'count': count, 'targets': [
                        {'target': 'Snake Sceptre', 'refs': [{'key': 'oteryn:item.tibia.i11371'}]}]}]}]},
        }
        for name, value in paths.items():
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps(value))

    def test_exact_accepted_tool_is_independent_of_generated_profiles(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            self.assertEqual(declared_keys(root), {'oteryn:item.tibia.i11371'})

    def test_donor_replica_cannot_replace_selected_sceptre(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root, tool='oteryn:item.tibia.i11362')
            with self.assertRaisesRegex(ValueError, 'differs from accepted recipe'):
                declared_keys(root)

    def test_recipe_quota_cannot_be_silently_changed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root, count=3)
            with self.assertRaisesRegex(ValueError, 'identity/count disagreement'):
                declared_keys(root)
