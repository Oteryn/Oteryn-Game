"""The engine-only corpse fixture must also be authorable without widening its gates."""
import copy
import json
import unittest

from jsonschema import ValidationError
import qengine_rewards as rewards


class CorpseTargets(unittest.TestCase):
    def test_exact_corpse_requires_a_carried_input_and_closed_source(self):
        path = rewards.ROOT / 'crates/gameplay/quests/src/quest/reward_callbacks/corpse_fixture.json'
        document = json.loads(path.read_text())
        rewards.validate_callbacks(document)
        for field in ('item', 'revision', 'creature', 'creature_revision'):
            bad = copy.deepcopy(document)
            bad['callbacks'][0]['corpse_target'][field] = ''
            with self.assertRaises((ValueError, ValidationError)):
                rewards.validate_callbacks(bad)
        bad = copy.deepcopy(document)
        del bad['callbacks'][0]['carried_item']
        with self.assertRaises((ValueError, ValidationError)):
            rewards.validate_callbacks(bad)
