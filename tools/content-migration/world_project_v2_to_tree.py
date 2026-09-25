#!/usr/bin/env python3
from __future__ import annotations
import json
from pathlib import Path
R=Path(__file__).resolve().parents[2];L=R/"content/world";S=500
def cb(v):return (json.dumps(v,ensure_ascii=False,sort_keys=True,separators=(",",":"))+"\n").encode()
def load(p):return json.loads(Path(p).read_text(encoding="utf-8"))
def wr(rel,v):p=R/rel;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(cb(v))
def tid(t):return(t["family"],t["key"],t["revision"])
def main():
 ref=load(L/"definitions/reference.json");dec=load(L/"definitions/declarations.json");ed=load(L/"editor/author.json");src=load(L/"provenance/sources.json");imp=load(L/"provenance/imports.json")
 em={tid(x["target"]):x for x in ed["entries"]};bm={}
 for x in src["source_identity_bindings"]:bm.setdefault(tid(x["target"]),[]).append(x)
 for a in bm.values():a.sort(key=cb)
 paths=[]
 for start in range(0,len(ref["records"]),S):
  rows=[]
  for d in ref["records"][start:start+S]:
   x={"definition":d};k=tid(d["identity"])
   if k in em:x["editor"]=em[k]
   if k in bm:x["source_bindings"]=bm[k]
   rows.append(x)
  end=start+len(rows)-1;rel=f"content/items/definitions/items-{start:05d}-{end:05d}.json";paths.append(rel);wr(rel,{"schema":"OTERYN_ITEM_AUTHORING_SHARD/v1","family":"Item","source_legacy_role":"content/world/definitions/reference.json","shard":{"index":start//S,"start":start,"end":end,"count":len(rows)},"records":rows})
 mr=[]
 for d in dec["records"]:
  t={"family":"Mount","key":d["identity"]["key"],"revision":d["identity"]["revision"]};x={"declaration":d};k=tid(t)
  if k in em:x["editor"]=em[k]
  if k in bm:x["source_bindings"]=bm[k]
  mr.append(x)
 mrel="content/cosmetics/mounts/mounts-00000-00251.json";wr(mrel,{"schema":"OTERYN_MOUNT_AUTHORING_SHARD/v1","family":"Mount","source_legacy_role":"content/world/definitions/declarations.json","shard":{"index":0,"start":0,"end":251,"count":252},"records":mr})
 ib=[x for x in src["source_identity_bindings"] if x["target"]["family"]=="Item"];mb=[x for x in src["source_identity_bindings"] if x["target"]["family"]=="Mount"]
 outs={"imports/crystalserver/sources.json":{"schema":"OTERYN_IMPORT_SOURCES/v1","sources":[x for x in src["sources"] if x["key"]=="oteryn:source.crystalserver"]},"imports/crystalserver/batches.json":{"schema":"OTERYN_IMPORT_BATCHES/v1","batches":[x for x in imp["batches"] if x["source_repository"]=="zimbadev/crystalserver"]},"imports/tibiawiki/sources.json":{"schema":"OTERYN_IMPORT_SOURCES/v1","sources":[x for x in src["sources"] if x["key"]=="oteryn:source.tibiawiki"]},"imports/tibiawiki/batches.json":{"schema":"OTERYN_IMPORT_BATCHES/v1","batches":[x for x in imp["batches"] if x["source_repository"]=="tibiawiki.com.br"]},"imports/tibiawiki/bindings/items.json":{"schema":"OTERYN_SOURCE_IDENTITY_BINDINGS/v1","family":"Item","bindings":ib},"imports/tibiawiki/bindings/mounts.json":{"schema":"OTERYN_SOURCE_IDENTITY_BINDINGS/v1","family":"Mount","bindings":mb}}
 for p,v in outs.items():wr(p,v)
 wr("content/items/index.json",{"schema":"OTERYN_FAMILY_INDEX/v1","family":"Item","record_count":len(ref["records"]),"shard_size":S,"shards":paths,"legacy_source":{"path":"content/world/definitions/reference.json","schema":ref["schema"],"world_id":ref["world_id"],"coordinate_frame":ref["coordinate_frame"]},"attached_editor_entries":sum(x["target"]["family"]=="Item" for x in ed["entries"]),"attached_source_bindings":len(ib)})
 wr("content/cosmetics/mounts/index.json",{"schema":"OTERYN_FAMILY_INDEX/v1","family":"Mount","record_count":252,"shards":[mrel],"legacy_source":{"path":"content/world/definitions/declarations.json","schema":dec["schema"]},"attached_editor_entries":sum(x["target"]["family"]=="Mount" for x in ed["entries"]),"attached_source_bindings":len(mb)})
 print(f"PASS items={len(ref['records'])} item_shards={len(paths)} mounts={len(mr)}")
if __name__=="__main__":raise SystemExit(main())
