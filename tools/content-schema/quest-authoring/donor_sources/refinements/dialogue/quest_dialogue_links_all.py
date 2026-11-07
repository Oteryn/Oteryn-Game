"""Bounded exact Quest-progress-to-Source-dialogue joins; never runtime admission."""
import argparse, base64, gzip, hashlib, importlib.util, json, tarfile
from pathlib import Path
QUESTS = ['oteryn:quest.a_father_s_burden_quest','oteryn:quest.the_djinn_war_efreet_faction','oteryn:quest.threatened_dreams_quest','oteryn:quest.tibia_tales']
def stable(x):return json.dumps(x,sort_keys=True,ensure_ascii=False,separators=(',',':')).encode()
def sha(b):return hashlib.sha256(b).hexdigest()
def library(repo_root):
    path=repo_root/'tools/content-schema/quest-authoring/donor_sources/components/dialogue/dialogue_supplement.py'
    spec=importlib.util.spec_from_file_location('pinned_dialogue_source_library',path);mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod);return mod,path

def qualified_symbol(node, aliases):
    if not isinstance(node,dict):return None
    t=node.get('node_type');f=node.get('fields',{})
    if t=='Name':return aliases.get(f['id'],f['id'])
    if t=='Index':
        base=qualified_symbol(f['value'],aliases)
        if not base:return None
        if f.get('notation',{}).get('name')=='DOT':
            member=f.get('idx',{}).get('fields',{}).get('id')
            return base+'.'+member if member else None
        idx=f.get('idx',{});v=idx.get('fields',{}).get('n')
        if idx.get('node_type')=='Number' and type(v) in (int,float) and v==int(v):return base+'['+str(int(v))+']'
    return None

def storage_root_mutated(ast,ds):
    def root(node):
        if not isinstance(node,dict):return None
        t=node.get('node_type');f=node.get('fields',{})
        if t=='Name':return f['id']
        if t=='Index':return root(f.get('value'))
        return None
    for _,n,_ in ds.walk(ast):
        t=n.get('node_type');f=n.get('fields',{})
        if t in ('Assign','LocalAssign') and any(root(v)=='Storage' for v in f['targets']):return True
        if t=='Function' and root(f.get('name'))=='Storage':return True
        if t=='Method' and root(f.get('source'))=='Storage':return True
    return False

def storage_aliases(ast,row,ds):
    top=ast.get('fields',{}).get('body',{}).get('fields',{}).get('body',[]);counts={};aliases={};proof=[]
    for _,n,_ in ds.walk(ast):
        if n.get('node_type') in ('Assign','LocalAssign'):
            for target in n['fields']['targets']:
                if target.get('node_type')=='Name':counts[target['fields']['id']]=counts.get(target['fields']['id'],0)+1
        if n.get('node_type') in ('Function','LocalFunction') and n['fields'].get('name',{}).get('node_type')=='Name':
            symbol=n['fields']['name']['fields']['id'];counts[symbol]=counts.get(symbol,0)+1
        if n.get('node_type') in ('Function','LocalFunction','AnonymousFunction','Method'):
            for arg in n['fields']['args']:
                if arg.get('node_type')=='Name':counts[arg['fields']['id']]=counts.get(arg['fields']['id'],0)+1
        if n.get('node_type')=='Fornum':
            v=n['fields'].get('target')
            if isinstance(v,dict) and v.get('node_type')=='Name':counts[v['fields']['id']]=counts.get(v['fields']['id'],0)+1
        if n.get('node_type')=='Forin':
            for v in n['fields'].get('targets',[]):
                if v.get('node_type')=='Name':counts[v['fields']['id']]=counts.get(v['fields']['id'],0)+1
    if counts.get("Storage",0) or storage_root_mutated(ast,ds):return {},[]
    for i,n in enumerate(top):
        f=n.get('fields',{})
        if n.get('node_type')=='LocalAssign' and len(f['targets'])==len(f['values'])==1:
            symbol=ds.name(f['targets'][0]);value=qualified_symbol(f['values'][0],{})
            if symbol and counts.get(symbol)==1 and value and value.startswith('Storage.'):
                aliases[symbol]=value;proof.append({'symbol':symbol,'exact_storage_expression':value,'witness':ds.witness(row,'$.fields.body.fields.body['+str(i)+']',n),'scope':'SOLE_TOP_LEVEL_LOCAL_BINDING_NO_LOCAL_REASSIGNMENT_OR_PARAMETER_SHADOW'})
    return aliases,proof

def function_storage_aliases(ast,row,ds):
    """One direct function-body local, before use; all competing writes/shadows reject."""
    proofs=[]
    def binding_counts(tree):
        counts={}
        def add(n):
            if isinstance(n,dict) and n.get('node_type')=='Name':
                key=n['fields']['id'];counts[key]=counts.get(key,0)+1
        for _,n,_ in ds.walk(tree):
            t=n.get('node_type');f=n.get('fields',{})
            if t in ('Assign','LocalAssign'):
                for v in f['targets']:add(v)
            if t in ('Function','LocalFunction') and f.get('name'):add(f['name'])
            if t in ('Function','LocalFunction','AnonymousFunction','Method'):
                for v in f['args']:add(v)
            if t=='Fornum':add(f.get('target'))
            if t=='Forin':
                for v in f.get('targets',[]):add(v)
        return counts
    # Reject any lexical redefinition of the Storage root in the file. This is
    # deliberately conservative and never treats a parameter/local table as global Storage.
    if binding_counts(ast).get('Storage',0) or storage_root_mutated(ast,ds):return []
    for path,fn,_ in ds.walk(ast):
        if fn.get('node_type') not in ('Function','LocalFunction','AnonymousFunction','Method'):continue
        f=fn['fields'];counts=binding_counts(fn)
        for i,decl in enumerate(f['body']['fields']['body']):
            df=decl.get('fields',{})
            if decl.get('node_type')!='LocalAssign' or len(df['targets'])!=1 or len(df['values'])!=1:continue
            symbol=ds.name(df['targets'][0]);value=qualified_symbol(df['values'][0],{})
            if symbol and counts.get(symbol)==1 and value and value.startswith('Storage.') and decl.get('span'):
                proofs.append({'symbol':symbol,'exact_storage_expression':value,'witness':ds.witness(row,path+'.fields.body.fields.body['+str(i)+']',decl),'function_witness':ds.witness(row,path,fn),'scope':'SOLE_DIRECT_FUNCTION_BODY_LOCAL_NO_COMPETING_BINDING_OR_SHADOW'})
    return proofs

def event_storage_aliases(event,file_aliases,function_proofs):
    aliases=dict(file_aliases);used=[];sp=event['witness']['span'];path=event['witness']['ast_path']
    if not sp:return aliases,used
    for proof in function_proofs:
        ds=proof['witness']['span'];prefix=proof['function_witness']['ast_path']+'.fields.body'
        if path.startswith(prefix) and ds and ds['end_char_exclusive']<=sp['start_char']:
            aliases[proof['symbol']]=proof['exact_storage_expression'];used.append(proof)
    return aliases,used

def literal_number(node):
    if not isinstance(node,dict):return None
    if node.get('node_type')=='Number':return node['fields']['n']
    if node.get('node_type')=='UMinusOp':
        value=literal_number(node['fields'].get('operand'));return -value if value is not None else None
    return None

def find_event(record,occ,ds):
    line=occ['line'];target=occ['target']
    candidates=[]
    for e in record['events']:
        sp=e['witness']['span']
        if e['type']=='StorageWrite' and sp and sp['line']<=line<=sp['end_token_line'] and e['arguments'] and qualified_symbol(e['arguments'][e.get('storage_target_argument_index',0)],e.get('lexical_storage_alias_map',record['storage_alias_map']))==target:
            candidates.append(e)
    return candidates

def enclosing_function(record,event):
    path=event['witness']['ast_path']
    candidates=[f for f in record['function_scopes'] if path.startswith(f['witness']['ast_path']+'.fields.body')]
    return max(candidates,key=lambda f:len(f['witness']['ast_path'])) if candidates else None

def build(corpus_manifest,ast_root,repo_root,all_source_quests=False):
    ds,libpath=library(repo_root);manifest=json.loads(corpus_manifest.read_bytes()); inp=[];occurrences=[]
    idxp='content/quests/definitions/index.json';raw=(repo_root/idxp).read_bytes();idx=json.loads(raw);
    for shard in idx['shards']:
        blob=(repo_root/shard).read_bytes();
        for entry in json.loads(blob)['records']:
            q=entry['definition']
            if not all_source_quests and q['identity']['key'] not in QUESTS:continue
            if all_source_quests and not any('/npc/' in o['path'] for p in q.get('source_data',{}).get('progress',[]) for t in p['transitions'] for o in t.get('source_occurrences',[])):continue
            inp.append({'quest':q['identity'],'source_data_sha256':sha(stable(q['source_data']))})
            for p in q['source_data']['progress']:
                for t in p['transitions']:
                    for oi,o in enumerate(t.get('source_occurrences',[])):
                        if '/npc/' in o['path']:
                            occurrences.append({'quest':q['identity'],'progress_key':p['key'],'transition_key':t['key'],'source_occurrence_index':oi,'occurrence':o})
    wanted={(o['occurrence']['source'],o['occurrence']['revision'],o['occurrence']['path']) for o in occurrences}
    rows={}
    for r in manifest['files']:
        k=(r['source'],r['revision'],r['path'])
        if k in wanted:
            if k in rows:raise ValueError('Duplicate corpus Source identity')
            rows[k]=r
    if set(rows)!=wanted:raise ValueError('Missing exact pinned NPC Source paths')
    ip=ast_root/'index.json';index=json.loads(ip.read_bytes() if ip.exists() else gzip.decompress((ast_root/'index.json.gz').read_bytes()))
    containers=None;needed={r['sha256'] for r in rows.values()}
    if not (ast_root/'captures').exists():
        containers={}
        for shard in sorted(ast_root.glob('*.tar.gz')):
            with tarfile.open(shard,'r|gz') as tf:
                for m in tf:
                    if m.name.startswith('captures/') and m.name.endswith('.json.gz'):
                        key=Path(m.name).name[:-8]
                        if key in needed:
                            if not m.isfile() or key in containers:raise ValueError('Invalid/duplicate AST capture')
                            containers[key]=tf.extractfile(m).read()
        if set(containers)!=needed:raise ValueError('Missing selected AST capture')
    records=[];lookup={};rawlookup={}
    for k,r in sorted(rows.items()):
        if r['revision']!=ds.PINS.get(r['source']):raise ValueError('Unexpected donor pin')
        bp=Path(r.get('cache_path',r.get('blob_path','')));bp=bp if bp.is_absolute() else corpus_manifest.parent/bp;raw=bp.read_bytes()
        if sha(raw)!=r['sha256'] or len(raw)!=r['byte_count'] or hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()!=r['git_blob_sha1']:raise ValueError('Raw NPC Source witness mismatch')
        ast=ds.load_ast(ast_root,r['sha256'],index,containers)
        if base64.b64decode(ast['raw_bytes_base64'])!=raw:raise ValueError('AST/raw NPC mismatch')
        matches=[s for s in index['sources'] if (s['source'],s['revision'],s['path'])==k]
        if len(matches)!=1 or any(matches[0][key]!=r[key] for key in ['sha256','byte_count','git_blob_sha1']):raise ValueError('AST Source identity mismatch')
        proj=ds.project(r,ast['ast']);scopes=[];aliases,alias_proofs=storage_aliases(ast['ast'],r,ds);function_alias_proofs=function_storage_aliases(ast['ast'],r,ds)
        for e in proj['events']:
            if e['type']=='StorageWrite' or (e['type']=='Call' and e.get('callee')=='setPlayerStorageValue'):
                e['lexical_storage_alias_map'],e['lexical_storage_alias_witnesses']=event_storage_aliases(e,aliases,function_alias_proofs)
        astpaths={id(n):path for path,n,_ in ds.walk(ast['ast'])}
        def compact_expr(n):return {'ast_type':n.get('node_type'),'expression_ref':ds.witness(r,astpaths[id(n)],n),'resolved_runtime_value':False}
        if all_source_quests:
            for e in proj['events']:
                if e['type']=='Call' and e.get('callee')=='setPlayerStorageValue' and len(e['arguments'])==3:
                    e['type']='StorageWrite';e['storage_target_argument_index']=1;e['source_call_kind']='LEGACY_GLOBAL_HELPER_CALL_SOURCE_ONLY';e['helper_execution']='NOT_PROVEN'
                if e['type']=='StorageWrite' and e['arguments']:
                    raw_target=qualified_symbol(e['arguments'][e.get('storage_target_argument_index',0)],{})
                    e['lexical_storage_alias_witnesses']=[p for p in e.get('lexical_storage_alias_witnesses',[]) if raw_target and (raw_target==p['symbol'] or raw_target.startswith(p['symbol']+'.') or raw_target.startswith(p['symbol']+'['))]
                e['guards']=[{'branch':g['branch'],'test':compact_expr(g['test']['structure'])} for g in e['guards']]
                if e['type']!='StorageWrite':
                    for field in ['arguments','targets','values']:
                        if field in e:e[field+'_refs']=[compact_expr(v) for v in e.pop(field)]
                    if 'structure' in e:e.pop('structure');e['structure_ref']=e['witness']
            for b in proj['branches']:
                b['test']=compact_expr(b['test']['structure']);b['outer_guards']=[{'branch':g['branch'],'test':compact_expr(g['test']['structure'])} for g in b['outer_guards']]
        for path,n,_ in ds.walk(ast['ast']):
            if n.get('node_type') in ('Function','LocalFunction','AnonymousFunction','Method'):
                f=n['fields'];scopes.append({'witness':ds.witness(r,path,n),'name':ds.name(f.get('name')),'kind':n['node_type'],'parameters':[ds.name(a) for a in f['args']]})
        rec={'source':r['source'],'revision':r['revision'],'path':r['path'],'source_sha256':r['sha256'],'git_blob_sha1':r['git_blob_sha1'],'function_scopes':scopes,'storage_alias_map':aliases,'storage_alias_witnesses':alias_proofs,'function_storage_alias_witnesses':function_alias_proofs,'events':proj['events'],'branches':proj['branches']}
        records.append(rec);lookup[k]=rec;rawlookup[k]=raw
    links=[];uniquevents=set();callbackrefs=set()
    for o in occurrences:
        occ=o['occurrence'];k=(occ['source'],occ['revision'],occ['path']);raw=rawlookup[k]
        if sha(raw)!=occ['blob_sha256'] or occ['line']<1 or occ['line']>len(raw.splitlines()) or sha(raw.splitlines()[occ['line']-1])!=occ['line_sha256']:raise ValueError('Canonical progress occurrence Source witness mismatch')
        rec=lookup[k];events=find_event(rec,occ,ds)
        if len(events)==1 and events[0].get('lexical_storage_alias_witnesses'):
            ev=events[0];value_index=ev.get('storage_target_argument_index',0)+1
            value=literal_number(ev['arguments'][value_index]) if len(ev['arguments'])>value_index else None
            if value is None or type(occ['write']['to']) not in (int,float) or value!=occ['write']['to']:
                events=[]
        link=o|{'join_status':'EXACT_AST_STORAGE_WRITE' if len(events)==1 else 'AMBIGUOUS_OR_NONLITERAL_AST_WRITE','write_event_ref':None,'callback_scope_ref':None,'guard_refs':[],'dialogue_event_refs':[],'effect_event_refs':[],'runtime_readiness':'NOT_ASSESSED','quest_complete':False}
        if len(events)==1:
            e=events[0];fun=enclosing_function(rec,e);link['write_event_ref']=e['witness'];link['callback_scope_ref']=fun
            link['guard_refs']=e['guards'];link['literal_to_value_checked']=literal_number(e['arguments'][e.get('storage_target_argument_index',0)+1]) if e.get('lexical_storage_alias_witnesses') else None;link['literal_source_target']=qualified_symbol(e['arguments'][e.get('storage_target_argument_index',0)],{});link['resolved_source_target']=qualified_symbol(e['arguments'][e.get('storage_target_argument_index',0)],e.get('lexical_storage_alias_map',rec['storage_alias_map']));link['storage_alias_witnesses']=rec['storage_alias_witnesses']+e.get('lexical_storage_alias_witnesses',[]) if link['literal_source_target']!=link['resolved_source_target'] else [];uniquevents.add((k,e['witness']['ast_path']))
            if fun:
                prefix=fun['witness']['ast_path']+'.fields.body';callbackrefs.add((k,fun['witness']['ast_path']))
                relevant=[v for v in rec['events'] if v['witness']['ast_path'].startswith(prefix)]
                link['dialogue_event_refs']=[v['witness'] for v in relevant if v['type'] in ('Utterance','MessageTemplate')]
                link['effect_event_refs']=[v['witness'] for v in relevant if v['type'] in ('StorageWrite','ItemRemoval','ItemGrant','TopicWrite','ReleaseFocus','ControlTransfer')]
            link['link_semantics']='EXACT_PROGRESS_WRITE_AND_ENCLOSING_SOURCE_FUNCTION_CONTEXT_NOT_EXECUTION_PATH'
        else:
            link['link_semantics']='SOURCE_FILE_PRESENT_TARGET_WRITE_NOT_UNIQUELY_JOINED'
            candidates=[e for e in rec['events'] if e['type']=='StorageWrite' and e['witness']['span'] and e['witness']['span']['line']<=occ['line']<=e['witness']['span']['end_token_line']]
            link['source_line_storage_write_candidates']=[{'witness':e['witness'],'literal_source_target':qualified_symbol(e['arguments'][e.get('storage_target_argument_index',0)],{}),'static_storage_target':qualified_symbol(e['arguments'][e.get('storage_target_argument_index',0)],e.get('lexical_storage_alias_map',rec['storage_alias_map']))} for e in candidates if e['arguments']]
            link['hold_reason']='NONUNIQUE_EXACT_TARGET_WRITE' if events else 'SOURCE_LOCAL_STORAGE_TARGET_NOT_PROVEN_EQUAL_TO_CANONICAL_TARGET'

        links.append(link)
    supplement_path=repo_root/'tools/content-schema/quest-authoring/donor_sources/components/dialogue/dialogue.json.gz';supraw=supplement_path.read_bytes();supp=json.loads(gzip.decompress(supraw))
    helpers=[l for l in supp['helper_links'] if l['definitions'] and (l['definitions'][0]['witness']['source'],l['definitions'][0]['witness']['revision'],l['definitions'][0]['witness']['path']) in wanted]
    contexts=[];context_ids={}
    legacy_write_witnesses={stable(e['witness']) for r in records for e in r['events'] if e.get('source_call_kind')=='LEGACY_GLOBAL_HELPER_CALL_SOURCE_ONLY'}
    selected_quests=sorted({l['quest']['key'] for l in links}) if all_source_quests else QUESTS
    if all_source_quests:
        for rec in records:
            key=(rec['source'],rec['revision'],rec['path'])
            prefixes={l['callback_scope_ref']['witness']['ast_path']+'.fields.body' for l in links if l['callback_scope_ref'] and (l['occurrence']['source'],l['occurrence']['revision'],l['occurrence']['path'])==key}
            writes={l['write_event_ref']['ast_path'] for l in links if l['write_event_ref'] and (l['occurrence']['source'],l['occurrence']['revision'],l['occurrence']['path'])==key}
            writes.update(c['witness']['ast_path'] for l in links if (l['occurrence']['source'],l['occurrence']['revision'],l['occurrence']['path'])==key for c in l.get('source_line_storage_write_candidates',[]))
            rec['events']=[e for e in rec['events'] if e['witness']['ast_path'] in writes or any(e['witness']['ast_path'].startswith(prefix) for prefix in prefixes)]
            for e in rec['events']:
                if e['type']=='StorageWrite':
                    e['literal_source_target']=qualified_symbol(e['arguments'][e.get('storage_target_argument_index',0)],{});e['resolved_source_target']=qualified_symbol(e['arguments'][e.get('storage_target_argument_index',0)],e.get('lexical_storage_alias_map',rec['storage_alias_map']));e['argument_ast_types']=[v.get('node_type') for v in e.pop('arguments')]
                    e['arguments_ref']={'call_node':e['witness'],'argument_field':'.fields.args'}
            rec['branches']=[b for b in rec['branches'] if any(b['witness']['ast_path'].startswith(prefix) for prefix in prefixes)]
            rec['function_scopes']=[f for f in rec['function_scopes'] if f['witness']['ast_path']+'.fields.body' in prefixes]
        for ri,rec in enumerate(records):
            for fun in rec['function_scopes']:
                prefix=fun['witness']['ast_path']+'.fields.body';key=(rec['source'],rec['revision'],rec['path'],fun['witness']['ast_path'])
                context_ids[key]=len(contexts)
                contexts.append({'source_record_index':ri,'function_scope':fun,'dialogue_event_indices':[i for i,e in enumerate(rec['events']) if e['witness']['ast_path'].startswith(prefix) and e['type'] in ('Utterance','MessageTemplate')],'effect_event_indices':[i for i,e in enumerate(rec['events']) if e['witness']['ast_path'].startswith(prefix) and e['type'] in ('StorageWrite','ItemRemoval','ItemGrant','TopicWrite','ReleaseFocus','ControlTransfer')]})
        for link in links:
            occ=link['occurrence'];fun=link.pop('callback_scope_ref')
            link['callback_context_ref']=context_ids.get((occ['source'],occ['revision'],occ['path'],fun['witness']['ast_path'])) if fun else None
            link.pop('dialogue_event_refs');link.pop('effect_event_refs')
    return {'schema':'OTERYN_QUEST_DIALOGUE_SOURCE_LINKS/v1','scope':'ALL_SOURCE_QUEST_EXACT_PROGRESS_COMPACT_CALLBACK_CONTEXT_SOURCE_ONLY' if all_source_quests else 'FOUR_QUEST_EXACT_PROGRESS_CALLBACK_CONTEXT_SOURCE_ONLY','quest_inputs':inp,'inputs':{'corpus_manifest_sha256':sha(corpus_manifest.read_bytes()),'ast_index_sha256':sha(stable(index)),'dialogue_library_sha256':sha(libpath.read_bytes()),'helper_semantic_links_sha256':sha(stable(helpers))},'native_admission':False,'quests_complete':False,'source_records':records,'quest_dialogue_links':links,'helper_context_links':helpers,'callback_contexts':contexts,'summary':{'quest_keys':selected_quests,'source_files':len(records),'npc_progress_occurrences':len(links),'exact_ast_write_joins':sum(l['join_status']=='EXACT_AST_STORAGE_WRITE' for l in links),'unresolved_ast_write_joins':sum(l['join_status']!='EXACT_AST_STORAGE_WRITE' for l in links),'unique_ast_writes':len(uniquevents),'distinct_enclosing_function_contexts':len(callbackrefs),'helper_context_links':len(helpers),'function_scoped_alias_joins':sum(bool(l.get('storage_alias_witnesses')) and any(p['scope']=='SOLE_DIRECT_FUNCTION_BODY_LOCAL_NO_COMPETING_BINDING_OR_SHADOW' for p in l['storage_alias_witnesses']) for l in links),'legacy_helper_write_joins':sum(l['write_event_ref'] is not None and stable(l['write_event_ref']) in legacy_write_witnesses for l in links),'exact_storage_alias_joins':sum(bool(l.get('storage_alias_witnesses')) for l in links),'per_quest':[{'quest_key':key,'npc_progress_occurrences':sum(l['quest']['key']==key for l in links),'exact_ast_write_joins':sum(l['quest']['key']==key and l['join_status']=='EXACT_AST_STORAGE_WRITE' for l in links)} for key in selected_quests]},'holds':['Context events are sibling witnesses, not all reachable on the linked write path','Conditional expressions, loops, returns, dynamic message-table lookups and item operation results retain Source AST; no runtime resolution inferred','Canonical attribution is retained as exact occurrence linkage, not sole quest ownership','Helper global loading and Oteryn NPC identity remain unproven']}

def qualify(packet,corpus_manifest,ast_root,repo_root,all_source_quests=False):
    expected=build(corpus_manifest,ast_root,repo_root,all_source_quests)
    if packet!=expected:raise ValueError("Dialogue links differ from Source-backed regeneration")
    return expected["summary"]

def main():
    p=argparse.ArgumentParser()
    for k in ('corpus-manifest','ast-root','repo-root','out'):p.add_argument('--'+k,type=Path,required=True)
    p.add_argument('--all-source-quests',action='store_true');a=p.parse_args();out=build(a.corpus_manifest,a.ast_root,a.repo_root,a.all_source_quests);raw=stable(out)+b'\n';a.out.parent.mkdir(parents=True,exist_ok=True);a.out.write_bytes(gzip.compress(raw,mtime=0) if a.out.suffix=='.gz' else raw);print(json.dumps(out['summary']))
if __name__=='__main__':main()
