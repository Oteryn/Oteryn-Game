"""Preserve upstream Lua AST data without executing, binding, or admitting source programs."""
import argparse
import base64
from bisect import bisect_left
import copy
from collections import Counter
from contextlib import redirect_stderr
from enum import Enum
import gzip
import hashlib
from importlib.metadata import version, distribution
import io
import json
import math
from pathlib import Path
import subprocess
from luaparser import ast, astnodes, builder

HERE = Path(__file__).resolve().parent
PINS = {'canary':'04b83b512114bfd888000d6e1433ed8ecaec7c5b','crystal':'00ce02a57ca5a12e48f32a3476e37471167e4c3f'}
DEPENDENCIES = {'luaparser':'4.2.0','antlr4-python3-runtime':'4.13.2','multimethod':'2.1'}
MAX_BYTES=2_000_000
MAX_NODES=100_000
MAX_DEPTH=500
MAX_PACKET_BYTES=100_000_000
# Explicit audited upstream semantic field contracts; N=node, O=optional node,
# L=ordered nodes, D=node or scalar default numeric for-loop step.
FIELDS = {
 'Chunk':{'body':'N'},'Block':{'body':'L'},'Attribute':{'name':'N'},
 'Name':{'id':'S','attribute':'O'},'Index':{'idx':'N','value':'N','notation':'index_notation'},
 'Assign':{'targets':'L','values':'L'},'LocalAssign':{'targets':'L','values':'L'},
 'While':{'test':'N','body':'N'},'Do':{'body':'N'},'Repeat':{'body':'N','test':'N'},
 'ElseIf':{'test':'N','body':'N','orelse':'O'},'If':{'test':'N','body':'N','orelse':'O'},
 'Label':{'id':'N'},'Goto':{'label':'N'},'SemiColon':{},'Break':{},'Return':{'values':'L'},
 'Fornum':{'target':'N','start':'N','stop':'N','step':'D','body':'N'},
 'Forin':{'body':'N','iter':'L','targets':'L'},
 'Call':{'func':'N','args':'L','style':'call_style'},
 'Invoke':{'source':'N','func':'N','args':'L','style':'call_style'},
 'Function':{'name':'N','args':'L','body':'N'},'LocalFunction':{'name':'N','args':'L','body':'N'},
 'Method':{'source':'N','name':'N','args':'L','body':'N'},
 'Nil':{},'TrueExpr':{'value':'true'},'FalseExpr':{'value':'false'},'Number':{'n':'numeric'},
 'Varargs':{},'String':{'s':'bytes','raw':'S','delimiter':'string_delimiter'},
 'Field':{'key':'O','value':'N','between_brackets':'B','index':'optional_index'},
 'Table':{'fields':'L'},'Dots':{},'AnonymousFunction':{'args':'L','body':'N'},
}
BINARY = ('AddOp SubOp MultOp FloatDivOp FloorDivOp ModOp ExpoOp BAndOp BOrOp BXorOp BShiftROp BShiftLOp LessThanOp GreaterThanOp LessOrEqThanOp GreaterOrEqThanOp EqToOp NotEqToOp AndLoOp OrLoOp Concat').split()
UNARY = ('UMinusOp UBNotOp ULNotOp ULengthOP').split()
for kind in BINARY: FIELDS[kind]={'left':'N','right':'N'}
for kind in UNARY: FIELDS[kind]={'operand':'N'}
for kind, fields in FIELDS.items():
    if issubclass(getattr(astnodes,kind),astnodes.Expression): fields['wrapped']='B'
DROPPED = {'_name','comments','_first_token','_last_token'}


def sha(data): return hashlib.sha256(data).hexdigest()


def versions():
    actual = {name:version(name) for name in DEPENDENCIES}
    if actual != DEPENDENCIES: raise ValueError('source parser dependency versions differ from exact lock')
    return actual


def dependency_provenance():
    """Bind exact installed distribution metadata bytes, without newline conversion."""
    versions()
    result={}
    for name in DEPENDENCIES:
        installed=distribution(name)
        files=installed.files
        if files is None: raise ValueError('distribution RECORD file listing unavailable: '+name)
        row={}
        for filename,field in (('METADATA','metadata_sha256'),('RECORD','record_sha256')):
            matches=[entry for entry in files if str(entry).endswith('.dist-info/'+filename)]
            if len(matches)!=1: raise ValueError('ambiguous distribution metadata file: '+name+'/'+filename)
            row[field]=sha(Path(installed.locate_file(matches[0])).read_bytes())
        result[name]=row
    return result


def serialize(tree, text):
    token_stream=ast.get_token_stream(text)
    token_stream.fill()
    literal_tokens={'Number':[],'String':[]}
    for token in token_stream.tokens:
        name=token_stream.tokenSource.symbolicNames[token.type] if token.type>0 else ''
        if name in ('INT','HEX','FLOAT','HEX_FLOAT'): literal_tokens['Number'].append(token)
        elif name in ('NORMALSTRING','CHARSTRING','LONGSTRING'): literal_tokens['String'].append(token)
    literal_starts={kind:[t.start for t in tokens] for kind,tokens in literal_tokens.items()}
    nodes=[]
    counts=Counter()
    maximum_depth=0
    def visit(node,depth):
        nonlocal maximum_depth
        if depth>MAX_DEPTH or len(nodes)>=MAX_NODES: raise ValueError('AST safeguard exceeded; no truncation allowed')
        maximum_depth=max(maximum_depth,depth)
        kind=type(node).__name__
        if kind not in FIELDS: raise ValueError('unmapped upstream node class: '+kind)
        expected=set(FIELDS[kind])-{'index'}
        allowed=expected | DROPPED | ({'_index'} if kind=='Field' else set())
        if set(vars(node))-allowed: raise ValueError('unmapped upstream attributes: '+repr(set(vars(node))-allowed))
        if expected-set(vars(node)): raise ValueError('missing upstream semantic attributes')
        node_id=len(nodes)
        row={'id':node_id,'kind':kind,'fields':{},'source_span':None,'source_span_coverage':'unavailable',
             'numeric_literal_spelling':None,'string_literal_spelling':None,
             'parser_decoded_string_qualified':False}
        nodes.append(row)
        counts[kind]+=1
        first,last=node.first_token,node.last_token
        if first is not None and last is not None and first.start>=0 and last.stop>=first.start:
            start,end=first.start,last.stop+1
            if end>len(text): raise ValueError('upstream source span outside input')
            row['source_span_coverage']='upstream_token_interval_not_full_node_qualified'
            row['source_span']={'char_start':start,'char_end_exclusive':end,'line':first.line,'column':first.column,
                                'sha256':sha(text[start:end].encode('utf-8'))}
        if kind in literal_tokens and first is not None and last is not None:
            offset=bisect_left(literal_starts[kind],first.start)
            candidates=literal_tokens[kind][offset:bisect_left(literal_starts[kind],last.stop+1)]
            if len(candidates)!=1: raise ValueError('upstream literal interval does not own exactly one lexer literal')
            row['numeric_literal_spelling' if kind=='Number' else 'string_literal_spelling']=candidates[0].text
        for name, field_type in FIELDS[kind].items():
            value=getattr(node,name)
            if field_type=='N': value={'node_ref':visit(value,depth+1)}
            elif field_type=='O': value=None if value is None else {'node_ref':visit(value,depth+1)}
            elif field_type=='L': value=[{'node_ref':visit(child,depth+1)} for child in value]
            elif field_type=='D':
                if isinstance(value,astnodes.Node): value={'node_ref':visit(value,depth+1)}
                elif type(value) is not int: raise ValueError('unknown default numeric for-loop step')
            elif field_type=='bytes': value={'encoding':'base64','data':base64.b64encode(value).decode('ascii')}
            elif isinstance(value,Enum): value=value.name
            elif isinstance(value,float) and not math.isfinite(value):
                value={'encoding':'ieee754_special','value':repr(value)}
            row['fields'][name]=value
        return node_id
    root=visit(tree,0)
    return {'root_node_ref':root,'nodes':nodes},dict(counts),maximum_depth


def parse_record(item,data):
    if len(data)>MAX_BYTES: raise ValueError('source size safeguard exceeded; no truncation allowed')
    if item['revision'] != PINS[item['source']] or sha(data)!=item['sha256'] or len(data)!=item['bytes'] or hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()!=item['git_blob']: raise ValueError('immutable source identity mismatch')
    record={'schema':'OTERYN_SOURCE_SYNTAX/v1','source':item['source'],'revision':item['revision'],'path':item['path'],
            'git_blob':item['git_blob'],'source_sha256':item['sha256'],'source_bytes':len(data),'syntax_valid':False,'parse_status':'syntax_error','syntax_error':None,
            'ast':None,'node_count':0,'maximum_depth':0,'node_kind_counts':{},
            'execution_qualified':False,'render_activation':False,'binding_qualified':False,'runtime_activation':False,'source_code_activation':False,'native_admission':False}
    try:
        with redirect_stderr(io.StringIO()): tree=ast.parse(data.decode('utf-8'))
    except (ast.SyntaxException,UnicodeDecodeError,ValueError,SyntaxError,RecursionError) as error:
        if not isinstance(error,ast.SyntaxException):
            record['syntax_valid']=None
            record['parse_status']='upstream_builder_error'
        # No original source snippets/token dumps in errors; preserve category and location.
        position=__import__('re').search(r'line (\d+):(\d+)',str(error))
        record['syntax_error']={'category':type(error).__name__,'line':int(position[1]) if position else None,
                                'column':int(position[2]) if position else None,'diagnostic_sha256':sha(str(error).encode())}
        return record
    record['ast'],record['node_kind_counts'],record['maximum_depth']=serialize(tree,data.decode('utf-8'))
    record['node_count']=len(record['ast']['nodes'])
    record['syntax_valid']=True
    record['parse_status']='parsed'
    return record


def generate(inventory_path,out,schema_path):
    versions()
    inventory_bytes=inventory_path.read_bytes()
    inventory=json.loads(gzip.decompress(inventory_bytes))
    from jsonschema import Draft202012Validator
    schema=json.loads(schema_path.read_text())
    # Validate the record envelope once, then dispatch each node to its exact
    # closed class schema, avoiding quadratic oneOf scans over 59 kinds.
    header_schema=copy.deepcopy(schema)
    header_schema['properties']['ast']['anyOf'][1]['properties']['nodes']['items']={'type':'object'}
    validator=Draft202012Validator(header_schema)
    node_validators={kind:Draft202012Validator(definition) for kind,definition in schema['$defs'].items()}
    out.mkdir(parents=True,exist_ok=True)
    artifact=out/'source-syntax.jsonl.gz'
    payload_hash=hashlib.sha256()
    totals=Counter(); kinds=Counter(); maximum_depth=0; largest_nodes=0; largest_source=0; largest_row=0
    with artifact.open('wb') as raw,gzip.GzipFile(filename='',fileobj=raw,mode='wb',mtime=0) as stream:
        for index,item in enumerate(sorted(inventory['files'],key=lambda r:(r['source'],r['path']))):
            data=subprocess.check_output(['git','-C','/workspace/spell-sources/'+item['source'],'show',item['revision']+':'+item['path']])
            row=parse_record(item,data)
            validator.validate(row)
            if row['ast'] is not None:
                for node in row['ast']['nodes']: node_validators[node['kind']].validate(node)
            encoded=(json.dumps(row,sort_keys=True,separators=(',',':'),ensure_ascii=False,allow_nan=False)+'\n').encode()
            if len(encoded)>MAX_PACKET_BYTES: raise ValueError('serialized row safeguard exceeded; no truncation allowed')
            stream.write(encoded);payload_hash.update(encoded)
            totals['records']+=1;totals[row['parse_status']]+=1
            totals['nodes']+=row['node_count'];kinds.update(row['node_kind_counts'])
            maximum_depth=max(maximum_depth,row['maximum_depth']);largest_nodes=max(largest_nodes,row['node_count'])
            largest_source=max(largest_source,len(data));largest_row=max(largest_row,len(encoded))
            if (index+1)%200==0: print(json.dumps({'processed':index+1,'nodes':totals['nodes']}),flush=True)
    proof={'schema':'OTERYN_SOURCE_SYNTAX_RECEIPT/v1','record_count':totals['records'],'counts':dict(totals),'node_kind_counts':dict(sorted(kinds.items())),
           'measured_maxima':{'depth':maximum_depth,'nodes_per_file':largest_nodes,'source_bytes':largest_source,'serialized_row_bytes':largest_row},
           'safeguards':{'maximum_depth':MAX_DEPTH,'maximum_nodes_per_file':MAX_NODES,'maximum_source_bytes':MAX_BYTES,'maximum_serialized_row_bytes':MAX_PACKET_BYTES,'truncation_allowed':False},
           'gzip_path':str(artifact),'gzip_sha256':sha(artifact.read_bytes()),'payload_sha256':payload_hash.hexdigest(),
           'schema_path':str(schema_path.relative_to(HERE.parents[2])),'schema_sha256':sha(schema_path.read_bytes()),
           'inventory_path':str(inventory_path),'inventory_sha256':sha(inventory_bytes),'source_revisions':sorted(PINS.values()),
           'dependencies':versions(),'dependency_provenance':dependency_provenance(),'exporter_sha256':sha(Path(__file__).read_bytes()),
           'upstream_astnodes_sha256':sha(Path(astnodes.__file__).read_bytes()),'upstream_parser_api_sha256':sha(Path(ast.__file__).read_bytes()),
           'upstream_builder_sha256':sha(Path(builder.__file__).read_bytes()),
           'runtime_activation':False,'source_code_activation':False,'native_admission':False,'execution_qualified':False,'render_activation':False,'external_sources_used':False}
    (out/'source-syntax-receipt.json').write_text(json.dumps(proof,indent=2)+'\n')
    return proof


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('--inventory',type=Path,required=True)
    parser.add_argument('--out',type=Path,required=True)
    parser.add_argument('--schema',type=Path,default=HERE/'source-syntax.schema.json')
    args=parser.parse_args()
    print(json.dumps(generate(args.inventory,args.out,args.schema)['counts'],sort_keys=True))
