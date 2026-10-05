import base64
import json
import unittest
from builder import convert, validate_capture, digest, symbol, load_captures, build


def node(kind, **fields):
    return {'node_type': kind, 'span': None, 'fields': fields}


def original(ir, refs):
    if isinstance(ir, list): return [original(v, refs) for v in ir]
    if not isinstance(ir, dict): return ir
    if 'lua_node_type' in ir:
        return {'node_type': ir['lua_node_type'], 'span': refs[ir['source_ref']]['char_span'],
                'fields': {k: original(v, refs) for k, v in ir['fields'].items()}}
    return {k: original(v, refs) for k, v in ir.items()}


class EventIRTests(unittest.TestCase):
    def test_order_branches_loops_returns_and_unknowns_retained(self):
        ast = node('Chunk', body=node('Block', body=[node('If', test=node('Name', id='guard'),
                   body=node('Block', body=[node('Return', values=[node('TrueExpr', value=True)])]), orelse=node('Block', body=[])),
                   node('Forin', body=node('Block', body=[]), iter=[node('Name', id='custom')]),
                   node('Invoke', source=node('Name', id='custom'), func=node('Name', id='opaque'), args=[])]))
        ir = convert({'ast': ast}, b'')
        self.assertEqual(original(ir['program'], ir['source_refs']), ast)
        self.assertEqual([x['lua_node_type'] for x in ir['control_flow']], ['Return', 'If', 'Forin'])
        self.assertEqual(ir['calls'][0]['operation'], 'SOURCE_CALL_UNRESOLVED')
        self.assertEqual([x['kind'] for x in ir['program']['fields']['body']['fields']['body']], ['BRANCH','LOOP','CALL'])

    def test_time_does_not_mean_event_schedule(self):
        call = node('Call', func=node('Index', value=node('Name', id='os'), idx=node('Name', id='time')), args=[])
        ir = convert({'ast': call}, b'')
        self.assertEqual(ir['calls'][0]['operation'], 'SOURCE_CALL_UNRESOLVED')

    def test_event_interval_receiver_configured(self):
        ast = node('Chunk', body=node('Block', body=[node('LocalAssign', targets=[node('Name', id='event')], values=[node('Call', func=node('Name', id='GlobalEvent'), args=[])]),
            node('Invoke', source=node('Name', id='event'), func=node('Name', id='interval'), args=[node('Number', n=1000)])]))
        ir = convert({'ast': ast}, b'')
        self.assertEqual(ir['calls'][-1]['operation'], 'SOURCE_INTERVAL_CONFIG')
        self.assertEqual(ir['event_receiver_candidates'], ['event'])

    def test_provenance_byte_encoding(self):
        ast = node('String', raw='ą'); ast['span']={'start_char':0,'end_char_exclusive':1}
        ir = convert({'ast': ast}, 'ą'.encode())
        self.assertEqual(ir['source_refs'][0]['byte_end_exclusive'], 2)
        self.assertEqual(ir['source_refs'][0]['span_sha256'], digest('ą'.encode()))

    def test_capture_admission_and_raw_mismatch_rejected(self):
        c={'status':'PARSED','native_semantic_admission':False,'raw_bytes_base64':base64.b64encode(b'a').decode(),'sha256':digest(b'a')}
        validate_capture(c,b'a')
        with self.assertRaises(ValueError): validate_capture(c,b'b')
        c['native_semantic_admission']=True
        with self.assertRaises(ValueError): validate_capture(c,b'a')

    def test_out_of_bounds_rejected(self):
        ast=node('Name', id='x'); ast['span']={'start_char':0,'end_char_exclusive':2}
        with self.assertRaises(ValueError): convert({'ast':ast},b'x')

    def test_method_symbol(self):
        self.assertEqual(symbol(node('Invoke',source=node('Name',id='event'),func=node('Name',id='register'),args=[])), 'event:register')


class ProvenanceControls(unittest.TestCase):
    def test_index_tamper_rejected(self):
        import tempfile
        import gzip
        from pathlib import Path
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); (root/'captures').mkdir()
            payload=b'{"status":"PARSED"}\n'; packed=gzip.compress(payload,mtime=0); sha='a'*64
            (root/'captures'/(sha+'.json.gz')).write_bytes(packed)
            (root/'index.json').write_text(json.dumps({'captures':[{'sha256':sha,'container_sha256':digest(packed),'capture_sha256':digest(payload)}]}))
            self.assertEqual(load_captures(root,{sha})[sha]['status'],'PARSED')
            (root/'captures'/(sha+'.json.gz')).write_bytes(gzip.compress(b'{"status":"ALTERED"}\n',mtime=0))
            with self.assertRaisesRegex(ValueError,'container'):load_captures(root,{sha})

    def test_donor_scope_witnesses(self):
        import tempfile
        import gzip
        import hashlib
        from pathlib import Path
        ast=node('Chunk',body=node('Block',body=[node('LocalAssign',targets=[node('Name',id='event')],values=[node('Call',func=node('Name',id='CreatureEvent'),args=[node('String',s={'bytes_base64':base64.b64encode(b'Shared').decode()})])])]))
        raw=b'local event = CreatureEvent("Shared")'; sha=digest(raw)
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); (root/'captures').mkdir()
            capture={'ast':ast,'status':'PARSED','native_semantic_admission':False,'sha256':sha,'raw_bytes_base64':base64.b64encode(raw).decode()}
            payload=json.dumps(capture).encode();packed=gzip.compress(payload,mtime=0)
            (root/'captures'/(sha+'.json.gz')).write_bytes(packed)
            (root/'index.json').write_text(json.dumps({'captures':[{'sha256':sha,'container_sha256':digest(packed),'capture_sha256':digest(payload)}]}))
            files=[]
            for source,path,data in [('crystalserver','event.lua',raw),('crystalserver','caller.lua',b'creature:registerEvent("Shared")'),('canary','other.lua',b'creature:registerEvent("Shared")')]:
                filename=source+'-'+path; (root/filename).write_bytes(data)
                files.append({'source':source,'revision':'fixed','path':path,'sha256':digest(data),'git_blob_sha1':hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest(),'byte_count':len(data),'cache_path':filename})
            (root/'manifest.json').write_text(json.dumps({'files':files}))
            provenance={k:v for k,v in files[0].items() if k!='cache_path'}
            assignment={'records':[{'source_component_id':'crystalserver:fixed:event.lua','provenance':provenance,'exact_existing_canonical_owners':[]}]}
            result=build(assignment,root/'manifest.json',root)
            self.assertEqual(len(result['caller_witnesses']),1)
            self.assertEqual(result['caller_witnesses'][0]['provenance']['source'],'crystalserver')

if __name__ == '__main__': unittest.main()
