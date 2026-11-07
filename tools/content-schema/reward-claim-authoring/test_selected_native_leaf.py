"""A chosen reward must keep exact source binding and independently verify pinned identity."""
import copy
import json
import unittest
from unittest.mock import patch
import reward_claim_authoring as author
import selected_native_leaf as selected


class SelectedLeaf(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.items=author.load_items()
        cls.original,_=author.build_records(json.loads((author.ROOT/author.CLAIMS_REL).read_bytes())['claims'],
            json.loads((author.ROOT/author.MANIFEST_REL).read_bytes()),cls.items)
        cls.choice=selected.choices(author.ROOT)[0]

    def test_selected_output_is_reproducible_without_rewriting_donor_inputs(self):
        rows=selected.apply(copy.deepcopy(self.original),author.ROOT,self.items)
        rows=[r for r in rows if r['definition'].get('definition_profile')==selected.PROFILE]
        self.assertEqual(selected.validate(rows,author.ROOT),[])
        d=rows[0]['definition']
        self.assertEqual(d['native_selection']['original_reward']['item']['key'],'oteryn:item.tibia.i3031')
        self.assertEqual(d['placements'][0]['reward']['items'][0]['item']['key'],'oteryn:item.tibia.i3050')
        tampered=copy.deepcopy(rows);tampered[0]['definition']['native_selection']['wiki']['revid']+=1
        self.assertTrue(selected.validate(tampered,author.ROOT))

    def test_wrong_source_or_active_named_variant_is_not_a_valid_selection(self):
        for change in ['old_count','active_variant','digest']:
            row=copy.deepcopy(self.choice)
            if change=='old_count':row['expected_reward']['count']=2
            elif change=='active_variant':row['reward']['item']['key']='oteryn:item.tibia.i3087'
            else:row['wiki']['sha256']='0'*64
            with patch.object(selected,'choices',return_value=[row]),self.assertRaises(ValueError,msg=change):
                selected.apply(copy.deepcopy(self.original),author.ROOT,self.items)
