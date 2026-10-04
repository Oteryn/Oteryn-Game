import gzip,hashlib,io,json,pathlib,sys,tarfile,tempfile,unittest
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from quest_donor_source_authoring import verify_ast
class AstControls(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.d=pathlib.Path(self.tmp.name);self.sha='a'*64;body=gzip.compress(b'fixture',mtime=0);self.body=body;self.meta=b'original';self.corpus={'files':[{'source':'canary','repository':'opentibiabr/canary','revision':'b'*40,'path':'data/test.lua','sha256':self.sha,'git_blob_sha1':'c'*40,'byte_count':7}]}
  index={'scope':'SOURCE_STRUCTURE_ONLY_NOT_QUEST_OR_RUNTIME_EQUIVALENCE','sources':self.corpus['files'],'captures':[{'sha256':self.sha,'status':'PARSED','container_sha256':hashlib.sha256(body).hexdigest()}],'summary':{'fixture':True}};raw=json.dumps(index).encode();p=self.d/'index.json.gz';p.write_bytes(gzip.compress(raw,mtime=0));self.manifest={'schema':'OTERYN_PORTABLE_LUA_AST_ARCHIVES/v1','native_semantic_admission':False,'index':{'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'uncompressed_sha256':hashlib.sha256(raw).hexdigest()},'metadata_sidecar_sha256':{'proof.json':hashlib.sha256(self.meta).hexdigest()},'archives':[]};self.pack()
 def pack(self,metadata=None,duplicate=False):
  p=self.d/'ast.tar.gz'
  with tarfile.open(p,'w:gz')as t:
   for name,b in [('captures/'+self.sha+'.json.gz',self.body),('metadata/proof.json',self.meta if metadata is None else metadata)]+([('metadata/proof.json',self.meta)] if duplicate else []):
    m=tarfile.TarInfo(name);m.size=len(b);t.addfile(m,io.BytesIO(b))
  self.manifest['archives']=[{'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'byte_count':p.stat().st_size,'capture_count':1,'capture_sha256s':[self.sha]}];self.save()
 def save(self):(self.d/'manifest.json').write_text(json.dumps(self.manifest))
 def test_valid_archive(self):
  self.assertEqual(verify_ast(self.d,self.corpus), {'fixture': True})
 def test_source_revision_mismatch(self):
  self.corpus['files'][0]['revision']='d'*40
  with self.assertRaisesRegex(ValueError,'source membership'):verify_ast(self.d,self.corpus)
 def test_metadata_digest_mismatch_rejected_after_archive_digest_updated(self):
  self.pack(metadata=b'changed')
  with self.assertRaisesRegex(ValueError,'sidecar digest'):verify_ast(self.d,self.corpus)
 def test_duplicate_metadata_rejected(self):
  self.pack(duplicate=True)
  with self.assertRaisesRegex(ValueError,'duplicate AST metadata'):verify_ast(self.d,self.corpus)
 def test_shard_membership_mismatch_rejected(self):
  self.manifest['archives'][0]['capture_sha256s']=[];self.save()
  with self.assertRaisesRegex(ValueError,'shard membership'):verify_ast(self.d,self.corpus)
 def test_native_promotion_rejected(self):
  self.manifest['native_semantic_admission']=True;self.save()
  with self.assertRaisesRegex(ValueError,'Native admission'):verify_ast(self.d,self.corpus)
if __name__=='__main__':unittest.main()
