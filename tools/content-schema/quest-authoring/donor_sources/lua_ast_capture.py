"""Lossless byte witness plus upstream structural Lua AST; never runtime admission."""
from __future__ import annotations
import argparse, base64, concurrent.futures, contextlib, enum, gzip, hashlib, io, json
from collections import Counter
from pathlib import Path
from antlr4 import CommonTokenStream, InputStream
from antlr4.error.ErrorListener import ErrorListener
from luaparser import ast
from luaparser.astnodes import Node
from luaparser.parser.LuaLexer import LuaLexer

PARSER = {'distribution': 'luaparser', 'version': '4.2.0', 'antlr_runtime': '4.13.2', 'license': 'MIT', 'upstream': 'https://github.com/boolangery/py-lua-parser'}

class Errors(ErrorListener):
    def __init__(self): self.errors=[]
    def syntaxError(self, recognizer, offendingSymbol, line, column, msg, e):
        self.errors.append({'line': line, 'column': column, 'message': msg})

def encode(value):
    if isinstance(value, Node):
        first, last = value.first_token, value.last_token
        span = None
        if first is not None and last is not None and first.start >= 0 and last.stop >= first.start:
            span={'start_char':first.start,'end_char_exclusive':last.stop+1,'line':first.line,'column':first.column,'end_token_line':last.line,'end_token_column':last.column}
        return {'node_type':type(value).__name__, 'span':span, 'fields':{k:encode(v) for k,v in vars(value).items() if k not in ('_name','_first_token','_last_token')}}
    if isinstance(value, bytes): return {'bytes_base64':base64.b64encode(value).decode('ascii')}
    if isinstance(value, enum.Enum): return {'enum_type':type(value).__name__,'name':value.name,'value':value.value}
    if isinstance(value, list): return [encode(x) for x in value]
    if isinstance(value, dict): return {k:encode(v) for k,v in value.items()}
    if value is None or isinstance(value,(str,int,float,bool)): return value
    raise TypeError(type(value).__name__)

def capture(raw: bytes):
    out={'schema':'OTERYN_LUA_STRUCTURAL_SOURCE/v1','parser':PARSER,'sha256':hashlib.sha256(raw).hexdigest(),'byte_count':len(raw),'raw_bytes_base64':base64.b64encode(raw).decode('ascii'),'native_semantic_admission':False,'ast':None,'tokens':[],'diagnostics':[]}
    try: text=raw.decode('utf-8-sig'); out['encoding']='utf-8-sig'
    except UnicodeDecodeError as exc:
        out.update(status='ENCODING_FAILED',encoding=None); out['diagnostics']=[{'message':str(exc)}]; return out
    listener=Errors(); lexer=LuaLexer(InputStream(text)); lexer.removeErrorListeners(); lexer.addErrorListener(listener)
    stream=CommonTokenStream(lexer); stream.fill()
    for token in stream.tokens:
        if token.type == -1: continue
        out['tokens'].append({'type':token.type,'symbol':lexer.symbolicNames[token.type] if token.type<len(lexer.symbolicNames) else None,'channel':token.channel,'text':token.text,'start_char':token.start,'end_char_exclusive':token.stop+1,'line':token.line,'column':token.column})
    out['diagnostics'].extend(listener.errors)
    if listener.errors:
        out['status']='LEX_FAILED'; return out
    try:
        with contextlib.redirect_stderr(io.StringIO()),contextlib.redirect_stdout(io.StringIO()): parsed=ast.parse(text)
        out['ast']=encode(parsed); out['status']='PARSED'
    except Exception as exc:
        out['status']='PARSE_FAILED'; out['diagnostics'].append({'exception':type(exc).__name__,'message':str(exc)})
    return out

def process(args):
    sha,path,out_dir=args
    raw=Path(path).read_bytes()
    if hashlib.sha256(raw).hexdigest()!=sha: raise ValueError('SOURCE_HASH_MISMATCH '+path)
    result=capture(raw)
    payload=json.dumps(result,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()
    target=Path(out_dir)/f'{sha}.json.gz'; container=gzip.compress(payload+b'\n',compresslevel=6,mtime=0); target.write_bytes(container)
    return {'sha256':sha,'capture_sha256':hashlib.sha256(payload+b'\n').hexdigest(),'status':result['status'],'token_count':len(result['tokens']),'diagnostics':result['diagnostics'],'capture_path':str(target),'container_sha256':hashlib.sha256(container).hexdigest(),'compression':'gzip-mtime0-level6'}

def is_lua_source(path):
    return path.lower().endswith(('.lua', '.lua.dist'))

def main():
    p=argparse.ArgumentParser(); p.add_argument('--manifest',type=Path,required=True); p.add_argument('--output',type=Path,required=True); p.add_argument('--workers',type=int,default=4); a=p.parse_args()
    manifest=json.loads(a.manifest.read_text()); rows=[r for r in manifest['files'] if is_lua_source(r['path'])]
    bysha={r['sha256']:r for r in rows}; a.output.mkdir(parents=True,exist_ok=True)
    captures={}
    with concurrent.futures.ProcessPoolExecutor(max_workers=a.workers) as pool:
        for i,result in enumerate(pool.map(process,[(sha,str(Path(r['cache_path']) if Path(r['cache_path']).is_absolute() else a.manifest.parent/r['cache_path']),str(a.output)) for sha,r in sorted(bysha.items())],chunksize=8),1):
            result['capture_path']=str(Path(result['capture_path']).relative_to(a.output.parent))
            captures[result['sha256']]=result
            if i%250==0: print(f'captured {i}/{len(bysha)}',flush=True)
    index={'schema':'OTERYN_LUA_STRUCTURAL_SOURCE_INDEX/v1','scope':'SOURCE_STRUCTURE_ONLY_NOT_QUEST_OR_RUNTIME_EQUIVALENCE','parser':PARSER,'corpus_manifest_sha256':hashlib.sha256(a.manifest.read_bytes()).hexdigest(),'summary':{'lua_occurrences':len(rows),'unique_lua_blobs':len(captures),'config_template_occurrences':sum(r['path'].lower().endswith('.lua.dist') for r in rows),'unique_statuses':dict(Counter(r['status'] for r in captures.values())),'occurrence_statuses':dict(Counter(captures[r['sha256']]['status'] for r in rows))},'captures':list(captures.values()),'sources':[{k:v for k,v in r.items() if k!='cache_path'}|{'capture_sha256':captures[r['sha256']]['capture_sha256'],'status':captures[r['sha256']]['status']} for r in rows]}
    (a.output.parent/'index.json').write_text(json.dumps(index,ensure_ascii=False,indent=2)+'\n'); print(json.dumps(index['summary']),flush=True)
if __name__=='__main__': main()
