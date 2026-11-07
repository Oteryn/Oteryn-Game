"""Guard factual curations against changed pins and incomplete source transcription."""
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from collections import Counter
import ots_questlog as q

class SourceCurationTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name);p=self.root/'data/scripts/event.lua';p.parent.mkdir(parents=True);p.write_text('local event = Action()\nevent:register()\n')
        expected=q.SOURCES['canary']
        self.entry={'title':'Example','source_paths':['data/scripts/event.lua'],'coverage_gap':'World effects unconverted',
                    'source_evidence':[{'source':'canary','repository':expected['repository'],'revision':expected['revision'],
                         'path':'data/scripts/event.lua','line':2,'blob_sha256':hashlib.sha256(p.read_bytes()).hexdigest(),
                         'role':'executable','classification':'OTS_HYPOTHESIS_ONLY'}]}
    def test_exact_pinned_source_is_accepted(self):
        q.validate_source_curations([self.entry],{'canary':self.root})
    def test_changed_blob_is_rejected(self):
        (self.root/'data/scripts/event.lua').write_text('changed\n')
        with self.assertRaisesRegex(ValueError,'stale source'):q.validate_source_curations([self.entry],{'canary':self.root})
    def test_wrong_pin_is_rejected(self):
        self.entry['source_evidence'][0]['revision']='0'*40
        with self.assertRaisesRegex(ValueError,'invalid source'):q.validate_source_curations([self.entry],{'canary':self.root})
    def test_summer_comparison_alone_cannot_promote(self):
        self.entry['source_evidence'][0]['source']='crystal-summer'
        with self.assertRaisesRegex(ValueError,'missing pinned canonical'):q.validate_source_curations([self.entry],{'canary':self.root})
    def test_source_presence_emits_explicit_gap(self):
        cat=[{'display_name':'Example','identity':{'key':'canary:quest/example'}}]
        self.assertEqual(q.curation_coverage_holds(cat,[self.entry])[0]['quest'],'canary:quest/example')
        self.entry['npc_source']='npc/example.lua'
        self.assertEqual(q.curation_coverage_holds(cat,[self.entry]),[]) # counted by existing NPC holds
    def test_multi_npc_curation_requires_all_and_only_observed_npc_writers(self):
        found={'transitions':{'a':{'script':'npc/a.lua','owner':'npc'},'b':{'script':'npc/b.lua','owner':'npc'}}}
        self.assertTrue(q.exclusive_npc_writers(found,{'npc_sources':['npc/a.lua','npc/b.lua']}))
        self.assertFalse(q.exclusive_npc_writers(found,{'npc_source':'npc/a.lua'}))
        found['transitions']['b']['owner']='library'
        self.assertFalse(q.exclusive_npc_writers(found,{'npc_sources':['npc/a.lua','npc/b.lua']}))

    def test_storage_prefix_requires_whole_segment(self):
        prefixes={'quest/u7_8/citizen_outfits':{'citizen'}}
        self.assertIsNone(q.unique_track_prefix({'quest/u7_8/citizen_outfits_rook/addon'},prefixes))
        self.assertEqual(q.unique_track_prefix({'quest/u7_8/citizen_outfits/addon'},prefixes),'quest/u7_8/citizen_outfits')
    def test_distinct_underscored_fields_are_not_prefix_aliases(self):
        self.assertIsNone(q.unique_track_prefix({'quest/a_b/stage'},{'quest/ab':{'q'}}))
    def test_exclusive_npc_owner_wins_over_broad_release_prefix_and_keeps_write(self):
        target='quest/u10_50/glooth_engineer_outfits/addon1'
        source={'path':'data-otservbr-global/npc/ezebeth.lua','line':10,
                'registrations':[], 'dialogue':{'keywords':['yes'],'topics':[1]}}
        write={'owner':'npc','callback':None,'from':[{'test':'==','value':0}], 'to':1,
               'script':'npc/ezebeth.lua','sources':{'canary':source}}
        index={q.norm(target):{'count':Counter({'canary':1}),'paths':{('canary',target)},'transitions':{'write':write}}}
        catalogue=[{'identity':{'key':'canary:quest/hero'},'display_name':'Hero',
                    'missions':[{'progress':'canary:quest-progress/quest/u10_50/hero/mission'}]},
                   {'identity':{'key':'canary:quest/glooth'},'display_name':'Glooth'}]
        curator={q.norm(target):{'npc_source':'npc/ezebeth.lua','wiki_quest':'Glooth'}}
        with patch.object(q,'TRACK_OWNERS',curator):
            result=q.auxiliary_tracks(index,[],catalogue,[],{})[0]
        self.assertEqual(result['auxiliary_of'],['canary:quest/glooth'])
        descriptor=result['transitions'][0]['write']
        self.assertEqual(descriptor['from'],write['from'])
        self.assertEqual(descriptor['to'],1)
        self.assertEqual(descriptor['owner'],'npc')
        self.assertIn('requested_by',descriptor)

    def test_missing_gap_is_rejected(self):
        self.entry.pop('coverage_gap')
        with self.assertRaisesRegex(ValueError,'coverage gap'):q.validate_source_curations([self.entry],{'canary':self.root})

if __name__=='__main__':unittest.main()
