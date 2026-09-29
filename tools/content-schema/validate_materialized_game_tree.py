#!/usr/bin/env python3
from __future__ import annotations
import json
import sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
CONTRACT=ROOT/"docs/agents/evidence/OTV2-20260925-full-game-content-ruleset-tree-v1.json"
EVIDENCE=ROOT/"docs/agents/evidence/OTV2-20260925-full-game-tree-materialization-v1.json"
CLOSURE=ROOT/"docs/agents/evidence/OTV2-20260925-world-successor-tree-closure-v1.json"
LEGACY_ROOT="content/world/"
WORLD_STATES={"READY_UNPOPULATED","LEGACY_COMPAT_PRESENT"}
class ValidationError(RuntimeError): pass
def req(ok: bool, code: str)->None:
    if not ok: raise ValidationError(code)
def directory_nodes()->list[dict]:
    contract=json.loads(CONTRACT.read_text(encoding="utf-8"))
    return [row for row in contract["target_tree_nodes"] if row["path"].endswith("/")]
def world_markers(dirs: list[dict])->list[str]:
    """Successor markers below the legacy WorldProject root, relative to that root."""
    return sorted(node["path"][len(LEGACY_ROOT):]+"index.json" for node in dirs if node["path"].startswith(LEGACY_ROOT))
def legacy_locators()->set[str]:
    manifest=json.loads((ROOT/LEGACY_ROOT/"manifest.json").read_text(encoding="utf-8"))
    return {"project.json","manifest.json","content.lock.json"}|{row["locator"] for row in manifest["documents"]}
def main()->int:
    dirs=directory_nodes()
    if sys.argv[1:]==["--print-world-markers"]:
        print("\n".join(world_markers(dirs)))
        return 0
    req(not sys.argv[1:],"USAGE")
    evidence=json.loads(EVIDENCE.read_text(encoding="utf-8"))
    closure=json.loads(CLOSURE.read_text(encoding="utf-8"))
    req(len(dirs)==97,"DIRECTORY_COUNT")
    req(len({node["path"] for node in dirs})==97,"DIRECTORY_UNIQUENESS")
    rows={row["path"]:row for row in evidence["rows"]}
    req(set(rows)=={row["path"] for row in dirs},"ROW_COVERAGE")
    markers=world_markers(dirs)
    req(len(markers)==10,"WORLD_MARKER_COUNT")
    req(sorted(closure["world_markers"])==markers,"CLOSURE_WORLD_MARKERS")
    req(set(evidence["deferred_directories"])=={LEGACY_ROOT+m[:-len("index.json")] for m in markers},"CLOSURE_SCOPE")
    req(not set(markers)&legacy_locators(),"WORLD_MARKER_IS_LEGACY_LOCATOR")
    materialized=0
    for node in dirs:
        path=node["path"]; marker=ROOT/(path+"index.json")
        req(".." not in path.split("/") and not path.startswith("/"),f"UNSAFE_PATH:{path}")
        req(marker.is_file() and not marker.is_symlink(),f"MISSING_INDEX:{path}")
        materialized+=1
        payload=json.loads(marker.read_text(encoding="utf-8"))
        if path.startswith(LEGACY_ROOT):
            req(payload.get("schema")=="OTERYN_GAME_TREE_DIRECTORY/v1",f"WORLD_MARKER_SCHEMA:{path}")
            req(payload.get("kind")==node["kind"],f"KIND_MISMATCH:{path}")
            req(payload.get("population_state") in WORLD_STATES,f"WORLD_MARKER_STATE:{path}")
        if payload.get("schema")=="OTERYN_GAME_TREE_DIRECTORY/v1":
            req(payload.get("path")==path,f"PATH_MISMATCH:{path}")
            req(payload.get("owner")==node["owner"],f"OWNER_MISMATCH:{path}")
    req(materialized==97 and closure["materialized_directories"]==97,"MATERIALIZED_COUNT")
    req(closure["unmaterialized_directories"]==0,"UNMATERIALIZED_COUNT")
    req(json.loads((ROOT/"content/project.json").read_text(encoding="utf-8"))["runtime_source"]=="legacy_until_separately_qualified","RUNTIME_SWITCHED_EARLY")
    req((ROOT/"content/items/index.json").is_file(),"ITEM_INDEX_MISSING")
    req((ROOT/"content/cosmetics/mounts/index.json").is_file(),"MOUNT_INDEX_MISSING")
    req((ROOT/"content/world/definitions/reference.json").is_file(),"LEGACY_REFERENCE_MISSING")
    req((ROOT/"rulesets/progression/prey/index.json").is_file(),"PREY_TREE_MISSING")
    req((ROOT/"rulesets/progression/wheel-of-destiny/index.json").is_file(),"WHEEL_TREE_MISSING")
    req((ROOT/"rulesets/items/imbuements/index.json").is_file(),"IMBUEMENTS_TREE_MISSING")
    print(f"PASS materialized={materialized}/97 world_markers={len(markers)}")
    return 0
if __name__=="__main__": raise SystemExit(main())
