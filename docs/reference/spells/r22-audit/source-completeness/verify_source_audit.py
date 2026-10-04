"""Verify complete immutable Git-tree coverage, source byte identity, and no skipped constructor files."""
import argparse,hashlib,json,subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--audit',type=Path,required=True);p.add_argument('--source-root',type=Path,required=True);a=p.parse_args()
summ=json.loads((a.audit/'source-completeness-summary.json').read_text());checks=[]
for label,v in summ['sources'].items():
 upstream='canary' if label.startswith('canary') else 'crystal';revision=v['revision'];checkout=a.source_root/upstream
 tree=subprocess.check_output(['git','-C',str(checkout),'ls-tree','-rz',revision]);want={}
 for row in tree.split(b'\0'):
  if not row:continue
  meta,path=row.split(b'\t');path=path.decode();fields=meta.decode().split()
  if fields[1]=='blob' and path.endswith('.lua'):want[path]=fields[2]
 files=json.loads((a.audit/(label+'-file-classification.json')).read_text())
 assert len(files)==len(want) and {r['file']:r['git_blob'] for r in files}==want
 checks.append({'snapshot':label,'check':'Every tracked Lua has exactly one classification and exact immutable Git blob','rows':len(files),'result':'PASS'})
 for f in files:
  data=(a.audit/'upstream-local-only'/label/f['file']).read_bytes()
  assert hashlib.sha256(data).hexdigest()==f['sha256']
  assert hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()==f['git_blob']
 checks.append({'snapshot':label,'check':'Every local-only source byte equals recorded SHA256 and immutable Git blob','rows':len(files),'result':'PASS'})
 regs=json.loads((a.audit/(label+'-registration-inventory.json')).read_text());errors=json.loads((a.audit/(label+'-unreadable-registration-files.json')).read_text());lex=json.loads((a.audit/(label+'-lexical-spell-constructor-files.json')).read_text())
 assert len({r['registration_key'] for r in regs})==len(regs)
 assert all(r['file'] in want and r['git_blob']==want[r['file']] for r in regs)
 assert {r['file'] for r in regs}|{r['file'] for r in errors}=={r['file'] for r in lex}
 checks.append({'snapshot':label,'check':'Every constructor file has captured registration or explicit unresolved/helper classification','files':len(lex),'result':'PASS'})
 slots=json.loads((a.audit/(label+'-monster-attack-defense-slots.json')).read_text())
 assert len({s['slot_key'] for s in slots})==len(slots)
 assert len(slots)==v['summary']['monster_slots']
 assert all(s['file'] in want and s['git_blob']==want[s['file']] for s in slots)
 # Actual mixed-table defense evidence: defense array is retained even alongside armor/defense named fields.
 assert any(s['block']=='defenses' and s['name']=='combat' and s['entry'].get('type')=='@COMBAT_HEALING' for s in slots)
 checks.append({'snapshot':label,'check':'Attack and mixed-table defense slots are unique and bound to actual source blobs','slots':len(slots),'result':'PASS'})
report={'checks':checks,'all_passed':True,'scope':'Source identity and census completeness only; no external wiki semantic verification or gameplay claim'}
(a.audit/'inventory-verification.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS',len(checks),'independent coverage, source byte and slot checks')
