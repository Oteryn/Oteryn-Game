"""Bounded source-only clock/position condition normalization; no compiler mutation."""
from __future__ import annotations
import argparse,base64,gzip,hashlib,json,re,sys,tarfile
from collections import Counter
from pathlib import Path
OPS={'EqToOp':'==','NotEqToOp':'~=','LessThanOp':'<','LessOrEqThanOp':'<=','GreaterThanOp':'>','GreaterOrEqThanOp':'>='}
REFLECTIVE=r'\b(?:_G|_ENV|rawset|getfenv|setfenv|load|loadstring|loadfile|dofile|require|debug|getmetatable|setmetatable)\b'

def kind(n):return n.get('node_type') if isinstance(n,dict) else None

def fields(n):return n.get('fields',{})

def path(n):
    f=fields(n)
    if kind(n)=='Name':return f['id']
    if kind(n)=='Index' and f['notation']['name']=='DOT' and kind(f['idx'])=='Name':
        parent=path(f['value']);return parent+'.'+fields(f['idx'])['id'] if parent else None
    if kind(n)=='Index' and f['notation']['name']=='SQUARE' and kind(f['idx'])=='Number' and type(fields(f['idx'])['n']) is int:
        parent=path(f['value']);return parent+'['+str(fields(f['idx'])['n'])+']' if parent else None
    return None

def integer(n):
    if kind(n)=='Number' and type(fields(n)['n']) is int:return fields(n)['n']
    if kind(n)=='UMinusOp' and kind(fields(n)['operand'])=='Number' and type(fields(fields(n)['operand'])['n']) is int:return -fields(fields(n)['operand'])['n']
    return None

def nodes(v):
    if isinstance(v,dict):
        if 'node_type' in v:yield v
        for value in v.values():yield from nodes(value)
    elif isinstance(v,list):
        for value in v:yield from nodes(value)

def expression_text(n,text):
    span=n.get('span')
    return text[span['start_char']:span['end_char_exclusive']] if span else path(n)

class Normalizer:
    def __init__(self,script,baseline,known_progress,source_text=None):
        self.script=script;self.baseline=baseline;self.known_progress=set(known_progress)
        self.text=source_text if source_text is not None else '\n'.join(script.lines);self.dynamic=bool(re.search(REFLECTIVE,baseline.mask_code(self.text)))
        self.pristine={root:baseline.builtin_binding_is_pristine(script.lines,root) for root in ('os','Game','Storage')}
        self.position_pristine=baseline.pristine_constructor(script.lines,'Position')
    def receiver(self,name,line,player=False):
        s=self.script
        if not name or self.dynamic or line not in s.line_scopes or any(part[0]=='opaque' for part in s.line_scopes[line]):return False
        if player and not s.proven_player(name,line):return False
        if not player and name not in s.roles and not s.proven_player(name,line):return False
        body=[s.lines[n-1] for n in sorted(s.line_scopes) if n!=s.player_aliases.get(name,(None,))[0]]
        body=[re.sub(rf'^(\s*(?:if|elseif)\s+(?:not\s+)?){re.escape(name)}(\s+then\s*)$', rf'\1{name}.__identity_truthiness_read__\2', line) for line in body]
        return self.baseline.builtin_binding_is_pristine(body,name)
    def operand(self,n,line):
        f=fields(n);s=self.script;s.current_line=line
        if kind(n)=='Call' and path(f['func'])=='os.time' and not f['args'] and self.pristine['os'] and not self.dynamic:
            return {'kind':'donor_wall_clock','api':'os.time','sampling':'AT_EXPRESSION_EVALUATION','runtime_owner':'UNRESOLVED'}
        if kind(n)=='Invoke' and path(f['func'])=='getStorageValue' and len(f['args'])==1:
            actor=path(f['source'])
            if self.receiver(actor,line,player=True):
                arg=path(f['args'][0]);raw_value=integer(f['args'][0])
                arg=str(raw_value) if raw_value is not None else arg
                if arg and (not arg.startswith('Storage.') or self.pristine['Storage']):
                    target=s.resolve_storage(self.baseline.expand_aliases(arg,s.aliases))
                    if target is not None:
                        track=s.track(target)
                        if track in self.known_progress:return {'kind':'quest_progress_read','progress':track,'receiver_role':'actor','api':'getStorageValue'}
        if kind(n)=='Call' and path(f['func'])=='Game.getStorageValue' and len(f['args'])==1 and self.pristine['Game'] and not self.dynamic:
            arg=path(f['args'][0]);raw_value=integer(f['args'][0]);arg=str(raw_value) if raw_value is not None else arg
            if arg and (not arg.startswith('Storage.') or self.pristine['Storage']):
                target=s.resolve_storage(self.baseline.expand_aliases(arg,s.aliases))
                if target is not None:return {'kind':'world_state_read','key':f'{s.namespace}:world-state/{self.baseline.slug(str(target))}','api':'Game.getStorageValue'}
        if kind(n)=='Invoke' and path(f['func'])=='getPosition' and not f['args']:
            receiver=path(f['source'])
            if self.receiver(receiver,line):return {'kind':'role_position_read','role':s.roles.get(receiver,'actor'),'api':'getPosition'}
        if kind(n)=='Call' and path(f['func'])=='Position' and self.position_pristine and not self.dynamic:
            vals=[integer(v) for v in f['args']]
            if len(vals)==3 and all(type(v)is int for v in vals) and 0<=vals[0]<=65535 and 0<=vals[1]<=65535 and 0<=vals[2]<=255:
                return {'kind':'literal_position','x':vals[0],'y':vals[1],'z':vals[2],'api':'Position'}
        return None
    def normalize(self,n,line):
        f=fields(n);k=kind(n)
        if k in ('AndLoOp','OrLoOp'):
            children=[self.normalize(f[name],line) for name in ('left','right')]
            return {'kind':'boolean','operator':'and' if k=='AndLoOp' else 'or','children':children,'source_order_preserved':True}
        if k=='ULNotOp':return {'kind':'not','child':self.normalize(f['operand'],line)}
        if k in OPS:
            left,right=self.operand(f['left'],line),self.operand(f['right'],line)
            pair={x['kind'] for x in (left,right) if x}
            if left and right and (pair=={'donor_wall_clock','quest_progress_read'} or pair=={'donor_wall_clock','world_state_read'} or (pair=={'role_position_read','literal_position'} and OPS[k] in ('==','~='))):
                return {'kind':'comparison','operator':OPS[k],'left':left,'right':right,'runtime_admission':'NOT_IMPLEMENTED'}
        # Preserve existing admitted D36 vocabulary for siblings; unknown leaves
        # remain explicit and are never silently dropped from boolean expressions.
        text=expression_text(n,self.text)
        if text and not self.dynamic:
            existing=self.script.condition(text,line)
            if not any('unresolved' in item for item in dict_nodes(existing)):return {'kind':'existing_condition','condition':existing}
        return {'kind':'opaque','source_line':line,'source_expression':text or '<UPSTREAM_SPAN_UNAVAILABLE>'}

def dict_nodes(v):
    if isinstance(v,dict):
        yield v
        for child in v.values():yield from dict_nodes(child)
    elif isinstance(v,list):
        for child in v:yield from dict_nodes(child)

def load_ast_inputs(root,wanted):
    index_bytes=(root/'index.json').read_bytes() if (root/'index.json').exists() else gzip.decompress((root/'index.json.gz').read_bytes())
    index=json.loads(index_bytes);rows={r['sha256']:r for r in index['captures']};data={}
    if (root/'captures').exists():
        for sha in wanted:data[sha]=(root/rows[sha]['capture_path']).read_bytes()
    else:
        manifest=json.loads((root/'manifest.json').read_text());assert hashlib.sha256(index_bytes).hexdigest()==manifest['index']['uncompressed_sha256']
        for archive in manifest['archives']:
            targets=wanted.intersection(archive['capture_sha256s'])
            if not targets:continue
            target=root/archive['path'];assert hashlib.sha256(target.read_bytes()).hexdigest()==archive['sha256']
            names={rows[sha]['capture_path']:sha for sha in targets}
            with tarfile.open(target,'r:gz') as tar:
                for member in tar:
                    if member.name in names:
                        assert member.isfile();sha=names[member.name];assert sha not in data;data[sha]=tar.extractfile(member).read()
        assert set(data)==wanted
    for sha,payload in data.items():assert hashlib.sha256(payload).hexdigest()==rows[sha]['container_sha256']
    return index_bytes,rows,data

def profile(v):
    result=Counter()
    if isinstance(v,dict):
        if v.get('kind')=='opaque':result['opaque_leaves']+=1
        if v.get('kind')=='comparison':
            result['new_comparisons']+=1
            result['clock_comparisons' if 'donor_wall_clock' in {v['left']['kind'],v['right']['kind']} else 'position_comparisons']+=1
        if v.get('kind')=='existing_condition':result['existing_leaves_retained']+=1
        for child in v.values():result.update(profile(child))
    elif isinstance(v,list):
        for child in v:result.update(profile(child))
    return result

def unresolved_refs(value,pointer=''):
    if isinstance(value,dict):
        if 'unresolved'in value and isinstance(value['unresolved'],dict):yield pointer,value['unresolved']['line']
        for k,v in value.items():
            if k!='unresolved':yield from unresolved_refs(v,pointer+'/'+k)
    elif isinstance(value,list):
        for i,v in enumerate(value):yield from unresolved_refs(v,pointer+'/'+str(i))

def main():
    p=argparse.ArgumentParser();p.add_argument('--authoring',type=Path,required=True);p.add_argument('--corpus',type=Path,required=True);p.add_argument('--ast-root',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--source-root',type=Path);a=p.parse_args();sys.path.insert(0,str(a.authoring));import ots_interactions as oi
    corpus=json.loads(a.corpus.read_text());sources={(r['source'],r['path']):r for r in corpus['files']};
    source_file=a.authoring/'samples/interactions/interactions.json';graphs=json.loads(source_file.read_text())['interactions'];manifest=json.loads((a.authoring/'samples/interactions/manifest.json').read_text());entries={r['destination']:r for r in manifest['entries']};progress=json.loads((a.authoring/'samples/questlog/progress.json').read_text())['progress'];declared=oi.declared_progress_paths(progress)
    wanted=set()
    for graph in graphs:
        if not list(unresolved_refs(graph['rules'])):continue
        key=graph['identity']['key'];entry=entries[key];donor=oi.CONFLICT_DECISIONS['interactions'].get(key,{}).get('decision',key.split(':',1)[0]);src=next((r for r in entry['sources'] if r['source']==donor),entry['sources'][0]);wanted.add(sources[(src['source'],src['path'])]['sha256'])
    index_bytes,captures,capture_payloads=load_ast_inputs(a.ast_root,wanted)
    rows=[];unjoined=[];cache={};counts=Counter()
    for graph_index,graph in enumerate(graphs):
        key=graph['identity']['key'];entry=entries[key];donor=oi.CONFLICT_DECISIONS['interactions'].get(key,{}).get('decision',key.split(':',1)[0]);src=next((r for r in entry['sources'] if r['source']==donor),entry['sources'][0]);source=sources[(src['source'],src['path'])];sha=source['sha256'];semantic_key=(source['source'],source['revision'],source['path'],sha)
        candidates=list(unresolved_refs(graph['rules'],'/'+str(graph_index)+'/rules'));counts['baseline_unresolved_conditions']+=len(candidates)
        if not candidates:continue
        if semantic_key not in cache:
            capture=json.loads(gzip.decompress(capture_payloads[sha]));raw=base64.b64decode(capture['raw_bytes_base64']);assert hashlib.sha256(raw).hexdigest()==sha
            roots=a.source_root/source['source'] if a.source_root else Path(source['cache_path']).parents[len(Path(source['path']).parts)-1]
            assert hashlib.sha256((roots/source['path']).read_bytes()).hexdigest()==sha
            script=oi.Script(source['source'],roots,source['path'],{},'supplement');script.declared=declared
            tests={}
            for node in nodes(capture['ast']):
                if kind(node) in ('If','ElseIf') and node.get('span'):tests.setdefault(node['span']['line'],[]).append(fields(node)['test'])
            cache[semantic_key]=(capture,script,tests,raw)
        capture,script,tests,raw=cache[semantic_key]
        for pointer,line in candidates:
            starts=[n for n,l in enumerate(script.code_lines,1) if oi.CALLBACK.match(l) and n<line];callback=max(starts) if starts else None
            if callback is None or len(tests.get(line,[]))!=1:
                unjoined.append({'interaction':key,'source':source['source'],'path':source['path'],'line':line,'reason':'AMBIGUOUS_OR_UNJOINED_AST_CONDITION'});continue
            cb=oi.CALLBACK.match(script.lines[callback-1]);script.bind(callback,cb.group(2));normalizer=Normalizer(script,oi,{r['key'] for r in progress},raw.decode('utf-8-sig'));normalized=normalizer.normalize(tests[line][0],line);stats=profile(normalized)
            if not stats['new_comparisons']:continue
            span=tests[line][0].get('span');expression=expression_text(tests[line][0],raw.decode('utf-8-sig'))
            rows.append({'interaction':key,'baseline_gap':{'file':'samples/interactions/interactions.json','file_sha256':hashlib.sha256(source_file.read_bytes()).hexdigest(),'json_pointer':'/interactions'+pointer,'line':line},'source':{k:source[k] for k in ('source','repository','revision','path','git_blob_sha1','sha256')},'condition_span':span,'condition_expression':expression,'condition_expression_sha256':hashlib.sha256((expression or '').encode()).hexdigest(),'normalized':normalized,'counts':dict(stats),'normalization_status':'FULL_SOURCE_CONDITION' if not stats['opaque_leaves'] else 'PARTIAL_SOURCE_CONDITION','canonical_opaque_replaced':False,'native_semantic_admission':False})
            counts.update(stats);counts['conditions_supplemented']+=1;counts['fully_typed_source_conditions' if not stats['opaque_leaves'] else 'partially_typed_source_conditions']+=1
    result={'schema':'OTERYN_DONOR_SEMANTIC_CONDITION_SUPPLEMENT/v1','scope':'SOURCE_ONLY_CLOCK_AND_LITERAL_POSITION_COMPARISONS','native_semantic_admission':False,'canonical_opaque_replacements':0,'corpus_manifest_sha256':hashlib.sha256(a.corpus.read_bytes()).hexdigest(),'ast_index_sha256':hashlib.sha256(index_bytes).hexdigest(),'decoder_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'baseline_compiler_sha256':hashlib.sha256((a.authoring/'ots_interactions.py').read_bytes()).hexdigest(),'counts':dict(counts),'conditions':rows,'unjoined_conditions':unjoined}
    a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result['counts']))
if __name__=='__main__':main()
