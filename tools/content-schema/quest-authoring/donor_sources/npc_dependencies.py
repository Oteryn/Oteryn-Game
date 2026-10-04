"""Resolve declared donor dependency paths under pinned profiles; no Lua execution."""
import argparse,collections,gzip,hashlib,json,pathlib,re,sys
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent.parent))
try:
 from donor_sources.npc_capture import PINS,resolve_manifest_file,lexical
except ModuleNotFoundError:
 from capture import PINS,resolve_manifest_file,lexical
SCHEMA='OTERYN_DONOR_DIALOGUE_DEPENDENCIES/v1'
def sha(b):return hashlib.sha256(b).hexdigest()
def literal_token(raw,start):
 if start>=len(raw) or raw[start] not in '"\'':return None
 quote=raw[start];end=start+1
 while end<len(raw):
  char=raw[end]
  if char in '\\\r\n':return None
  if char==quote:return raw[start+1:end],end+1
  end+=1
 return None
def literal(s):
 s=s.strip();token=literal_token(s,0)
 return token[0] if token and token[1]==len(s) else None
def static_expression(raw,values):
 # Consume exactly literal/declared-symbol terms, separated by concatenation.
 # Delimiters inside quoted terms are data; executable suffixes never qualify.
 position=0;parts=[]
 while True:
  while position<len(raw) and raw[position].isspace():position+=1
  if position>=len(raw):return None
  token=literal_token(raw,position)
  if token:
   value,position=token;parts.append(value)
  else:
   symbol=re.match(r'[A-Za-z_][A-Za-z0-9_]*',raw[position:])
   if not symbol or symbol.group() not in ('DATA_DIRECTORY','CORE_DIRECTORY') or symbol.group() not in values:return None
   parts.append(values[symbol.group()]);position+=len(symbol.group())
  while position<len(raw) and raw[position].isspace():position+=1
  if position==len(raw):return ''.join(parts)
  if raw[position:position+2]!='..':return None
  position+=2
def build(manifest_path,capture_path):
 manifest=json.loads(manifest_path.read_text());raw=capture_path.read_bytes();capture=json.loads(gzip.decompress(raw) if capture_path.suffix=='.gz' else raw);rows={(r['source'],r['revision'],r['path']):r for r in manifest['files']};captured={(r['source'],r['revision'],r['path']) for r in capture['files']};proofs=[];proof_by={}
 def body(source,path):
  row=rows[(source,PINS[source][1],path)];return resolve_manifest_file(manifest_path,row).read_bytes().decode('utf8')
 def evidence(source,path,pattern):
  text=body(source,path);row=rows[(source,PINS[source][1],path)];matches=[(i,line) for i,line in enumerate(text.splitlines(),1) if re.search(pattern,line)]
  if not matches:raise ValueError('Missing expected donor profile proof '+path+' '+pattern)
  ident=source+':'+PINS[source][1]+':'+path+':'+pattern
  if ident not in proof_by:
   proof_by[ident]=len(proofs);proofs.append({'source':source,'repository':PINS[source][0],'revision':PINS[source][1],'path':path,'blob_sha256':row['sha256'],'git_blob_sha1':row['git_blob_sha1'],'lines':[{'line':i,'line_sha256':sha(line.encode()),'text':line} for i,line in matches]})
  return proof_by[ident]
 profiles={}
 for source in PINS:
  config=body(source,'config.lua.dist');core=literal(re.search(r'^coreDirectory\s*=([^\r\n]+)',config,re.M).group(1));data=literal(re.search(r'^dataPackDirectory\s*=([^\r\n]+)',config,re.M).group(1));defaults=[evidence(source,'config.lua.dist',r'^(?:coreDirectory|dataPackDirectory)\s*='),evidence(source,'src/config/configmanager.cpp',r'loadStringConfig\(L, (?:CORE_DIRECTORY|DATA_DIRECTORY),'),evidence(source,'data/core.lua',r'^(?:DATA_DIRECTORY|CORE_DIRECTORY)\s*=')]
  if not core or not data:raise ValueError('Config distribution profile not literal')
  alt='data-canary' if source=='canary' else 'data-crystal';altproof=evidence(source,'config.lua.dist',re.escape(alt))
  profiles[source]=[{'name':'DISTRIBUTION_DEFAULT','DATA_DIRECTORY':data,'CORE_DIRECTORY':core,'proof_refs':defaults,'deployment_profile_verified':False},{'name':'DECLARED_ALTERNATIVE','DATA_DIRECTORY':alt,'CORE_DIRECTORY':core,'proof_refs':defaults+[altproof],'deployment_profile_verified':False}]
 # Require paths are not inferred from unique filename. Retain pinned preload declarations.
 preload={};preload_refs=[]
 init='data/modules/scripts/gamestore/init.lua';text=body('canary',init)
 route_proved=all(re.search(pattern,text,re.M) for pattern in [r'^local gamestoreLibPath = CORE_DIRECTORY \.\. "/libs/gamestore"$',r'^\s*return dofile\(gamestoreLibPath \.\. "/" \.\. name \.\. "\.lua"\)$',r'^\s*package\.preload\[name\] = loader$'])
 if route_proved:
  for m in re.finditer(r'\["(gamestore\.[^"]+)"\]\s*=\s*function\(\)\s*return moduleLoader\("([^"\\]+)"\)',text):preload[m.group(1)]=m.group(2)
 if preload:preload_refs=[evidence('canary',init,r'^local gamestoreLibPath|return dofile\(|package\.preload\[name\]')]
 package_evidence={s:evidence(s,'src/lua/functions/lua_functions_loader.cpp',r'luaL_openlibs\(L\)') for s in PINS}
 output=[]
 for file in capture['files']:
  candidates=(capture['annotation_blobs'][file['annotations_ref']] or {}).get('includes',[])
  if not candidates:continue
  source=file['source'];text=body(source,file['path'])
  if sha(text.encode('utf8'))!=file['blob_sha256']:raise ValueError('Capture/corpus caller body changed')
  masked,_,_=lexical(text)
  shadowed={name for name in ['dofile','loadfile','load','require','DATA_DIRECTORY','CORE_DIRECTORY'] if re.search(r'\blocal\s+(?:function\s+)?'+name+r'\b|\bfunction\s+'+name+r'\b|(?:^|[;\n])\s*'+name+r'\s*=',masked)}
  for index,inc in enumerate(candidates):
   function=inc['match'].split('(')[0].strip();prev=masked[:inc['start_char']].rstrip();method=bool(prev and prev[-1] in '.:');entry={'source':source,'repository':file['repository'],'revision':file['revision'],'caller_path':file['path'],'caller_blob_sha256':file['blob_sha256'],'include_index':index,'line':inc['line'],'offset_char':inc['start_char'],'function_candidate':function,'argument_raw':inc['argument_raw'],'profiles':[],'runtime_execution_verified':False}
   for profile in profiles[source]:
    ownerfolder=file['path'].split('/')[0];activity='SHARED_CORE_OR_SELECTED_DATAPACK' if ownerfolder in [profile['CORE_DIRECTORY'],profile['DATA_DIRECTORY']] else 'INACTIVE_OTHER_DATAPACK';res={'source_profile':profile['name'],'profile_values':{k:profile[k] for k in ['DATA_DIRECTORY','CORE_DIRECTORY']},'profile_proof_refs':profile['proof_refs'],'caller_profile_activity':activity,'expected_paths':[],'exact_corpus_matches':[],'helper_present_in_dialogue_capture':False,'holds':[]}
    if function in shadowed or any(symbol in shadowed and re.search(r'\b'+symbol+r'\b',inc['argument_raw']) for symbol in ['DATA_DIRECTORY','CORE_DIRECTORY']):res.update(status='UNRESOLVED_SHADOWED_LOADER_OR_PROFILE_SYMBOL',why_unresolved='Caller contains assignment/declaration shadowing loader or directory symbol; no scope/temporal inference made.')
    elif method:res.update(status='NOT_BARE_BUILTIN_LOADER',why_unresolved='Lexical candidate is a member call; original code preserved. No builtin loader semantics admitted.')
    elif function=='load':res.update(status='RUNTIME_CODE_COMPILATION_NOT_FILE_DEPENDENCY',why_unresolved='Lua load compiles runtime source expression; no filesystem dependency admitted.')
    elif function in ['dofile','loadfile']:
     path=static_expression(inc['argument_raw'],profile)
     if path is None:res.update(status='UNRESOLVED_RUNTIME_EXPRESSION',why_unresolved='Argument uses runtime parameter/local/computation outside finite literal+DATA_DIRECTORY/CORE_DIRECTORY grammar.')
     elif pathlib.PurePosixPath(path).is_absolute() or '..' in pathlib.PurePosixPath(path).parts:res.update(status='UNRESOLVED_EXTERNAL_OR_ESCAPING_PATH',why_unresolved='Path cannot be interpreted as an in-repository donor path.',expected_paths=[path])
     else:
      path=pathlib.PurePosixPath(path).as_posix();res['expected_paths']=[path];target=rows.get((source,file['revision'],path));res['holds']=['SOURCE_PATH_UNDER_REPOSITORY_ROOT_CWD_PROFILE; actual process CWD/execution not certified']
      if target:
       res.update(status='STATIC_DECLARED_PATH_CAPTURED',why_unresolved=None,exact_corpus_matches=[{k:target[k] for k in ['source','repository','revision','path','git_blob_sha1','sha256','byte_count']}],helper_present_in_dialogue_capture=(source,file['revision'],path) in captured)
      else:res.update(status='STATIC_DECLARED_PATH_MISSING_FROM_CORPUS',why_unresolved='Expected exact path is absent from recovered corpus; no basename fallback or config.lua.dist substitution.')
    elif function=='require':
     name=literal(inc['argument_raw'])
     if source=='canary' and name in preload:
      path=profile['CORE_DIRECTORY']+'/libs/gamestore/'+preload[name]+'.lua';target=rows.get((source,file['revision'],path));res.update(expected_paths=[path],loader_proof_refs=preload_refs,status='STATIC_PRELOAD_DECLARATION_CAPTURED' if target else 'STATIC_PRELOAD_DECLARATION_MISSING',why_unresolved=None if target else 'Pinned preload expected path absent',holds=['Donor package.preload declaration applies only if no prior preload override','package.loaded may return prior module; runtime outcome not certified'])
      if target:res.update(exact_corpus_matches=[{k:target[k] for k in ['source','repository','revision','path','git_blob_sha1','sha256','byte_count']}],helper_present_in_dialogue_capture=(source,file['revision'],path) in captured)
     else:
      expected=name.replace('.','/')+'.lua' if name else None;res.update(status='UNRESOLVED_PACKAGE_SEARCH_PATH' if name else 'UNRESOLVED_RUNTIME_MODULE_NAME',why_unresolved='Pinned code opens standard Lua libraries; runtime/default LuaJIT package.path/searchers/environment and cache precedence are not fully pinned.',loader_proof_refs=[package_evidence[source]],holds=['Unique corpus filename does not prove require loader route'])
      if expected:
       res['expected_paths']=[expected];target=rows.get((source,file['revision'],expected))
       if target:res['candidate_only_corpus_matches']=[{k:target[k] for k in ['source','repository','revision','path','git_blob_sha1','sha256','byte_count']}]
    else:res.update(status='UNRESOLVED_UNKNOWN_LOADER',why_unresolved='Not an admitted builtin loader')
    entry['profiles'].append(res)
   output.append(entry)
 active_default=collections.Counter(p['status'] for e in output for p in e['profiles'] if p['source_profile']=='DISTRIBUTION_DEFAULT' and p['caller_profile_activity']=='SHARED_CORE_OR_SELECTED_DATAPACK')
 summary={'active_distribution_default_status_counts':dict(active_default),'lexical_include_candidates':len(output),'profiles_per_candidate':2,'status_counts_per_profile':dict(collections.Counter(p['status'] for e in output for p in e['profiles'])),'missing_static_active_default':[{'source':e['source'],'caller_path':e['caller_path'],'line':e['line'],'expected_paths':p['expected_paths']} for e in output for p in e['profiles'] if p['source_profile']=='DISTRIBUTION_DEFAULT' and p['caller_profile_activity']=='SHARED_CORE_OR_SELECTED_DATAPACK' and p['status']=='STATIC_DECLARED_PATH_MISSING_FROM_CORPUS'],'runtime_execution_verified':False,'complete_dynamic_dependency_closure':False}
 return {'schema':SCHEMA,'scope':'Declared Source dependency paths under pinned distribution/alternative profiles; never native or actual runtime loader completion','input_provenance':{'corpus_manifest_sha256':sha(manifest_path.read_bytes()),'npc_capture_sha256':sha(raw)},'source_profiles':profiles,'proofs':proofs,'includes':output,'summary':summary}
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--corpus-manifest',type=pathlib.Path,required=True);ap.add_argument('--capture',type=pathlib.Path,required=True);ap.add_argument('--out',type=pathlib.Path,required=True);ap.add_argument('--check',action='store_true');a=ap.parse_args();packet=build(a.corpus_manifest,a.capture);rendered=json.dumps(packet,ensure_ascii=False,sort_keys=True,indent=2)+'\n'
 if a.check:
  if not a.out.is_file() or a.out.read_text()!=rendered:raise SystemExit('Dependency packet differs from deterministic source replay')
 else:a.out.write_text(rendered)
 print(json.dumps(packet['summary'],indent=2))
if __name__=='__main__':main()
