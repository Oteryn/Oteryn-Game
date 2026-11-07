"""Every coalesced write retains its exact source and requester context."""
import copy
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from collections import Counter

import ots_questlog as q
import bundle_semantics as s
from test_bundle_authoring import fixture


class SourceOccurrenceTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.roots={source:Path(self.tmp.name)/source for source in ('canary','crystalserver')}
        for root in self.roots.values():root.mkdir()
    def source(self,name,words):
        path=self.roots[name]/q.SOURCES[name]['datapack']/'npc/example.lua'
        path.parent.mkdir(parents=True,exist_ok=True)
        lines=['local function creatureSayCallback(npc, creature, type, message)','  local player = Player(creature)']
        for word in words:
            lines+=['  if MsgContains(message, "'+word+'") then',
                    '    player:setStorageValue(Storage.Quest.Example.Track, 1)','  end']
        lines+=['end'];path.write_text('\n'.join(lines)+'\n');return path
    def projection(self):
        index=q.transition_index(self.roots)
        found=index[q.norm('quest/example/track')]
        transition=q.progress_transition(q.mission_transitions(found)[0])
        value=fixture();row=value['progress'][0];row['key']='canary:quest-progress/quest/example/track'
        row['transitions']=[transition];row['writes']={name:found['count'][name] for name in self.roots}
        value['sources']=[{'repository':q.SOURCES[name]['repository'],'revision':q.SOURCES[name]['revision']} for name in self.roots]
        return value,found,transition
    def test_two_same_effect_occurrences_survive_coalescing(self):
        path=self.source('canary',['first','second'])
        value,found,transition=self.projection()
        self.assertEqual(len(found['transitions']),1)
        self.assertEqual(found['count']['canary'],2)
        occurrences=transition['source_occurrences'];self.assertEqual(len(occurrences),2)
        self.assertEqual([o['occurrence'] for o in occurrences],[1,2])
        self.assertEqual([o['line'] for o in occurrences],[4,7])
        self.assertEqual(transition['sources']['canary']['line'],4)
        self.assertEqual(occurrences[0]['blob_sha256'],hashlib.sha256(path.read_bytes()).hexdigest())
        self.assertEqual(occurrences[1]['line_sha256'],hashlib.sha256(path.read_bytes().splitlines()[6]).hexdigest())
        s.validate_relations(value)
    def test_each_occurrence_retains_requester_sourcecontext(self):
        self.source('canary',['first','second']);self.source('crystalserver',['crystal'])
        value,_,transition=self.projection()
        requesters=[o['write']['requested_by'] for o in transition['source_occurrences']]
        self.assertEqual([r['keywords'] for r in requesters],[['first'],['second'],['crystal']])
        self.assertEqual(requesters[-1]['npc'],'crystal:npc/example')
        self.assertEqual(transition['write']['requested_by']['keywords'],['first'])
        s.validate_relations(value)
    def test_distinct_caller_npcs_keep_their_own_requester(self):
        first=self.source('canary',['yes'])
        second=first.with_name('other.lua');second.write_bytes(first.read_bytes())
        index=q.transition_index(self.roots);found=index[q.norm('quest/example/track')]
        value=fixture();row=value['progress'][0]
        row['key']='canary:quest-progress/quest/example/track'
        row['transitions']=[q.progress_transition(t) for t in q.mission_transitions(found)]
        row['writes']={name:found['count'][name] for name in self.roots}
        occurrences=[o for t in row['transitions'] for o in t['source_occurrences']]
        self.assertEqual({o['write']['requested_by']['npc'] for o in occurrences},
                         {'canary:npc/example','canary:npc/other'})
        self.assertEqual(len(occurrences),2)
        s.validate_relations(value)

    def test_missing_occurrence_rejected(self):
        self.source('canary',['first','second']);value,_,transition=self.projection()
        transition['source_occurrences'].pop()
        with self.assertRaisesRegex(ValueError,'every observed write'):s.validate_relations(value)
    def test_requester_cannot_borrow_first_context(self):
        self.source('canary',['first','second']);value,_,transition=self.projection()
        transition['source_occurrences'][1]['write']['requested_by']=copy.deepcopy(transition['source_occurrences'][0]['write']['requested_by'])
        with self.assertRaisesRegex(ValueError,'requester context'):s.validate_relations(value)
    def test_unexpected_action_writer_blocks_curated_npc_before_all_fallbacks(self):
        target='quest/u10_50/glooth_engineer_outfits/addon1';quest='canary:quest/glooth'
        npc='npc/ezebeth.lua';other='scripts/quests/unknown/shared.lua'
        entries={'npc':{'owner':'npc','callback':None,'from':None,'to':1,'script':npc,
                        'sources':{'canary':{'path':'data-otservbr-global/'+npc,'line':1,
                                             'registrations':[],'dialogue':{'keywords':['yes'],'topics':[1]}}}},
                 'action':{'owner':'action','callback':'onUse','from':None,'to':1,'script':other,
                           'sources':{'canary':{'path':'data-otservbr-global/'+other,'line':1,'registrations':[]}}}}
        found={'count':Counter({'canary':2}),'paths':{('canary',target)},'transitions':entries}
        catalogue=[{'identity':{'key':quest},'display_name':'Glooth','missions':[]},
                   {'identity':{'key':'canary:quest/hero'},'display_name':'Hero',
                    'missions':[{'progress':'canary:quest-progress/quest/u10_50/hero/mission'}]}]
        curator={q.norm(target):{'quest':quest,'npc_source':npc}}
        gate={'identity':{'key':'canary:door-gate/progress/'+target},'quest':{'key':quest},
              'condition':{'progress':'canary:quest-progress/'+target}}
        for gates in [[],[gate]]:
            with self.subTest(gates=bool(gates)), patch.object(q,'TRACK_OWNERS',curator), patch.object(q,'storage_declarations',return_value={}):
                result=q.auxiliary_tracks({q.norm(target):found},[],catalogue,gates,{})[0]
            self.assertEqual(result['owner_basis'],'UNKNOWN')
            self.assertEqual(result['auxiliary_of'],[])
            self.assertIn('writer set changed',result['source_checks']['owner'])
            self.assertEqual(len(result['transitions']),2)

    def test_duplicate_ordinal_rejected(self):
        self.source('canary',['first','second']);value,_,transition=self.projection()
        transition['source_occurrences'][1]['occurrence']=1
        with self.assertRaisesRegex(ValueError,'occurrence ordinal'):s.validate_relations(value)


if __name__=='__main__':unittest.main()
