"""Bounded offline authored68 target/consumer association. No runtime admission."""
import argparse,collections,hashlib,json,re,subprocess,unicodedata
from pathlib import Path
PIN='ad7a08f96caa4e7bd0e7fa90c67b39229d637277'
RECIPE='tools/content-schema/quest-authoring/samples/authored68/recipes.json'
def sha(raw):return hashlib.sha256(raw).hexdigest()
def semantic(x):return sha(json.dumps(x,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode())
def norm(s):return ' '.join(unicodedata.normalize('NFC',s).casefold().split())
def resolve(table,families,name):
 rows={semantic(x['ref']):x for family in families for x in table.get((family,norm(name)),[])}
 return list(rows.values())
class Epoch:
 def __init__(self,root):
  self.root=Path(root); self.pin=self.git('rev-parse','HEAD').decode().strip(); assert self.pin==PIN,('wrong epoch',self.pin)
  self.paths=self.git('ls-tree','-r','--name-only',self.pin,'content').decode().splitlines();self.inputs=[]
 def git(self,*args):return subprocess.check_output(['git','-C',str(self.root),*args])
 def read(self,path):
  raw=self.git('show',self.pin+':'+path);self.inputs.append({'path':path,'sha256':sha(raw),'epoch':self.pin});return json.loads(raw)
 def evidence(self,path,pointer,record):
  return {'path':path,'json_pointer':pointer,'record_sha256':semantic(record),'file_sha256':next(x['sha256'] for x in self.inputs if x['path']==path),'epoch':self.pin}

def build(repo_root,epoch_root,world_path):
 e=Epoch(epoch_root);root=Path(repo_root);raw=(root/RECIPE).read_bytes();recipes=json.loads(raw)['recipes'];assert len(recipes)==68
 table=collections.defaultdict(list);definitions={};outcomes=collections.defaultdict(list)
 def add(family,name,identity,ev,record,source='canonical_named_record'):
  if not name:return
  ref={'family':family,**identity};ref.pop('family',None);ref={'family':family,**ref}
  row={'ref':ref,'declared_name':name,'identity_basis':source,'evidence':[ev],'source_bindings':record.get('source_bindings',[]),'materializable':record.get('definition',{}).get('materializable'),'stack_class':record.get('definition',{}).get('stack_class'),'definition':record.get('definition',{})}
  table[(family,norm(name))].append(row);definitions[(family,identity['key'],identity['revision'])]=row
 prefixes=('content/items/definitions/items-','content/creatures/definitions/creatures-','content/cosmetics/mounts/mounts-','content/achievements/achievements-','content/world/areas/hunting-places/hunting-','content/world/areas/regions/areas-','content/encounters/definitions/encounters-','content/npcs/definitions/npcs-')
 for path in e.paths:
  if not path.endswith('.json') or not path.startswith(prefixes):continue
  d=e.read(path)
  for i,r in enumerate(d.get('records',d.get('areas',[]))):
   ev=e.evidence(path,('/records/' if 'records'in d else '/areas/')+str(i),r);de=r.get('definition',r.get('declaration',r));ident=de.get('identity',{});fam=de.get('kind',ident.get('family',d.get('family')))
   if not ident:continue
   if fam=='Item':
    n=de.get('semantics',{}).get('presentation',{}).get('value',{}).get('name',{});name=n.get('value') if n.get('state')=='KNOWN' else None
   elif fam=='Creature':name=r.get('authoring',{}).get('profile',{}).get('details',{}).get('display_name')
   elif fam=='Mount':name=r.get('editor',{}).get('display_name')
   elif fam=='Achievement':name=r.get('name')
   elif fam in ('Area','region','city'):fam='Area';name=de.get('name')
   else:name=r.get('editor',{}).get('display_name')
   if fam=='Encounter':
    details=r.get('authoring',{}).get('profile',{}).get('details',{})
    roles={p['role']:p.get('creatures',[]) for p in details.get('participants',[])}
    for j,rule in enumerate(details.get('rules',[])):
     trigger=rule.get('trigger',{})
     if trigger.get('kind')!='creature_died':continue
     for a in rule.get('actions',[]):
      if a.get('kind')=='emit_outcome':
       for cr in roles.get(trigger.get('role'),[]):outcomes[cr['key']].append({'encounter_ref':{'family':'Encounter',**ident},'outcome':a['outcome'],'credit_policy':a.get('credited'),'rule_key':rule['key'],'conditions':rule.get('conditions',[]),'evidence':ev,'rule_json_pointer':ev['json_pointer']+'/authoring/profile/details/rules/'+str(j)})
   add(fam,name,ident,ev,r)
 # Editor labels are joined to declarations by complete identity, never guessed from slug.
 path='content/world/definitions/declarations.json';d=e.read(path);decl={(x.get('kind'),x['identity']['key'],x['identity']['revision']):(i,x) for i,x in enumerate(d['records']) if 'identity'in x}
 path2='content/world/editor/author.json';ed=e.read(path2)
 for i,x in enumerate(ed['entries']):
  ref=x['target'];key=(ref['family'],ref['key'],ref['revision'])
  if key not in decl:continue
  if ref['family'] not in ('Outfit','NPC'):continue
  j,r=decl[key];add(ref['family'],x.get('display_name'),r['identity'],e.evidence(path,'/records/'+str(j),r),r,'canonical_editor_to_declared_complete_identity')
  table[(ref['family'],norm(x.get('display_name','')))][-1]['evidence'].append(e.evidence(path2,'/entries/'+str(i),x))
 # Qualified world snapshot confirms presence only; it never makes historical filename ownership exact.
 wb=Path(world_path).read_bytes();world=json.loads(wb);world_ids={(x.get('kind'),x['identity']['key'],x['identity']['revision']) for x in world['records'] if 'identity'in x}
 authored=[]
 for path in e.paths:
  if not path.startswith('content/quests/definitions/quests-') or not path.endswith('.json'):continue
  for i,r in enumerate(e.read(path)['records']):
   de=r['definition']
   if de.get('definition_profile')=='oteryn_authored_v1':authored.append({'definition':de,'evidence':e.evidence(path,'/records/'+str(i)+'/definition',de)})
 assert len(authored)==68
 qs={(q['definition']['identity']['key'],q['definition']['identity']['revision']):q for q in authored}; assert len(qs)==68, 'duplicate canonical identity'
 identity_tool='tools/content-schema/quest-authoring/authored_quest_authoring.py';tool=e.git('show',e.pin+':'+identity_tool)
 expected=b"slug = re.sub('[^a-z0-9]+', '_', title.lower()).strip('_')"
 assert expected in tool and b"'oteryn:quest.authored.' + slug" in tool, 'authoritative identity algorithm changed'
 e.inputs.append({'path':identity_tool,'sha256':sha(tool),'epoch':e.pin})
 records=[];seen=set()
 def mapped(name,families):
  hits=resolve(table,families,name)
  if len(hits)!=1:return {'target':name,'status':'AMBIGUOUS_CANONICAL_NAME' if hits else 'CANONICAL_EXACT_NAME_NOT_FOUND','refs':[],'match_count':len(hits)}
  h=hits[0];ref=h['ref'];return {'target':name,'status':'EXACT_CANONICAL_IDENTITY_ASSOCIATION','refs':[ref],'declared_name':h['declared_name'],'identity_basis':h['identity_basis'],'evidence':h['evidence'],'source_bindings':h['source_bindings'],'qualified_world_presence':(ref['family'],ref['key'],ref['revision'])in world_ids,'materializable':h['materializable'],'stack_class':h['stack_class']}
 for recipe in recipes:
  key='oteryn:quest.authored.'+re.sub('[^a-z0-9]+','_',recipe['wiki_title'].lower()).strip('_')
  identity=(key,'authored-r1');assert identity not in seen, 'duplicate recipe identity';seen.add(identity)
  entry=qs[identity];q=entry['definition'];assert q['display_name']==recipe['wiki_title'], 'canonical display mismatch'
  assert semantic(q['recipe'])==semantic(recipe), 'canonical/local recipe payload differs'
  stages=[]
  for s in recipe['stages']:
   kind=s['kind'];families={'kill':['Creature'],'collect':['Item'],'use':['Item','NPC'],'explore':['Area'],'complete':[],'talk':['NPC']}[kind]
   targets=[mapped(n,families) for n in s['targets']] if families else []
   seam=[];holds=[]
   if kind=='kill':
    for t in targets:
     for ref in t['refs']:
      matches=outcomes.get(ref['key'],[])
      if len(matches)==1:seam.append({'target':t['target'],**matches[0],'status':'EXISTING_DECLARED_ENCOUNTER_OUTCOME','execution_verified':False})
      elif len(matches)>1:holds.append('MULTIPLE_ENCOUNTER_OUTCOMES_REQUIRE_EXPLICIT_SELECTION')
    holds.append('CHOSEN_STAGE_KILL_EVENT_AND_CREDIT_BINDING_PENDING')
   elif kind=='collect':holds+=['INVENTORY_COUNT_CONSUMER_AND_PER_TARGET_QUANTITY_BINDING_PENDING']
   elif kind=='use':holds+=['ITEM_NPC_IDENTITY_IS_NOT_USE_ACTION_OR_PLACEMENT_BINDING','NATIVE_USE_CONSUMER_BINDING_PENDING']
   elif kind=='explore':holds+=['AREA_LABEL_OR_POSITION_IS_NOT_QUEST_TRIGGER_BOUNDARY','NATIVE_AREA_ENTRY_CONSUMER_BINDING_PENDING']
   elif kind=='complete':
    seam=[{'kind':'EXISTING_CHOSEN_STAGE_GRAPH','incoming_stage_keys':[x['key'] for x in recipe['stages'] if s['key'] in x['next']],'next_stage_keys':s['next'],'status':'AUTHORED_GRAPH_INTENT_ONLY','execution_verified':False}];holds+=['NATIVE_COMPLETION_REDUCER_BINDING_PENDING']
   else:holds+=['DIALOGUE_LANE_OWNS_TALK_BINDING']
   if any(t['status']!='EXACT_CANONICAL_IDENTITY_ASSOCIATION' for t in targets):holds+=['ONE_OR_MORE_EXACT_TARGET_IDENTITIES_UNRESOLVED']
   stages.append({'stage_key':s['key'],'kind':kind,'count':s['count'],'basis':s['basis'],'targets':targets,'consumer_seams':seam,'unresolved':sorted(set(holds)),'runtime_admitted':False})
  rewards=[]
  for i,r in enumerate(recipe['reward_intents']):
   fam={'item':'Item','achievement':'Achievement','outfit':'Outfit','mount':'Mount','access':'Area'}.get(r['kind']);hit=mapped(r['name'],[fam]) if fam else {'target':r['name'],'status':'REWARD_KIND_HAS_NO_IDENTITY_MAPPING','refs':[]}
   if fam=='Outfit' and not hit['refs']:
    # Only the two explicit authored naming grammars; no fuzzy/substring lookup.
    match=re.fullmatch(r'(.+ Outfits): (base|first addon|second addon)',r['name'])
    short=re.fullmatch(r'(.+) (first addon|second addon)',r['name']) if not match else None
    if match or short:
     base,part=(match.group(1),match.group(2)) if match else (short.group(1)+' Outfits',short.group(2))
     base_hit=mapped(base,['Outfit'])
     if base_hit['refs']:
      hit={**base_hit,'target':r['name'],'authored_intent_name_grammar':'EXPLICIT_BASE_OR_ORDINAL_ADDON_SUFFIX','base_outfit_declared_name':base,'requested_component':part,'requested_addon_index':{'first addon':1,'second addon':2}.get(part),'chosen_interpretation_only':True}
   if r['kind']=='none' and r['count']==0:
    hit={'target':r['name'],'status':'EXPLICIT_NO_DELIVERY_INTENT','refs':[],'delivery_intent':'none'}
   elif r['kind']=='experience':
    hit={'target':r['name'],'status':'EXPLICIT_AUTHORED_EXPERIENCE_AMOUNT','refs':[],'delivery_intent':'experience','amount':r['count'],'native_consumer_bound':False}
   holds=['NATIVE_REWARD_DELIVERY_CONSUMER_BINDING_PENDING']
   if hit['status'] not in ('EXACT_CANONICAL_IDENTITY_ASSOCIATION','EXPLICIT_NO_DELIVERY_INTENT','EXPLICIT_AUTHORED_EXPERIENCE_AMOUNT'):holds+=['REWARD_EXACT_IDENTITY_UNRESOLVED']
   if fam=='Area':holds+=['AREA_IDENTITY_IS_NOT_ACCESS_CAPABILITY_GATE']
   if hit['status']=='EXPLICIT_NO_DELIVERY_INTENT':holds=[]
   if fam=='Item' and hit.get('materializable') is not True:holds+=['CANONICAL_ITEM_NOT_MATERIALIZABLE']
   if fam=='Outfit':holds+=['BASE_OUTFIT_OR_ADDON_DELIVERY_REQUIRES_EXPLICIT_NATIVE_POLICY']
   rewards.append({'reward_index':i,'kind':r['kind'],'count':r['count'],'basis':r['basis'],'mapping':hit,'unresolved':holds,'runtime_admitted':False})
  records.append({'quest_ref':{'family':'Quest',**q['identity']},'wiki_title':recipe['wiki_title'],'recipe_sha256':semantic(recipe),'canonical_recipe_sha256':semantic(q['recipe']),'canonical_definition_evidence':entry['evidence'],'identity_binding_basis':'EXACT_AUTHORING_IDENTITY_AND_CANONICAL_RECIPE_PAYLOAD','stages':stages,'reward_intents':rewards,'native_readiness_unchanged':True})
 counter=collections.Counter()
 for r in records:
  for s in r['stages']:
   counter['stages']+=1;counter['non_dialogue_stages']+=s['kind']!='talk'
   counter['exact_stage_target_refs']+=sum(t['status']=='EXACT_CANONICAL_IDENTITY_ASSOCIATION' for t in s['targets']);counter['encounter_outcome_seams']+=sum(x.get('status')=='EXISTING_DECLARED_ENCOUNTER_OUTCOME' for x in s['consumer_seams'])
  for reward in r['reward_intents']:counter['reward_intents']+=1;counter['exact_reward_refs']+=reward['mapping']['status']=='EXACT_CANONICAL_IDENTITY_ASSOCIATION';counter['explicit_non_identity_reward_intents']+=reward['mapping']['status'] in ('EXPLICIT_NO_DELIVERY_INTENT','EXPLICIT_AUTHORED_EXPERIENCE_AMOUNT')
 return {'schema':'OTERYN_AUTHORED68_EVENT_REWARD_ASSOCIATIONS/v1','epoch':e.pin,'runtime_admitted':False,'basis':'CHOSEN_OTERYN_APPROXIMATION','limits':['Exact canonical name-to-identity association does not prove Source quest ownership, placement or execution.','Source bindings retain their original epochs; no parity between those donor revisions and quest baseline is claimed.','Talk identity rows are informational and remain owned by the dialogue lane.','No fuzzy names, slug guesses, donor numeric IDs or implicit addon grants.'],'input_refs':{'recipe_path':RECIPE,'recipe_sha256':sha(raw),'qualified_world_sha256':sha(wb)},'canonical_inputs':e.inputs,'counts':{'quests':len(records),**counter},'records':records}
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--repo-root',required=True);p.add_argument('--epoch-root',required=True);p.add_argument('--qualified-world',required=True);p.add_argument('--out',required=True);a=p.parse_args();packet=build(a.repo_root,a.epoch_root,a.qualified_world);Path(a.out).write_text(json.dumps(packet,indent=2,ensure_ascii=False)+'\n');print(json.dumps(packet['counts']))
