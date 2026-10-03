import copy
from collections import Counter
from unittest.mock import patch
import ots_questlog as questlog
import json
import os
from pathlib import Path
import unittest
import source_fix_guard as guard
import source_kilmaresh_owner_guard as owner


class KilmareshPairTests(unittest.TestCase):
    def setUp(self):
        root = Path(os.environ.get('OTERYN_KILMARESH_PROOF_ROOT', Path(__file__).parent/'samples/kilmaresh-owner-fix'))
        self.snapshots = json.loads((root/'snapshots.json').read_text())
        self.parent_graph = json.loads((root/'parent-graph.json').read_text())

    def cores(self, key):
        old = {'identity': {'key': key, 'revision': 'quest-r1'},
            'source_data': {'progress': [], 'interactions': [], 'quest': {'kind': 'script_only'}},
            'reported_source_readiness': {'data_gaps': {'unresolved_items': 1}},
            'missing_data': [{'code': 'reported_source_gap', 'field': 'unresolved_items', 'count': 1},
                             {'code': 'quest_native_lowering_missing'}], 'native_lowering': {'state': 'WAITING_IMPLEMENTATION'}}
        new = copy.deepcopy(old)
        side = old if key == owner.GRAVE else new
        side['source_data']['progress'] = [r['old' if key == owner.GRAVE else 'new'] for r in self.snapshots['tracks']]
        if key == owner.GRAVE:old['source_data']['interactions'] = [self.snapshots['original_grave_graph']]
        return copy.deepcopy(old),copy.deepcopy(new)

    def approve(self, old, new):
        row = {'key': old['identity']['key'], 'old_core': old, 'new_core': new,
               'from_digest': guard.digest(old), 'to_digest': guard.digest(new)}
        approval = {'approved_core_digests': [{k: row[k] for k in ('key','from_digest','to_digest')}], 'approved_graphs': []}
        guard.validate_change(row, {row['key']:row['from_digest']}, new, approval)

    def test_both_exact_pairs_pass_without_mutation(self):
        for key in owner.CORES:
            old,new=self.cores(key);before=copy.deepcopy((old,new))
            self.approve(old,new);self.assertEqual(owner.reviewed_pair(old,new)[0],owner.reviewed_pair(old,new)[1])
            self.assertEqual((old,new),before)

    def test_resealed_track_inventory_effects_pins_and_aliases_reject(self):
        old,new=self.cores(owner.KILMARESH)
        for mutate in [lambda c:c['source_data']['progress'].pop(),
            lambda c:c['source_data']['progress'].append(copy.deepcopy(c['source_data']['progress'][0])),
            lambda c:c['source_data']['progress'][0].update(auxiliary_of=['canary:quest/grave_danger_quest']),
            lambda c:c['source_data']['progress'][0]['transitions'][0]['write'].update(increment=2),
            lambda c:c['source_data']['progress'][0]['transitions'][0]['source_occurrences'][0].update(revision='0'*40),
            lambda c:c.update(native_lowering={'state':'READY'}),
            lambda c:c.update(missing_data=[])]:
            bad=copy.deepcopy(new);mutate(bad)
            with self.subTest(mutate=mutate),self.assertRaises(ValueError):self.approve(old,bad)

    def test_original_grave_graph_is_exact_and_cannot_survive(self):
        old,new=self.cores(owner.GRAVE)
        for variant in ['changed','duplicate','surviving']:
            left,right=copy.deepcopy((old,new))
            if variant=='changed':left['source_data']['interactions'][0]['unresolved']=[]
            elif variant=='duplicate':left['source_data']['interactions']*=2
            else:right['source_data']['interactions']=copy.deepcopy(left['source_data']['interactions'])
            with self.subTest(variant=variant),self.assertRaises(ValueError):self.approve(left,right)

    def test_kilmaresh_graph_remains_visible_to_standard_guard(self):
        old,new=self.cores(owner.KILMARESH)
        graph=self.snapshots['original_grave_graph'];old['source_data']['interactions']=[copy.deepcopy(graph)]
        new['source_data']['interactions']=[copy.deepcopy(graph)];new['source_data']['interactions'][0]['unresolved']=[]
        with self.assertRaisesRegex(ValueError,'unreviewed graph'):self.approve(old,new)

    def test_fresh_parent_cannot_replace_original_baseline(self):
        old,new=self.cores(owner.GRAVE)
        old['source_data']['interactions']=[self.parent_graph]
        self.assertEqual(*owner.reviewed_parent_pair(old,new))
        with self.assertRaises(ValueError):owner.reviewed_pair(old,new)
        old['source_data']['interactions']=[self.snapshots['original_grave_graph']]
        with self.assertRaises(ValueError):owner.reviewed_parent_pair(old,new)

    def test_exact_canary_curation_precedes_version_root_inference(self):
        path = next(iter(owner.TRACKS)).split(':quest-progress/')[1]
        key = questlog.norm(path)
        quest = lambda name: {'identity': {'key': 'canary:quest/'+name}, 'display_name': name, 'missions': []}
        grave,kilmaresh=quest('grave_danger_quest'),quest('kilmaresh_quest')
        grave['missions']=[{'progress':'canary:quest-progress/quest/u12_20/grave_danger/questline'}]
        found = {'count': Counter(canary=1), 'paths': {('canary',path)}, 'transitions': {'a': {
            'owner':'action','script':'scripts/quests/kilmaresh_quest/action.lua','callback':'onUse',
            'from':None,'to':1,'sources':{'canary':{'path':'data-otservbr-global/action.lua','line':1,'registrations':[]}}}}}
        with patch.object(questlog,'storage_declarations',return_value={}), patch.object(questlog,'storage_evidence',return_value={}):
            for curated,expected in [({},grave),({key:{'quest':kilmaresh['identity']['key']}},kilmaresh)]:
                with patch.object(questlog,'TRACK_OWNERS',curated):
                    rows=questlog.auxiliary_tracks({key:found},[],[grave,kilmaresh],[],{'canary':Path('.')})
                self.assertEqual(rows[0]['auxiliary_of'],[expected['identity']['key']])

    def test_foreign_core_or_other_removed_graph_rejects(self):
        old,new=self.cores(owner.GRAVE)
        other=copy.deepcopy(self.snapshots['original_grave_graph']);other['identity']['key']='canary:interaction/other/callback'
        old['source_data']['interactions'].append(other)
        with self.assertRaisesRegex(ValueError,'removal'):self.approve(old,new)
        old['identity']['key']='oteryn:quest.other'
        with self.assertRaises(ValueError):owner.reviewed_pair(old,new)


if __name__=='__main__':unittest.main()
