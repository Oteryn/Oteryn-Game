exec(Path('.merge-work/resolve.py').read_text().split('chunks={};')[0])
import difflib
pat=re.compile(r'^<<<<<<< [^\n]*\n(.*?)^=======\n(.*?)^>>>>>>> [^\n]*\n',re.M|re.S)
allchunks={}; pending=[]
for p in paths:
 if Path(p).suffix not in ('.rs','.py','.toml','.md'):continue
 original=Path(p).read_text(); target=Path('.merge-work/originals')/p;target.parent.mkdir(parents=True,exist_ok=True);target.write_text(original)
 cs=[]
 for m in pat.finditer(original):
  left,c=m.groups(); ab=re.split(r'^\|\|\|\|\|\|\| [^\n]*\n',left,maxsplit=1,flags=re.M); a=ab[0];b=ab[1] if len(ab)>1 else None;cs.append([a,b,c])
 allchunks[p]=cs
Path('.merge-work/allchunks.json').write_text(json.dumps(allchunks))
print('all',sum(map(len,allchunks.values())))
