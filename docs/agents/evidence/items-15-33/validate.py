"""Validate this bounded evidence packet; optional source blobs/owner custody."""
import argparse,collections,csv,hashlib,json,pathlib,re,subprocess,xml.etree.ElementTree as ET
HERE=pathlib.Path(__file__).resolve().parent
REPO=HERE.parents[3]
PIN="83be72ad403ebc221dadba4b161c85fa1d47219a"
def read(name):return json.loads((HERE/name).read_text(encoding="utf-8"))
def sha(raw):return hashlib.sha256(raw).hexdigest()
def main():
 p=argparse.ArgumentParser(__doc__);p.add_argument("--source-blobs",action="store_true");p.add_argument("--evidence-root",type=pathlib.Path);args=p.parse_args()
 facts=read("client-basic110.json");items=facts["items"];assert len(items)==110
 assert len({x["name_signature_sha256"]for x in items})==110
 assert facts["source_commit"]==PIN and facts["scope"]["basic_capture_range"]==[24,110]
 captures=[c for x in items for c in x["captures"]]
 full=[c for c in captures if c["capture_key"].startswith("items-current:")]
 basic=[c for c in captures if c["capture_key"].startswith("items-basic:")]
 assert len(full)==33 and len(basic)==87
 assert {int(c["capture_key"].split(":")[1])for c in basic}==set(range(24,111))
 assert sum(c["review_kind"]=="new_basic_pages_manually_read"for c in basic)==77
 assert sum(c["review_kind"]=="prior_verified_values_reused_by_exact_identity"for c in basic)==10
 assert all(not x["candidate_source_binding"]["client_numeric_id_read"]for x in items)
 assert all("Description"not in x["properties"]for x in items)
 byname={x["client_name"]:x for x in items}
 assert byname["goo shell"]["properties"]["Weight"]=="58.00 oz"
 assert byname["death oyoroi"]["properties"]["Mantra"]=="16"
 assert byname["fireheart cuirass"]["candidate_source_binding"]["source_key"]=="oteryn:item.tibia.i22518"
 proofs=read("client-proof-receipts.json")["files"];pr={x["path"]:x for x in proofs};assert len(pr)==len(proofs)
 for c in captures:assert set(c["proof_paths"])<=set(pr)
 for name in ["client-basic110.json","corpus-summary.json","field-catalogue.json","qualified-differences.json"]:
  raw=(HERE/name).read_text(encoding="utf-8");assert not any(x in raw for x in ["C:\\Users\\","raw_name_ocr_candidate","name_ocr_confidence"])
 with(HERE/"field-coverage.csv").open(encoding="utf-8-sig",newline="")as f:rows=list(csv.DictReader(f))
 assert len({(r["corpus"],r["path"])for r in rows})==len(rows)
 assert len({r["corpus"]for r in rows})==20
 summary=read("corpus-summary.json");assert summary["pinned_commit"]==PIN
 assert len(rows)==sum(x["field_paths"]for x in summary["coverage"])
 for corpus in summary["coverage"]:
  subset=[r for r in rows if r["corpus"]==corpus["corpus"]]
  assert len(subset)==corpus["field_paths"] and all(int(r["record_count"])==corpus["records"]for r in subset)
 from census import generate
 reproduced=[{key:str(value)for key,value in row.items()}for row in generate()]
 assert rows==reproduced,"Coverage must match receipt-reproduced source traversal"
 assert summary["counts"]["canonical"]==34111 and summary["counts"]["selected_membership_union"]==6090
 assert len(read("field-catalogue.json")["fields"])==53
 assert len(read("field-catalogue.json")["source_schema_paths"])==202
 sources=read("source-receipts.json");assert sources["commit"]==PIN
 assert len({x["repository_path"]for x in sources["files"]})==len(sources["files"])
 assert all(re.fullmatch(r"[0-9a-f]{40}",x["git_blob_sha1"])for x in sources["files"])
 checked=0
 if args.source_blobs:
  blobs={}
  for x in sources["files"]:
   raw=subprocess.check_output(["git","cat-file","blob",x["git_blob_sha1"]],cwd=REPO)
   assert len(raw)==x["bytes"]and sha(raw)==x["sha256"],x["repository_path"]
   assert hashlib.sha1(b"blob "+str(len(raw)).encode()+b"\0"+raw).hexdigest()==x["git_blob_sha1"]
   blobs[x["repository_path"]]=raw;checked+=1
  canonical=[]
  for name,raw in blobs.items():
   if re.fullmatch(r"content/items/definitions/items-\d+-\d+\.json",name):canonical+=json.loads(raw)["records"]
  assert len(canonical)==34111
  C={r["definition"]["identity"]["key"]:r for r in canonical};assert len(C)==34111
  manifest=json.loads(blobs["content/spells.manifest.json"])
  for field in ["item_profiles","item_profile_updates","starting_loadout_item_profiles"]:
   name=(pathlib.PurePosixPath("content")/manifest[field]["path"]).as_posix()
   assert sha(blobs[name])==manifest[field]["sha256"]
  merged={}
  for name in ["content/items/definitions/spell-native-profiles.json","content/items/definitions/srx1-item-updates.json","content/items/definitions/starting-loadout-profiles.json"]:
   for r in json.loads(blobs[name])["records"]:merged[r["production_definition"]["production_key"]]=r
  assert len(merged)==6090
  assert C["oteryn:item.tibia.i19372"]["definition"]["semantics"]["physical"]["value"]["weight"]["value"]==4800
  assert merged["oteryn:item.tibia.i19372"]["semantics"]["physical"]["value"]["weight"]["value"]==5800
  assert "oteryn:item.tibia.i50260"not in merged
  mods=C["oteryn:item.tibia.i50260"]["definition"]["semantics"]["skill_modifiers"]["value"]["modifiers"]["value"]
  assert next(m for m in mods if m["kind"]=="MANTRA")["parameter"]["value"]["value"]==15
  xml=ET.fromstring(blobs["imports/canary/items-xml/items.xml"])
  assert xml.find("./item[@id='50260']/attribute[@key='mantra']").get("value")=="16"
 if args.evidence_root:
  for x in proofs:
   raw=(args.evidence_root/x["path"]).read_bytes()
   assert len(raw)==x["bytes"]and sha(raw)==x["sha256"],x["path"]
  raw=(args.evidence_root/"outputs/items-evidence-freeze-basic110/reader-basic110.json").read_bytes()
  assert sha(raw)==facts["scope"]["reader_subset_sha256"]
 print(json.dumps({"passed":True,"name_signature_groups":110,"basic_records":87,"cutoff":110,"proof_receipts":len(proofs),"source_receipts":len(sources["files"]),"source_blobs_checked":checked,"owner_proof_bytes_checked":len(proofs)if args.evidence_root else 0,"field_paths":len(rows)}))
if __name__=="__main__":main()
