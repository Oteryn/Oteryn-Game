import gzip,json,struct,subprocess,tempfile,unittest
from pathlib import Path
P=Path(__file__).resolve().parent

def escape(b):return b''.join((b'\xfd'+bytes([v])) if v in (253,254,255) else bytes([v]) for v in b)
def node(typ,payload=b'',children=()):return b'\xfe'+escape(bytes([typ])+payload)+b''.join(children)+b'\xff'
class NodeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.compiler_tmp=tempfile.TemporaryDirectory()
        cls.exe=Path(cls.compiler_tmp.name)/'otbm_nodes'
        subprocess.run(['g++','-O2','-std=c++17',str(P/'otbm_nodes.cpp'),'-lz','-o',str(cls.exe)],check=True)
    @classmethod
    def tearDownClass(cls):cls.compiler_tmp.cleanup()
    def run_case(self,data,success=True):
        with tempfile.TemporaryDirectory() as tmp:
            src=Path(tmp)/'src';idx=Path(tmp)/'idx';src.write_bytes(data)
            r=subprocess.run([str(self.exe),str(src),str(idx)],capture_output=True,text=True)
            if not success:self.assertNotEqual(r.returncode,0);return
            self.assertEqual(r.returncode,0,r.stderr);stats=json.loads(r.stdout)
            rows=list(struct.iter_unpack('<6I',gzip.decompress(idx.read_bytes())))
            self.assertEqual(len(rows),stats['node_count'])
            # Independent source reconstruction uses only parent/order and payload ranges.
            byid={r[0]:r for r in rows};children={i:[] for i in byid}
            for r in rows:
                if r[1]!=0xffffffff:children[r[1]].append(r[0])
                self.assertEqual(data[r[2]],254);self.assertEqual(data[r[3]-1],255)
            def replay(i):
                r=byid[i]
                return b'\xfe'+data[r[4]:r[5]]+b''.join(replay(k) for k in sorted(children[i],key=lambda k:byid[k][2]))+b'\xff'
            self.assertEqual(data[:4]+replay(0),data)
            self.assertEqual(sum(r[5]-r[4] for r in rows)+2*len(rows)+4,len(data))
            return stats
    def test_unknown_types_attrs_and_escaped_markers(self):
        d=b'OTBM'+node(0,b'head',[node(231,b'\x99\xfd\xfe\xff',[node(6,b'\x04\x01\x00')])])
        s=self.run_case(d);self.assertEqual(s['types']['231'],1)
    def test_deep_container_tree_above_old_depth_eight(self):
        d=node(6,b'item')
        for i in range(20):d=node(6,bytes([i]),[d])
        s=self.run_case(b'\0'*4+node(0,b'',[d]));self.assertEqual(s['max_depth'],22)
    def test_preserve_duplicate_siblings_and_order(self):
        self.run_case(b'\0'*4+node(0,b'',[node(6,b'A'),node(6,b'A'),node(6,b'B')]))
    def test_empty_payload_and_unknown_escaped_type(self):self.run_case(b'\0'*4+node(0,b'',[node(253)]))
    def test_reject_missing_close(self):self.run_case(b'\0'*4+b'\xfe\0',False)
    def test_reject_trailing_source(self):self.run_case(b'\0'*4+node(0)+b'X',False)
    def test_reject_properties_after_child(self):self.run_case(b'\0'*4+b'\xfe\0'+node(6)+b'X\xff',False)
    def test_reject_truncated_escape(self):self.run_case(b'\0'*4+b'\xfe\0\xfd',False)
    def test_reject_multiple_roots(self):self.run_case(b'\0'*4+node(0)+node(0),False)
    def test_reject_corrupt_gzip_crc(self):
        raw=b'\0'*4+node(0)
        with tempfile.TemporaryDirectory() as tmp:
            src=Path(tmp)/'src';idx=Path(tmp)/'idx';b=bytearray(gzip.compress(raw));b[-8]^=1;src.write_bytes(b)
            r=subprocess.run([str(self.exe),str(src),str(idx)],capture_output=True,text=True)
            self.assertNotEqual(r.returncode,0)
    def test_gzip_matches_raw_structure(self):
        raw=b'\0'*4+node(0,b'x',[node(6,b'item')])
        with tempfile.TemporaryDirectory() as tmp:
            src=Path(tmp)/'src';idx=Path(tmp)/'idx';src.write_bytes(gzip.compress(raw))
            r=subprocess.run([str(self.exe),str(src),str(idx)],capture_output=True,text=True)
            self.assertEqual(r.returncode,0,r.stderr);self.assertEqual(json.loads(r.stdout)['decoded_bytes'],len(raw))
if __name__=='__main__':unittest.main()
