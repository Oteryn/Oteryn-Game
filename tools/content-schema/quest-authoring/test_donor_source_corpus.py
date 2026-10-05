import copy,hashlib,json,pathlib,tempfile,unittest
from donor_sources.validate_corpus import validate,PINS
class CorpusControls(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.root=pathlib.Path(self.tmp.name);self.addCleanup(self.tmp.cleanup)
  self.x={'schema':'OTERYN_LOSSLESS_DONOR_RUNTIME_SOURCE_CORPUS/v1','sources':[],'files':[],'failures':[],'archives':[],'summary':{'selected_git_tree_files':3,'captured_files':3,'missing_files':0,'unique_git_blobs':1,'excluded_git_blobs':0},'native_semantic_admission':False,'external_wiki_read':False}
  body=b'-- unchanged raw source\r\nreturn "caf\xc3\xa9"\r\n';git=hashlib.sha1(b'blob '+str(len(body)).encode()+b'\0'+body).hexdigest()
  for name,(repo,pin) in PINS.items():
   root=self.root/name;p=root/'data/npc/test.lua';p.parent.mkdir(parents=True);p.write_bytes(body);t=self.root/(name+'.json');t.write_text(json.dumps({'truncated':False,'sha':pin,'tree':[{'type':'blob','path':'data/npc/test.lua','sha':git,'size':len(body)}]}))
   self.x['sources'].append({'source':name,'repository':repo,'revision':pin,'git_tree_path':str(t),'git_tree_sha256':hashlib.sha256(t.read_bytes()).hexdigest(),'selection':'explicit test scope','corpus_root':str(root),'git_tree_selected_files':1,'git_tree_all_blobs':1})
   self.x['files'].append({'source':name,'repository':repo,'revision':pin,'path':'data/npc/test.lua','git_blob_sha1':git,'sha256':hashlib.sha256(body).hexdigest(),'byte_count':len(body),'cache_path':str(p),'read_method':'CACHE_REUSE'})
   if name!='crystal-summer':self.x['archives'].append({'source':name,'repository':repo,'revision':pin,'url':'https://public/source','read_method':'NORMAL_HTTPS_GITHUB_CODELOAD','path':str(self.root/'unused.tar'),'sha256':'0'*64,'bytes':0,'selected_verified_files':1})
 def test_exact_raw_capture_does_not_claim_semantics(self):
  r=validate(self.x);self.assertEqual(r['summary']['captured_files'],3);self.assertIs(r['source_1_to_1_quest_completion'],False)
 def test_raw_bytes_tampered(self):
  pathlib.Path(self.x['files'][0]['cache_path']).write_bytes(b'changed')
  with self.assertRaisesRegex(ValueError,'Git blob byte mismatch'):validate(self.x)
 def test_missing_inventory_member_not_hidden(self):
  self.x['files'].pop()
  with self.assertRaisesRegex(ValueError,'missing file accounting'):validate(self.x)
 def test_explicit_missing_is_reported_not_complete(self):
  f=self.x['files'].pop();self.x['failures']=[{'source':f['source'],'path':f['path'],'error':'unavailable'}];self.x['summary'].update(captured_files=2,missing_files=1)
  self.assertEqual(validate(self.x)['result'],'PASS_EXPLICIT_INCOMPLETE_CAPTURE')
 def test_duplicate_capture_rejected(self):
  self.x['files'].append(copy.deepcopy(self.x['files'][0]))
  with self.assertRaisesRegex(ValueError,'duplicate corpus'):validate(self.x)
 def test_truncated_tree_not_exhaustive(self):
  s=self.x['sources'][0];p=pathlib.Path(s['git_tree_path']);tree=json.loads(p.read_text());tree['truncated']=True;p.write_text(json.dumps(tree));s['git_tree_sha256']=hashlib.sha256(p.read_bytes()).hexdigest()
  with self.assertRaisesRegex(ValueError,'non-truncated'):validate(self.x)
 def test_donor_revision_substitution_rejected(self):
  self.x['sources'][0]['revision']='a'*40
  with self.assertRaisesRegex(ValueError,'revision substitution'):validate(self.x)
 def test_stale_byte_hash_rejected(self):
  self.x['files'][0]['sha256']='0'*64
  with self.assertRaisesRegex(ValueError,'raw SHA256'):validate(self.x)
 def test_capture_path_escape_rejected(self):
  self.x['files'][0]['cache_path']=self.x['files'][1]['cache_path']
  with self.assertRaisesRegex(ValueError,'corpus path escapes'):validate(self.x)
 def test_relative_paths_resolve_from_manifest_directory(self):
  for source in self.x['sources']:
   source['git_tree_path']=str(pathlib.Path(source['git_tree_path']).relative_to(self.root));source['corpus_root']=source['source']
  for f in self.x['files']:f['cache_path']=str(pathlib.Path(f['cache_path']).relative_to(self.root))
  self.assertEqual(validate(self.x,self.root)['summary']['captured_files'],3)
 def test_shared_content_addressed_blob_preserves_all_file_identities(self):
  shared=self.root/'blobs';shared.mkdir();f0=self.x['files'][0];blob=shared/f0['git_blob_sha1'];blob.write_bytes(pathlib.Path(f0['cache_path']).read_bytes())
  for source in self.x['sources']:source['corpus_root']=str(shared)
  for f in self.x['files']:f['cache_path']=str(blob);f['file_id']=f['source']+':'+f['revision']+':'+f['path']
  self.assertEqual(validate(self.x)['summary']['captured_files'],3)
 def test_forged_file_id_is_not_a_source_binding(self):
  self.x['files'][0]['file_id']='forged'
  with self.assertRaisesRegex(ValueError,'file identity differs'):validate(self.x)
if __name__=='__main__':unittest.main()
