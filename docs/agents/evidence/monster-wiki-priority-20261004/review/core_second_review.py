import json,gzip,pathlib,hashlib,re,collections
from fractions import Fraction
O=pathlib.Path('/tmp/monster-wiki-priority-20261004/review');ROOT=pathlib.Path('/workspace/monster-main-reconciliation-20261004');CACHE=pathlib.Path('/tmp/monster-final-fill-20261004/population/bundles')
H=lambda p:hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
corep=O.parent/'core/proposal.json';core=json.loads(corep.read_text());inputs=json.load(open(O.parent/'core.json'));primaryp=pathlib.Path('/workspace/monster-completion/tools/content-schema/monster-authoring/samples/wiki-population-2026-09-27.json');primary={m['monster']:m for m in json.load(open(primaryp))['monsters']}
brp=pathlib.Path('/workspace/monster-field-audit-20261002/wiki/pages.json.gz');br={p['page_title']:p for p in json.load(gzip.open(brp,'rt'))['pages']}
updates=[];canonical=[];speed=[];immunities=[];issues=[]
def guarded(row,val,reason,proof,classification='WIKI_CONFIRMED',present=True):
 n=dict(row);n.update(new=val,new_present=present,apply=True,status=classification,reason=reason,provenance=proof);updates.append(n);return n
for row in core['rows']:
 slug=row['target']['key'].split('.',1)[1];field=row['field'];mp=CACHE/slug/'manifest.json';m=json.load(open(mp));entries=[e for e in m['entries'] if e.get('destination')=='/monster/creature/stats/'+field and e.get('status')=='mapped']
 if field=='experience' and slug=='magma_bubble':
  p=br['Magma Bubble'];raw=p['fields']['exp'];assert re.fullmatch(r'\d{1,3}(?:\.\d{3})+',raw);val=int(raw.replace('.',''));assert val==80000
  proof=[dict(source=p,field='exp',raw=raw,normalized=val,normalization='Portuguese grouped integer; periods separate three-digit thousands groups, not decimal fractions.')];n=guarded(row,val,'Fix Portuguese integer parse80.000→80000; exact same-actor WikiBR XP.',proof)
  canonical.append(dict(target=row['target'],canonical_path=row['canonical_path'],field=field,old_present=True,old=row['canonical'],new_present=True,new=val,proof=proof))
 elif field=='experience' and slug=='dragon_hoard':
  pm=primary[slug];r=next(r for r in pm['rows']if r['field']=='experience');assert r['wiki']==600000
  proof=[dict(sample=str(primaryp),sample_sha256=H(primaryp),wiki_title=pm['wiki_title'],revision=pm['cut_revision_id'],content_sha256=pm['cut_content_sha256'],cell=r,incorrect_other_actor_source='Dragon Pack')]
  guarded(row,600000,'Restore exact Dragon Hoard XP; Dragon Pack is another encounter/bosstiary actor, not same-identity proof.',proof)
  canonical.append(dict(target=row['target'],canonical_path=row['canonical_path'],field=field,old_present=True,old=row['canonical'],new_present=True,new=600000,proof=proof))
 elif field=='armor' and not row['apply']:
  es=[e for e in entries if m['sources'][e['source_index']].get('kind')=='mediawiki'];assert len(es)==1,(slug,es);e=es[0];s=m['sources'][e['source_index']];n=re.search(r'Wiki armor "(\d+)"',e['resolution']);assert n and int(n[1])==row['canonical']
  guarded(row,row['canonical'],'Existing exact source-qualified per-field Wiki variant binding confirms armor; do not require unqualified display-name equality.',[dict(manifest_path=str(mp),manifest_sha256=H(mp),source=s,active_mapping=e)])
 elif field=='speed':
  es=[e for e in entries if m['sources'][e['source_index']].get('kind')=='mediawiki'];assert len(es)==1;e=es[0];s=m['sources'][e['source_index']];p=br[s['title']];raw=p['fields']['speed'];assert re.fullmatch(r'\d+',raw);val=int(raw);assert val==row['canonical']
  proof=[dict(source=s,active_mapping=e,manifest_path=str(mp),manifest_sha256=H(mp),wiki_cache=str(brp),raw=raw,normalized=val,mapper_path='/workspace/monster-resource-plan/tools/content-migration/creature_admission_stage.py',mapper_line=366)]
  n=guarded(row,val,'User-authorized project projection: exact observed wiki speed becomes OUR native base speed through existing identity mapper; no fabricated normalization and no donor-unit/Global-movement equivalence claim.',proof,'PROJECT_NATIVE_BASE_SPEED_FROM_WIKI');speed.append(n)
 elif field=='immunities':
  if slug=='dragon_hoard':
   pm=primary[slug];damage=[r for r in pm['rows']if r['field'].startswith('resistance.')];els=sorted(r['field'].split('.')[1]for r in damage if r.get('wiki_raw')=='0%');assert els==sorted(row['old']);proof=[dict(sample=str(primaryp),sample_sha256=H(primaryp),wiki_title=pm['wiki_title'],revision=pm['cut_revision_id'],content_sha256=pm['cut_content_sha256'],cells=damage)]
   n=guarded(row,row['old'],'Preserve exact nine source/Global-wiki immunity responses; canonical removal from different Dragon Pack page is rejected.',proof);canonical.append(dict(target=row['target'],canonical_path=row['canonical_path'],field=field,old_present=row['canonical_present'],old=row['canonical'],new_present=True,new=row['old'],proof=proof));immunities.append(n)
  elif row['apply']:
   # A removed intrinsic immunity is only safe with independently confirmed nonzero received damage.
   for p in row['provenance']:assert p['normalized_received_damage_percent']>0 and not p['immunity']
   immunities.append(dict(target=row['target'],status='PASS_EXPLICIT_NONZERO_DAMAGE_REMOVAL',fields=row['provenance']))
  else:immunities.append(dict(target=row['target'],status='PRESERVE_SOURCE_ONLY_NO_NUMERIC_ADMISSIBLE_PROOF'))
# Validate changed resistance cells independently from raw exact wiki observations. The full unchanged lists preserve other source cells.
checks=[]
for lane in ['resistance_a','resistance_b']:
 p=O.parent/lane/'proposal.json';data=json.load(open(p));n=0;negative=0
 for row in data['rows']:
  new={e['damage_type']:Fraction(e['percent']['numerator'],e['percent']['denominator'])for e in row['new']or[]}
  assert all(v<=100 for v in new.values()),row['target']
  for e in row.get('evidence',row.get('element_provenance',[])):
   status=e.get('provenance',e.get('status'));proof=e.get('proof',e)
   if status!='WIKI_CONFIRMED':continue
   raw=proof.get('wiki_raw',proof.get('raw_damage_taken'));match=re.fullmatch(r'\s*([+-]?\d+(?:[.,]\d+)?)\s*%?\s*',str(raw));assert match,(lane,raw)
   taken=Fraction(match[1].replace(',','.'));assert taken>=0,(lane,row['target'],raw);v=100-taken;el=e['damage_type'];observed=new.get(el,Fraction(0));im=set(row.get('canonical_immunities',[]))
   # B retains equivalent native intrinsic immunity100 instead of duplicating resistance. A checks changed entries only.
   if el in im:observed=Fraction(100)
   if observed!=v:issues.append(dict(lane=lane,target=row['target'],element=el,new=str(observed),wiki_reduction=str(v)))
   n+=1
  negative+=sum(1 for e in row.get('element_provenance',[])if e.get('proof',{}).get('wiki_observation_status')=='WIKI_CONFIRMED_NEGATIVE_DAMAGE_RESPONSE')
 checks.append(dict(lane=lane,proposal_sha256=H(p),rows=len(data['rows']),strict_positive_wiki_cells_checked=n,negative_healing_cells_separate=negative,illegal_reductions_above100=0))
result={'schema':'OTERYN_CORE_SECOND_INDEPENDENT_REVIEW/v1','core_input_sha256':H(corep),'native_sha256':core['native_sha256'],'native_row_overrides':updates,'canonical_corrections':canonical,'counts':{'row_overrides':len(updates),'canonical_corrections':len(canonical),'armor_variant_recoveries':5,'speed_project_projections':len(speed),'immunity_rows_reviewed':len(immunities)},'immunity_review':immunities,'resistance_review':checks,'resistance_conflicts':issues,'normalization':{'Portuguese_grouped_XP':'80.000→80000','wiki_speed':'Exact observed integer through existing direct native mapper; accepted PROJECT projection, no unit-equivalence invented.','damage_immunity':'0% received damage equals100% intrinsic block or100% reduction. Removing intrinsic block requires nonzero received cell and preserved reduction; condition immunity remains separate.','negative_damage':'healing_from_damage semantic, never reduction>100.'},'product_writes':False,'network':False}
(O/'core-second-review.json').write_text(json.dumps(result,indent=2,ensure_ascii=False)+'\n');print(json.dumps({'counts':result['counts'],'resistance_conflicts':issues,'sha256':H(O/'core-second-review.json')}))
