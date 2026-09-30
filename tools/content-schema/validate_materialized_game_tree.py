#!/usr/bin/env python3
from __future__ import annotations
import json
import re
import sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
CONTRACT=ROOT/"docs/agents/evidence/OTV2-20260925-full-game-content-ruleset-tree-v1.json"
EVIDENCE=ROOT/"docs/agents/evidence/OTV2-20260925-full-game-tree-materialization-v1.json"
CLOSURE=ROOT/"docs/agents/evidence/OTV2-20260925-world-successor-tree-closure-v1.json"
LEGACY_ROOT="content/world/"
WORLD_STATES={"READY_UNPOPULATED","LEGACY_COMPAT_PRESENT"}
# The one successor directory that shares space with a legacy locator (worlds/world.json).
# Legacy lookups scan it, so it may hold a family index plus at most this many shards.
SHARED_WITH_LOCATOR="content/world/worlds/"
SHARED_MAX_SHARDS=1
FAMILY_INDEX="OTERYN_FAMILY_INDEX/v1"
# WO-2: the Terrain and WorldObject catalogues are populated beside the legacy package. They keep
# the plain directory marker (population_state POPULATED); their shards are pinned by
# world-object-authoring/build_catalogue.py --check.
POPULATED_WORLD_CATALOGUES={"content/world/terrain/":"terrain-","content/world/objects/":"objects-"}
CATALOGUE_SHARD=re.compile(r"^(terrain|objects)-\d{5}-\d{5}\.json$")
class ValidationError(RuntimeError): pass
def req(ok: bool, code: str)->None:
    if not ok: raise ValidationError(code)
def directory_nodes()->list[dict]:
    contract=json.loads(CONTRACT.read_text(encoding="utf-8"))
    return [row for row in contract["target_tree_nodes"] if row["path"].endswith("/")]
def world_markers(dirs: list[dict])->list[str]:
    """Successor markers below the legacy WorldProject root, relative to that root."""
    return sorted(node["path"][len(LEGACY_ROOT):]+"index.json" for node in dirs if node["path"].startswith(LEGACY_ROOT))
def world_successor_files(dirs: list[dict])->list[str]:
    """Markers plus the shards of successor family indexes, relative to the legacy root.

    None of them is a WorldProject locator, so the legacy package seed removes them all."""
    files=set(world_markers(dirs))
    for marker in world_markers(dirs):
        payload=json.loads((ROOT/LEGACY_ROOT/marker).read_text(encoding="utf-8"))
        if payload.get("schema")==FAMILY_INDEX:
            files|={shard[len(LEGACY_ROOT):] for shard in payload["shards"]}
    for path in POPULATED_WORLD_CATALOGUES:
        files|={n[len(LEGACY_ROOT):] for n in catalogue_shards(path)}
    return sorted(files)
def catalogue_shards(path: str)->list[str]:
    return sorted(path+f.name for f in (ROOT/path).iterdir() if CATALOGUE_SHARD.match(f.name))
def legacy_locators()->set[str]:
    manifest=json.loads((ROOT/LEGACY_ROOT/"manifest.json").read_text(encoding="utf-8"))
    return {"project.json","manifest.json","content.lock.json"}|{row["locator"] for row in manifest["documents"]}
def check_family_index(path: str, payload: dict, local: set[str], locators: set[str])->None:
    """A populated family index below the legacy root: shards stay in its directory."""
    shared=path==SHARED_WITH_LOCATOR
    req(shared or not locators,f"WORLD_FAMILY_BESIDE_LOCATOR:{path}")
    req(payload.get("population_state")=="POPULATED",f"WORLD_FAMILY_STATE:{path}")
    shards=payload.get("shards",[])
    req(not shared or len(shards)==SHARED_MAX_SHARDS,f"WORLD_SHARED_SHARD_COUNT:{path}")
    req(all(s.startswith(path) and "/" not in s[len(path):] for s in shards),f"WORLD_SHARD_OUTSIDE:{path}")
    req(local=={"index.json",*locators,*(s[len(path):] for s in shards)},f"WORLD_STRAY_FILE:{path}")
def main()->int:
    dirs=directory_nodes()
    if sys.argv[1:]==["--print-world-markers"]:
        print("\n".join(world_markers(dirs)))
        return 0
    if sys.argv[1:]==["--print-world-successor-files"]:
        print("\n".join(world_successor_files(dirs)))
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
            # A successor directory holds either its marker or a populated family index
            # whose shards stay inside it. The one directory sharing space with a legacy
            # locator (worlds/) may hold a family index with exactly one shard: legacy
            # lookups scan it, so its entry count is bounded and accounted for in the
            # repository test's scan budget.
            local={f.name for f in (ROOT/path).iterdir()}
            rel=path[len(LEGACY_ROOT):]
            locators={loc[len(rel):] for loc in legacy_locators() if loc.startswith(rel) and "/" not in loc[len(rel):]}
            if payload.get("schema")==FAMILY_INDEX:
                check_family_index(path,payload,local,locators)
            else:
                req(payload.get("schema")=="OTERYN_GAME_TREE_DIRECTORY/v1",f"WORLD_MARKER_SCHEMA:{path}")
                req(payload.get("kind")==node["kind"],f"KIND_MISMATCH:{path}")
                catalogue=path in POPULATED_WORLD_CATALOGUES
                req(payload.get("population_state") in WORLD_STATES|({"POPULATED"} if catalogue else set()),f"WORLD_MARKER_STATE:{path}")
                if catalogue:
                    req(not locators and catalogue_shards(path),f"WORLD_CATALOGUE_EMPTY:{path}")
                    req(all(CATALOGUE_SHARD.match(n) and n.startswith(POPULATED_WORLD_CATALOGUES[path]) for n in local-{"index.json"}),f"WORLD_STRAY_FILE:{path}")
                else:
                    req(local=={"index.json"}|locators,f"WORLD_STRAY_FILE:{path}")
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
    successors=world_successor_files(dirs)
    req(not set(successors)&legacy_locators(),"WORLD_SUCCESSOR_IS_LEGACY_LOCATOR")
    print(f"PASS materialized={materialized}/97 world_markers={len(markers)} world_successor_files={len(successors)}")
    return 0
if __name__=="__main__": raise SystemExit(main())
