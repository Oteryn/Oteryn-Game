"""Lossless pinned NPC/helper source capture. No networking or Lua execution."""
import argparse,collections,gzip,hashlib,json,pathlib,re
SCHEMA='OTERYN_QUEST_DONOR_DIALOGUE_CAPTURE/v1'
PINS={'canary':('opentibiabr/canary','04b83b512114bfd888000d6e1433ed8ecaec7c5b'),'crystalserver':('zimbadev/crystalserver','9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d')}
def sha(b):return hashlib.sha256(b).hexdigest()
def lexical(text):
 """Mask comments/literals preserving character offsets, capture every raw literal."""
 masked=list(text);strings=[];comments=[];i=0;n=len(text)
 def hide(start,end):
  for j in range(start,end):
   if text[j] not in '\r\n':masked[j]=' '
 while i<n:
  start=i;comment=text.startswith('--',i);p=i+2 if comment else i
  long=re.match(r'\[(=*)\[',text[p:])
  if long:
   close=']'+long.group(1)+']';end=text.find(close,p+len(long.group(0)));terminated=end>=0;end=end+len(close) if terminated else n
   (comments if comment else strings).append({'start_char':start,'end_char':end,'line':text.count('\n',0,start)+1,'raw':text[start:end],'kind':'long_comment' if comment else 'long_string','terminated':terminated});hide(start,end);i=end;continue
  if comment:
   end=text.find('\n',i);end=n if end<0 else end;comments.append({'start_char':start,'end_char':end,'line':text.count('\n',0,start)+1,'raw':text[start:end],'kind':'line_comment','terminated':True});hide(start,end);i=end;continue
  if text[i] in '\"\'':
   quote=text[i];i+=1;terminated=False
   while i<n:
    if text[i]=='\\':i+=2;continue
    if text[i]==quote:i+=1;terminated=True;break
    i+=1
   end=min(i,n);strings.append({'start_char':start,'end_char':end,'line':text.count('\n',0,start)+1,'raw':text[start:end],'kind':'quoted_string','terminated':terminated});hide(start,end);continue
  i+=1
 return ''.join(masked),strings,comments

def annotations(text):
 masked,strings,comments=lexical(text);lines=text.splitlines(keepends=True)
 def row(m,kind):
  line=text.count('\n',0,m.start())+1;return {'kind':kind,'start_char':m.start(),'end_char':m.end(),'line':line,'source_line':lines[line-1].rstrip('\r\n'),'match':text[m.start():m.end()]}
 functions=[row(m,'function_declaration_candidate') for m in re.finditer(r'\bfunction\s+([\w.:]+)\s*\(|\b([\w.]+)\s*=\s*function\s*\(',masked)]
 controls=[row(m,'control_flow_token') for m in re.finditer(r'\b(?:if|elseif|else|for|while|repeat|until|return|break|goto)\b',masked)]
 calls=[row(m,'call_candidate') for m in re.finditer(r'\b[\w]+(?:[.:][\w]+)*\s*\(',masked)]
 includes=[]
 for m in re.finditer(r'\b(?:dofile|require|loadfile|load)\s*\(',masked):
  depth=1;i=m.end()
  while i<len(masked) and depth:
   depth+=(masked[i]=='(')-(masked[i]==')');i+=1
  inc=row(m,'include_or_dynamic_load_candidate');inc['argument_raw']=text[m.end():i-1 if not depth else i];inc['call_end_char']=i;inc['terminated']=depth==0;inc['resolution']='UNRESOLVED_EXPRESSION_PRESERVED';includes.append(inc)
 return {'classification':'LEXICAL_EVIDENCE_ONLY_NOT_LUA_AST','offset_unit':'decoded_UTF8_character','functions':functions,'control_flow':controls,'calls':calls,'includes':includes,'string_literals':strings,'comment_spans':comments,'lexically_unterminated_regions':sum(not x['terminated'] for x in strings+comments)}
def selected(path):
 parts=path.parts;return path.suffix.lower()=='.lua' and parts and parts[0].startswith('data') and bool({'npc','lib','libs','npclib'} & set(parts))
def capture_file(source,path,root,relative_path=None):
 blob=path.read_bytes();rel=relative_path or path.relative_to(root).as_posix();repo,revision=PINS[source]
 try:text=blob.decode('utf-8');a=annotations(text);decode='UTF8_EXACT'
 except UnicodeDecodeError:a=None;decode='NON_UTF8_RAW_ONLY'
 git_blob=hashlib.sha1(b'blob '+str(len(blob)).encode()+b'\0'+blob).hexdigest()
 return {'source':source,'repository':repo,'revision':revision,'path':rel,'blob_sha256':sha(blob),'byte_count':len(blob),'file_id':source+':'+revision+':'+rel,'source_bytes_ref':{'schema':'OTERYN_DONOR_BLOB_REF/v1','git_blob_sha1':git_blob,'sha256':sha(blob)},'capture':'BYTE_EXACT_REFERENCE_ONLY','text_decode':decode,'annotations':a,'native_semantics':'NOT_TRANSCRIBED','closure_role':'NPC_OR_NPC_HELPER' if 'npc' in pathlib.PurePosixPath(rel).parts else 'LIBRARY_SUPPORT'}
def resolve_manifest_file(manifest_path,row):
 path=pathlib.Path(row.get('blob_path',row.get('cache_path','')))
 if 'blob_path' in row and (path.is_absolute() or '..' in path.parts):raise ValueError('Portable corpus blob path escapes manifest root')
 if not path.is_absolute():path=manifest_path.parent/path
 if 'blob_path' in row and not path.resolve().is_relative_to(manifest_path.parent.resolve()):raise ValueError('Portable corpus blob symlink escapes manifest root')
 if not path.is_file():raise ValueError('Missing source corpus blob: '+row['path'])
 blob=path.read_bytes()
 if sha(blob)!=row['sha256'] or len(blob)!=row['byte_count']:raise ValueError('Corpus byte/hash mismatch: '+row['path'])
 git_blob=hashlib.sha1(b'blob '+str(len(blob)).encode()+b'\0'+blob).hexdigest()
 if git_blob!=row['git_blob_sha1']:raise ValueError('Corpus Git blob mismatch: '+row['path'])
 return path
def resolve_source_bytes(record,manifest_path):
 manifest=json.loads(manifest_path.read_text())
 rows=[r for r in manifest['files'] if (r['source'],r['revision'],r['path'])==(record['source'],record['revision'],record['path'])]
 if len(rows)!=1:raise ValueError('Source identity absent/duplicate in corpus')
 path=resolve_manifest_file(manifest_path,rows[0]);blob=path.read_bytes()
 if sha(blob)!=record['blob_sha256']:raise ValueError('Record/corpus blob differs')
 return blob
def build(roots,repo_root=None,corpus_manifest=None):
 records=[]
 if corpus_manifest:
  manifest=json.loads(corpus_manifest.read_text())
  if manifest.get('failures'):raise ValueError('Corpus incomplete')
  for row in sorted(manifest['files'],key=lambda r:(r['source'],r['path'])):
   if row['source'] not in PINS:continue
   if row['revision']!=PINS[row['source']][1]:raise ValueError('Unexpected donor pin')
   if not selected(pathlib.PurePosixPath(row['path'])):continue
   path=resolve_manifest_file(corpus_manifest,row);records.append(capture_file(row['source'],path,corpus_manifest.parent,row['path']))
 else:
  for source,root in sorted(roots.items()):
   for path in sorted(root.rglob('*.lua')):
    if selected(path.relative_to(root)):records.append(capture_file(source,path,root))
 annotation_blobs={}
 for record in records:
  key=record['source_bytes_ref']['git_blob_sha1'];annotation=record.pop('annotations')
  if key in annotation_blobs and annotation_blobs[key]!=annotation:raise ValueError('Same Git blob yielded different annotations')
  annotation_blobs[key]=annotation;record['annotations_ref']=key
 lookup={(r['source'],r['revision'],r['path']):r for r in records}
 if len(lookup)!=len(records):raise ValueError('Duplicate donor file identity')
 questlinks=[];quest_inputs=[]
 if repo_root:
  index_path='content/quests/definitions/index.json';index_blob=(repo_root/index_path).read_bytes();quest_inputs.append({'path':index_path,'sha256':sha(index_blob)});idx=json.loads(index_blob)
  for shard in idx['shards']:
   shard_blob=(repo_root/shard).read_bytes();quest_inputs.append({'path':shard,'sha256':sha(shard_blob)})
   for rec in json.loads(shard_blob)['records']:
    d=rec['definition'];refs=[]
    for track in d.get('source_data',{}).get('progress',[]):
     for tr in track['transitions']:
      for occ in tr.get('source_occurrences',[]):
       if '/npc/' not in occ['path']:continue
       record=lookup.get((occ['source'],occ['revision'],occ['path']));refs.append({'progress_key':track['key'],'source':occ['source'],'revision':occ['revision'],'path':occ['path'],'line':occ['line'],'expected_blob_sha256':occ['blob_sha256'],'byte_exact_capture_present':bool(record and record['blob_sha256']==occ['blob_sha256']),'callback_semantics_complete':False})
    questlinks.append({'quest':d['identity'],'source_npc_occurrence_refs':refs,'canonical_NPC_family_binding':'NOT_ADMITTED_DIFFERENT_DONOR_REVISIONS'})
  for descriptor in quest_inputs:
   if sha((repo_root/descriptor['path']).read_bytes())!=descriptor['sha256']:raise ValueError('Quest input moved during capture: '+descriptor['path'])
 summary={'files':len(records),'bytes':sum(r['byte_count'] for r in records),'by_source':dict(collections.Counter(r['source'] for r in records)),'roles':dict(collections.Counter(r['closure_role'] for r in records)),'raw_byte_capture_complete_for_selected_files':True,'semantic_callback_completion_certified':False,'computed_include_closure_certified':False,'quest_definitions_audited':len(questlinks),'quest_npc_occurrence_refs':sum(len(q['source_npc_occurrence_refs']) for q in questlinks),'quest_npc_occurrence_refs_missing_capture':sum(not r['byte_exact_capture_present'] for q in questlinks for r in q['source_npc_occurrence_refs'])}
 return {'schema':SCHEMA,'scope':'Lossless donor reference capture; not native translation or complete dynamic include closure','donor_pins':{s:{'repository':p[0],'revision':p[1]} for s,p in PINS.items()},'summary':summary,'files':records,'annotation_blobs':annotation_blobs,'quest_links':questlinks,'quest_inputs':quest_inputs}
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--canary',type=pathlib.Path);ap.add_argument('--crystalserver',type=pathlib.Path);ap.add_argument('--corpus-manifest',type=pathlib.Path);ap.add_argument('--repo-root',type=pathlib.Path);ap.add_argument('--out',type=pathlib.Path,required=True);ap.add_argument('--summary-out',type=pathlib.Path);a=ap.parse_args()
 if not a.corpus_manifest and not (a.canary and a.crystalserver):ap.error('Supply corpus manifest or both donor roots')
 packet=build({'canary':a.canary,'crystalserver':a.crystalserver} if not a.corpus_manifest else {},a.repo_root,a.corpus_manifest);a.out.parent.mkdir(parents=True,exist_ok=True);payload=(json.dumps(packet,ensure_ascii=False,sort_keys=True,separators=(',',':'))+'\n').encode()
 encoded=gzip.compress(payload,mtime=0) if a.out.suffix=='.gz' else payload;a.out.write_bytes(encoded)
 if a.summary_out:
  summary={'schema':SCHEMA+'/index','capture_artifact':{'path':a.out.name,'sha256':sha(encoded),'encoding':'gzip' if a.out.suffix=='.gz' else 'json'},'summary':packet['summary'],'donor_pins':packet['donor_pins'],'corpus_manifest_sha256':sha(a.corpus_manifest.read_bytes()) if a.corpus_manifest else None}
  a.summary_out.write_text(json.dumps(summary,ensure_ascii=False,sort_keys=True,indent=2)+'\n')
 print(json.dumps(packet['summary'],indent=2))
if __name__=='__main__':main()
