import gzip, json, subprocess, re
from pathlib import Path
paths = subprocess.check_output(["git", "diff", "29dcc3111", "--name-only", "--", "content", "tools"], text=True).splitlines()
report = []
for path in paths:
 if not path.endswith((".json", ".json.gz")):
  continue
 raw_before = subprocess.check_output(["git", "show", "29dcc3111:" + path])
 raw_after = Path(path).read_bytes()
 if path.endswith(".gz"):
  raw_before,raw_after = gzip.decompress(raw_before),gzip.decompress(raw_after)
 before,after = json.loads(raw_before),json.loads(raw_after)
 changes = []
 def compare(a, b, key=""):
  if type(a) != type(b):
   changes.append((key,a,b))
  elif isinstance(a, dict):
   for k in a.keys() | b.keys():
    if k not in a or k not in b:
     changes.append((key+"/"+k,a.get(k),b.get(k)))
    else:
     compare(a[k],b[k],key+"/"+k)
  elif isinstance(a, list):
   if len(a) != len(b):
    changes.append((key+"/length",len(a),len(b)))
   for i,(x,y) in enumerate(zip(a,b)):
    compare(x,y,key+"/"+str(i))
  elif a != b:
   if isinstance(a,str) and isinstance(b,str) and a.startswith("{") and b.startswith("{"):
    try:
     left,right = json.loads(a),json.loads(b)
    except ValueError:
     changes.append((key,a,b))
    else:
     compare(left,right,key+"/@json")
   else:
    changes.append((key,a,b))
 compare(before, after)
 unexpected = [(k,a,b) for k,a,b in changes if not (
  all(isinstance(v,str) and re.fullmatch(r"(?:sha256:)?[0-9a-f]{64}",v) for v in (a,b)) or
  (path == "content/quests/world-callbacks.json" and k.endswith("/require_state") and a is None and b is True))]
 report.append(dict(path=path, changed_values=len(changes), non_hash_changes=unexpected[:10]))
Path(".questpc7/asymmetric-generated-diff-audit.json").write_text(json.dumps(report,indent=2)+"\n")
errors = sum(len(r["non_hash_changes"]) for r in report)
print(f"JSON files={len(report)} unexpected non-hash changes={errors}")
raise SystemExit(bool(errors))
