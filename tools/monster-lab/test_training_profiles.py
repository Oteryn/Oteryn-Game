"""Focused isolated-profile safety tests with real schema-qualified fixtures."""
import copy
import importlib.util
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('training_profiles', HERE / 'training_profiles.py')
training = importlib.util.module_from_spec(spec)
spec.loader.exec_module(training)
FIXTURE = training.ROOT / 'tools/content-schema/monster-authoring/samples'


class TrainingProfilesTests(unittest.TestCase):
    def fixture(self):
        # Tiny ordinary fixture: add a real Encounter dependency to an existing Ability.
        return (
            {'creature': {'identity': {'key': 'canary:creature/sample', 'revision': 'r1'},
                          'stats': {'max_health': 100, 'initial_health': 20},
                          'abilities': [{'family': 'Ability', 'key': 'canary:ability/cast', 'revision': 'r1'}]},
             'behavior': {'identity': {'key': 'canary:behavior/sample', 'revision': 'r1'},
                          'attacks': [{'ability': {'family': 'Ability', 'key': 'canary:ability/cast', 'revision': 'r1'}}],
                          'defenses': []}},
            {'abilities': [{'identity': {'key': 'canary:ability/cast', 'revision': 'r1'},
                            'effects': [{'family': 'Effect', 'key': 'canary:effect/cast', 'revision': 'r1'}]}],
             'effects': [{'identity': {'key': 'canary:effect/cast', 'revision': 'r1'},
                          'encounter': {'family': 'Encounter', 'key': 'canary:encounter/event', 'revision': 'r1'}}],
             'formulas': []},
            {'definitions': [{'family': 'Encounter', 'key': 'canary:encounter/event', 'revision': 'r1'}], 'assets': []})

    def test_transitive_unsafe_ability_and_schedule_removed(self):
        m, d, c, disabled, _ = training.simplify(*self.fixture(), 'sample')
        self.assertEqual(len(disabled), 1)
        self.assertEqual(m['behavior']['attacks'], [])
        self.assertNotIn('abilities', m['creature'])
        self.assertEqual(d['effects'], [])
        self.assertEqual(d['abilities'], [])
        self.assertEqual(c['definitions'], [])

    def test_originals_and_health_unchanged(self):
        original = self.fixture()
        expected = copy.deepcopy(original)
        m, *_ = training.simplify(*original, 'sample')
        self.assertEqual(original, expected)
        self.assertEqual(m['creature']['stats'], expected[0]['creature']['stats'])

    def test_safe_ability_keeps_private_identity_and_item_reference(self):
        m, d, c = self.fixture()
        item = {'family': 'Item', 'key': 'canary:item/123', 'revision': 'r1'}
        d['effects'][0].pop('encounter')
        d['effects'][0]['item'] = item.copy()
        c['definitions'] = [item.copy()]
        m, d, c, disabled, _ = training.simplify(m, d, c, 'sample')
        self.assertEqual(disabled, [])
        self.assertEqual(d['effects'][0]['item'], item)
        self.assertEqual(m['behavior']['attacks'][0]['ability']['key'], 'canary:ability/lab/sample/cast')
        self.assertEqual(m['behavior']['attacks'][0]['ability']['revision'], training.REVISION)
        self.assertEqual(c['definitions'], [item])

    def test_melee_defense_removed_shared_attack_preserved(self):
        m, d, c = self.fixture()
        d['effects'][0].pop('encounter')
        d['abilities'][0]['kind'] = 'melee'
        m['behavior']['defenses'] = copy.deepcopy(m['behavior']['attacks'])
        m, d, c, disabled, _ = training.simplify(m, d, c, 'sample')
        self.assertEqual(m['behavior']['defenses'], [])
        self.assertEqual(len(m['behavior']['attacks']), 1)
        self.assertEqual(len(d['abilities']), 1)
        self.assertEqual(disabled[0]['reason'], 'native_defence_melee')
        self.assertEqual(disabled[0]['scope'], 'defense_action_only')
        self.assertEqual(disabled[0]['original_index'], 0)

    def test_missing_summon_only_removed_available_entry_retained(self):
        m, d, c = self.fixture()
        d['effects'][0].pop('encounter')
        missing = {'family': 'Creature', 'key': 'canary:creature/missing', 'revision': 'r1'}
        available = {'family': 'Creature', 'key': 'canary:creature/present', 'revision': 'r1'}
        m['behavior']['summons'] = {'max_summons': 2, 'entries': [
            {'creature': missing, 'count': 1}, {'creature': available, 'count': 1}]}
        m, d, c, disabled, _ = training.simplify(
            m, d, c, 'sample', unavailable_refs={training.identity('Creature', missing)})
        self.assertEqual(m['behavior']['summons']['entries'], [{'creature': available, 'count': 1}])
        self.assertEqual(disabled[0]['action']['creature'], missing)
        self.assertEqual(len(m['behavior']['attacks']), 1)

    def test_unavailable_familiar_master_keeps_independent_creature(self):
        m, d, c = self.fixture()
        d['effects'][0].pop('encounter')
        master = {'family': 'Ability', 'key': 'canary:ability/master', 'revision': 'r1'}
        m['creature']['summoning'] = {'is_familiar': True, 'summonable': False,
                                     'familiar': {'summon_ability': master, 'mana_cost': 10}}
        m, d, c, disabled, _ = training.simplify(
            m, d, c, 'sample', unavailable_refs={training.identity('Ability', master)})
        self.assertFalse(m['creature']['summoning']['is_familiar'])
        self.assertNotIn('familiar', m['creature']['summoning'])
        self.assertEqual(disabled[0]['action']['familiar']['summon_ability'], master)
        self.assertEqual(m['creature']['stats']['initial_health'], 20)

    def test_unregistered_decay_omits_corpse_without_alias_or_loot_change(self):
        m, d, c = self.fixture()
        d['effects'][0].pop('encounter')
        missing = {'family': 'Item', 'key': 'canary:item/48296', 'revision': 'r1'}
        corpse = {'family': 'Item', 'key': 'canary:item/48267', 'revision': 'r1'}
        m['creature']['corpse_item'] = corpse
        d['items'] = [{'identity': {'key': corpse['key'], 'revision': 'r1'}, 'temporal': {'decay_target': missing}},
                      {'identity': {'key': missing['key'], 'revision': 'r1'}}]
        m, d, c, disabled, _ = training.simplify(
            m, d, c, 'sample', unavailable_refs={training.identity('Item', missing)})
        self.assertNotIn('corpse_item', m['creature'])
        self.assertEqual(d['items'], [])
        self.assertEqual(disabled[0]['reason'], 'unregistered_decay_target')
        self.assertEqual(disabled[0]['action'], corpse)

    def test_unregistered_loot_drop_omitted_only_that_entry(self):
        m, d, c = self.fixture()
        d['effects'][0].pop('encounter')
        missing = {'family': 'Item', 'key': 'canary:item/1', 'revision': 'r1'}
        present = {'family': 'Item', 'key': 'canary:item/2', 'revision': 'r1'}
        m['loot'] = {'identity': {'key': 'canary:loot/sample', 'revision': 'r1'},
                     'entries': [{'item': missing, 'probability_percent': 50},
                                 {'item': present, 'probability_percent': 75}]}
        m, d, c, disabled, _ = training.simplify(
            m, d, c, 'sample', unavailable_refs={training.identity('Item', missing)})
        self.assertEqual(m['loot']['entries'], [{'item': present, 'probability_percent': 75}])
        self.assertEqual(disabled[0]['action']['item'], missing)

    def test_same_name_wrong_revision_not_rekeyed(self):
        m, d, c = self.fixture()
        m['creature']['abilities'][0]['revision'] = 'other'
        m, *_ = training.simplify(m, d, c, 'sample')
        self.assertEqual(m['creature']['abilities'][0]['revision'], 'other')


if __name__ == '__main__':
    unittest.main()
