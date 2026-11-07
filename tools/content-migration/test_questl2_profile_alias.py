import copy
import json
import unittest
from pathlib import Path
from questl2_profile_alias import canonical_authoring


class ExactAliasTest(unittest.TestCase):
    def test_existing_3723_alias_preserves_all_facts_and_binding(self):
        root = Path(__file__).resolve().parents[2]
        document = json.loads((root / 'tools/content-schema/native-gameplay/qengine-reward-item-profiles.json').read_text())
        record = next(row for row in document['records']
                      if row['production_definition']['production_key'] == 'oteryn:item.tibia.i3723')
        prior = copy.deepcopy(record)
        prior['authoring'] = {'item': prior['production_binding']['target']}
        record = copy.deepcopy(record)
        record['authoring'] = {'item': {'family': 'Item', 'key': 'candidate:item/3723', 'revision': 'spell-p2-r21'}}
        self.assertTrue(canonical_authoring(prior, record))
        for field in ['production_definition', 'production_binding']:
            changed = copy.deepcopy(prior)
            changed[field]['unexpected'] = True
            self.assertFalse(canonical_authoring(changed, record), field)
        record['authoring']['item']['key'] = 'candidate:item/3724'
        self.assertFalse(canonical_authoring(prior, record))

    def test_canonical_alias_never_admits_conflicting_known_weight(self):
        import qengine_reward_item_profiles as producer
        root = Path(__file__).resolve().parents[2]
        document = json.loads((root / 'tools/content-schema/native-gameplay/qengine-reward-item-profiles.json').read_text())
        prior = copy.deepcopy(next(row for row in document['records']
                    if row['production_definition']['production_key'] == 'oteryn:item.tibia.i3723'))
        prior['authoring'] = {'item': prior['production_binding']['target']}
        record = copy.deepcopy(prior)
        record['authoring'] = {'item': {'family': 'Item', 'key': 'candidate:item/3723', 'revision': 'spell-p2-r21'}}
        record['semantics']['physical']['value']['weight']['value'] += 1
        generated = {'profiles': json.dumps({'records': [prior]}).encode()}
        with self.assertRaisesRegex(ValueError, 'QENGINE_REWARD_ITEM_NATIVE_CONFLICT'):
            producer.admit_records(root, generated, 'profiles', 'unused', None, None, [record])
