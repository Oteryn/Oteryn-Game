"""Read back each specific canonical gap, pinned AST binding and raw guard span."""
import argparse,base64,gzip,hashlib,json,sys,tempfile
from pathlib import Path
import jsonschema
import builder

def get(value,pointer):
 for part in pointer.split('/')[1:]:value=value[int(part)] if isinstance(value,list)else value[part]
 return value

def qualify(packet,authoring,corpus,ast_root):
 data=json.loads(packet.read_text());schema=json.loads(Path(__file__).with_name('schema.json').read_text());jsonschema.validate(data,schema)
 if data['decoder_sha256']!=builder.sha(Path(builder.__file__).read_bytes()):raise ValueError('Wrong decoder pin')
 canonical=json.loads((authoring/'samples/interactions/interactions.json').read_text());source_lookup={(r['source'],r['revision'],r['path']):r for r in json.loads(corpus.read_text())['files']}
 sys.path.insert(0,str(authoring));import donor_semantic_conditions as base;import ots_interactions as oi
 progress=json.loads((authoring/'samples/questlog/progress.json').read_text())['progress'];declared=oi.declared_progress_paths(progress)
 temp=tempfile.TemporaryDirectory();source_cache=Path(temp.name)
 _,rows,payloads=base.load_ast_inputs(ast_root,{r['provenance']['sha256']for r in data['records']});captures={};verified=0
 for record in data['records']:
  p=record['provenance'];row=source_lookup[(p['source'],p['revision'],p['path'])]
  if any(row[k]!=p[k] for k in p):raise ValueError('Source provenance drift')
  gap=get(canonical,record['baseline_gap']['json_pointer'])
  if gap['unresolved']['line']!=record['baseline_gap']['line']:raise ValueError('Not the specific unresolved gap')
  if p['sha256']not in captures:captures[p['sha256']]=json.loads(gzip.decompress(payloads[p['sha256']]))
  capture=captures[p['sha256']];raw=base64.b64decode(capture['raw_bytes_base64']);ref=record['source_ref'];source_path=Path(row['cache_path']);source_path=source_path if source_path.is_absolute()else corpus.parent/source_path
  if source_path.read_bytes()!=raw:raise ValueError('Raw corpus differs from AST raw')
  if builder.sha(raw)!=p['sha256']or hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()!=p['git_blob_sha1']:raise ValueError('Source raw hashes mismatch')
  span=raw[ref['byte_start']:ref['byte_end_exclusive']]
  if builder.sha(span)!=ref['slice_sha256']or span.decode('utf-8')!=ref['expression']:raise ValueError('Source guard span mismatch')
  node=get(capture['ast'],ref['condition_ast_pointer']);material=source_cache/p['source']/p['path'];material.parent.mkdir(parents=True,exist_ok=True);material.write_bytes(raw)
  script=oi.Script(p['source'],source_cache/p['source'],p['path'],{},'source-truthiness');script.declared=declared
  line=record['baseline_gap']['line'];starts=[n for n,l in enumerate(script.code_lines,1) if oi.CALLBACK.match(l)and n<line];callback=max(starts)if starts else None;semantic=None
  if callback is not None:
   cb=oi.CALLBACK.match(script.lines[callback-1]);script.bind(callback,cb.group(2));semantic=base.Normalizer(script,oi,{r['key']for r in progress},raw.decode('utf-8'))
  bindings=builder.Bindings(capture['ast'],semantic=semantic,source_text=raw.decode('utf-8'))
  if bindings.normalize(node,ref['condition_ast_pointer'])!=record['normalized']:raise ValueError('Source normalized binding drift')
  if builder.count(record['normalized'],'opaque')!=record['opaque_leaves']:raise ValueError('Hidden opaque siblings')
  if (record['status']=='FULL_SOURCE_GUARD')!=(record['opaque_leaves']==0):raise ValueError('Full/partial status inconsistent')
  def check_values(v):
   if isinstance(v,dict):
    if v.get('kind')in {'bound_value_truthiness','bound_value_read'}:
     binding=v['binding'];declaration=get(capture['ast'],binding['declaration_pointer'])
     if builder.name(declaration)!=binding['name']:raise ValueError('Lexical binding name mismatch')
     if not ref['condition_ast_pointer'].startswith(binding['scope_pointer']+'/'):raise ValueError('Binding outside lexical scope')
    for x in v.values():check_values(x)
   elif isinstance(v,list):
    for x in v:check_values(x)
  check_values(record['normalized']);verified+=1
 temp.cleanup()
 return {'schema':'OTERYN_BOUND_SOURCE_GUARD_QUALIFICATION/v1','state':'PASS','records_verified':verified,'counts':data['counts'],'quest_count':data['quest_count'],'packet_sha256':builder.sha(packet.read_bytes()),'canonical_gap_replacements':0,'native_admission':False}

def main():
 p=argparse.ArgumentParser();p.add_argument('--packet',type=Path,required=True);p.add_argument('--authoring',type=Path,required=True);p.add_argument('--corpus-manifest',type=Path,required=True);p.add_argument('--ast-root',type=Path,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args();d=qualify(a.packet,a.authoring,a.corpus_manifest,a.ast_root);a.out.write_text(json.dumps(d,indent=2)+'\n');print(json.dumps(d))
if __name__=='__main__':main()
