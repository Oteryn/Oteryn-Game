"""Offline identity associations for 139 chosen-source Quest recipes. No runtime admission."""
import argparse,collections,hashlib,json,re,subprocess,unicodedata
from pathlib import Path
PIN="ad7a08f96caa4e7bd0e7fa90c67b39229d637277"
PROFILE="chosen_source_completion_v1"
CANDIDATE="content/quests/missions/quest-state-completion-candidate.json"
WORLD_SHA="e20b2e849ce03e00acab7d38196610ab1ba05db18f057d073450b83fcb51fe4b"
def sha(raw):return hashlib.sha256(raw).hexdigest()
def semantic(x):return sha(json.dumps(x,sort_keys=True,separators=(",",":"),ensure_ascii=False).encode())
def norm(s):return " ".join(unicodedata.normalize("NFC",s).casefold().split())
def resolve(table,families,name):
 rows={semantic(x["ref"]):x for family in families for x in table.get((family,norm(name)),[])}
 return list(rows.values())
class Epoch:
 def __init__(self,root):
  self.root=Path(root);self.pin=self.git("rev-parse","HEAD").decode().strip();assert self.pin==PIN,("wrong epoch",self.pin)
  self.paths=self.git("ls-tree","-r","--name-only",self.pin,"content").decode().splitlines();self.inputs=[]
 def git(self,*args):return subprocess.check_output(["git","-C",str(self.root),*args])
 def read(self,path):
  raw=self.git("show",self.pin+":"+path);self.inputs.append({"path":path,"sha256":sha(raw),"epoch":self.pin});return json.loads(raw)
 def evidence(self,path,pointer,record):
  return {"path":path,"json_pointer":pointer,"record_sha256":semantic(record),"file_sha256":next(x["sha256"] for x in self.inputs if x["path"]==path),"epoch":self.pin}
def current_defs(root):
 out=[]
 for p in sorted((Path(root)/"content/quests/definitions").glob("quests-*.json")):
  raw=p.read_bytes();psha=sha(raw);rel=str(p.relative_to(root)).replace("\\","/")
  for i,row in enumerate(json.loads(raw)["records"]):
   q=row["definition"];w=q.get("oteryn_recipe") or {};payload=w.get("payload") or {};recipe=payload.get("recipe")
   if w.get("profile")==PROFILE and w.get("chosen_data_complete") and isinstance(recipe,dict):
    out.append({"definition":q,"recipe":recipe,"evidence":{"path":rel,"json_pointer":f"/records/{i}/definition","file_sha256":psha,"record_sha256":semantic(q)}})
 return out
def build(repo_root,epoch_root,world_path):
 e=Epoch(epoch_root);root=Path(repo_root)
 cand=json.loads((root/CANDIDATE).read_text(encoding="utf-8"))
 chosen={q["quest"] for q in cand["quests"] if isinstance(q["completion"],dict) and q["completion"].get("state")=="CHOSEN_SOURCE_TYPED_PROGRESS_ONLY"};assert len(chosen)==139
 table=collections.defaultdict(list);outcomes=collections.defaultdict(list)
 def add(family,name,identity,ev,record,basis="canonical_named_record"):
  if not name:return
  ref={"family":family,**identity};table[(family,norm(name))].append({"ref":ref,"declared_name":name,"identity_basis":basis,"evidence":[ev],"source_bindings":record.get("source_bindings",[]),"materializable":record.get("definition",{}).get("materializable"),"stack_class":record.get("definition",{}).get("stack_class")})
 prefixes=("content/items/definitions/items-","content/creatures/definitions/creatures-","content/cosmetics/mounts/mounts-","content/achievements/achievements-","content/world/areas/hunting-places/hunting-","content/world/areas/regions/areas-","content/encounters/definitions/encounters-","content/npcs/definitions/npcs-")
 for path in e.paths:
  if not path.endswith(".json") or not path.startswith(prefixes):continue
  d=e.read(path)
  for i,r in enumerate(d.get("records",d.get("areas",[]))):
   ev=e.evidence(path,("/records/" if "records" in d else "/areas/")+str(i),r);de=r.get("definition",r.get("declaration",r));ident=de.get("identity",{});fam=de.get("kind",ident.get("family",d.get("family")))
   if not ident:continue
   if fam=="Item":
    n=de.get("semantics",{}).get("presentation",{}).get("value",{}).get("name",{});name=n.get("value") if n.get("state")=="KNOWN" else None
   elif fam=="Creature":name=r.get("authoring",{}).get("profile",{}).get("details",{}).get("display_name")
   elif fam=="Mount":name=r.get("editor",{}).get("display_name")
   elif fam=="Achievement":name=r.get("name")
   elif fam in ("Area","region","city"):fam="Area";name=de.get("name")
   else:name=r.get("editor",{}).get("display_name")
   if fam=="Encounter":
    details=r.get("authoring",{}).get("profile",{}).get("details",{});roles={p["role"]:p.get("creatures",[]) for p in details.get("participants",[])}
    for j,rule in enumerate(details.get("rules",[])):
     trigger=rule.get("trigger",{})
     if trigger.get("kind")!="creature_died":continue
     for action in rule.get("actions",[]):
      if action.get("kind")=="emit_outcome":
       for cr in roles.get(trigger.get("role"),[]):outcomes[cr["key"]].append({"encounter_ref":{"family":"Encounter",**ident},"outcome":action["outcome"],"credit_policy":action.get("credited"),"rule_key":rule["key"],"conditions":rule.get("conditions",[]),"evidence":ev,"rule_json_pointer":ev["json_pointer"]+f"/authoring/profile/details/rules/{j}"})
   add(fam,name,ident,ev,r)
 path="content/world/definitions/declarations.json";d=e.read(path);decl={(x.get("kind"),x["identity"]["key"],x["identity"]["revision"]):(i,x) for i,x in enumerate(d["records"]) if "identity" in x}
 path2="content/world/editor/author.json";ed=e.read(path2)
 for i,x in enumerate(ed["entries"]):
  ref=x["target"];key=(ref["family"],ref["key"],ref["revision"])
  if key not in decl or ref["family"] not in ("Outfit","NPC"):continue
  j,r=decl[key];add(ref["family"],x.get("display_name"),r["identity"],e.evidence(path,f"/records/{j}",r),r,"canonical_editor_to_declared_complete_identity");table[(ref["family"],norm(x.get("display_name","")))][-1]["evidence"].append(e.evidence(path2,f"/entries/{i}",x))
 wb=Path(world_path).read_bytes();assert sha(wb)==WORLD_SHA,"qualified world snapshot drift";world=json.loads(wb);world_ids={(x.get("kind"),x["identity"]["key"],x["identity"]["revision"]) for x in world["records"] if "identity" in x}
 def mapped(name,families):
  hits=resolve(table,families,name)
  if len(hits)!=1:return {"target":name,"status":"AMBIGUOUS_CANONICAL_NAME" if hits else "CANONICAL_EXACT_NAME_NOT_FOUND","refs":[],"match_count":len(hits)}
  h=hits[0];ref=h["ref"];return {"target":name,"status":"EXACT_CANONICAL_IDENTITY_ASSOCIATION","refs":[ref],"declared_name":h["declared_name"],"identity_basis":h["identity_basis"],"evidence":h["evidence"],"source_bindings":h["source_bindings"],"qualified_world_presence":(ref["family"],ref["key"],ref["revision"]) in world_ids,"materializable":h["materializable"],"stack_class":h["stack_class"]}
 defs={x["definition"]["identity"]["key"]:x for x in current_defs(root)};assert chosen<=set(defs)
 records=[]
 for key in sorted(chosen):
  entry=defs[key];q=entry["definition"];recipe=entry["recipe"];stages=[]
  for s in recipe["stages"]:
   kind=s["kind"];families={"kill":["Creature"],"collect":["Item"],"use":["Item","NPC"],"explore":["Area"],"complete":[],"talk":["NPC"]}[kind];targets=[mapped(n,families) for n in s["targets"]] if families else [];seam=[];holds=[]
   if kind=="kill":
    for t in targets:
     for ref in t["refs"]:
      matches=outcomes.get(ref["key"],[])
      if len(matches)==1:seam.append({"target":t["target"],**matches[0],"status":"EXISTING_DECLARED_ENCOUNTER_OUTCOME","execution_verified":False})
      elif len(matches)>1:holds.append("MULTIPLE_ENCOUNTER_OUTCOMES_REQUIRE_EXPLICIT_SELECTION")
    holds.append("CHOSEN_STAGE_KILL_EVENT_AND_CREDIT_BINDING_PENDING")
   elif kind=="collect":holds.append("INVENTORY_COUNT_CONSUMER_AND_PER_TARGET_QUANTITY_BINDING_PENDING")
   elif kind=="use":holds+=["ITEM_NPC_IDENTITY_IS_NOT_USE_ACTION_OR_PLACEMENT_BINDING","NATIVE_USE_CONSUMER_BINDING_PENDING"]
   elif kind=="explore":holds+=["AREA_LABEL_OR_POSITION_IS_NOT_QUEST_TRIGGER_BOUNDARY","NATIVE_AREA_ENTRY_CONSUMER_BINDING_PENDING"]
   elif kind=="complete":seam=[{"kind":"EXISTING_CHOSEN_STAGE_GRAPH","incoming_stage_keys":[x["key"] for x in recipe["stages"] if s["key"] in x["next"]],"next_stage_keys":s["next"],"status":"CHOSEN_SOURCE_GRAPH_INTENT_ONLY","execution_verified":False}];holds.append("NATIVE_COMPLETION_REDUCER_BINDING_PENDING")
   else:holds.append("DIALOGUE_LANE_OWNS_TALK_BINDING")
   if any(t["status"]!="EXACT_CANONICAL_IDENTITY_ASSOCIATION" for t in targets):holds.append("ONE_OR_MORE_EXACT_TARGET_IDENTITIES_UNRESOLVED")
   stages.append({"stage_key":s["key"],"kind":kind,"count":s["count"],"basis":s["basis"],"targets":targets,"consumer_seams":seam,"unresolved":sorted(set(holds)),"runtime_admitted":False})
  rewards=[]
  for i,r in enumerate(recipe["reward_intents"]):
   fam={"item":"Item","achievement":"Achievement","outfit":"Outfit","mount":"Mount","access":"Area"}.get(r["kind"]);hit=mapped(r["name"],[fam]) if fam else {"target":r["name"],"status":"REWARD_KIND_HAS_NO_IDENTITY_MAPPING","refs":[]}
   if fam=="Outfit" and not hit["refs"]:
    match=re.fullmatch(r"(.+ Outfits): (base|first addon|second addon)",r["name"]);short=re.fullmatch(r"(.+) (first addon|second addon)",r["name"]) if not match else None
    if match or short:
     base,part=(match.group(1),match.group(2)) if match else (short.group(1)+" Outfits",short.group(2));base_hit=mapped(base,["Outfit"])
     if base_hit["refs"]:hit={**base_hit,"target":r["name"],"authored_intent_name_grammar":"EXPLICIT_BASE_OR_ORDINAL_ADDON_SUFFIX","base_outfit_declared_name":base,"requested_component":part,"requested_addon_index":{"first addon":1,"second addon":2}.get(part),"chosen_interpretation_only":True}
   if r["kind"]=="none" and r["count"]==0:hit={"target":r["name"],"status":"EXPLICIT_NO_DELIVERY_INTENT","refs":[],"delivery_intent":"none"}
   elif r["kind"]=="experience":hit={"target":r["name"],"status":"EXPLICIT_AUTHORED_EXPERIENCE_AMOUNT","refs":[],"delivery_intent":"experience","amount":r["count"],"native_consumer_bound":False}
   holds=["NATIVE_REWARD_DELIVERY_CONSUMER_BINDING_PENDING"]
   if hit["status"] not in ("EXACT_CANONICAL_IDENTITY_ASSOCIATION","EXPLICIT_NO_DELIVERY_INTENT","EXPLICIT_AUTHORED_EXPERIENCE_AMOUNT"):holds.append("REWARD_EXACT_IDENTITY_UNRESOLVED")
   if fam=="Area":holds.append("AREA_IDENTITY_IS_NOT_ACCESS_CAPABILITY_GATE")
   if hit["status"]=="EXPLICIT_NO_DELIVERY_INTENT":holds=[]
   if fam=="Item" and hit.get("materializable") is not True:holds.append("CANONICAL_ITEM_NOT_MATERIALIZABLE")
   if fam=="Outfit":holds.append("BASE_OUTFIT_OR_ADDON_DELIVERY_REQUIRES_EXPLICIT_NATIVE_POLICY")
   rewards.append({"reward_index":i,"kind":r["kind"],"count":r["count"],"basis":r["basis"],"mapping":hit,"unresolved":holds,"runtime_admitted":False})
  records.append({"quest_ref":{"family":"Quest",**q["identity"]},"wiki_title":recipe["wiki_title"],"recipe_sha256":semantic(recipe),"canonical_definition_evidence":entry["evidence"],"identity_binding_basis":"EXACT_CANONICAL_QUEST_KEY_AND_CHOSEN_RECIPE_PAYLOAD","stages":stages,"reward_intents":rewards,"native_readiness_unchanged":True})
 counter=collections.Counter()
 for r in records:
  for s in r["stages"]:counter["stages"]+=1;counter["non_dialogue_stages"]+=s["kind"]!="talk";counter["exact_stage_target_refs"]+=sum(t["status"]=="EXACT_CANONICAL_IDENTITY_ASSOCIATION" for t in s["targets"]);counter["encounter_outcome_seams"]+=sum(x.get("status")=="EXISTING_DECLARED_ENCOUNTER_OUTCOME" for x in s["consumer_seams"])
  for reward in r["reward_intents"]:counter["reward_intents"]+=1;counter["exact_reward_refs"]+=reward["mapping"]["status"]=="EXACT_CANONICAL_IDENTITY_ASSOCIATION";counter["explicit_non_identity_reward_intents"]+=reward["mapping"]["status"] in ("EXPLICIT_NO_DELIVERY_INTENT","EXPLICIT_AUTHORED_EXPERIENCE_AMOUNT")
 return {"schema":"OTERYN_CHOSEN_SOURCE139_EVENT_REWARD_ASSOCIATIONS/v1","epoch":e.pin,"runtime_admitted":False,"basis":"CHOSEN_OTERYN_APPROXIMATION","limits":["Exact canonical name-to-identity association does not prove Source quest ownership, placement or execution.","Chosen-source recipes remain approximations; original source holds are preserved.","Talk identity rows remain owned by the dialogue lane.","No fuzzy names, donor numeric IDs or implicit addon grants."],"input_refs":{"completion_candidate":CANDIDATE,"qualified_world_sha256":sha(wb)},"canonical_inputs":e.inputs,"counts":{"quests":len(records),**counter},"records":records}
if __name__=="__main__":
 p=argparse.ArgumentParser();p.add_argument("--repo-root",required=True);p.add_argument("--epoch-root",required=True);p.add_argument("--qualified-world",required=True);p.add_argument("--out",required=True);a=p.parse_args();packet=build(a.repo_root,a.epoch_root,a.qualified_world);Path(a.out).write_text(json.dumps(packet,indent=2,ensure_ascii=False)+"\n",encoding="utf-8");print(json.dumps(packet["counts"],sort_keys=True))
