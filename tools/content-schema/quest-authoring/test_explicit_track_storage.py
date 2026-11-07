"""An admitted Source owner never removes an explicitly required declaration witness."""
import copy
import unittest
from collections import Counter
from unittest.mock import patch
import ots_questlog as q

class ExplicitTrackStorageTests(unittest.TestCase):
 def setUp(self):
  self.path='quest/u10_10/the_gravedigger_of_drefia/mission05'
  self.key='canary:quest-progress/'+self.path
  self.quest='canary:quest/the_gravedigger_of_drefia_quest'
  self.owner={'wiki_quest':'The Gravedigger of Drefia Quest','npc_source':'npc/omrabas.lua','source_declaration_required':True}
  self.declaration={'storage_id':44206,'expression':'Storage.Quest.U10_10.TheGravediggerOfDrefia.Mission05',
   'repository':q.SOURCES['canary']['repository'],'revision':q.SOURCES['canary']['revision'],
   'path':'data-otservbr-global/lib/core/storages.lua','line':1841}
  self.entry={'owner':'npc','callback':None,'from':None,'to':1,'script':'npc/omrabas.lua',
   'sources':{'canary':{'path':'data-otservbr-global/npc/omrabas.lua','line':82,'registrations':[],
     'dialogue':{'keywords':['mission'],'topics':[]}}}}
  self.found={'count':Counter({'canary':1}),'paths':{('canary',self.path)},'transitions':{'one':self.entry}}
 def build(self,declarations):
  catalog=[{'identity':{'key':self.quest},'display_name':'The Gravedigger of Drefia Quest','missions':[]}]
  with patch.object(q,'TRACK_OWNERS',{q.norm(self.path):self.owner}),patch.object(q,'storage_declarations',return_value=declarations):
   return q.auxiliary_tracks({q.norm(self.path):self.found},[],catalog,[],{'canary':'/unused','crystalserver':'/unused'})[0]
 def test_owned_track_preserves_full_declaration_pin(self):
  row=self.build({self.key:self.declaration})
  self.assertEqual(row['auxiliary_of'],[self.quest]);self.assertEqual(row['source_storage'],self.declaration)
  self.assertEqual(row['transitions'][0]['write']['to'],1)
  self.assertNotIn('initial',row);self.assertNotIn('bounds',row)
 def test_missing_decl_keeps_explicit_unknown(self):
  row=self.build({});self.assertIn('UNKNOWN',row['source_checks']['storage_declaration'])
  self.assertNotIn('source_storage',row);self.assertEqual(row['auxiliary_of'],[self.quest])
 def test_conflict_preserves_both_candidates(self):
  other={**self.declaration,'storage_id':44207};candidates=[self.declaration,other]
  row=self.build({self.key:{'state':'CONFLICT','candidates':candidates}})
  self.assertEqual(row['source_storage_candidates'],candidates)
  self.assertIn('CONFLICT',row['source_checks']['storage_declaration']);self.assertNotIn('source_storage',row)
 def test_explicit_flag_does_not_change_other_curations(self):
  self.owner.pop('source_declaration_required');row=self.build({self.key:self.declaration})
  self.assertNotIn('source_storage',row)

if __name__=='__main__':unittest.main()
