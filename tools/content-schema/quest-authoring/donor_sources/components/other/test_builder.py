import base64
import copy
import gzip
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
import jsonschema
import importlib.util
_spec = importlib.util.spec_from_file_location('other_component_builder', Path(__file__).with_name('builder.py'))
producer = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(producer)

ROOT = Path(__file__).parent

def fixture():
 raw=b'if ready then return 7 end\n'
 def node(t,f,span=None):return {'node_type':t,'fields':f,'span':span}
 ret=node('Return',{'values':[node('Number',{'n':7})]}, {'start_char':14,'end_char_exclusive':22,'line':1})
 ast=node('Chunk',{'body':node('Block',{'body':[node('If',{'test':node('Name',{'id':'ready'}),'body':node('Block',{'body':[ret]}),'orelse':None})]})})
 p={'source':'test','repository':'owner/repo','revision':'a'*40,'path':'data/quest.lua','git_blob_sha1':hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest(),'sha256':producer.digest(raw),'byte_count':len(raw)}
 r={'source_component_id':'test:'+'a'*40+':data/quest.lua','provenance':p}
 c={'status':'PARSED','sha256':p['sha256'],'raw_bytes_base64':base64.b64encode(raw).decode(),'ast':ast}
 return raw,r,c

class SourceBehaviorTests(unittest.TestCase):
 def test_roundtrip_preserves_guard_and_return(self):
  raw,r,c=fixture(); out=producer.component(r,r['provenance'],c,raw)
  self.assertEqual(producer.restore(out['definition']['behavior']),c['ast'])
  self.assertEqual([x['kind'] for x in out['definition']['control_flow']],['If','Return'])
  self.assertFalse(out['native_admission']); self.assertFalse(out['coverage']['oteryn_semantic_equivalence'])
 def test_tampered_ast_container_rejected(self):
  with tempfile.TemporaryDirectory() as temp:
   root=Path(temp);(root/'captures').mkdir()
   raw,r,c=fixture();c['native_semantic_admission']=False
   payload=gzip.compress(json.dumps(c).encode(),mtime=0)
   (root/'index.json').write_text(json.dumps({'captures':[{'sha256':r['provenance']['sha256'],'container_sha256':producer.digest(payload)}]}))
   (root/'captures'/(r['provenance']['sha256']+'.json.gz')).write_bytes(payload+b'tampered')
   with self.assertRaises(ValueError):producer.ASTCache(root,set()).get(r['provenance']['sha256'])
 def test_ast_native_admission_rejected(self):
  with tempfile.TemporaryDirectory() as temp:
   root=Path(temp);(root/'captures').mkdir()
   raw,r,c=fixture();c['native_semantic_admission']=True
   payload=gzip.compress(json.dumps(c).encode(),mtime=0)
   (root/'index.json').write_text(json.dumps({'captures':[{'sha256':r['provenance']['sha256'],'container_sha256':producer.digest(payload)}]}))
   (root/'captures'/(r['provenance']['sha256']+'.json.gz')).write_bytes(payload)
   with self.assertRaises(ValueError):producer.ASTCache(root,set()).get(r['provenance']['sha256'])
 def test_call_context_and_symbol_candidates_retained(self):
  raw,r,c=fixture()
  name={'node_type':'Name','fields':{'id':'helper'},'span':None}
  call={'node_type':'Call','fields':{'func':name,'args':[]},'span':None}
  function={'node_type':'LocalFunction','fields':{'name':name,'args':[],'body':{'node_type':'Block','fields':{'body':[call]},'span':None}},'span':None}
  c['ast']['fields']['body']['fields']['body'].append(function)
  out=producer.component(r,r['provenance'],c,raw)
  site=out['definition']['calls'][0]
  self.assertEqual(site['caller_function_symbol'],'helper')
  self.assertEqual(site['same_file_definition_candidates'],[site['caller_function_pointer']])
  self.assertEqual(site['binding_status'],'EXACT_SOURCE_SYMBOL_CANDIDATES_ONLY_SCOPE_AND_RUNTIME_BINDING_UNPROVEN')
 def test_changed_raw_rejected(self):
  raw,r,c=fixture()
  with self.assertRaises(ValueError): producer.component(r,r['provenance'],c,raw+b'-- changed')
 def test_wrong_capture_pin_rejected(self):
  raw,r,c=fixture(); c['sha256']='0'*64
  with self.assertRaises(ValueError): producer.component(r,r['provenance'],c,raw)
 def test_failed_parse_rejected(self):
  raw,r,c=fixture();c['status']='FAILED'
  with self.assertRaises(ValueError):producer.component(r,r['provenance'],c,raw)
 def test_utf8_span_is_converted_to_byte_offsets(self):
  raw='ąreturn 7'.encode();n={'span':{'start_char':1,'end_char_exclusive':9}}
  w=producer.witness(n,'/node',raw,raw.decode());self.assertEqual(w['byte_start'],2);self.assertEqual(w['source_slice_sha256'],producer.digest(b'return 7'))
 def test_out_of_range_span_rejected(self):
  with self.assertRaises(ValueError):producer.witness({'span':{'start_char':0,'end_char_exclusive':500}},'',b'a','a')
 def test_symbol_preserves_dotted_callback(self):
  n={'node_type':'Index','fields':{'value':{'node_type':'Name','fields':{'id':'event'}},'idx':{'node_type':'Name','fields':{'id':'onDeath'}},'notation':{'name':'DOT'}}}
  self.assertEqual(producer.symbol(n),'event.onDeath')
 def test_checked_packet_schema_and_coverage(self):
  path=ROOT/'components.json.gz'
  if not path.exists():self.skipTest('Closed real donor packet not present beside tests')
  data=json.loads(gzip.decompress(path.read_bytes()));schema=json.loads((ROOT/'schema.json').read_text())
  jsonschema.validate(data,schema)
  self.assertEqual(len(data['records']),90);self.assertEqual(len({r['source_component_id'] for r in data['records']}),90)
  for r in data['records']:
   counts={}
   for _,n in producer.iter_nodes(producer.restore(r['definition']['behavior'])):counts[n['node_type']]=counts.get(n['node_type'],0)+1
   self.assertEqual(counts,r['definition']['node_counts'])
   self.assertEqual(counts.get('Return',0),sum(x['kind']=='Return' for x in r['definition']['control_flow']))
   self.assertEqual(counts.get('If',0),sum(x['kind']=='If' for x in r['definition']['control_flow']))
 def test_schema_blocks_native_promotion(self):
  path=ROOT/'components.json.gz'
  if not path.exists():self.skipTest('Closed real donor packet not present beside tests')
  data=json.loads(gzip.decompress(path.read_bytes()));data['records'][0]['native_admission']=True
  schema=json.loads((ROOT/'schema.json').read_text())
  with self.assertRaises(jsonschema.ValidationError):jsonschema.validate(data,schema)

if __name__=='__main__':unittest.main()
