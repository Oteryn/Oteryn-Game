import unittest,base64,pathlib,tempfile
from donor_sources.npc_capture import lexical,annotations,capture_file,build,resolve_source_bytes,PINS
class CaptureTests(unittest.TestCase):
 def test_comments_strings_not_executable_annotations(self):
  text='-- npcHandler:say("comment")\nlocal s="if x then dofile(\\\"fake\\\") end"\nif player:getStorageValue(42)==1 then npcHandler:say("yes",cid) end\n'
  a=annotations(text);self.assertEqual(len(a['control_flow']),1);self.assertEqual(a['includes'],[]);self.assertEqual(len(a['string_literals']),2);self.assertEqual(len(a['comment_spans']),1)
 def test_long_brackets_and_escaped_quote_preserve_offsets(self):
  text='--[==[ if fake then ]==]\nlocal s=[=[raw\ntext]=]\nlocal x="a\\\"b"\nreturn s'
  mask,s,c=lexical(text);self.assertEqual(len(mask),len(text));self.assertEqual(mask.count('\n'),text.count('\n'));self.assertEqual(len(s),2);self.assertEqual(len(c),1)
  for span in s+c:self.assertEqual(span['raw'],text[span['start_char']:span['end_char']])
 def test_dynamic_include_expression_preserved_no_resolution(self):
  a=annotations('dofile(DATA_DIRECTORY .. "/npc/lib/npclib.lua")\nrequire("foo")')
  self.assertEqual(len(a['includes']),2);self.assertEqual(a['includes'][0]['argument_raw'],'DATA_DIRECTORY .. "/npc/lib/npclib.lua"');self.assertTrue(all(x['resolution']=='UNRESOLVED_EXPRESSION_PRESERVED' for x in a['includes']))
 def test_unterminated_string_kept_not_accepted(self):
  a=annotations('local x="unterminated\nif fake then');self.assertEqual(a['lexically_unterminated_regions'],1);self.assertEqual(a['control_flow'],[])
 def test_byte_exact_non_utf8(self):
  with tempfile.TemporaryDirectory() as d:
   root=pathlib.Path(d);p=root/'data/npc/x.lua';p.parent.mkdir(parents=True);body=b'\xff\r\n';p.write_bytes(body);r=capture_file('canary',p,root);import hashlib;self.assertEqual(r['source_bytes_ref']['sha256'],hashlib.sha256(body).hexdigest());self.assertEqual(r['source_bytes_ref']['git_blob_sha1'],hashlib.sha1(b'blob '+str(len(body)).encode()+b'\0'+body).hexdigest());self.assertEqual(r['text_decode'],'NON_UTF8_RAW_ONLY');self.assertIsNone(r['annotations'])
 def test_selected_file_scope_and_determinism(self):
  with tempfile.TemporaryDirectory() as d:
   root=pathlib.Path(d)
   for name in ['data/npc/main.lua','data/npc/helpers/f.lua','data/lib/npc.lua','data/libs/functions/string.lua','data/npclib/npc_system/npc_handler.lua','data/scripts/unrelated.lua']:
    p=root/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text('return true\n')
   a=build({'canary':root});b=build({'canary':root});self.assertEqual(a,b);self.assertEqual(a['summary']['files'],5);self.assertFalse(a['summary']['semantic_callback_completion_certified']);self.assertFalse(a['summary']['computed_include_closure_certified'])
 def test_portable_manifest_shared_cas(self):
  import json,hashlib
  with tempfile.TemporaryDirectory() as d:
   root=pathlib.Path(d);body=b'local s="hello"\n';git=hashlib.sha1(b'blob '+str(len(body)).encode()+b'\0'+body).hexdigest();p=root/'blobs'/git;p.parent.mkdir();p.write_bytes(body)
   row={'source':'canary','repository':PINS['canary'][0],'revision':PINS['canary'][1],'path':'data/npc/x.lua','git_blob_sha1':git,'sha256':hashlib.sha256(body).hexdigest(),'byte_count':len(body),'blob_path':'blobs/'+git};manifest=root/'manifest.json';manifest.write_text(json.dumps({'files':[row],'failures':[]}));packet=build({},corpus_manifest=manifest);self.assertEqual(resolve_source_bytes(packet['files'][0],manifest),body);self.assertNotIn(str(root),str(packet));self.assertNotIn('source_bytes_base64',packet['files'][0])
   row['blob_path']='../private';manifest.write_text(json.dumps({'files':[row]}))
   with self.assertRaises(ValueError):build({},corpus_manifest=manifest)
 def test_wrong_pin_and_duplicate_file_reject(self):
  import json,hashlib
  with tempfile.TemporaryDirectory() as d:
   root=pathlib.Path(d);p=root/'blob';body=b'-- npc';p.write_bytes(body);row={'source':'canary','revision':PINS['canary'][1],'path':'data/npc/x.lua','sha256':hashlib.sha256(body).hexdigest(),'byte_count':len(body),'git_blob_sha1':hashlib.sha1(b'blob '+str(len(body)).encode()+b'\0'+body).hexdigest(),'blob_path':'blob'};m=root/'manifest.json';m.write_text(json.dumps({'files':[row,row]}))
   with self.assertRaisesRegex(ValueError,'Duplicate'):build({},corpus_manifest=m)
   row['revision']='0'*40;m.write_text(json.dumps({'files':[row]}))
   with self.assertRaisesRegex(ValueError,'pin'):build({},corpus_manifest=m)
 def test_schema_rejects_semantic_completion_claim(self):
  import json,jsonschema,copy
  schema=json.loads((pathlib.Path(__file__).parent/'donor_sources/npc_capture.schema.json').read_text())
  with tempfile.TemporaryDirectory() as d:
   root=pathlib.Path(d);p=root/'data/npc/x.lua';p.parent.mkdir(parents=True);p.write_text('return true\n');packet=build({'canary':root});jsonschema.validate(packet,schema)
   bad=copy.deepcopy(packet);bad['summary']['semantic_callback_completion_certified']=True
   with self.assertRaises(jsonschema.ValidationError):jsonschema.validate(bad,schema)
if __name__=='__main__':unittest.main()
