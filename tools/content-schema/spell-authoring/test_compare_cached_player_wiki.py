import copy,gzip,json
from pathlib import Path
import tempfile,unittest
from jsonschema import Draft202012Validator
from jsonschema.exceptions import ValidationError
import compare_cached_player_wiki as c
class CachedWikiTests(unittest.TestCase):
 def page(self,template='Infobox Spell',fields=None):
  raw={'template':template,'title':'Fireball Rune','content_sha256':'a'*64,'revision_id':42,'timestamp':'2026-01-01T00:00:00Z','fields':fields or {}}
  return c.page_fact(raw,'fandom','cache.json','b'*64,'2026-09-27',None,1)
 def test_rune_use_and_creation_never_share_cost_words_or_level_context(self):
  creation=self.page(fields={'type':'Rune','words':'adori flam','mana':'460','levelrequired':'27'});use=self.page('Infobox Object',{'name':'Fireball Rune','words':'adori flam','levelrequired':'27'})
  self.assertEqual(c.matching_pages([creation,use],{'name':'Fireball Rune','carrier':'rune'},'rune_use'),[use])
  self.assertEqual(c.matching_pages([creation,use],{'name':'Fireball Rune','words':'adori flam'},'rune_creation'),[creation])
  self.assertIsNone(next(f for f in use['facts'] if f['field']=='words')['normalized'])
  self.assertEqual(c.compare_field('mana',None,[creation])['status'],'SOURCE_FIELD_ABSENT')
 def test_seconds_exact_milliseconds_and_ambiguous_values_not_guessed(self):
  self.assertEqual(c.normalize('cooldown_ms','1.5','spell_cast')[0],c.value('integer',1500))
  self.assertEqual(c.source_values({'targeting':{'range_tiles':0}})['range'],c.value('integer',0))
  for field,raw in [('mana','20%'),('cooldown_ms','2 / 10'),('range','7')]:self.assertIsNone(c.normalize(field,raw,'spell_cast')[0])
 def test_vocation_families_and_promoted_only_not_false_agreement(self):
  self.assertEqual(c.vocation('[[Druid]]s, [[Sorcerer]]s and [[Monk]]s.'),(['druid','monk','sorcerer'],None))
  self.assertIsNone(c.vocation('[[Druid|Elder Druids]]')[0])
  self.assertIsNone(c.source_values({'requirements':{'vocations':['elder_druid']}})['vocations'])
  self.assertEqual(c.compare_field('vocations',None,[self.page(fields={'voc':'Druid'})],True)['status'],'UNCOMPARABLE')
 def test_wiki_conflicts_differences_missing_zero_false_preserved(self):
  a=self.page(fields={'mana':'0','premium':'no'});b=self.page(fields={'mana':'10'});b['page_key']='c'*64
  self.assertEqual(c.compare_field('mana',c.value('integer',0),[a])['status'],'AGREE')
  self.assertEqual(c.compare_field('premium',c.value('boolean',False),[a])['status'],'AGREE')
  self.assertEqual(c.compare_field('mana',c.value('integer',5),[a,b])['status'],'WIKI_DISAGREEMENT')
  self.assertEqual(c.compare_field('mana',c.value('integer',5),[a])['status'],'DIFFERS')
  self.assertEqual(c.compare_field('mana',c.value('integer',5),[])['status'],'NO_MATCHING_PAGE')
 def test_cached_provenance_not_invented_and_schema_strict(self):
  p=self.page(fields={'name':'Fireball Rune'});self.assertIsNone(p['fetched_timestamp']);self.assertIsNone(p['remote_browser_used'])
  self.assertEqual(p['original_read_method'],'historical_capture_method_not_recorded');self.assertEqual(p['url'],'https://tibia.fandom.com/index.php?title=Fireball_Rune&oldid=42')
  Draft202012Validator(c.schema()).validate(p)
  for field,val in [('network_fetched_this_run',True),('untyped_fact',{})]:
   altered=copy.deepcopy(p);altered[field]=val
   with self.assertRaises(ValidationError):Draft202012Validator(c.schema()).validate(altered)
 def test_full_population_source_variants_manifest_and_immutable_output(self):
  with tempfile.TemporaryDirectory() as tmp:
   out=Path(tmp)/'packet';result=c.run(c.ROOT,out);self.assertEqual(result['source_records'],483);self.assertEqual(result['candidate_records_unchanged'],308)
   rows=[json.loads(l) for l in gzip.decompress((out/'source-wiki-header-comparisons.jsonl.gz').read_bytes()).splitlines()]
   self.assertEqual(len({r['registration_key'] for r in rows}),483)
   fb=[r for r in rows if r['name'].lower()=='fireball rune'];self.assertEqual(len(fb),4);self.assertEqual({r['context'] for r in fb},{'rune_creation','rune_use'})
   self.assertTrue(all(len(r['reference_coverage'])==3 for r in rows))
   pages=[json.loads(l) for l in gzip.decompress((out/'cached-wiki-page-facts.jsonl.gz').read_bytes()).splitlines()];ids={p['page_key'] for p in pages};self.assertTrue(all(set(r['matched_page_keys'])<=ids for r in rows))
   for name,digest in json.loads((out/'package-manifest.json').read_text())['files'].items():self.assertEqual(digest,c.sha((out/name).read_bytes()))
   self.assertFalse((out/'spell.json').exists())
   with self.assertRaises(ValueError):c.run(c.ROOT,out)
 def test_rune_typo_identity_uses_exact_cached_itemid_without_invented_alias(self):
  p=self.page('Infobox Object',{'name':'Fire Bomb Rune','itemid':'3192'});p['title']='Fire Bomb Rune'
  matched,proof=c.match_pages_and_proofs([p],{'name':'firebomb rune','carrier':'rune','reference_rune_item_id':3192},'rune_use')
  self.assertEqual(matched,[p]);self.assertEqual(proof[0]['basis'],'source_rune_item_id')
  self.assertEqual(c.matching_pages([p],{'name':'firebomb rune','carrier':'rune','reference_rune_item_id':9999},'rune_use'),[])
 def test_named_secondary_cooldown_requires_owning_group(self):
  p=self.page(fields={'cooldowngroup2':'2','secondarygroup':'Stance'})
  f=next(f for f in p['facts'] if f['field']=='secondary_group_cooldown_ms')
  self.assertEqual(f['normalized'],c.value('group_cooldown',{'group':'stance','milliseconds':2000}))
  unknown=self.page(fields={'cooldowngroup2':'2'})
  self.assertIsNone(next(f for f in unknown['facts'] if f['field']=='secondary_group_cooldown_ms')['normalized'])
 def test_cached_wheel_unlock_levels_not_registration_minimum_claims(self):
  pages=c.load_pages(c.ROOT,{})
  avatars=[p for p in pages if p['title']=='Avatar of Light' and p['context']=='spell_cast']
  self.assertTrue(avatars)
  self.assertTrue(all(next(f for f in p['facts'] if f['field']=='level')['normalized'] is None for p in avatars))
  self.assertTrue(all(p['level_context_evidence_page_keys'] for p in avatars))
 def test_failed_generation_not_published(self):
  with tempfile.TemporaryDirectory() as tmp:
   out=Path(tmp)/'packet'
   with self.assertRaises(FileNotFoundError):c.run(Path(tmp)/'missing',out)
   self.assertFalse(out.exists());self.assertEqual(list(Path(tmp).iterdir()),[])
if __name__=='__main__':unittest.main()
