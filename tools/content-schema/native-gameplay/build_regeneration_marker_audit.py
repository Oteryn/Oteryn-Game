#!/usr/bin/env python3
"""Verify the complete pinned Lua/C++ cache and record regeneration88888 consumers."""
import argparse
import ast
import hashlib
import json
from pathlib import Path
import re

ROOT=Path(__file__).resolve().parents[3]
PINS={'canary':['opentibiabr/canary','47dfd51f45280a59a1d3e50ba7edd573d7234446'],
      'crystal':['zimbadev/crystalserver','201189f871d23597b82caf88fd611454a2ddefb8']}
OUTPUT=ROOT/'tools/content-schema/native-gameplay/srx3-regeneration-marker-audit.json'
IGNORED={
 'data-global/monster/quests/rotten_blood_quest/bakragore.lua':'monster storage namespace, essence count',
 'data-global/scripts/quests/rotten_blood_quest/bakragore_mechanics_death.lua':'monster storage namespace, essence count',
 'data-global/scripts/quests/a_pirates_tail_quest/creaturescripts_rum_barrel.lua':'damage upper bound, not a condition ID',
}

def verify_blob(expected,body):
    actual=hashlib.sha1(b'blob '+str(len(body)).encode()+b'\0'+body).hexdigest()
    if actual!=expected: raise ValueError('Pinned Git blob mismatch')


def number(expression):
    def visit(node):
        if isinstance(node,ast.Constant) and type(node.value) in (int,float): return node.value
        if isinstance(node,ast.BinOp) and isinstance(node.op,ast.Mult): return visit(node.left)*visit(node.right)
        raise ValueError('Unsupported source numeric expression')
    return visit(ast.parse(expression.strip(),mode='eval').body)


def classify(body):
    text=body.decode('utf-8')
    found=[]
    for declaration in re.finditer(r'local\s+(\w+)\s*=\s*Condition\(CONDITION_REGENERATION,\s*CONDITIONID_DEFAULT\)',text):
        variable=declaration.group(1)
        def parameter(name):
            match=re.search(re.escape(variable)+r':setParameter\('+name+r',\s*([^\n)]+)\)',text)
            return None if match is None else (number(match.group(1)),text[:match.start()].count('\n')+1)
        subid=parameter('CONDITION_PARAM_SUBID')
        if subid is None or subid[0]!=88888:continue
        duration=parameter('CONDITION_PARAM_TICKS')
        gain=parameter('CONDITION_PARAM_HEALTHGAIN')
        if duration is None or gain is None:raise ValueError('Incomplete88888 source marker')
        found.append({'sub_id':88888,'duration_ms':int(duration[0]),'health_gain':int(gain[0]),
                      'declaration_line':text[:declaration.start()].count('\n')+1,'sub_id_line':subid[1]})
    if len(found)>1:raise ValueError('More than one88888 source declaration')
    return found[0] if found else None


def spell_reference_lines(body, names):
    """Literal monster table-field references, not a dynamic-call proof."""
    known={name.casefold() for name in names}
    references=[]
    for line,text in enumerate(body.decode('utf-8').splitlines(),1):
        if text.lstrip().startswith('--'):continue
        for match in re.finditer(r"(?:\{|,)\s*name\s*=\s*([\"'])(.*?)\1",text):
            name=match.group(2).casefold()
            if name in known:references.append({'name':name,'line':line,'text':text.strip()})
    return references


def build(lua_audit,cpp_audit,canary_tree,crystal_tree):
    trees={name:json.loads(Path(path).read_bytes()) for name,path in [('canary',canary_tree),('crystal',crystal_tree)]}
    for name,tree in trees.items():
        if tree.get('truncated') is not False or tree.get('sha')!=PINS[name][1]:
            raise ValueError('Wrong or truncated pinned source tree')
    consumers=[];ignored=[];scopes=[]
    for label,path,predicate in [
        ('Lua',lua_audit,lambda p:p.endswith('.lua')),
        ('C++',cpp_audit,lambda p:p.startswith('src/') and p.endswith(('.cpp','.hpp','.h'))),
    ]:
        path=Path(path);audit=json.loads(path.read_bytes())
        if audit['pins']!=PINS or audit['failures']:raise ValueError('Incomplete pinned audit')
        expected={(name,r['path'],r['sha']) for name,tree in trees.items() for r in tree['tree']
                  if r['type']=='blob' and predicate(r['path'])}
        actual=set();hashes=[]
        for record in audit['scanned']:
            sha=record['git_blob'];body=(path.parent/'blobs'/(sha+'.lua')).read_bytes()
            verify_blob(sha,body)
            digest=hashlib.sha256(body).hexdigest()
            if digest!=record['sha256']:raise ValueError('Wrong source SHA256')
            hashes.append((sha,digest))
            for alias in record['paths']:actual.add((alias['repo'],alias['path'],sha))
            if b'88888' not in body:continue
            marker=classify(body) if label=='Lua' else None
            row={'git_blob':sha,'sha256':digest,'paths':record['paths'],
                 'lines':[{'line':i,'text':line} for i,line in enumerate(body.decode().splitlines(),1) if '88888' in line]}
            if marker:
                if marker['health_gain']!=0:raise ValueError('Changed88888 marker gain')
                row['marker']=marker;consumers.append(row)
            else:
                reasons={IGNORED[a['path']] for a in record['paths'] if a['path'] in IGNORED}
                if not reasons or any(a['path'] not in IGNORED for a in record['paths']):raise ValueError('Unreviewed88888 source consumer')
                row['reason']='; '.join(sorted(reasons));ignored.append(row)
        if actual!=expected:raise ValueError('Source audit does not cover the full pinned tree')
        scopes.append({'kind':label,'paths':len(actual),'unique_blobs':len(hashes),
                       'verified_blob_closure_sha256':hashlib.sha256(json.dumps(sorted(hashes),separators=(',',':')).encode()).hexdigest()})
    if len(consumers)!=16 or len(ignored)!=3:raise ValueError('Changed source marker roster')
    names=set()
    for row in consumers:
        body=(Path(lua_audit).parent/'blobs'/(row['git_blob']+'.lua')).read_text()
        match=re.search(r"spell:name\(([\"'])(.*?)\1\)",body)
        if match:names.add(match.group(2).casefold())
    references={name:[] for name in sorted(names)}
    for repo,tree in trees.items():
        for entry in tree['tree']:
            path=entry['path']
            if entry['type']!='blob' or '/monster/' not in path or not path.endswith('.lua'):continue
            body=(Path(lua_audit).parent/'blobs'/(entry['sha']+'.lua')).read_bytes()
            verify_blob(entry['sha'],body)
            for reference in spell_reference_lines(body,names):
                name=reference.pop('name')
                references[name].append(dict(reference,repo=repo,path=path,git_blob=entry['sha'],sha256=hashlib.sha256(body).hexdigest()))
    return {'schema':'OTERYN_SOURCE_REGENERATION88888_AUDIT/v2','pins':PINS,'scopes':scopes,
            'literal_monster_vector_references':references,
            'consumers':sorted(consumers,key=lambda r:r['paths'][0]['path']),
            'unrelated_uses':sorted(ignored,key=lambda r:r['paths'][0]['path'])}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for option in ['lua-audit','cpp-audit','canary-tree','crystal-tree']:parser.add_argument('--'+option,type=Path,required=True)
    parser.add_argument('--check',action='store_true');args=parser.parse_args()
    data=build(args.lua_audit,args.cpp_audit,args.canary_tree,args.crystal_tree)
    raw=(json.dumps(data,ensure_ascii=False,sort_keys=True,indent=2)+'\n').encode()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_bytes()!=raw:raise ValueError('Stale88888 audit receipt')
    else:OUTPUT.write_bytes(raw)
    print('SOURCE_MARKER88888_AUDIT',len(raw),hashlib.sha256(raw).hexdigest())

if __name__=='__main__':main()
