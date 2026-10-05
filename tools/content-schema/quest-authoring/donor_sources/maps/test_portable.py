import copy,gzip,hashlib,json,struct,tempfile,unittest
from pathlib import Path
import verify
sha=lambda b:hashlib.sha256(b).hexdigest()
class PortableTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.root=Path(self.tmp.name);self.data=self.root/'thin';self.cache=self.root/'relocated-cache';self.data.mkdir();(self.cache/'source-blobs').mkdir(parents=True);(self.cache/'complete-source-maps/node-indexes').mkdir(parents=True)
        schema=(Path(__file__).resolve().parent/'source_map_structural.schema.json').read_bytes();(self.data/'source_map_structural.schema.json').write_bytes(schema)
        raw=b'\0'*4+b'\xfe\0\xff';d=sha(raw);self.blob=self.cache/'source-blobs'/d;self.blob.write_bytes(raw)
        idx=gzip.compress(struct.pack('<6I',0,0xffffffff,4,7,5,6));ip=self.cache/'complete-source-maps/node-indexes'/(d+'.nodes.gz');ip.write_bytes(idx)
        row={'artifact_sha256':d,'artifact_path':'source-blobs/'+d,'source_refs':[{'source':'canary','repository':'opentibiabr/canary','revision':'04b83b512114bfd888000d6e1433ed8ecaec7c5b','source_path':'data-canary/world/fixture.otbm','git_blob_sha1':hashlib.sha1(b'blob 7\0'+raw).hexdigest()}],'codec':'OTERYN_OTBM_STRUCTURAL_NODE_INDEX/v1','attributes_interpreted':False,'native_promotion':False,'runtime_enabled':False,'status':'STRUCTURAL_AST_COMPLETE','index_path':'complete-source-maps/node-indexes/'+d+'.nodes.gz','index_sha256':sha(idx),'index_byte_count':len(idx),'decoded_stream_sha256':d,'decoded_stream_bytes':7,'node_count':1,'decoded_bytes':7,'max_depth':1,'types':{'0':1}}
        self.receipt={'schema':'OTERYN_SOURCE_MAP_STRUCTURAL_RECEIPT/v1','records':[row],'semantics_complete':False};self.inputs={'archives':[],'archive_members':[],'acquisitions':[]};(self.data/'inputs.normalized.json').write_text(json.dumps(self.inputs))
        self.m={'schema':'OTERYN_PORTABLE_SOURCE_MAP_QUALIFICATION/v1','files':[],'receipt':{},**self.inputs,'summary':{'unique_otbm_streams':1,'unique_stream_nodes':1},'semantics_complete':False,'native_promotion':False,'runtime_enabled':False};self.save()
    def tearDown(self):self.tmp.cleanup()
    def save(self):
        p=self.data/'structural-receipt.json';p.write_text(json.dumps(self.receipt));self.m['receipt']={'path':p.name,'sha256':sha(p.read_bytes())};self.m['files']=[{'path':n,'sha256':sha((self.data/n).read_bytes())} for n in ['source_map_structural.schema.json','inputs.normalized.json']];self.manifest=self.data/'qualification.json';self.manifest.write_text(json.dumps(self.m))
    def test_relocated_cache(self):self.assertEqual(verify.qualify(self.manifest,self.cache)['mode'],'SOURCE_BACKED')
    def test_explicit_directories(self):self.assertTrue(verify.qualify(self.manifest,blob_root=self.cache/'source-blobs',index_root=self.cache/'complete-source-maps/node-indexes')['valid'])
    def test_structural_only_does_not_claim_cache_proof(self):self.assertEqual(verify.qualify(self.manifest,structural_only=True)['cache_provenance'],'NOT_VERIFIED')
    def test_cache_required(self):
        with self.assertRaises(ValueError):verify.qualify(self.manifest)
    def test_missing_blob(self):
        self.blob.unlink()
        with self.assertRaises(OSError):verify.qualify(self.manifest,self.cache)
    def test_corrupt_blob(self):
        self.blob.write_bytes(b'changed')
        with self.assertRaises(ValueError):verify.qualify(self.manifest,self.cache)
    def test_traversal(self):
        self.receipt['records'][0]['artifact_path']='../private';self.save()
        with self.assertRaises(ValueError):verify.qualify(self.manifest,self.cache)
    def test_wrong_pin(self):
        self.receipt['records'][0]['source_refs'][0]['revision']='latest';self.save()
        with self.assertRaises(Exception):verify.qualify(self.manifest,self.cache)
    def test_promotion(self):
        self.m['runtime_enabled']=True;self.save()
        with self.assertRaises(ValueError):verify.qualify(self.manifest,self.cache)
    def test_wrong_count(self):
        self.receipt['records'][0]['node_count']=2;self.save()
        with self.assertRaises(ValueError):verify.qualify(self.manifest,self.cache)
    def test_wrong_index_digest(self):
        self.receipt['records'][0]['index_sha256']='0'*64;self.save()
        with self.assertRaises(ValueError):verify.qualify(self.manifest,self.cache)
    def test_receipt_mutation_without_new_binding(self):
        (self.data/'structural-receipt.json').write_text('{}')
        with self.assertRaises(ValueError):verify.qualify(self.manifest,self.cache)
if __name__=='__main__':unittest.main()
