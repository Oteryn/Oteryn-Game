from pathlib import Path
import hashlib,json,os,re,subprocess,time
root=Path(r"C:\Users\barte\work\oteryn-test-value-20261123")
out=Path(r"C:\Users\barte\work\oteryn-protocol-audit-20261123\hireling_ui_qualification")
report=json.loads((out/"results.json").read_text())
assert not report["completed"]
assert report["runs"][-1]["label"]=="candidate_format_check" and report["runs"][-1]["returncode"]==1
(out/"results_initial_format_failure.json").write_text(json.dumps(report,indent=2)+"\n",encoding="utf-8")
paths=["tools/architecture-check/src/client_fragments.rs","tools/architecture-check/src/client_fragments/registrations.rs","tools/architecture-check/src/client_fragments/schema.rs"]
log=(out/"candidate_format_check.log").read_text()
assert all(line.startswith("Incorrect newline style in ") for line in log.splitlines() if line.strip())
assert all(any(p.replace("/",chr(92)) in line for p in paths) for line in log.splitlines() if line.strip())
fixed=[]
for p in paths:
    raw=(root/p).read_bytes()
    base=subprocess.check_output(["git","show","HEAD:"+p],cwd=root)
    assert b"\r\n" in raw and raw.replace(b"\r\n",b"\n")==base,p
    (root/p).write_bytes(base)
    fixed.append({"path":p,"before_sha256":hashlib.sha256(raw).hexdigest(),"canonical_sha256":hashlib.sha256(base).hexdigest(),"semantic_bytes_equal":True})
report["format_environment_correction"]={"cause":"Inherited CRLF in three unchanged test-imported architecture-check sources","canonicalized":fixed,"initial_failure_preserved":"results_initial_format_failure.json","test_or_clippy_rerun":False}
env=dict(os.environ,CARGO_BUILD_JOBS="4",CARGO_TARGET_DIR=str(root)+"-target",CARGO_TERM_COLOR="never")
for label,cmd in [("candidate_format_check_canonical",["cargo","+1.100.0","fmt","--check","-p","oteryn-client-ui"]),("candidate_diff_check",["git","diff","--check","--","crates/client-ui"])]:
    print("START",label,flush=True);t=time.monotonic();p=subprocess.run(cmd,cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    (out/(label+".log")).write_bytes(p.stdout)
    report["runs"].append({"label":label,"command":cmd,"returncode":p.returncode,"seconds":round(time.monotonic()-t,3),"summaries":[],"log_sha256":hashlib.sha256(p.stdout).hexdigest()})
    (out/"results.json").write_text(json.dumps(report,indent=2)+"\n",encoding="utf-8")
    assert p.returncode==0,(label,p.stdout.decode()[-4000:])
    print("END",label,p.returncode,flush=True)
report["final_sha256"]={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in report["candidate_sha256"]}
assert report["final_sha256"]==report["candidate_sha256"]
changes=[p for p,h in report["protected_source_sha256"].items() if hashlib.sha256((root/p).read_bytes()).hexdigest()!=h]
assert not changes,changes
report["protected_source_changes"]=changes
report["production_restored"]=True
report["completed"]=True
(out/"results.json").write_text(json.dumps(report,indent=2)+"\n",encoding="utf-8")
print("QUALIFICATION_COMPLETE",json.dumps({k:v for k,v in report.items() if k in ["completed","final_sha256","full_suite_outcome_totals","library_id_selection_unchanged"]}),flush=True)

