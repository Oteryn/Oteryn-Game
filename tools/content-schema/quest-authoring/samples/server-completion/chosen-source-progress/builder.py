"""Project qualified chosen source-derived Quest recipes into QuestState candidate format.

Chosen Oteryn approximation only: no donor equivalence and no runtime activation.
"""
import argparse, copy, hashlib, json, pathlib, re

BASIS="CHOSEN_OTERYN_APPROXIMATION"
PROFILE="chosen_source_completion_v1"
SOURCE_STATE="content/quests/missions/quest-state.json"
ALLOWED={"talk","kill","use","collect","explore","complete"}

def enc(v): return json.dumps(v,ensure_ascii=False,sort_keys=True,separators=(",",":"))
def digest(v): return hashlib.sha256(enc(v).encode()).hexdigest()
def req(ok,msg):
    if not ok: raise ValueError(msg)
def valid(k): return isinstance(k,str) and len(k.encode())<=128 and bool(re.fullmatch(r"oteryn:[A-Za-z0-9._:/-]+",k))

def recipe_of(q):
    w=q.get("oteryn_recipe") or {}
    if w.get("profile")!=PROFILE or not w.get("chosen_data_complete"): return None
    p=w.get("payload") or {}
    r=p.get("recipe")
    return r if isinstance(r,dict) else None

def validate_recipe(q,r):
    req(valid(q["identity"]["key"]),"Invalid Quest owner")
    stages=r.get("stages") or []; req(stages,"Empty chosen source recipe")
    ids=[s.get("key") for s in stages]; req(len(ids)==len(set(ids)),"Duplicate stage keys")
    for i,s in enumerate(stages):
        req(bool(re.fullmatch(r"s[1-9][0-9]*",str(s.get("key","")))),"Stage key")
        req(type(s.get("count")) is int and 1<=s["count"]<=2**63-1,"Stage count")
        req(s.get("next")==([ids[i+1]] if i+1<len(ids) else []),"Unsupported graph/order")
        req(s.get("kind") in ALLOWED,"Unknown event intent")
        req((s.get("kind")=="complete")==(i==len(stages)-1),"Completion only terminal")
    req(stages[-1]["count"]==1,"Terminal completion count must be one")
    req((r.get("repeat") or {}).get("kind") in {"once","daily"},"Unsupported repeat")

def project(q,ref):
    r=recipe_of(q); req(r is not None,"Not a chosen source recipe"); validate_recipe(q,r)
    owner=q["identity"]["key"]; tail=owner.removeprefix("oteryn:quest."); req(tail!=owner,"Chosen source namespace")
    tracks=[]; trans=[]
    for i,s in enumerate(r["stages"]):
        tk=f"oteryn:quest-progress/chosen-source/{tail}/{s['key']}"
        xk=f"oteryn:quest-transition/chosen-source/{tail}/{s['key']}"
        req(valid(tk) and valid(xk),"Track/transition key budget")
        tracks.append({"key":tk,"quest":owner,"initial":0,"min":0,"max":s["count"],
                       "bounds_basis":"EXPLICIT_CHOSEN_STAGE_OCCURRENCE_COUNT",
                       "source_key":f"oteryn:chosen-source-stage/{tail}/{s['key']}"})
        effects=[]
        if i:
            p=tracks[-2]
            effects.append({"track":p["key"],"from":{"op":"EQ","value":p["max"]},"from_exact":True,
                            "effect":{"kind":"SET","value":p["max"]}})
        effects.append({"track":tk,"from":{"op":"LT","value":s["count"]},"from_exact":True,
                        "effect":{"kind":"ADD","value":1}})
        trans.append({"key":xk,"quest":owner,"completes":i==len(r["stages"])-1,"effects":effects,
                      "requested_by":None,
                      "source":{"basis":BASIS,"chosen_stage":copy.deepcopy(s),
                                "canonical_ref":dict(ref,definition_sha256=digest(q)),
                                "native_dispatch_binding":None,"event_intent_only":True,
                                "runtime_admission":False,"source_equivalence":False}})
    repeat=r["repeat"]["kind"]
    return {"quest":owner,"source_quest":owner,
            "completion":{"basis":BASIS,"state":"CHOSEN_SOURCE_TYPED_PROGRESS_ONLY",
                          "native_admission":False,"runtime_enabled":False,"source_equivalence":False,
                          "event_dispatch_binding":None,"NPC_dialogue_binding":None,
                          "reward_delivery_binding":None,
                          "repeat_lowering":"FIRST_CYCLE_ONLY" if repeat=="once" else "HELD_NO_CYCLE_RESET_BINDING",
                          "canonical_ref":dict(ref,definition_sha256=digest(q)),
                          "recipe_metadata":copy.deepcopy({k:v for k,v in r.items() if k!="stages"}),
                          "counter_assumption":"Explicit chosen occurrence counters only; native dispatch/dialogue/reward bindings remain unresolved."},
            "tracks":tracks,"transitions":trans}

def build(root):
    root=pathlib.Path(root)
    source=json.loads((root/SOURCE_STATE).read_text(encoding="utf-8"))
    excluded={q["quest"] for q in source["quests"]}
    quests=[]; refs=[]; held=[]
    for p in sorted((root/"content/quests/definitions").glob("quests-*.json")):
        raw=p.read_bytes(); psha=hashlib.sha256(raw).hexdigest(); rel=str(p.relative_to(root)).replace("\\","/")
        refs.append({"path":rel,"sha256":psha})
        for i,row in enumerate(json.loads(raw)["records"]):
            q=row["definition"]; r=recipe_of(q)
            if r is None or q["identity"]["key"] in excluded: continue
            ref={"path":rel,"packet_sha256":psha,"json_pointer":f"/records/{i}/definition"}
            try: quests.append(project(q,ref))
            except ValueError as e: held.append({"quest":q["identity"]["key"],"reason":str(e),"canonical_ref":ref})
    quests.sort(key=lambda q:q["quest"]); held.sort(key=lambda h:h["quest"])
    req(len(quests)==139,"Qualified chosen source recipe count changed")
    req(len(held)==7 and all(h["reason"]=="Terminal completion count must be one" for h in held),"Chosen source hold set changed")
    return {"schema":"OTERYN_CHOSEN_SOURCE_QUEST_PROGRESS_IMPORT/v1","basis":BASIS,
            "native_admission":False,"runtime_enabled":False,"authoring_sources":refs,
            "summary":{"quests":len(quests),"tracks":sum(len(q["tracks"]) for q in quests),
                       "transitions":sum(len(q["transitions"]) for q in quests),
                       "completion_transitions":len(quests),
                       "repeat_cycles_held":sum(q["completion"]["repeat_lowering"]=="HELD_NO_CYCLE_RESET_BINDING" for q in quests),
                       "held_quests":len(held),"NPC_bindings":0,"event_dispatch_bindings":0,"reward_delivery_bindings":0},
            "held":held,"quests":quests}

def validate_packet(packet):
    req(packet.get("schema")=="OTERYN_CHOSEN_SOURCE_QUEST_PROGRESS_IMPORT/v1","Packet schema")
    req(packet.get("basis")==BASIS and packet.get("native_admission") is False and packet.get("runtime_enabled") is False,"Packet admission")
    quests=packet.get("quests") or []; held=packet.get("held") or []
    req(len(quests)==139 and len({q["quest"] for q in quests})==139,"All139 unique owners")
    req(len(held)==7 and all(h["reason"]=="Terminal completion count must be one" for h in held),"All7 holds retained")
    for q in quests:
        req(valid(q["quest"]),"Quest key")
        c=q["completion"]
        req(c["state"]=="CHOSEN_SOURCE_TYPED_PROGRESS_ONLY" and c["runtime_enabled"] is False and c["source_equivalence"] is False,"No promotion")
        tracks={t["key"]:t for t in q["tracks"]}
        req(len(tracks)==len(q["tracks"]),"Unique tracks")
        for t in q["transitions"]:
            req(t["quest"]==q["quest"] and t["requested_by"] is None,"Owned transition")
            req(t["source"]["native_dispatch_binding"] is None and t["source"]["runtime_admission"] is False and t["source"]["source_equivalence"] is False,"No binding promotion")
            req(1<=len(t["effects"])<=8,"Effect budget")
            for e in t["effects"]:
                req(e["track"] in tracks and e["from_exact"] is True,"Closed exact effect")

def merge(source,packet):
    validate_packet(packet)
    out=copy.deepcopy(source); old={q["quest"] for q in out["quests"]}; inc={q["quest"] for q in packet["quests"]}
    req(not old&inc,"Duplicate existing owner")
    out["quests"]=sorted(out["quests"]+copy.deepcopy(packet["quests"]),key=lambda q:q["quest"])
    ts=[t for q in out["quests"] for t in q["transitions"]]
    effects=[e["effect"]["kind"] for t in ts for e in t["effects"]]
    out["counts"]={"quests":len(out["quests"]),"tracks":sum(len(q["tracks"]) for q in out["quests"]),
                   "transitions":len(ts),"effects":{k:effects.count(k) for k in sorted(set(effects))},
                   "inexact_from":sum(any(not e["from_exact"] for e in t["effects"]) for t in ts),
                   "requested_by":sum(t["requested_by"] is not None for t in ts),
                   "completes":sum(t["completes"] for t in ts),"completion":{}}
    for q in out["quests"]:
        st=q["completion"] if isinstance(q["completion"],str) else q["completion"]["state"]
        out["counts"]["completion"][st]=out["counts"]["completion"].get(st,0)+1
    out["authoring_sources"]=copy.deepcopy(source.get("authoring_sources",[]))+copy.deepcopy(packet["authoring_sources"])
    return out

def main():
    p=argparse.ArgumentParser(); p.add_argument("--repo-root",type=pathlib.Path,required=True)
    p.add_argument("--out",type=pathlib.Path,required=True); p.add_argument("--source-state",type=pathlib.Path); p.add_argument("--check",action="store_true")
    a=p.parse_args(); packet=build(a.repo_root)
    value=merge(json.loads(a.source_state.read_text(encoding="utf-8")),packet) if a.source_state else packet
    text=json.dumps(value,ensure_ascii=False,sort_keys=True,indent=2)+"\n"
    if a.check: req(a.out.exists() and a.out.read_text(encoding="utf-8")==text,"Generated candidate differs")
    else: a.out.parent.mkdir(parents=True,exist_ok=True); a.out.write_text(text,encoding="utf-8")
    print(value.get("summary",value.get("counts")))

if __name__=="__main__": main()
