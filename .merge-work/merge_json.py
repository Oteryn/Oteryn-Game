import json,subprocess,gzip
from pathlib import Path
missing=object(); conflicts=[]
def stage(n,p):
 r=subprocess.run(['git','show',f':{n}:{p}'],capture_output=True)
 return r.stdout if r.returncode==0 else None
def identity(v):
 if not isinstance(v,dict):return None
 binding=v.get('source_binding',{})
 if isinstance(binding,dict) and isinstance(binding.get('project_position'),dict):
  return ('source-placement',json.dumps(binding['project_position'],sort_keys=True))
 if 'unique_id' in v:return ('unique_id',v['unique_id'])
 for k in ('production_definition','definition'):
  d=v.get(k)
  if isinstance(d,dict):
   if 'production_key' in d:return (k,d.get('family'),d['production_key'])
   if isinstance(d.get('identity'),dict):return (k,d['identity'].get('family'),d['identity']['key'])
 for k in ('production_key','key','id','quest_key','quest','npc_key','npc','path','file','batch_id','source_path','name','track','item_key','outfit_key','grant_key','reference'):
  if isinstance(v.get(k),(str,int)):return(k,v[k])
 return None
def merge(b,a,c,path):
 if a==c:return a
 if c==b:return a
 if a==b:return c
 if a is missing:return c
 if c is missing:return a
 if isinstance(a,dict) and isinstance(c,dict):
  bb=b if isinstance(b,dict) else {};return {k:merge(bb.get(k,missing),a.get(k,missing),c.get(k,missing),path+'/'+k) for k in dict.fromkeys([*a,*c])}
 if isinstance(a,list) and isinstance(c,list):
  bb=b if isinstance(b,list) else []
  ids=[identity(v) for v in a+c]
  if ids and all(i is not None for i in ids) and len(set(map(identity,a)))==len(a) and len(set(map(identity,c)))==len(c):
   aa={identity(v):v for v in a};cc={identity(v):v for v in c};ba={identity(v):v for v in bb}
   return [merge(ba.get(k,missing),aa.get(k,missing),cc.get(k,missing),path+'/'+str(k)) for k in dict.fromkeys([*aa,*cc])]
  out=list(a)
  from collections import Counter
  seen=Counter(json.dumps(v,sort_keys=True) for v in a)
  required=Counter()
  for v in c:
   key=json.dumps(v,sort_keys=True);required[key]+=1
   if required[key]>seen[key]:out.append(v);seen[key]+=1
  return out
 conflicts.append((path,a,c));return a
paths=Path('.merge-conflict-paths').read_text().splitlines()
for p in paths:
 if not (p.endswith('.json') or p.endswith('.json.gz')):continue
 vals=[]
 for n in (1,2,3):
  raw=stage(n,p)
  if raw is None:vals.append(missing);continue
  if p.endswith('.gz'):raw=gzip.decompress(raw)
  try:vals.append(json.loads(raw))
  except Exception as e:
   
   if n == 1: vals.append(missing)
   else: raise
 
 print('merging',p,flush=True)
 result=merge(*vals,p)
 def clean(v):
  if isinstance(v,dict):return {k:clean(x) for k,x in v.items() if x is not missing}
  if isinstance(v,list):return [clean(x) for x in v if x is not missing]
  return v
 result=clean(result)
 oursraw=stage(2,p)
 compact=oursraw is not None and not p.endswith('.gz') and b'\n' not in oursraw.rstrip(b'\n')
 raw=((json.dumps(result,separators=(',',':'),ensure_ascii=False) if compact else json.dumps(result,indent=2,ensure_ascii=False))+'\n').encode()
 if p.endswith('.gz'):raw=gzip.compress(raw,mtime=0)
 Path(p).write_bytes(raw)
Path('.merge-work/json-scalar-conflicts.json').write_text(json.dumps(conflicts,indent=2))
print('JSON scalar conflicts',len(conflicts))
