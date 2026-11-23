"""Lexical Rust test-body triage, never runtime or semantic coverage."""
import hashlib,json,re,csv,subprocess
from pathlib import Path
import argparse
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--repo',type=Path,required=True)
parser.add_argument('--output',type=Path,required=True)
args=parser.parse_args()
ROOT=args.repo.resolve()
OUT=args.output.resolve()
OUT.mkdir(parents=True,exist_ok=True)
SCOPES=['crates/gameplay/durability','apps/game-server/tests','apps/game-server/src/lifted_tests']
HEAD=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
records=subprocess.check_output(['git','ls-tree','-r','HEAD'],cwd=ROOT,text=True).splitlines()
blobs={r.split('\t',1)[1]:r.split('\t',1)[0].split()[2] for r in records}
def mask(s):
    a=list(s);i=0;n=len(s)
    def blank(start,end):
        for k in range(start,end):
            if a[k]!='\n':a[k]=' '
    while i<n:
        start=i
        if s.startswith('//',i):
            i=s.find('\n',i)
            if i<0:i=n
            blank(start,i);continue
        if s.startswith('/*',i):
            i+=2;depth=1
            while i<n and depth:
                if s.startswith('/*',i):depth+=1;i+=2
                elif s.startswith('*/',i):depth-=1;i+=2
                else:i+=1
            blank(start,i);continue
        m=re.match(r'(?:br|cr|r)(#*)"',s[i:]) if i==0 or not(s[i-1].isalnum() or s[i-1]=='_') else None
        if m:
            end=s.find('"'+m[1],i+len(m[0]));i=n if end<0 else end+1+len(m[1]);blank(start,i);continue
        if s[i]=='"':
            i+=1
            while i<n:
                if s[i]=='\\':i+=2
                elif s[i]=='"':i+=1;break
                else:i+=1
            blank(start,min(i,n));continue
        if s[i]=="'":
            m=re.match(r"'(?:\\u\{[0-9a-fA-F_]+\}|\\x[0-9a-fA-F]{2}|\\.|[^'\\\n])'",s[i:])
            if m:i+=len(m[0]);blank(start,i);continue
        i+=1
    return ''.join(a)
attr=re.compile(r'#\[\s*(?:\w+::)*test(?:\s*\([^]]*\))?\s*\]')
rows=[];failed=[];modified=[];files=[]
for p in blobs:
    if not p.endswith('.rs') or not any(p.startswith(g+'/') for g in SCOPES):continue
    raw=(ROOT/p).read_bytes();s=raw.decode();code=mask(s);files.append(p)
    if hashlib.sha1(f'blob {len(raw)}\0'.encode()+raw).hexdigest()!=blobs[p]:modified.append(p)
    for m in attr.finditer(code):
        fn=re.search(r'\b(?:async\s+)?fn\s+(\$?\w+)\s*\(',code[m.end():])
        if not fn:failed.append((p,s.count('\n',0,m.start())+1,'missing fn'));continue
        if fn[1].startswith('$'):failed.append((p,s.count('\n',0,m.start())+1,'macro-generated function template'));continue
        offset=m.end()+fn.start();enddecl=code.find('{',offset)
        if enddecl<0:failed.append((p,s.count('\n',0,m.start())+1,'missing body'));continue
        end=enddecl+1;depth=1
        while end<len(code) and depth:
            if code[end]=='{':depth+=1
            elif code[end]=='}':depth-=1
            end+=1
        if depth:failed.append((p,s.count('\n',0,m.start())+1,'unclosed body'));continue
        body=code[enddecl+1:end-1];original=s[enddecl+1:end-1]
        assertions=len(re.findall(r'\b(?:debug_)?assert(?:_eq|_ne)?!\s*\(',body))
        normalized=re.sub(r'\s+','',original)
        calls=re.findall(r'\b([A-Za-z_]\w*)\s*(?:!\s*)?\(',body)
        rows.append(dict(path=p,test=fn[1],line=s.count('\n',0,offset)+1,end_line=s.count('\n',0,end)+1,
                         body_lines=original.count('\n')+1,assertion_macros=assertions,
                         result_return='Result' in code[offset:enddecl],
                         env_gate=bool(re.search(r'env::var|env::var_os|std::env',body)),
                         roundtrip_name=bool(re.search(r'round.?trip',fn[1])),
                         direct_calls=','.join(sorted(set(calls))),
                         body_sha256=hashlib.sha256(normalized.encode()).hexdigest(),
                         semantic_status='UNREVIEWED_LEXICAL_CANDIDATE'))
with (OUT/'durability_test_case_inventory.tsv').open('w',newline='') as f:
    w=csv.DictWriter(f,fieldnames=list(rows[0]),delimiter='\t', quoting=csv.QUOTE_ALL, lineterminator='\n');w.writeheader();w.writerows(rows)
groups={scope:dict(source_files=sum(p.startswith(scope+'/') for p in files),test_cases=sum(r['path'].startswith(scope+'/') for r in rows)) for scope in SCOPES}
dup={}
for r in rows:dup.setdefault(r['body_sha256'],[]).append({k:r[k] for k in ('path','test','line')})
summary=dict(head=HEAD,method=__doc__,groups=groups,failed_parse_candidates=failed,modified_sources=modified,
             no_direct_assert=[r for r in rows if r['assertion_macros']==0],
             exact_body_candidates=[v for v in dup.values() if len(v)>1])
(OUT/'durability_test_case_inventory_summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:summary[k] for k in ('head','groups','failed_parse_candidates','modified_sources')},indent=2))
print('no_direct_assert',len(summary['no_direct_assert']),'exact_body_groups',len(summary['exact_body_candidates']))
