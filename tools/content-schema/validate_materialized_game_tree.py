#!/usr/bin/env python3
from __future__ import annotations
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
CONTRACT=ROOT/"docs/agents/evidence/OTV2-20260925-full-game-content-ruleset-tree-v1.json"
EVIDENCE=ROOT/"docs/agents/evidence/OTV2-20260925-full-game-tree-materialization-v1.json"

class ValidationError(RuntimeError): pass
def req(ok: bool, code: str) -> None:
    if not ok: raise ValidationError(code)

def main() -> int:
    contract=json.loads(CONTRACT.read_text(encoding="utf-8"))
    evidence=json.loads(EVIDENCE.read_text(encoding="utf-8"))
    dirs=[row for row in contract["target_tree_nodes"] if row["path"].endswith("/")]
    req(evidence["directory_nodes"]==len(dirs),"DIRECTORY_COUNT")
    req(evidence["unmaterialized_directories"]==0,"UNMATERIALIZED_NONZERO")
    rows={row["path"]:row for row in evidence["rows"]}
    req(set(rows)=={row["path"] for row in dirs},"ROW_COVERAGE")
    for node in dirs:
        path=node["path"]
        marker=ROOT/(path+"index.json")
        req(marker.is_file(),f"MISSING_INDEX:{path}")
        payload=json.loads(marker.read_text(encoding="utf-8"))
        # Existing populated family index schemas are valid; new markers use the directory schema.
        if payload.get("schema")=="OTERYN_GAME_TREE_DIRECTORY/v1":
            req(payload.get("path")==path,f"PATH_MISMATCH:{path}")
            req(payload.get("owner")==node["owner"],f"OWNER_MISMATCH:{path}")
            req(payload.get("repository")=="Oteryn/Oteryn-Game",f"REPOSITORY_MISMATCH:{path}")
            req(payload.get("population_state") in {"READY_UNPOPULATED","POPULATED","LEGACY_COMPAT_PRESENT"},f"STATE_INVALID:{path}")
    # Safety: the migrated Item and Mount payloads still exist.
    req((ROOT/"content/items/index.json").is_file(),"ITEM_INDEX_MISSING")
    req((ROOT/"content/cosmetics/mounts/index.json").is_file(),"MOUNT_INDEX_MISSING")
    req((ROOT/"content/world/definitions/reference.json").is_file(),"LEGACY_REFERENCE_MISSING")
    print(f"PASS directories={len(dirs)} unmaterialized=0")
    return 0

if __name__=="__main__":
    raise SystemExit(main())
