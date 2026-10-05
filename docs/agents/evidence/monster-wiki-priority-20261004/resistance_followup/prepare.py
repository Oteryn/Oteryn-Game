from pathlib import Path
import json,re,hashlib,gzip
from fractions import Fraction
out=Path('/tmp/monster-wiki-priority-20261004/resistance_followup');base=Path('/tmp/monster-wiki-priority-20261004');samplepath=Path('/workspace/monster-resource-plan/tools/content-schema/monster-authoring/samples/wiki-population-2026-09-27.json');sample=json.loads(samplepath.read_text());by={r['monster']:r for r in sample['monsters']};inputrows=[]
for lane in ['resistance_a','resistance_b']:
 for r in json.loads((base/lane/'proposal.json').read_text())['rows']:
  if r.get('provenance')=='UNKNOWN':inputrows.append((lane,r))
assert len(inputrows)==15
rows=[];changes=[];confirmed=0
canon={}
for path in Path("/workspace/monster-main-reconciliation-20261004/content/creatures/definitions").glob("creatures-*.json"):
 for rec in json.loads(path.read_text())["records"]:canon[rec["definition"]["identity"]["key"]]=rec["authoring"]["profile"]
for lane,r in inputrows:
 actor=r['target']['key'].split('.')[-1];fact=by.get(actor);proofs=[];m={x['damage_type']:x['percent'] for x in r['old'] or []};canonical={x['damage_type']:x['percent'] for x in r['canonical'] or []};new=dict(m)
 if fact and fact.get('status')=='COMPARED':
  for f in fact.get('rows',[]):
   if f.get('field','').startswith('resistance.') and f.get('status')=='DIFF' and isinstance(f.get('wiki'),(int,float)) and re.fullmatch(r'\s*[+-]?\d+(?:[.,]\d+)?\s*%?\s*',str(f.get('wiki_raw',''))):
    element=f['field'].split('.')[1];taken=Fraction(re.sub(r'[%\s]','',f['wiki_raw']).replace(',','.'));value=100-taken
    if value!=Fraction(str(f['wiki'])):continue
    if actor.endswith('_dirty') and fact.get('creature_identity',{}).get('status')!='VERIFIED':continue
    evidence={'damage_type':element,'provenance':'WIKI_CONFIRMED','value':{'numerator':value.numerator,'denominator':value.denominator},'wiki_raw':f['wiki_raw'],'page':fact['wiki_title'],'url':fact['page_url'],'revision_id':fact['cut_revision_id'],'content_sha256':fact['cut_content_sha256'],'line':f['wiki_line'],'cache':str(samplepath),'cache_sha256':hashlib.sha256(samplepath.read_bytes()).hexdigest(),'exact_actor_binding':fact.get('creature_identity',{'method':'ExactActorNameBaselineCapture'})}
    if actor=='the_sinister_hermit_dirty':
     rawpath=Path('/workspace/monster-round3-output/wiki-cache/2026-09-27/the_sinister_hermit_yellow.json');raw=json.loads(rawpath.read_text())['cut'];assert hashlib.sha256(raw['content'].encode()).hexdigest()==fact['cut_content_sha256'];evidence['raw_page_cache']=str(rawpath);evidence['raw_content_hash_verified']=True
    if taken<0:evidence['not_applied']='negative damage taken needs healing response'
    else:
     immune=element in canon[r['target']['key']].get('immunities',[])
     if not(value==100 and immune) and (value!=0 or element in m):new[element]=evidence['value']
     if value==100 and immune:evidence['already_represented_by_canonical_immunity']=True
     confirmed+=1
     c=canonical.get(element,{'numerator':0,'denominator':1})
     if (Fraction(100) if immune else Fraction(c['numerator'],c['denominator']))!=value:changes.append({'target':r['target'],'field':'resistances','damage_type':element,'canonical_old':c,'new':evidence['value'],'proof':evidence})
    proofs.append(evidence)
 # Selective updates only: unproven current native elements remain exact current values.
 newlist=[{'damage_type':k,'percent':v} for k,v in sorted(new.items())]
 rows.append({'target':r['target'],'field':'resistances','old_present':r['old_present'],'old':r['old'],'new_present':r['old_present'] or bool(newlist),'new':newlist,'canonical':r['canonical'],'canonical_present':r['canonical_present'],'parent_lane':lane,'provenance':'UNKNOWN','apply':bool(proofs),'confirmed_element_updates':proofs,'unknown_policy':'Keep current native unconfirmed elements; exact cached Fandom DIFF observations confirm only listed fields. Missing/MATCH count summaries do not prove field values. Dirty phase uses qualified Yellow page, not generic BR article.','canonical_changes':[c for c in changes if c['target']==r['target']]})
packet={'schema':'monster-wiki-resistance-followup-v1','rows':rows,'counts':{'actors':15,'actors_with_additional_confirmed_fields':sum(bool(r['confirmed_element_updates']) for r in rows),'confirmed_fields':confirmed,'canonical_element_changes':len(changes)},'canonical_changes':changes,'no_network':True,'no_product_edits':True}
(out/'proposal.json').write_text(json.dumps(packet,indent=2)+'\n');print(packet['counts']);print(hashlib.sha256((out/'proposal.json').read_bytes()).hexdigest());print('changes',[(x['target']['key'],x['damage_type'],x['canonical_old'],x['new']) for x in changes])
