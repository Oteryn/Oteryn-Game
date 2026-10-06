"""Fail-closed classification of chosen-source terminal count>1 holds."""
import argparse,hashlib,json
from pathlib import Path
SCHEMA="OTERYN_QUEST_TERMINAL_COUNT_SEMANTICS_AUDIT/v1"
IDX="content/quests/definitions/index.json"
REW="tools/content-schema/quest-authoring/samples/binding_packets/rewards/perquest-352.json"
CHESTS="tools/content-schema/quest-authoring/samples/chests/claims.json"
OUT="tools/content-schema/quest-authoring/samples/server-completion/terminal-count-semantics/audit.json"
PROFILE="chosen_source_completion_v1"
EXPECTED={"oteryn:quest.barbarian_arena_quest","oteryn:quest.bear_room_quest","oteryn:quest.behemoth_quest","oteryn:quest.demon_helmet_quest","oteryn:quest.dragon_tower_quest","oteryn:quest.edron_goblin_quest","oteryn:quest.opticording_sphere_quest","oteryn:quest.rift_warrior_outfits_quest","oteryn:quest.the_ancient_tombs_quest"}
def read(root,p): return json.loads((root/p).read_text(encoding="utf-8"))
def sha(root,p): return hashlib.sha256((root/p).read_bytes()).hexdigest()
def canon(k):
 if not k.startswith("canary:reward-claim/"): raise ValueError("non-Canary claim")
 return "oteryn:reward-claim."+k.split("/",1)[1].replace("/",".")
def defs(root):
 i=read(root,IDX); return i,[r["definition"] for p in i["shards"] for r in read(root,p)["records"]]
def reward_rows(root):
 o=read(root,REW); rows=o.get("records") or o.get("quests") or []
 return {(r.get("quest") or {}).get("key",r.get("quest_key")):r for r in rows}
def claim_rows(root):
 return {canon(r["identity"]["key"]):r for r in read(root,CHESTS)["claims"] if r["identity"]["key"].startswith("canary:reward-claim/")}
def held(d):
 w=d.get("oteryn_recipe") or {}; stages=((w.get("payload") or {}).get("recipe") or {}).get("stages") or []
 return w.get("profile")==PROFILE and w.get("chosen_data_complete") and stages and stages[-1].get("kind")=="complete" and stages[-1].get("count",0)>1
def classify(n,c):
 if c==n and c>0:return "EXACT_CLAIM_SET_CARDINALITY_CANDIDATE","ARCH_REVIEW_COUNTED_TERMINAL_EVENT_SET"
 if c>0:return "CLAIM_SET_CARDINALITY_MISMATCH","SOURCE_REVIEW_CLAIM_SUBSET_OR_NONCLAIM_TERMINAL"
 return "NO_EXACT_REWARD_CLAIM_SET","RECIPE_SEMANTICS_REVIEW_QUANTITY_OR_NONCLAIM_EVENT"
def build(root):
 root=Path(root); idx,all_defs=defs(root); rewards=reward_rows(root); claims=claim_rows(root)
 selected=[d for d in all_defs if held(d)]
 keys={d["identity"]["key"] for d in selected}
 if keys!=EXPECTED: raise ValueError("terminal-count hold set changed: "+repr(sorted(keys^EXPECTED)))
 records=[]
 for d in sorted(selected,key=lambda x:x["identity"]["key"]):
  key=d["identity"]["key"]; recipe=d["oteryn_recipe"]["payload"]["recipe"]; terminal=recipe["stages"][-1]
  refs=(rewards.get(key) or {}).get("claim_refs") or []; src=[]
  for ref in refs:
   row=claims.get(ref["key"])
   src.append({"ref":ref,"source_claim_found":row is not None,"repeat":(row or {}).get("claim",{}).get("repeat"),"progress_write":(row or {}).get("progress_write"),"placement_count":len((row or {}).get("placements") or []),"positions":[p.get("position") for p in (row or {}).get("placements") or []]})
  cls,nxt=classify(terminal["count"],len(refs))
  records.append({"quest":{"key":key,"revision":d["identity"]["revision"],"display_name":d["display_name"]},"terminal_stage":{"key":terminal["key"],"kind":terminal["kind"],"chosen_count":terminal["count"],"objective":terminal["objective"],"targets":terminal["targets"],"basis":terminal["basis"]},"claim_set":{"count":len(refs),"refs":refs,"all_source_claims_found":all(x["source_claim_found"] for x in src),"source_claims":src},"classification":cls,"recommended_next":nxt,"runtime_admitted":False,"native_dispatch_binding":None,"source_equivalence":False})
 counts={}
 for r in records: counts[r["classification"]]=counts.get(r["classification"],0)+1
 expected={"EXACT_CLAIM_SET_CARDINALITY_CANDIDATE":4,"CLAIM_SET_CARDINALITY_MISMATCH":2,"NO_EXACT_REWARD_CLAIM_SET":3}
 if counts!=expected: raise ValueError("classification changed: "+repr(counts))
 if any(r["claim_set"]["count"] and not r["claim_set"]["all_source_claims_found"] for r in records): raise ValueError("retained RewardClaim ref missing")
 return {"schema":SCHEMA,"classification":"DERIVED_FAIL_CLOSED_AUDIT_NOT_RUNTIME_AUTHORITY","runtime_enabled":False,"inputs":{"definitions":{"path":IDX,"record_count":idx["record_count"],"sha256":sha(root,IDX)},"reward_bindings":{"path":REW,"sha256":sha(root,REW)},"source_chests":{"path":CHESTS,"sha256":sha(root,CHESTS)}},"summary":{"held_quests":len(records),"classifications":counts,"runtime_admitted":0},"holds":["Cardinality equality is a bounded candidate signal, not proof that every claim is a terminal occurrence.","No automatic counted-terminal reducer or event selector is admitted.","Cardinality mismatches require exact source review before selecting any subset.","No-claim holds may be quantity/resource semantics or a non-claim event; chosen count is not reinterpreted here."],"records":records}
def main():
 p=argparse.ArgumentParser();p.add_argument("--root",type=Path,required=True);p.add_argument("--check",action="store_true");a=p.parse_args()
 v=build(a.root.resolve()); raw=(json.dumps(v,ensure_ascii=False,sort_keys=True,indent=2)+"\n").encode()
 target=a.root.resolve()/OUT
 if a.check:
  if not target.is_file() or target.read_bytes()!=raw: raise ValueError("terminal-count audit drift")
 else: target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(raw)
 print(json.dumps(v["summary"],sort_keys=True))
if __name__=="__main__": main()
