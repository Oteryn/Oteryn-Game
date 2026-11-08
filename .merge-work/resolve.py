import subprocess,json,re,gzip
from pathlib import Path
paths=Path('.merge-conflict-paths').read_text().splitlines()
def stage(n,p):
 r=subprocess.run(['git','show',f':{n}:{p}'],capture_output=True)
 return r.stdout if r.returncode==0 else None
pat=re.compile(r'^<<<<<<< [^\n]*\n(.*?)^\|\|\|\|\|\|\| [^\n]*\n(.*?)^=======\n(.*?)^>>>>>>> [^\n]*\n',re.M|re.S)
chunks={}; summary=[]
for p in paths:
 if Path(p).suffix not in ('.rs','.py','.toml','.md'): continue
 s=Path(p).read_text(); cc=[]
 for i,m in enumerate(pat.finditer(s)):
  a,b,c=m.groups(); cc.append([a,b,c])
  if a==b or c==b or a==c or not a or not c: continue
  summary.append(f'FILE {p} #{i}\nOURS:\n{a[:10000]}BASE:\n{b[:1000]}THEIRS:\n{c[:10000]}')
 chunks[p]=cc
Path('.merge-work/chunks.json').write_text(json.dumps(chunks))
Path('.merge-work/manual.txt').write_text('\n'.join(summary))
print('manual lines',len('\n'.join(summary).splitlines()), 'chunks',sum(map(len,chunks.values())))
