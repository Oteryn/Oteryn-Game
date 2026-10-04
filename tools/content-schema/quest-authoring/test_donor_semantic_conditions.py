import sys,tempfile,unittest,json,gzip,hashlib,contextlib,io
from pathlib import Path
import os,base64
AUTHORING=Path(os.environ.get('QUEST_AUTHORING_DIR',Path(__file__).resolve().parent))
if not (AUTHORING/'ots_interactions.py').exists():
 for parent in Path(__file__).resolve().parents:
  if (parent/'ots_interactions.py').exists():AUTHORING=parent;break
sys.path.insert(0,str(AUTHORING));import ots_interactions as oi
FIXTURE_PATH=Path(__file__).with_name('condition-ast-fixtures.json.gz')
FIXTURE_SHA256='4ee057771a8bc9933aa574699f09196a6970d8ab9e3760341625c1f97fb1a978'
fixture_bytes=FIXTURE_PATH.read_bytes()
if hashlib.sha256(fixture_bytes).hexdigest()!=FIXTURE_SHA256:raise ValueError('AST fixture artifact mutated')
FIXTURES=json.loads(gzip.decompress(fixture_bytes))['fixtures']
for fixture_sha,row in FIXTURES.items():
 if hashlib.sha256(base64.b64decode(row['source_base64'],validate=True)).hexdigest()!=fixture_sha:raise ValueError('AST fixture source witness mutated')
def capture(raw):
 sha=hashlib.sha256(raw).hexdigest()
 if sha not in FIXTURES:raise ValueError('Unrecorded AST fixture source')
 row=FIXTURES[sha]
 if base64.b64decode(row['source_base64'])!=raw:raise ValueError('AST fixture source differs')
 return {'ast':row['ast'],'raw_bytes_base64':row['source_base64']}
from donor_semantic_conditions import Normalizer, nodes, fields,kind,profile
class Conditions(unittest.TestCase):
 def normal(self,expr,before='',callback='onUse',actor='player',crlf=False):
  tmp=tempfile.TemporaryDirectory();self.addCleanup(tmp.cleanup);root=Path(tmp.name)
  source=f'local a=Action()\nfunction a.{callback}({actor},item,fromPosition,target,toPosition)\n{before}\nif {expr} then\nreturn true\nend\nend\na:aid(1)\n';source=source.replace('\n','\r\n') if crlf else source;(root/'f.lua').write_bytes(source.encode())
  script=oi.Script('canary',root,'f.lua',{},'fixture');script.bind(2,callback);ast=capture(source.encode())['ast'];test=next(fields(n)['test'] for n in nodes(ast) if kind(n)=='If');line=next(n['span']['line'] for n in nodes(ast) if kind(n)=='If')
  normal=Normalizer(script,oi,{'canary:quest-progress/test'},source)
  script.track=lambda storage:'canary:quest-progress/test'
  return normal.normalize(test,line)
 def test_crlf_unicode_exact_opaque_sibling_span(self):
  out=self.normal('player:getStorageValue(Storage.Test)<os.time() and unsupported()',before='-- zażółć',crlf=True);self.assertEqual(out['children'][1]['source_expression'],'unsupported()')
 def test_schema_rejects_runtime_upgrade_or_wrong_operand_pair(self):
  from jsonschema import Draft202012Validator,ValidationError
  schema=json.loads(Path(__file__).with_name('donor_semantic_condition.schema.json').read_text());validator=Draft202012Validator(schema)
  out=self.normal('player:getStorageValue(Storage.Test)<os.time()');validator.validate(out);out['runtime_admission']='READY'
  with self.assertRaises(ValidationError):validator.validate(out)
  out=self.normal('player:getStorageValue(Storage.Test)<os.time()');out['left']=out['right']
  with self.assertRaises(ValidationError):validator.validate(out)
 def test_clock_exact_operand_and_operator(self):
  out=self.normal('player:getStorageValue(Storage.Test) <= os.time()');self.assertEqual(out['kind'],'comparison');self.assertEqual(out['operator'],'<=');self.assertEqual(out['right']['sampling'],'AT_EXPRESSION_EVALUATION');self.assertEqual(out['right']['runtime_owner'],'UNRESOLVED');self.assertEqual(out['runtime_admission'],'NOT_IMPLEMENTED')
 def test_reversed_clock_keeps_order(self):
  out=self.normal('os.time() > player:getStorageValue(Storage.Test)');self.assertEqual(out['left']['kind'],'donor_wall_clock');self.assertEqual(out['operator'],'>')
 def test_world_state_clock(self):
  out=self.normal('Game.getStorageValue(Storage.Test) ~= os.time()');self.assertEqual(out['left']['kind'],'world_state_read')
 def test_boolean_siblings_exact_unknown_retained(self):
  out=self.normal('player:getStorageValue(Storage.Test) < os.time() and unknown()');self.assertEqual(out['operator'],'and');self.assertEqual(out['children'][1]['kind'],'opaque');self.assertEqual(profile(out)['opaque_leaves'],1)
 def test_position_literal(self):
  out=self.normal('item:getPosition() ~= Position(100,200,7)');self.assertEqual(out['kind'],'comparison');self.assertEqual(out['right']['x'],100)
 def test_reject_clock_shadow_mutation_escape_reflection(self):
  for before in ['local os={time=function() return 1 end}','os.time=custom','os=other','mutate(os)','_G.os=other']:
   with self.subTest(before=before):self.assertEqual(self.normal('player:getStorageValue(Storage.Test) < os.time()',before)['kind'],'opaque')
 def test_reject_player_mutation_or_wrong_actor(self):
  for before in ['player=other','player.getStorageValue=custom','mutate(player)']:
   with self.subTest(before=before):self.assertEqual(self.normal('player:getStorageValue(Storage.Test) < os.time()',before)['kind'],'opaque')
  self.assertEqual(self.normal('other:getStorageValue(Storage.Test) < os.time()')['kind'],'opaque')
 def test_reject_clock_args_arithmetic_snapshot(self):
  for expr in ['player:getStorageValue(Storage.Test) < os.time({year=2026})','player:getStorageValue(Storage.Test) < os.time()+10','player:getStorageValue(Storage.Test) < currentTime']:
   with self.subTest(expr=expr):self.assertEqual(self.normal(expr,'local currentTime=os.time()')['kind'],'opaque')
 def test_reject_position_shadow_computed_bad_coordinates(self):
  for expr,before in [('item:getPosition()==Position(100+x,200,7)',''),('item:getPosition()==Position(-1,200,7)',''),('item:getPosition()==Position(100,200,300)',''),('item:getPosition()==Position(100,200,7)','local Position=custom')]:
   with self.subTest(expr=expr,before=before):self.assertEqual(self.normal(expr,before)['kind'],'opaque')
 def test_reject_source_object_shadow_mutation_escape(self):
  for before in ['local item=target','item=target','item.getPosition=custom','inspect(item)']:
   with self.subTest(before=before):self.assertEqual(self.normal('item:getPosition()==Position(100,200,7)',before)['kind'],'opaque')
 def test_same_bytes_two_donors_keep_world_state_namespace(self):
  from unittest.mock import patch
  from donor_semantic_conditions import main
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);authoring=root/'authoring';(authoring/'samples/interactions').mkdir(parents=True);(authoring/'samples/questlog').mkdir();(authoring/'ots_interactions.py').symlink_to(Path(oi.__file__))
   raw=b'local a=Action()\nfunction a.onUse(player,item,fromPosition,target,toPosition)\nif Game.getStorageValue(Storage.Test)<os.time() then\nreturn true\nend\nend\na:aid(1)\n';sha=hashlib.sha256(raw).hexdigest();rows=[];graphs=[];entries=[]
   for donor in ('canary','crystalserver'):
    source=root/'sources'/donor/'f.lua';source.parent.mkdir(parents=True);source.write_bytes(raw);key=donor+':interaction/fixture'
    rows.append({'source':donor,'repository':'fixture/'+donor,'revision':'1'*40,'path':'f.lua','sha256':sha,'git_blob_sha1':hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()})
    graphs.append({'identity':{'key':key},'rules':[{'branch':[{'when':{'unresolved':{'line':3}},'then':[]}]}]});entries.append({'destination':key,'sources':[{'source':donor,'path':'f.lua'}]})
   (authoring/'samples/interactions/interactions.json').write_text(json.dumps({'interactions':graphs}));(authoring/'samples/interactions/manifest.json').write_text(json.dumps({'entries':entries}));(authoring/'samples/questlog/progress.json').write_text(json.dumps({'progress':[]}));(root/'corpus.json').write_text(json.dumps({'files':rows}))
   ast_root=root/'ast';(ast_root/'captures').mkdir(parents=True);payload=json.dumps(capture(raw)).encode();packed=gzip.compress(payload,mtime=0);filename='captures/'+sha+'.json.gz';(ast_root/filename).write_bytes(packed);(ast_root/'index.json').write_text(json.dumps({'captures':[{'sha256':sha,'capture_path':filename,'container_sha256':hashlib.sha256(packed).hexdigest()}]}))
   args=['decoder','--authoring',str(authoring),'--corpus',str(root/'corpus.json'),'--source-root',str(root/'sources'),'--ast-root',str(ast_root),'--output',str(root/'out.json')]
   with patch.object(sys,'argv',args),contextlib.redirect_stdout(io.StringIO()):main()
   result=json.loads((root/'out.json').read_text());self.assertEqual(len(result['conditions']),2);self.assertEqual([r['normalized']['left']['key'].split(':')[0] for r in result['conditions']],['canary','crystalserver'])
 def test_fixture_lookup_rejects_unknown_or_mutated_source(self):
  with self.assertRaisesRegex(ValueError,'Unrecorded'):capture(b'return unknown_source_122')
 def test_no_false_clock_from_string(self):
  out=self.normal('message == "os.time()"');self.assertEqual(out['kind'],'opaque')
if __name__=='__main__':unittest.main()
