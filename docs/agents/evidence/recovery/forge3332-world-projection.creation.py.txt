from pathlib import Path
import json,hashlib
R=Path('/workspace/pr1437-resistances');p=R/'docs/agents/evidence/OTV2-20261002-item-forge3332-source-qualification-v2.json';j=json.loads(p.read_text());j['historical_world_owner_exclusion']=j['world_owner_exclusion'];files=[]
for family in ('terrain','objects'):
 for f in sorted((R/f'content/world/{family}').glob('*.json')):
  v=json.loads(f.read_text());hits=sum(e.get('provenance',{}).get('item_pointer',{}).get('key')==j['target']['key'] for e in v.get('records',[]));assert hits==0;files.append({'path':str(f.relative_to(R)),'sha256':hashlib.sha256(f.read_bytes()).hexdigest(),'exact_item_pointer_hits':hits})
j['world_owner_exclusion']={'native_parent':j['native_baseline_head'],'actual_current_files':files,'current_exact_pointer_hits':0,'historical_proof_retained':True}
p.write_text(json.dumps(j,ensure_ascii=False,indent=2)+'\n');print(hashlib.sha256(p.read_bytes()).hexdigest())
