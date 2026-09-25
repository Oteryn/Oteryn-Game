#!/usr/bin/env python3
from __future__ import annotations
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
CONTRACT=ROOT/"docs/agents/evidence/OTV2-20260925-full-game-content-ruleset-tree-v1.json"
EVIDENCE=ROOT/"docs/agents/evidence/OTV2-20260925-full-game-tree-materialization-v1.json"
class ValidationError(RuntimeError): pass
def req(ok: bool, code: str)->None:
    if not ok: raise ValidationError(code)
def main()->int:
    contract=json.loads(CONTRACT.read_text(encoding="utf-8"))
    evidence=json.loads(EVIDENCE.read_text(encoding="utf-8"))
    dirs=[row for row in contract["target_tree_nodes"] if row["path"].endswith("/")]
    deferred=set(evidence["deferred_directories"])
    req(len(dirs)==97,"DIRECTORY_COUNT")
    req(len(deferred)==10,"DEFERRED_COUNT")
    req(all(path.startswith("content/world/") for path in deferred),"DEFERRED_SCOPE")
    rows={row["path"]:row for row in evidence["rows"]}
    req(set(rows)=={row["path"] for row in dirs},"ROW_COVERAGE")
    for node in dirs:
        path=node["path"]; marker=ROOT/(path+"index.json")
        if path in deferred:
            req(not marker.exists(),f"DEFERRED_MARKER_PRESENT:{path}")
            req(rows[path]["materialization"]=="DEFERRED_LEGACY_ROOT_CONFLICT",f"DEFERRED_ROW_STATE:{path}")
            continue
        req(marker.is_file(),f"MISSING_INDEX:{path}")
        payload=json.loads(marker.read_text(encoding="utf-8"))
        if payload.get("schema")=="OTERYN_GAME_TREE_DIRECTORY/v1":
            req(payload.get("path")==path,f"PATH_MISMATCH:{path}")
            req(payload.get("owner")==node["owner"],f"OWNER_MISMATCH:{path}")
    req(evidence["materialized_directories"]==87,"MATERIALIZED_COUNT")
    req(evidence["unmaterialized_directories"]==10,"UNMATERIALIZED_COUNT")
    req((ROOT/"content/items/index.json").is_file(),"ITEM_INDEX_MISSING")
    req((ROOT/"content/cosmetics/mounts/index.json").is_file(),"MOUNT_INDEX_MISSING")
    req((ROOT/"content/world/definitions/reference.json").is_file(),"LEGACY_REFERENCE_MISSING")
    req((ROOT/"rulesets/progression/prey/index.json").is_file(),"PREY_TREE_MISSING")
    req((ROOT/"rulesets/progression/wheel-of-destiny/index.json").is_file(),"WHEEL_TREE_MISSING")
    req((ROOT/"rulesets/items/imbuements/index.json").is_file(),"IMBUEMENTS_TREE_MISSING")
    print("PASS materialized=87 deferred_world=10")
    return 0
if __name__=="__main__": raise SystemExit(main())
