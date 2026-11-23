import base64,hashlib,json,os,re,shutil,subprocess,time
from pathlib import Path
ROOT=Path(r"C:\Users\barte\work\oteryn-test-value-20261123")
OUT=Path(r"C:\Users\barte\work\oteryn-protocol-audit-20261123\hireling_ui_qualification")
OUT.mkdir(parents=True,exist_ok=True)
BASE="245ba7838e58318a2d851857b6c2b2d7c757aa09"
PAYLOAD=json.loads(r'''{"crates/client-ui/src/features/0165-hireling/mod.rs":{"bytes_base64":"cHViIHVzZSBvdGVyeW5fY2xpZW50X3J1bnRpbWU6OmhpcmVsaW5nOjp7CiAgICBDb21tYW5kU3RhdHVzLCBIaXJlbGluZ0NvbW1hbmQsIEhpcmVsaW5nUmVzcG9uc2UsIGhpcmVsaW5nLAp9Owp1c2Ugb3RlcnluX2lucHV0X2FjdGlvbnM6OntCdXR0b25TdGF0ZSwgTW9kaWZpZXIsIE5vcm1hbGl6ZWRJbnB1dEV2ZW50fTsKCnB1YiBmbiBvcGVuX2tleShldmVudHM6ICZbTm9ybWFsaXplZElucHV0RXZlbnRdKSAtPiBib29sIHsKICAgIGV2ZW50cy5pdGVyKCkuYW55KHxldmVudHwgewogICAgICAgIG1hdGNoZXMhKGV2ZW50LAogICAgICAgIE5vcm1hbGl6ZWRJbnB1dEV2ZW50OjpLZXkgeyBjb2RlLCBzdGF0ZTogQnV0dG9uU3RhdGU6OlByZXNzZWQsIG1vZGlmaWVycywgcmVwZWF0OiBmYWxzZSB9CiAgICAgICAgaWYgY29kZS5nZXQoKSA9PSAxMSAmJiBtb2RpZmllcnMuY29udGFpbnMoTW9kaWZpZXI6OkNvbnRyb2wpCiAgICAgICAgICAgICYmIG1vZGlmaWVycy5jb250YWlucyhNb2RpZmllcjo6U2hpZnQpICYmICFtb2RpZmllcnMuY29udGFpbnMoTW9kaWZpZXI6OkFsdCkKICAgICAgICAgICAgJiYgIW1vZGlmaWVycy5jb250YWlucyhNb2RpZmllcjo6U3VwZXIpKQogICAgfSkKfQoKI1tkZXJpdmUoRGVidWcsIENsb25lLCBDb3B5LCBQYXJ0aWFsRXEsIEVxKV0KcHViIGVudW0gSW50ZW50IHsKICAgIFJlZnJlc2gsCiAgICBTdW1tb24sCiAgICBEaXNtaXNzLAp9CgojW2Rlcml2ZShEZWJ1ZywgQ2xvbmUsIFBhcnRpYWxFcSwgRXEsIERlZmF1bHQpXQpwdWIgc3RydWN0IEhpcmVsaW5nUGFuZWwgewogICAgcHViIHNlbGVjdGVkOiBib29sLAogICAgcHViIG9wZW46IGJvb2wsCiAgICBwdWIgYWN0aXZlOiBPcHRpb248Ym9vbD4sCiAgICBwdWIgdGFyZ2V0OiBPcHRpb248KHUxNiwgdTE2LCBpOCk+LAogICAgcHViIHBlbmRpbmc6IE9wdGlvbjxIaXJlbGluZ0NvbW1hbmQ+LAogICAgcHViIGZlZWRiYWNrOiBPcHRpb248JidzdGF0aWMgc3RyPiwKfQoKcHViIGZuIHJlcXVlc3QocGFuZWw6ICZtdXQgSGlyZWxpbmdQYW5lbCwgaW50ZW50OiBJbnRlbnQpIC0+IE9wdGlvbjxIaXJlbGluZ0NvbW1hbmQ+IHsKICAgIGxldCByZXN1bHQgPSBpZiAhcGFuZWwuc2VsZWN0ZWQgewogICAgICAgIEVycih0ciEoIkhpcmVsaW5ncyBhcmUgdW5hdmFpbGFibGUgaW4gdGhpcyBzZXNzaW9uIikpCiAgICB9IGVsc2UgaWYgcGFuZWwucGVuZGluZy5pc19zb21lKCkgewogICAgICAgIEVycih0ciEoIldhaXRpbmcgZm9yIHRoZSBzZXJ2ZXIiKSkKICAgIH0gZWxzZSB7CiAgICAgICAgbWF0Y2ggaW50ZW50IHsKICAgICAgICAgICAgSW50ZW50OjpSZWZyZXNoID0+IE9rKEhpcmVsaW5nQ29tbWFuZChoaXJlbGluZzo6UmVxdWVzdDo6UXVlcnkpKSwKICAgICAgICAgICAgSW50ZW50OjpEaXNtaXNzID0+IE9rKEhpcmVsaW5nQ29tbWFuZChoaXJlbGluZzo6UmVxdWVzdDo6RGlzbWlzcykpLAogICAgICAgICAgICBJbnRlbnQ6OlN1bW1vbiA9PiBwYW5lbC50YXJnZXQubWFwX29yKAogICAgICAgICAgICAgICAgRXJyKHRyISgiU2VsZWN0IGFuIGFkamFjZW50IHN1bW1vbiBsb2NhdGlvbiIpKSwKICAgICAgICAgICAgICAgIHwoeCwgeSwgZmxvb3IpfCBPayhIaXJlbGluZ0NvbW1hbmQoaGlyZWxpbmc6OlJlcXVlc3Q6OlN1bW1vbiB7IHgsIHksIGZsb29yIH0pKSwKICAgICAgICAgICAgKSwKICAgICAgICB9CiAgICB9OwogICAgbWF0Y2ggcmVzdWx0IHsKICAgICAgICBPayhjb21tYW5kKSA9PiB7CiAgICAgICAgICAgIHBhbmVsLnBlbmRpbmcgPSBTb21lKGNvbW1hbmQpOwogICAgICAgICAgICBwYW5lbC5mZWVkYmFjayA9IE5vbmU7CiAgICAgICAgICAgIFNvbWUoY29tbWFuZCkKICAgICAgICB9CiAgICAgICAgRXJyKG1lc3NhZ2UpID0+IHsKICAgICAgICAgICAgcGFuZWwuZmVlZGJhY2sgPSBTb21lKG1lc3NhZ2UpOwogICAgICAgICAgICBOb25lCiAgICAgICAgfQogICAgfQp9CgpwdWIgZm4gYXBwbHlfcmVzdWx0KHBhbmVsOiAmbXV0IEhpcmVsaW5nUGFuZWwsIHJlc3BvbnNlOiAmSGlyZWxpbmdSZXNwb25zZSkgLT4gYm9vbCB7CiAgICBpZiBwYW5lbC5wZW5kaW5nLnRha2UoKS5pc19ub25lKCkgewogICAgICAgIHJldHVybiBmYWxzZTsKICAgIH0KICAgIG1hdGNoIChyZXNwb25zZS5zdGF0dXMsIHJlc3BvbnNlLmFjdGl2ZSkgewogICAgICAgIChDb21tYW5kU3RhdHVzOjpBY2NlcHRlZCwgU29tZShhY3RpdmUpKQogICAgICAgIHwgKENvbW1hbmRTdGF0dXM6OkR1cGxpY2F0ZVJlcGxheSwgU29tZShhY3RpdmUpKSA9PiB7CiAgICAgICAgICAgIHBhbmVsLmFjdGl2ZSA9IFNvbWUoYWN0aXZlKTsKICAgICAgICAgICAgcGFuZWwuZmVlZGJhY2sgPSBTb21lKHRyISgiVGhlIHNlcnZlciBjb25maXJtZWQgdGhlIGhpcmVsaW5nIHN0YXR1cyIpKTsKICAgICAgICAgICAgdHJ1ZQogICAgICAgIH0KICAgICAgICAoQ29tbWFuZFN0YXR1czo6UmVqZWN0ZWQsIF8pID0+IHsKICAgICAgICAgICAgcGFuZWwuZmVlZGJhY2sgPSBTb21lKHRyISgiVGhlIHNlcnZlciByZWZ1c2VkIHRoZSBoaXJlbGluZyByZXF1ZXN0IikpOwogICAgICAgICAgICBmYWxzZQogICAgICAgIH0KICAgICAgICBfID0+IHsKICAgICAgICAgICAgcGFuZWwuZmVlZGJhY2sgPSBTb21lKHRyISgiVGhlIHNlcnZlciByZXR1cm5lZCBhbiBpbnZhbGlkIGhpcmVsaW5nIHJlc3BvbnNlIikpOwogICAgICAgICAgICBmYWxzZQogICAgICAgIH0KICAgIH0KfQoKcHViIGZuIHBhbmVsX3RleHQocGFuZWw6ICZIaXJlbGluZ1BhbmVsKSAtPiBTdHJpbmcgewogICAgbGV0IHN0YXR1cyA9IG1hdGNoIHBhbmVsLmFjdGl2ZSB7CiAgICAgICAgU29tZSh0cnVlKSA9PiB0ciEoIkJhbmtlciBoaXJlbGluZzogc3VtbW9uZWQiKSwKICAgICAgICBTb21lKGZhbHNlKSA9PiB0ciEoIkJhbmtlciBoaXJlbGluZzogZGlzbWlzc2VkIiksCiAgICAgICAgTm9uZSA9PiB0ciEoIkJhbmtlciBoaXJlbGluZzogc3RhdHVzIHVua25vd24iKSwKICAgIH07CiAgICBsZXQgbXV0IHRleHQgPSBzdGF0dXMudG9fb3duZWQoKTsKICAgIGlmIHBhbmVsLnBlbmRpbmcuaXNfc29tZSgpIHsKICAgICAgICB0ZXh0LnB1c2hfc3RyKHRyISgiXHJcbldhaXRpbmcgZm9yIHRoZSBzZXJ2ZXLigKYiKSk7CiAgICB9CiAgICBpZiBsZXQgU29tZShmZWVkYmFjaykgPSBwYW5lbC5mZWVkYmFjayB7CiAgICAgICAgdGV4dC5wdXNoX3N0cigiXHJcbiIpOwogICAgICAgIHRleHQucHVzaF9zdHIoZmVlZGJhY2spOwogICAgfQogICAgdGV4dAp9CgojW2NmZyh0ZXN0KV0KbW9kIHRlc3RzIHsKICAgIHVzZSBzdXBlcjo6KjsKICAgIHVzZSBvdGVyeW5faW5wdXRfYWN0aW9uczo6e0tleUNvZGUsIE1vZGlmaWVyc307CgogICAgI1t0ZXN0XQogICAgZm4gY3RybF9zaGlmdF9oX29wZW5zX3BhbmVsKCkgLT4gUmVzdWx0PCgpLCBCb3g8ZHluIHN0ZDo6ZXJyb3I6OkVycm9yPj4gewogICAgICAgIGxldCBldmVudHMgPSBbTm9ybWFsaXplZElucHV0RXZlbnQ6OktleSB7CiAgICAgICAgICAgIGNvZGU6IEtleUNvZGU6Om5ldygxMSk/LAogICAgICAgICAgICBzdGF0ZTogQnV0dG9uU3RhdGU6OlByZXNzZWQsCiAgICAgICAgICAgIHJlcGVhdDogZmFsc2UsCiAgICAgICAgICAgIG1vZGlmaWVyczogTW9kaWZpZXJzOjpvbmUoTW9kaWZpZXI6OkNvbnRyb2wpLndpdGgoTW9kaWZpZXI6OlNoaWZ0KSwKICAgICAgICB9XTsKICAgICAgICBhc3NlcnQhKG9wZW5fa2V5KCZldmVudHMpKTsKICAgICAgICBPaygoKSkKICAgIH0KCiAgICAjW3Rlc3RdCiAgICBmbiByZXF1ZXN0c191c2VfdGhlX2NvZGVjX2FuZF9yZWZ1c2Fsc19hcmVfdmlzaWJsZSgpIC0+IFJlc3VsdDwoKSwgJidzdGF0aWMgc3RyPiB7CiAgICAgICAgbGV0IG11dCBwYW5lbCA9IEhpcmVsaW5nUGFuZWwgewogICAgICAgICAgICBzZWxlY3RlZDogdHJ1ZSwKICAgICAgICAgICAgdGFyZ2V0OiBTb21lKCg3LCA5LCAxKSksCiAgICAgICAgICAgIC4uRGVmYXVsdDo6ZGVmYXVsdCgpCiAgICAgICAgfTsKICAgICAgICBsZXQgY29tbWFuZCA9IHJlcXVlc3QoJm11dCBwYW5lbCwgSW50ZW50OjpTdW1tb24pLm9rX29yKCJzdW1tb24gY29tbWFuZCBtaXNzaW5nIik/OwogICAgICAgIGFzc2VydF9lcSEoCiAgICAgICAgICAgIGhpcmVsaW5nOjpSZXF1ZXN0OjpkZWNvZGUoJmNvbW1hbmQuMC5lbmNvZGUoKSksCiAgICAgICAgICAgIFNvbWUoY29tbWFuZC4wKQogICAgICAgICk7CiAgICAgICAgYXNzZXJ0ISghYXBwbHlfcmVzdWx0KAogICAgICAgICAgICAmbXV0IHBhbmVsLAogICAgICAgICAgICAmSGlyZWxpbmdSZXNwb25zZSB7CiAgICAgICAgICAgICAgICBzdGF0dXM6IENvbW1hbmRTdGF0dXM6OlJlamVjdGVkLAogICAgICAgICAgICAgICAgYWN0aXZlOiBOb25lCiAgICAgICAgICAgIH0KICAgICAgICApKTsKICAgICAgICBhc3NlcnQhKHBhbmVsX3RleHQoJnBhbmVsKS5jb250YWlucygicmVmdXNlZCIpKTsKICAgICAgICBPaygoKSkKICAgIH0KCiAgICAjW3Rlc3RdCiAgICBmbiBhY2NlcHRlZF9zdGF0dXNfdXBkYXRlc19wbGFpbl90ZXh0KCkgewogICAgICAgIGxldCBtdXQgcGFuZWwgPSBIaXJlbGluZ1BhbmVsIHsKICAgICAgICAgICAgc2VsZWN0ZWQ6IHRydWUsCiAgICAgICAgICAgIC4uRGVmYXVsdDo6ZGVmYXVsdCgpCiAgICAgICAgfTsKICAgICAgICBhc3NlcnQhKHJlcXVlc3QoJm11dCBwYW5lbCwgSW50ZW50OjpSZWZyZXNoKS5pc19zb21lKCkpOwogICAgICAgIGFzc2VydCEoYXBwbHlfcmVzdWx0KAogICAgICAgICAgICAmbXV0IHBhbmVsLAogICAgICAgICAgICAmSGlyZWxpbmdSZXNwb25zZSB7CiAgICAgICAgICAgICAgICBzdGF0dXM6IENvbW1hbmRTdGF0dXM6OkFjY2VwdGVkLAogICAgICAgICAgICAgICAgYWN0aXZlOiBTb21lKHRydWUpCiAgICAgICAgICAgIH0KICAgICAgICApKTsKICAgICAgICBhc3NlcnQhKHBhbmVsX3RleHQoJnBhbmVsKS5jb250YWlucygic3VtbW9uZWQiKSk7CiAgICB9Cn0K","sha256":"061a3c719bc7a297c7ffb10550bfc114663330ceb8719b913388d9713146b7bb"},"crates/client-ui/tests/chat_log.rs":{"bytes_base64":"dXNlIG90ZXJ5bl9jbGllbnRfcnVudGltZTo6Y2hhdDo6Q2hhdExpbmU7CnVzZSBvdGVyeW5fY2xpZW50X3VpOjpjaGF0Ojp7Q2hhdFBhbmVsLCB3cml0ZV9sb2d9OwojW3Rlc3RdCmZuIGNoYW5uZWxfbG9nX3dyaXRlc19hbmRfYXBwZW5kc191dGY4X2FuZF9wcm9wYWdhdGVzX2lvX2Vycm9ycygpIC0+IHN0ZDo6aW86OlJlc3VsdDwoKT4gewogICAgbGV0IHBhbmVsID0gQ2hhdFBhbmVsIHsKICAgICAgICBsaW5lczogdmVjIVtDaGF0TGluZTo6UHJpdmF0ZSB7CiAgICAgICAgICAgIHNwZWFrZXJfbmFtZTogIsW7w7PFgnciLmludG8oKSwKICAgICAgICAgICAgdGV4dDogIlphxbzDs8WCxIcgZ8SZxZtsxIUgamHFusWEIPCfkIkiLmludG8oKSwKICAgICAgICB9XSwKICAgICAgICAuLkNoYXRQYW5lbDo6ZGVmYXVsdCgpCiAgICB9OwogICAgbGV0IG11dCBieXRlcyA9IFZlYzo6bmV3KCk7CiAgICB3cml0ZV9sb2coJnBhbmVsLCAmbXV0IGJ5dGVzKT87CiAgICBhc3NlcnQhKAogICAgICAgIHN0ZDo6c3RyOjpmcm9tX3V0ZjgoJmJ5dGVzKS5pc19va19hbmQofHRleHR8IHRleHQuY29udGFpbnMoIsW7w7PFgnc6IFphxbzDs8WCxIcgZ8SZxZtsxIUgamHFusWEIPCfkIkiKSkKICAgICk7CiAgICBsZXQgb25jZSA9IGJ5dGVzLmNsb25lKCk7CiAgICB3cml0ZV9sb2coJnBhbmVsLCAmbXV0IGJ5dGVzKT87CiAgICBhc3NlcnRfZXEhKGJ5dGVzLCBvbmNlLnJlcGVhdCgyKSk7CiAgICBhc3NlcnQhKHdyaXRlX2xvZygmcGFuZWwsICZtdXQgc3RkOjppbzo6Q3Vyc29yOjpuZXcoJm11dCBbMHU4OyAxXVsuLl0pKS5pc19lcnIoKSk7CiAgICBPaygoKSkKfQo=","sha256":"ef84a79b75de54b714ddd644f285813f968a3e1e9b6338d7078566a53f440a66"}}''')
HIRE="crates/client-ui/src/features/0165-hireling/mod.rs"
CHAT="crates/client-ui/tests/chat_log.rs"
TARGET="hireling::tests::requests_use_the_codec_and_refusals_are_visible"
ENV=dict(os.environ,CARGO_BUILD_JOBS="4",CARGO_TARGET_DIR=str(ROOT)+"-target",CARGO_TERM_COLOR="never")
report={"baseline":BASE,"platform":"x86_64-pc-windows-msvc","candidate_sha256":{p:v["sha256"] for p,v in PAYLOAD.items()},"runs":[],"completed":False}
def sha(raw):return hashlib.sha256(raw).hexdigest()
def git(*args):return subprocess.check_output(["git",*args],cwd=ROOT)
def save(): (OUT/"results.json").write_text(json.dumps(report,indent=2)+"\n",encoding="utf-8")
def run(label,cmd,expected=0):
    print("START",label,flush=True);t=time.monotonic()
    proc=subprocess.run(cmd,cwd=ROOT,env=ENV,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    text=proc.stdout.decode("utf-8",errors="replace");(OUT/(label+".log")).write_bytes(proc.stdout)
    row={"label":label,"command":cmd,"returncode":proc.returncode,"seconds":round(time.monotonic()-t,3),"summaries":[l for l in text.splitlines() if l.startswith("test result:")],"log_sha256":sha(proc.stdout)}
    report["runs"].append(row);save();print("END",label,proc.returncode,row["summaries"],flush=True)
    assert proc.returncode==expected,(label,proc.returncode,expected,text[-6000:])
    return text
assert git("rev-parse","HEAD").decode().strip()==BASE
report["rust_version"]=subprocess.check_output(["rustc","+1.100.0","-Vv"],cwd=ROOT).decode()
report["free_disk_bytes"]=shutil.disk_usage(ROOT).free
assert report["free_disk_bytes"]>4*1024**3
modified=git("diff","--name-only","--no-renames").decode().splitlines()
protected={p:sha((ROOT/p).read_bytes()) for p in modified if p not in PAYLOAD and (ROOT/p).is_file()}
report["protected_source_sha256"]=protected
tracked=[p for p in git("ls-files","crates/client-ui").decode().splitlines() if p.endswith(".rs")]
request="".join("HEAD:"+p+"\n" for p in tracked).encode()
raw=subprocess.check_output(["git","cat-file","--batch"],cwd=ROOT,input=request)
offset=0;canonical=[]
for p in tracked:
    end=raw.index(b"\n",offset);header=raw[offset:end].split();n=int(header[-1]);body=raw[end+1:end+1+n];offset=end+2+n
    current=(ROOT/p).read_bytes()
    if b"\r\n" in current and current.replace(b"\r\n",b"\n")==body:
        (ROOT/p).write_bytes(body);canonical.append(p)
report["canonicalized_unchanged_paths"]=canonical
for p,item in PAYLOAD.items():
    candidate=base64.b64decode(item["bytes_base64"]);assert sha(candidate)==item["sha256"]
    current=(ROOT/p).read_bytes().replace(b"\r\n",b"\n")
    assert current in (git("show",BASE+":"+p),candidate),("unrecognized source",p,sha(current))
    (ROOT/p).write_bytes(candidate)
candidate=(ROOT/HIRE).read_bytes();baseline=git("show",BASE+":"+HIRE)
anchor=b"pub fn request(panel: &mut HirelingPanel, intent: Intent) -> Option<HirelingCommand> {\n"
mutation=anchor+b"    if matches!(intent, Intent::Summon) { return None; }\n"
assert baseline.count(anchor)==candidate.count(anchor)==1
old_ids=[]
try:
    (ROOT/HIRE).write_bytes(baseline.replace(anchor,mutation))
    text=run("baseline_mutant_missing_summon",["cargo","+1.100.0","test","--locked","-p","oteryn-client-ui","--lib",TARGET,"--","--exact"])
    assert re.search(r"test result: ok\. 1 passed; 0 failed; 0 ignored;",text)
    assert re.search(r"test "+re.escape(TARGET)+r" \.\.\. ok",text)
    exe=re.search(r"Running unittests.*\((.*\.exe)\)",text)
    if exe:
        binary=exe.group(1);binary=str(ROOT/binary) if not Path(binary).is_absolute() else binary
        listed=run("baseline_compiled_library_ids",[binary,"--list"])
        old_ids=[l[:-6] for l in listed.splitlines() if l.endswith(": test")]
        report["baseline_library_ids"]=old_ids;save()
    (ROOT/HIRE).write_bytes(candidate.replace(anchor,mutation))
    text=run("candidate_mutant_missing_summon",["cargo","+1.100.0","test","--locked","-p","oteryn-client-ui","--lib",TARGET,"--","--exact"],101)
    assert re.search(r"test result: FAILED\. 0 passed; 1 failed; 0 ignored;",text)
    assert 'summon command missing' in text
finally:
    (ROOT/HIRE).write_bytes(candidate)
    report["production_restored"]=sha((ROOT/HIRE).read_bytes())==PAYLOAD[HIRE]["sha256"]
    save()
try:
    text=run("candidate_full_ui",["cargo","+1.100.0","test","--locked","-p","oteryn-client-ui"])
    summary=[tuple(map(int,m)) for m in re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;",text)]
    report["full_suite_outcome_totals"]={"passed":sum(m[0] for m in summary),"failed":sum(m[1] for m in summary),"ignored":sum(m[2] for m in summary),"filtered":sum(m[4] for m in summary)}
    assert report["full_suite_outcome_totals"]=={"passed":729,"failed":0,"ignored":0,"filtered":0},report["full_suite_outcome_totals"]
    first=text.split("test result:",1)[0]
    current_ids=[m.group(1) for m in re.finditer(r"^test (.+?) \.\.\. ok$",first,re.M)]
    report["candidate_library_outcome_ids"]=current_ids
    if old_ids:
        assert sorted(old_ids)==sorted(current_ids),("library ids differ",len(old_ids),len(current_ids))
        report["library_id_selection_unchanged"]=True
    save()
    run("candidate_default_build",["cargo","+1.100.0","build","--locked","-p","oteryn-client-ui"])
    run("candidate_all_targets_clippy",["cargo","+1.100.0","clippy","--locked","-p","oteryn-client-ui","--all-targets","--","-D","warnings"])
    run("candidate_format_check",["cargo","+1.100.0","fmt","--check","-p","oteryn-client-ui"])
    run("candidate_diff_check",["git","diff","--check","--","crates/client-ui"])
    assert all(sha((ROOT/p).read_bytes())==v["sha256"] for p,v in PAYLOAD.items())
    changed=[p for p,h in protected.items() if sha((ROOT/p).read_bytes())!=h]
    report["protected_source_changes"]=changed;assert not changed,changed
    report["completed"]=True
finally:
    (ROOT/HIRE).write_bytes(candidate)
    report["production_restored"]=sha((ROOT/HIRE).read_bytes())==PAYLOAD[HIRE]["sha256"]
    report["final_sha256"]={p:sha((ROOT/p).read_bytes()) for p in PAYLOAD}
    save()
print("QUALIFICATION_COMPLETE",json.dumps({k:v for k,v in report.items() if k in ["completed","final_sha256","full_suite_outcome_totals","library_id_selection_unchanged"]}),flush=True)

