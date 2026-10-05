import sys,pathlib,unittest,os,copy,json,tempfile,subprocess,hashlib
authoring = os.environ.get('QUEST_AUTHORING_DIR')
if authoring:sys.path.insert(0,authoring)
else:
 for parent in pathlib.Path(__file__).resolve().parents:
  if (parent/'lua_writers.py').exists():sys.path.insert(0,str(parent));break
from lua_writers import mask_code
from donor_sources.reward_capture import capture_lua,digest,verify_component

class CaptureTests(unittest.TestCase):
 def scan(self,text):return capture_lua(text.encode(),'canary','test.lua',mask_code)
 def test_comments_and_string_decoys_are_ignored(self):
  rows=self.scan('-- player:addItem(1,2)\nlocal s="player:addItem(3,4)"\nplayer:addItem(5, 1)')
  self.assertEqual([x['argument_expressions']for x in rows],[['5','1']])
 def test_nested_dynamic_random_is_preserved(self):
  text='player:addItem(rewards[math.random(#rewards)].id, amount + 2)'
  rows=self.scan(text);self.assertEqual(rows[0]['raw_expression'],text)
  self.assertEqual(rows[0]['argument_expressions'],['rewards[math.random(#rewards)].id','amount + 2'])
  self.assertTrue(any(x['category']=='random_expression'for x in rows));self.assertFalse(any(x['semantic_decoded']or x['native_bound']for x in rows))
 def test_utf8_and_crlf_bytes(self):
  blob='-- ąę\r\nplayer:addItem(7,1)\r\n'.encode();rows=capture_lua(blob,'canary','test.lua',mask_code)
  self.assertEqual(blob[rows[0]['byte_start']:rows[0]['byte_end']],rows[0]['raw_expression'].encode())
  self.assertNotEqual(rows[0]['char_start'],rows[0]['byte_start'])
 def test_nested_literals_with_comma_are_exact(self):
  row=self.scan('player:addAchievement("A, B")')[0];self.assertEqual(row['argument_expressions'],['"A, B"'])
 def test_incomplete_call_is_not_complete(self):
  row=self.scan('player:addItem(rewards[')[0];self.assertFalse(row['capture_complete'])
 def test_raw_requirement_and_cooldown_do_not_invent_values(self):
  rows=self.scan('if player:getLevel() >= requiredLevel and os.time() > player:getStorageValue(Storage.Cooldown) then\nplayer:setStorageValue(Storage.Cooldown, os.time()+delay)\nend')
  self.assertEqual(len(rows),5);self.assertTrue(all(not x['semantic_decoded']for x in rows))

 def test_resealed_span_metadata_and_admission_mutants_reject(self):
  blob=b'player:addItem(7,1)';row=capture_lua(blob,'canary','test.lua',mask_code)[0]
  self.assertTrue(verify_component(blob,row))
  for key,value in [('byte_start',1),('byte_end',1),('line_start',99),('blob_sha256','0'*64),('raw_expression_sha256','0'*64),('native_bound',True),('semantic_decoded',True)]:
   with self.subTest(key=key):
    bad=copy.deepcopy(row);bad[key]=value
    with self.assertRaises(ValueError):verify_component(blob,bad)
 def test_relative_blob_manifest_and_changed_git_source_rejection(self):
  import lua_writers
  repo=pathlib.Path(lua_writers.__file__).resolve().parents[3]
  blob=b'player:addItem(7,1)';gitsha=hashlib.sha1(b'blob '+str(len(blob)).encode()+b'\0'+blob).hexdigest()
  with tempfile.TemporaryDirectory() as tmp:
   root=pathlib.Path(tmp);(root/'blobs').mkdir();(root/'blobs'/gitsha).write_bytes(blob)
   manifest={'files':[{'source':'canary','repository':'opentibiabr/canary','revision':'04b83b512114bfd888000d6e1433ed8ecaec7c5b','path':'test.lua','cache_path':'blobs/'+gitsha,'sha256':digest(blob),'git_blob_sha1':gitsha}]}
   p=root/'manifest.json';p.write_text(json.dumps(manifest));cmd=[sys.executable,str(pathlib.Path(__file__).parent/'donor_sources/reward_capture.py'),'--repo-root',str(repo),'--corpus-manifest',str(p),'--out',str(root/'out.json')]
   result=subprocess.run(cmd,capture_output=True);self.assertEqual(result.returncode,0,result.stderr)
   manifest['files'][0]['git_blob_sha1']='0'*40;p.write_text(json.dumps(manifest));self.assertNotEqual(subprocess.run(cmd,capture_output=True).returncode,0)

 def test_helper_declaration_is_not_call_candidate(self):
  row=self.scan('function player:addItem(id, quantity)\nend')[0];self.assertEqual(row['syntax_role'],'declaration')

if __name__=='__main__':unittest.main()
