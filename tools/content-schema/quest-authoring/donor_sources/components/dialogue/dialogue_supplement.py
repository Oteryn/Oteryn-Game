"""Pinned AST-derived dialogue supplement; no Lua execution or native admission."""
import argparse, base64, gzip, hashlib, json, tarfile
from pathlib import Path
PINS = {"canary":"04b83b512114bfd888000d6e1433ed8ecaec7c5b", "crystalserver":"9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"}
FILES = {"tereban.lua", "tereban_functions.lua", "alesar.lua", "alesar_functions.lua", "captain_haba.lua", "captain_haba_open_sea.lua"}
def digest(b): return hashlib.sha256(b).hexdigest()
def stable(x): return json.dumps(x, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
def walk(x, path="$", guards=()):
    if isinstance(x, dict):
        if "node_type" in x: yield path, x, guards
        for k,v in x.items():
            g=guards
            if x.get("node_type") in ("If","ElseIf") and k=="fields":
                for field,value in v.items():
                    gg=guards
                    if field=="body": gg=guards+(("TRUE",x["fields"]["test"]),)
                    if field=="orelse": gg=guards+(("FALSE",x["fields"]["test"]),)
                    yield from walk(value,path+".fields."+field,gg)
            else: yield from walk(v,path+"."+k,g)
    elif isinstance(x,list):
        for i,v in enumerate(x): yield from walk(v,path+"["+str(i)+"]",guards)
def name(n):
    if not isinstance(n,dict): return None
    t=n.get("node_type"); f=n.get("fields",{})
    if t=="Name": return f["id"]
    if t=="Index" and f.get("notation",{}).get("name")=="DOT":
        a,b=name(f["value"]),name(f["idx"])
        return a+"."+b if a and b else None
    return None
def literals(n):
    return [base64.b64decode(v["fields"]["s"]["bytes_base64"]).decode("utf-8") for _,v,_ in walk(n) if v.get("node_type")=="String"]
def witness(row, path, node):
    return {"source":row["source"],"revision":row["revision"],"path":row["path"],"source_sha256":row["sha256"],"git_blob_sha1":row["git_blob_sha1"],"ast_path":path,"node_sha256":digest(stable(node)),"span":node.get("span")}
def expression(n):
    # Whole typed expression, including opaque operators, retained without evaluation.
    return {"ast_type":n.get("node_type"),"structure":n,"resolved_runtime_value":False}
def event(node):
    t=node.get("node_type"); f=node.get("fields",{})
    if t=="Invoke":
        method=name(f["func"]); receiver=name(f["source"])
        typ={"say":"Utterance","setMessage":"MessageTemplate","setStorageValue":"StorageWrite","removeItem":"ItemRemoval","addItem":"ItemGrant","setTopic":"TopicWrite","releaseFocus":"ReleaseFocus"}.get(method,"OpaqueMethod")
        return {"type":typ,"receiver":receiver,"method":method,"arguments":f["args"],"literal_utterances":literals(f["args"][0]) if typ in ("Utterance","MessageTemplate") and f["args"] else [],"receiver_binding":"PARAMETER_OR_LOCAL_EXPRESSION_NOT_NATIVE_ACTOR"}
    if t in ("Assign","LocalAssign"):
        return {"type":"StateOrLocalAssignment","targets":f["targets"],"values":f["values"]}
    if t in ("Return","Break"):
        return {"type":"ControlTransfer","ast_type":t,"structure":node}
    if t=="Call": return {"type":"Call","callee":name(f["func"]),"arguments":f["args"]}
    return None
def project(row, ast):
    events=[]; branches=[]; functions=[]
    for path,node,guards in walk(ast):
        t=node.get("node_type"); f=node.get("fields",{})
        if t in ("Function","LocalFunction"):
            functions.append({"symbol":name(f["name"]),"scope":"GLOBAL_DECLARATION" if t=="Function" else "LOCAL_DECLARATION","parameters":[name(a) for a in f["args"]],"witness":witness(row,path,node)})
        if t in ("If","ElseIf"):
            branches.append({"witness":witness(row,path,node),"test":expression(f["test"]),"outer_guards":[{"branch":b,"test":expression(v)} for b,v in guards],"true_body_path":path+".fields.body","false_body_path":path+".fields.orelse"})
        e=event(node)
        if e:
            events.append(e|{"witness":witness(row,path,node),"preorder":len(events),"guards":[{"branch":b,"test":expression(v)} for b,v in guards],"evaluation_role":"CONDITION_EXPRESSION" if ".fields.test" in path else "STATEMENT_OR_NESTED_EXPRESSION", "execution_order":"SOURCE_OFFSET_ORDER_WITH_BRANCH_GUARDS_NOT_FLATTENED_RUNTIME_SEQUENCE"})
    events.sort(key=lambda e: (e["witness"]["span"]["start_char"] if e["witness"]["span"] else 10**20, e["witness"]["ast_path"]))
    for i,e in enumerate(events): e["preorder"]=i
    return {"functions":functions,"branches":branches,"events":events}
def helper_links(projects):
    result=[]
    for row,ast,proj in projects:
        # conservative whole-file local-shadow detection deliberately rejects possible scoped candidates
        local_names=set()
        for _,n,_ in walk(ast):
            f=n.get("fields",{})
            if n.get("node_type")=="LocalAssign": local_names.update(filter(None,(name(t) for t in f["targets"])))
            if n.get("node_type")=="LocalFunction": local_names.add(name(f["name"]))
            if n.get("node_type") in ("Function","LocalFunction","AnonymousFunction"): local_names.update(filter(None,(name(a) for a in f["args"])))
        for ev in proj["events"]:
            if ev["type"]!="Call" or not ev["callee"] or not ev["callee"].startswith(("ParseTereban","ClearTereban","ParseAlesar")): continue
            symbol=ev["callee"]
            candidates=[fun for rr,_,pp in projects if rr["source"]==row["source"] and rr["revision"]==row["revision"] and rr["path"].split("/")[0]==row["path"].split("/")[0] for fun in pp["functions"] if fun["scope"]=="GLOBAL_DECLARATION" and fun["symbol"]==symbol]
            status="LOCAL_SHADOW_PRESERVED" if symbol in local_names else "UNIQUE_SAME_DATAPACK_GLOBAL_DECLARATION_CANDIDATE" if len(candidates)==1 else "AMBIGUOUS_OR_MISSING_GLOBAL_DECLARATION"
            bindings=[]
            if status=="UNIQUE_SAME_DATAPACK_GLOBAL_DECLARATION_CANDIDATE":
                bindings=[{"formal":formal,"actual":actual} for formal,actual in zip(candidates[0]["parameters"],ev["arguments"])]
            result.append({"symbol":symbol,"call":ev["witness"],"definitions":candidates,"status":status,"parameter_bindings":bindings,"definition_search_scope":"BOUNDED_SELECTED_FILES_SAME_DATAPACK", "load_execution":"NOT_PROVEN","native_actor_binding":False})
    return result
def load_ast(ast_root, sha, index, containers=None):
    row=next(r for r in index["captures"] if r["sha256"]==sha)
    direct=ast_root/"captures"/(sha+".json.gz")
    if containers is not None: raw=containers[sha]
    elif direct.exists(): raw=direct.read_bytes()
    else:
        raw=None
        for shard in sorted(ast_root.glob("*.tar.gz")):
            with tarfile.open(shard) as tf:
                try: member=tf.getmember("captures/"+sha+".json.gz")
                except KeyError: continue
                if not member.isfile(): raise ValueError("AST member is not ordinary file")
                raw=tf.extractfile(member).read(); break
        if raw is None: raise ValueError("AST capture absent")
    if digest(raw)!=row["container_sha256"]: raise ValueError("AST container hash mismatch")
    decoded=gzip.decompress(raw)
    if digest(decoded)!=row["capture_sha256"]: raise ValueError("AST payload hash mismatch")
    obj=json.loads(decoded)
    if obj["status"]!="PARSED" or obj["sha256"]!=sha or obj["native_semantic_admission"] is not False: raise ValueError("AST status/source/admission invalid")
    return obj
def build(corpus_manifest,ast_root,repo_root):
    manifest=json.loads(corpus_manifest.read_bytes())
    ip=ast_root/"index.json"
    if ip.exists(): index=json.loads(ip.read_bytes())
    else: index=json.loads(gzip.decompress((ast_root/"index.json.gz").read_bytes()))
    needed={r["sha256"] for r in manifest["files"] if r["source"] in PINS and Path(r["path"]).name in FILES and "/npc/" in r["path"]}
    containers=None
    if not (ast_root/"captures").exists():
        containers={}
        for shard in sorted(ast_root.glob("*.tar.gz")):
            with tarfile.open(shard, "r|gz") as tf:
                for member in tf:
                    if member.name.startswith("captures/") and member.name.endswith(".json.gz"):
                        sha=Path(member.name).name[:-8]
                        if sha in needed:
                            if not member.isfile() or sha in containers: raise ValueError("Invalid/duplicate selected AST member")
                            containers[sha]=tf.extractfile(member).read()
        if set(containers)!=needed: raise ValueError("Selected AST captures missing")
    projects=[]; records=[]; seen=set()
    for row in sorted(manifest["files"],key=lambda r:(r["source"],r["path"])):
        if row["source"] not in PINS or Path(row["path"]).name not in FILES or "/npc/" not in row["path"]: continue
        identity=(row["source"],row["revision"],row["path"])
        if identity in seen: raise ValueError("Duplicate selected corpus Source identity")
        seen.add(identity)
        if row["revision"]!=PINS[row["source"]]: raise ValueError("Unexpected donor pin")
        bp=Path(row.get("cache_path",row.get("blob_path","")))
        if not bp.is_absolute(): bp=corpus_manifest.parent/bp
        raw=bp.read_bytes()
        if digest(raw)!=row["sha256"] or len(raw)!=row["byte_count"] or hashlib.sha1(b"blob "+str(len(raw)).encode()+b"\0"+raw).hexdigest()!=row["git_blob_sha1"]: raise ValueError("Source witness mismatch")
        ast=load_ast(ast_root,row["sha256"],index,containers)
        if base64.b64decode(ast["raw_bytes_base64"])!=raw: raise ValueError("AST raw source mismatch")
        sources=[r for r in index["sources"] if (r["source"],r["revision"],r["path"],r["sha256"] )==(row["source"],row["revision"],row["path"],row["sha256"])]
        if len(sources)!=1: raise ValueError("AST identity absent/duplicated")
        if sources[0]["git_blob_sha1"]!=row["git_blob_sha1"] or sources[0]["byte_count"]!=row["byte_count"]: raise ValueError("AST index Source witness mismatch")
        proj=project(row,ast["ast"]); projects.append((row,ast["ast"],proj))
        factory=[]; display=[]
        for path,n,_ in walk(ast["ast"]):
            f=n.get("fields",{})
            if n.get("node_type")=="LocalAssign" and [name(v) for v in f["targets"]]==["internalNpcName"] and len(f["values"])==1 and f["values"][0].get("node_type")=="String":
                display.append({"literal":literals(f["values"][0])[0],"witness":witness(row,path,n),"classification":"LOCAL_DISPLAY_NAME_DECLARATION_NOT_FACTORY_ID"})
            if n.get("node_type")=="Call" and name(f["func"])=="Game.createNpcType" and len(f["args"])==1:
                arg=f["args"][0]; value=literals(arg)[0] if arg.get("node_type")=="String" else display[0]["literal"] if name(arg)=="internalNpcName" and len(display)==1 else None
                factory.append({"literal":value,"argument_ast":arg,"witness":witness(row,path,n),"classification":"DECLARED_FACTORY_NAME_NOT_OTERYN_NPC_ID","resolution":"DIRECT_LITERAL" if arg.get("node_type")=="String" else "PRIOR_LOCAL_LITERAL" if value else "UNRESOLVED"})
        records.append({"source":row["source"],"revision":row["revision"],"path":row["path"],"source_sha256":row["sha256"],"git_blob_sha1":row["git_blob_sha1"],"factory_name_declarations":factory,"display_name_declarations":display,**proj})
    links=helper_links(projects)
    quest_inputs=[]; quest_links=[]
    index_path="content/quests/definitions/index.json"; qb=(repo_root/index_path).read_bytes(); qi=json.loads(qb)
    quest_inputs.append({"path":index_path,"sha256":digest(qb)})
    selected={(r["source"],r["revision"],r["path"],r["source_sha256"]) for r in records}
    for shard in qi["shards"]:
        blob=(repo_root/shard).read_bytes(); quest_inputs.append({"path":shard,"sha256":digest(blob)})
        for entry in json.loads(blob)["records"]:
            q=entry["definition"]; refs=[]
            for track in q.get("source_data",{}).get("progress",[]):
                for tr in track["transitions"]:
                    for occ in tr.get("source_occurrences",[]):
                        if (occ["source"],occ["revision"],occ["path"],occ["blob_sha256"]) in selected:
                            refs.append({"progress_key":track["key"],"occurrence":occ,"binding":"EXISTING_EXACT_PROGRESS_SOURCE_OCCURRENCE_ONLY"})
            if refs: quest_links.append({"quest":q["identity"],"source_progress_occurrences":refs,"native_npc_binding":False,"complete_dialogue_certification":False})
    for descriptor in quest_inputs:
        if digest((repo_root/descriptor["path"]).read_bytes())!=descriptor["sha256"]: raise ValueError("Quest input moved during build")
    return {"schema":"OTERYN_DONOR_DIALOGUE_SEMANTIC_SUPPLEMENT/v1","scope":"BOUNDED_AST_GUARDED_SOURCE_DIALOGUE_NOT_RUNTIME_EQUIVALENCE","corpus_manifest_sha256":digest(corpus_manifest.read_bytes()),"ast_index_sha256":digest(stable(index)),"native_admission":False,"source_complete_quest_certification":False,"records":records,"helper_links":links,"quest_inputs":quest_inputs,"quest_links":quest_links,"summary":{"source_files":len(records),"guarded_branches":sum(len(r["branches"]) for r in records),"ordered_source_events":sum(len(r["events"]) for r in records),"helper_calls":len(links),"quests_with_existing_exact_source_links":len(quest_links),"scope_filenames":sorted(FILES)},"holds":["Global declaration candidate does not prove loader execution or deployed binding","Expression values, random results, external helpers and message tables retain exact AST without runtime evaluation","No Oteryn NPC identifiers or quest completion are inferred from filename or lexical similarity", "Existing canonical progress occurrence joins preserve existing attribution and do not prove sole quest ownership or complete dialogue", "Branch guards do not prove prior-return reachability, loop execution, expression short circuit or deployed global reassignments"]}
def qualify(packet,corpus_manifest,ast_root,repo_root):
    expected=build(corpus_manifest,ast_root,repo_root)
    if packet!=expected: raise ValueError("Supplement differs from exact Source/AST regeneration")
    return expected["summary"]
def main():
    p=argparse.ArgumentParser(); p.add_argument("--corpus-manifest",type=Path,required=True);p.add_argument("--ast-root",type=Path,required=True);p.add_argument("--repo-root",type=Path,required=True);p.add_argument("--out",type=Path,required=True); a=p.parse_args()
    data=build(a.corpus_manifest,a.ast_root,a.repo_root);a.out.parent.mkdir(parents=True,exist_ok=True);payload=stable(data)+b"\n"; a.out.write_bytes(gzip.compress(payload,mtime=0) if a.out.suffix==".gz" else payload); print(json.dumps(data["summary"]))
if __name__=="__main__": main()
