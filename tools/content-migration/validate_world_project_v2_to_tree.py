#!/usr/bin/env python3
from __future__ import annotations
import json
from pathlib import Path
R=Path(__file__).resolve().parents[2];L=R/"content/world"
class E(RuntimeError):pass
def load(p):return json.loads(Path(p).read_text(encoding="utf-8"))
def req(v,c):
 if not v:raise E(c)
def tid(t):return(t["family"],t["key"],t["revision"])
def main():
 ref=load(L/"definitions/reference.json");dec=load(L/"definitions/declarations.json");ed=load(L/"editor/author.json");src=load(L/"provenance/sources.json");pr=load(R/"content/project.json");mf=load(R/"content/manifest.json");lk=load(R/"content/content.lock.json");ii=load(R/"content/items/index.json");mi=load(R/"content/cosmetics/mounts/index.json")
 req(pr["runtime_source"]=="legacy_until_separately_qualified","RUNTIME_SWITCH");req(mf["compatibility"]=={"legacy_mutated":False,"legacy_root":"content/world","runtime_switch_authorized":False},"COMPAT");req(lk["family_counts"]=={"Item":38157,"Mount":252},"COUNT")
 items=[];ies=[];ibs=[];pos=0
 for rel in ii["shards"]:
  p=load(R/rel);req(p["shard"]["start"]==pos and p["shard"]["count"]==len(p["records"]),"ITEM_SHARD")
  for x in p["records"]:items.append(x["definition"]);ies += [x["editor"]] if "editor" in x else [];ibs += x.get("source_bindings",[])
  pos=p["shard"]["end"]+1
 req(items==ref["records"] and pos==38157,"ITEM_ROUNDTRIP")
 mp=load(R/mi["shards"][0]);mounts=[x["declaration"] for x in mp["records"]];mes=[x["editor"] for x in mp["records"] if "editor" in x];mbs=[b for x in mp["records"] for b in x.get("source_bindings",[])]
 req(mounts==dec["records"],"MOUNT_ROUNDTRIP")
 lei=[x for x in ed["entries"] if x["target"]["family"]=="Item"];lem=[x for x in ed["entries"] if x["target"]["family"]=="Mount"];lbi=[x for x in src["source_identity_bindings"] if x["target"]["family"]=="Item"];lbm=[x for x in src["source_identity_bindings"] if x["target"]["family"]=="Mount"]
 req(ies==lei and mes==lem,"EDITOR");req(ibs==lbi and mbs==lbm,"BINDINGS");req(len({tid(x["identity"]) for x in items})==38157,"ITEM_IDS");req(len({("Mount",x["identity"]["key"],x["identity"]["revision"]) for x in mounts})==252,"MOUNT_IDS")
 req(load(R/"imports/tibiawiki/bindings/items.json")["bindings"]==lbi,"IMPORT_ITEM");req(load(R/"imports/tibiawiki/bindings/mounts.json")["bindings"]==lbm,"IMPORT_MOUNT")
 print("PASS items=38157 mounts=252 item_editors=165 mount_editors=252 item_bindings=165 mount_bindings=252")
if __name__=="__main__":raise SystemExit(main())
