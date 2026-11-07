"""Apply revision-pinned loot proposals to independent copies of changed bundles.

No network, source evaluator, Git mutation or native identity allocation. The packet
is evidence input; exact unchanged input index and four-file bundle digests guard
all replacements. Unrelated stats, attacks, mechanics and dependency data survive.
"""
from __future__ import annotations
import argparse, copy, hashlib, json, re
from fractions import Fraction
from collections import Counter,defaultdict
from pathlib import Path

def read(path): return json.loads(Path(path).read_text())
def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def digest(directory):
 h=hashlib.sha256()
 for name in ('monster.json','dependencies.json','catalog.json','manifest.json'):
  data=(directory/name).read_bytes();h.update(f'{name}\0{len(data)}\0'.encode()+data)
 return h.hexdigest()
def percent(ppm): return ppm/10000

def apply_loot_updates(monster,catalog,manifest,proposals):
 """Pure checked data mutation, preserving all non-loot monster content."""
 monster,catalog,manifest=copy.deepcopy(monster),copy.deepcopy(catalog),copy.deepcopy(manifest)
 sources=manifest['sources'];rows=manifest['entries'];changed=[];flags=['WIKI_LOOT_ESTIMATE']
 def source_index(source):
  if source not in sources:sources.append(copy.deepcopy(source))
  return sources.index(source)
 def wiki_row(p,index,field,destination,resolution):
  rows.append({'source_index':source_index(p['wiki_source']),'source_file':p['wiki_source']['title'],'source_line':p['wiki_line'],'source_field':field,'kind':'field','status':'mapped','destination':destination,'resolution':resolution})
 for p in proposals:
  if type(p['times']) is not int or type(p['kills']) is not int or not 0 < p['times'] <= p['kills']:
   raise ValueError('Wiki observed drop count/kills are invalid')
  share=len(p.get('probability_share_ids',[1])); denominator=p['kills']*share
  if share<1:raise ValueError('empty probability share')
  quotient,remainder=divmod(p['times']*1000000,denominator)
  expected_ppm=quotient+int(2*remainder>denominator or (2*remainder==denominator and quotient%2==1))
  if Fraction(str(p['after']['probability_percent']))*10000!=expected_ppm or expected_ppm<1:
   raise ValueError('proposal probability is not exact half-even 1ppm source estimate')
  if not 1<=p['after']['min_count']<=p['after']['max_count']:
   raise ValueError('invalid observed count range')
  if p['kind']=='REPLACE_SUPPORTED_WIKI_ESTIMATE':
   index=p['entry_index'];entries=monster['loot']['entries']
   if entries[index]!=p['before']:raise ValueError(f"{p['monster']}: current loot differs from checked before entry {index}")
   if p['before']['item']!=p['after']['item']:raise ValueError('replacement cannot change exact Item reference')
   source=next((r for r in rows if r.get('source_field')==p['source_field'] and r.get('destination')==f'/monster/loot/entries/{index}' and r.get('status')=='mapped'),None)
   if source is None:raise ValueError('source declaration pairing is absent')
   entries[index]=copy.deepcopy(p['after'])
   source['resolution']+=f" Existing donor probability/count retained in completion ledger; adopted pinned Wiki estimate {p['times']} drops/{p['kills']} kills, {p['version']}, revision {p['wiki_source']['revision_id']}; rounded half-even to 1ppm."
   for field in ('probability_percent','min_count','max_count'):
    if p['before'][field]!=p['after'][field]:
     wiki_row(p,index,'Loot2.'+p['wiki_item']+('.amount' if field!='probability_percent' else ''),f'/monster/loot/entries/{index}/{field}',f"Pinned source {p['times']} observed drops/{p['kills']} kills; amount {p['after']['min_count']}-{p['after']['max_count']}; prior donor value {p['before'][field]}; estimate rounded half-even to 1ppm.")
  elif p['kind']=='ADD_SUPPORTED_WIKI_OBSERVATION':
   if 'loot' not in monster:
    identity=monster['creature']['identity']; key=identity['key'].replace('creature/','loot/',1)
    if key==identity['key']:raise ValueError('cannot derive established source-scoped loot key')
    monster['loot']={'identity':{'key':key,'revision':identity['revision']},'algorithm':'IndependentBernoulli','entries':[]}
    monster['creature']['loot']={'family':'Loot',**monster['loot']['identity']}
   entries=monster['loot']['entries']
   if any(e['item']==p['after']['item'] for e in entries):raise ValueError('addition duplicates an existing exact Item reference')
   index=len(entries);entries.append(copy.deepcopy(p['after']))
   ref=p['after']['item']
   if ref not in catalog['definitions']:catalog['definitions'].append(copy.deepcopy(ref))
   evidence=p.get('item_page_evidence') or {}
   for page in [evidence,*evidence.get('variants',[])]:
    if all(k in page for k in ('page_title','page_id','cut_revision_id','cut_content_sha256')):
     source_index({'kind':'mediawiki','api':'https://tibia.fandom.com/api.php','title':page['page_title'],'page_id':page['page_id'],'revision_id':page['cut_revision_id'],'content_sha256':page['cut_content_sha256']})
   if p['classification']=='LOW_CONFIDENCE_WIKI_ADDITION':flags.append('LOW_CONFIDENCE_WIKI_LOOT')
   number=ref['key'].rsplit('/',1)[1]
   wiki_row(p,index,'Loot2.'+p['wiki_item']+('#'+number if len(p['probability_share_ids'])>1 else ''),f'/monster/loot/entries/{index}',f"Pinned Wiki observation: {p['times']} drops/{p['kills']} kills in version {p['version']}, exact accepted ItemID {number}; {len(p['probability_share_ids'])} explicitly resolved source variant(s), equal probability share; rounded half-even to 1ppm. {p['classification']}. Full original evidence retained in completion packet.")
  else:raise ValueError('unknown patch kind')
  changed.append(p)
 entries=monster['loot']['entries'];order=sorted(range(len(entries)),key=lambda n:entries[n]['probability_percent']);moved={old:new for new,old in enumerate(order)};entries[:]=[entries[i] for i in order]
 for row in rows:
  match=re.fullmatch(r'/monster/loot/entries/(\d+)(/.*)?',row.get('destination',''))
  if match:row['destination']=f'/monster/loot/entries/{moved[int(match[1])]}{match[2] or ""}'
 return monster,catalog,manifest,sorted(set(flags))

def prepare(baseline,index_path,packet_path,output):
 baseline,index_path,packet_path,output=map(Path,(baseline,index_path,packet_path,output))
 packet=read(packet_path);index=read(index_path)
 if sha(index_path)!=packet['baseline_index_sha256']:raise ValueError('baseline index differs from checked packet')
 by_name={r['monster']:r for r in index['monsters']};grouped=defaultdict(list)
 for p in packet['proposals']:grouped[p['monster']].append(p)
 if output.exists() and any(output.iterdir()):raise ValueError('output must be fresh to avoid mixing candidate evidence')
 output.mkdir(parents=True,exist_ok=True);changed=[]
 for slug,proposals in sorted(grouped.items()):
  source=baseline/slug
  if digest(source)!=by_name[slug]['sha256']:raise ValueError(slug+': baseline bundle digest differs')
  original={name:read(source/name) for name in ('monster.json','dependencies.json','catalog.json','manifest.json')}
  m,c,f,flags=apply_loot_updates(original['monster.json'],original['catalog.json'],original['manifest.json'],proposals)
  out=output/slug;out.mkdir()
  for name,data in [('monster.json',m),('dependencies.json',original['dependencies.json']),('catalog.json',c),('manifest.json',f)]:
   (out/name).write_text(json.dumps(data,indent=2,ensure_ascii=False)+'\n')
  if original['monster.json']['creature']['stats']!=m['creature']['stats']:raise ValueError('unrelated stats changed')
  if original['dependencies.json']!=read(out/'dependencies.json'):raise ValueError('unrelated dependencies changed')
  by_name[slug]['sha256']=digest(out)
  by_name[slug]['completion_flags']=sorted(set(by_name[slug].get('completion_flags',[])+flags))
  changed.append({'monster':slug,'completion_flags':flags,'before_entries':len(original['monster.json'].get('loot',{}).get('entries',[])),'after_entries':len(m['loot']['entries']),'bundle_sha256':digest(out)})
 return index,{'schema':'OTERYN_CHECKED_LOOT_COMPLETION_RESULT/v1','packet_sha256':sha(packet_path),'index_input_sha256':sha(index_path),'changed_monsters':len(changed),'proposal_counts':dict(Counter(p['kind'] for p in packet['proposals'])),'changed':changed,'preserved_unrelated_stats_and_dependencies':True,'runtime_verified':False}

def main():
 p=argparse.ArgumentParser(description=__doc__)
 for field in ('baseline','index','packet','output'):p.add_argument('--'+field,type=Path,required=True)
 args=p.parse_args();index,result=prepare(args.baseline,args.index,args.packet,args.output)
 (args.output.parent/'loot-patched-index.json').write_text(json.dumps(index,indent=2,ensure_ascii=False)+'\n')
 (args.output.parent/'loot-applied-receipt.json').write_text(json.dumps(result,indent=2,ensure_ascii=False)+'\n')
 print(json.dumps({k:v for k,v in result.items() if k!='changed'}))
if __name__=='__main__':main()
