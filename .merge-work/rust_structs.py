import re
from pathlib import Path
# Mask comments and strings so braces inside SQL/fixtures are not Rust syntax.
pat=re.compile(r'//[^\n]*|/\*.*?\*/|r(#+)".*?"\1|r".*?"|b?"(?:\\.|[^"\\])*"|b?\'(?:\\.|[^\'\\])\'',re.S)
def mask(s):return pat.sub(lambda m:' '*(m.end()-m.start()),s)
def fields(s):
 m=mask(s);depth=0;last=0;out=[]
 for i,ch in enumerate(m):
  if ch in '{([':depth+=1
  elif ch in '})]':depth-=1
  elif ch==',' and depth==0:out.append((last,i+1));last=i+1
 if s[last:].strip():out.append((last,len(s)))
 return out
def fix(s,types):
 m=mask(s);edits=[]
 for hit in re.finditer(r'\b('+ '|'.join(types)+r')\s*\{',m):
  prefix=m[max(0,hit.start()-40):hit.start()]
  if re.search(r'(struct|impl|enum|->)\s*$',prefix):continue
  start=hit.end();depth=1;end=start
  while end<len(m) and depth:
   if m[end]=='{':depth+=1
   elif m[end]=='}':depth-=1
   end+=1
  body=s[start:end-1];parts=fields(body);seen={};rem=[];has_rest=False
  for a,b in parts:
   v=body[a:b].strip().rstrip(',').strip()
   if v.startswith('..'):has_rest=True
   f=re.match(r'(\w+)(?:\s*:|$)',v)
   if not f:continue
   key=f[1]
   if key in seen:
    if re.sub(r'\s+','',v)!=re.sub(r'\s+','',seen[key]):print('DUP DIFFERENT',hit[1],key)
    rem.append((a,b))
   else:seen[key]=v
  if has_rest or not seen:continue
  defaults=types[hit[1]]
  add=[f'{k}: {v},' for k,v in defaults.items() if k not in seen]
  for a,b in reversed(rem):body=body[:a]+body[b:]
  if add:
   body=body.rstrip();body+=('' if body.endswith(',') else ',')+'\n'+'\n'.join(add)+'\n'
  if add or rem:edits.append((start,end-1,body))
 # Nested struct edits: apply innermost first by rerunning, outer entries only if no nested edit.
 for a,b,v in reversed(edits):
  if any(a<x and y<b for x,y,_ in edits):continue
  s=s[:a]+v+s[b:]
 return s
T={
'Callback':dict(use_range='1',disguise='None',completes='false',premium_required='false',present='Vec::new()',required_travel='Vec::new()',state_operations='Vec::new()',failure_teleport='None',achievements='Vec::new()',activation='None',item_reward='None',remote_teleport='None',requirements='None',reward_bundle='false'),
'Selector':dict(ordinal='None',position_aliases='Vec::new()'),
'Storage':dict(read_only='false',set_if='None',add='None',increment='false',at_least='false'),
'OutfitReward':dict(sex='None'),
'ItemAmount':dict(fluid_subtype='None'),
'ItemReward':dict(premium='false',key_number='None',backpack_outputs='Vec::new()'),
}
if __name__=='__main__':
 for root in ['crates/gameplay/quests','crates/gameplay/dispatch','crates/gameplay/durability','apps/game-server']:
  for p in Path(root).rglob('*.rs'):
   s=p.read_text();n=s
   for _ in range(3):n=fix(n,T)
   if n!=s:p.write_text(n);print(p)
