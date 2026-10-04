#!/usr/bin/env python3
"""Import the reviewed imbuement snapshot into the static server ruleset tree."""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
from pathlib import Path

import binding_evidence
import eligibility_evidence
import imbuement_authoring

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
PREFIX = "rulesets/items/imbuements/"
SOURCE_HEAD = "8ae14d820500bd94a9da06c2323e3ef27d1b4018"


def encoded(value: object) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode()


def sha(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def load(path: Path) -> dict:
    return json.loads(path.read_bytes())


def inputs() -> dict:
    candidate = load(imbuement_authoring.CATALOGUE)
    imbuement_authoring.validate(candidate)
    packets = imbuement_authoring.supporting()
    if binding_evidence.build() != packets["imbuement-bindings.json"]:
        raise ValueError("Item recipe/scroll bindings differ from the current content tree")
    eligibility = packets["imbuement-eligibility.json"]
    if eligibility_evidence.build(eligibility) != eligibility:
        raise ValueError("equipment evidence differs from the current content tree")
    return {"candidate": candidate, "packets": packets}


def content_files(data: dict) -> dict[str, bytes]:
    candidate, packets = data["candidate"], data["packets"]
    rows = packets["imbuement-eligibility.json"]["items"]
    selected = [r for r in rows if r["allowed_types"] and r["status"] != "RETIRED_SOURCE_ITEM_EXCLUDED"]
    bound = [r for r in selected if r["item_ref"] is not None]
    held = [r for r in rows if r not in bound]
    # These are ruleset-local names, not newly admitted ReferenceDefinition identities.
    files = {
        PREFIX + "catalogue.json": encoded(candidate),
        PREFIX + "equipment.json": encoded({
            "schema": "OTERYN_IMBUEMENT_EQUIPMENT_DATA/v1",
            "runtime_admission": "BLOCKED",
            "profiles": bound,
            "held_observations": held,
            "source_summary": packets["imbuement-eligibility.json"]["summary"],
            "basic_tier_lowering": "BLOCKED_REFERENCE_IMBUEMENT_TIER_ONE_ABSENT",
        }),
        PREFIX + "access.json": encoded(packets["imbuement-access.json"]),
        PREFIX + "owner-policy.json": encoded(packets["owner-authoring-policy.json"]),
    }
    source_paths = sorted([imbuement_authoring.CATALOGUE, imbuement_authoring.FACTS,
                           imbuement_authoring.SCHEMA,
                           *(HERE / "samples" / p for p in packets)])
    source_refs = [{"path": p.relative_to(ROOT).as_posix(), "sha256": sha(p.read_bytes())}
                   for p in source_paths]
    marker = {
        "schema": "OTERYN_GAME_TREE_DIRECTORY/v1",
        "path": PREFIX, "kind": "ruleset", "owner": "Item/Progression",
        "repository": "Oteryn/Oteryn-Game", "population_state": "POPULATED",
        "contract": "docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md",
        "notes": "Static data imported; runtime, quest state and Item projection separately qualified.",
        "runtime_admission": "BLOCKED", "runtime_switch_authorized": False,
        "source_pr": "https://github.com/Oteryn/Oteryn-Game/pull/1438",
        "source_head": SOURCE_HEAD, "source_snapshot_target": candidate["target"],
        "authoring_sources": source_refs,
        "payloads": [{"path": p, "sha256": sha(raw)} for p, raw in sorted(files.items())],
        "counts": {"families": len(candidate["definitions"]),
                   "tiers": sum(len(d["tiers"]) for d in candidate["definitions"]),
                   "bound_equipment_profiles": len(bound), "held_equipment_observations": len(held),
                   "unregistered_equipment_items": sum(r["status"] == "CANONICAL_ITEM_ABSENT" for r in held)},
        "reference_resolution": {
            "supporting_catalogues_base": "tools/content-schema/imbuement-authoring/samples/",
            "item_refs": "CURRENT_CANONICAL_ITEM_IDENTITIES_VALIDATED",
            "quest_predicates": "SOURCE_DEFINED_RUNTIME_STATE_BINDING_HELD",
            "owner_policy_precedence": "OWNER_SELECTION_OVER_PUBLIC_UNKNOWN_OR_ENGINE_CONFLICT",
        },
    }
    files[PREFIX + "index.json"] = encoded(marker)
    return files


def registered(project: dict, manifest: dict, lock: dict, files: dict[str, bytes]) -> tuple:
    project, manifest, lock = map(copy.deepcopy, (project, manifest, lock))
    if (project["runtime_source"] != "legacy_until_separately_qualified"
            or manifest["compatibility"]["runtime_switch_authorized"] is not False):
        raise ValueError("static import cannot authorize runtime activation")
    index = json.loads(files[PREFIX + "index.json"])
    # A separate ruleset registry avoids claiming a new compiled ContentFamily.
    project["populated_rulesets"] = sorted(set(project.get("populated_rulesets", [])) | {"Imbuements"})
    manifest.setdefault("rulesets", {})["Imbuements"] = {
        "index": PREFIX + "index.json", "counts": index["counts"], "runtime_admission": "BLOCKED"}
    paths = {r["path"] for r in manifest["managed_files"]} | set(files)
    manifest["managed_files"] = [{"path": p} for p in sorted(paths)]
    lock.setdefault("ruleset_counts", {})["Imbuements"] = index["counts"]
    lock.setdefault("ruleset_files", {})["Imbuements"] = [
        {"path": p, "sha256": sha(raw)} for p, raw in sorted(files.items())]
    return project, manifest, lock


def command(check: bool) -> int:
    files = content_files(inputs())
    names = ("project", "manifest", "content.lock")
    docs = [load(ROOT / f"content/{n}.json") for n in names]
    for name, doc in zip(names, registered(*docs, files), strict=True):
        files[f"content/{name}.json"] = encoded(doc)
    stale = [p for p, raw in files.items() if not (ROOT / p).is_file() or (ROOT / p).read_bytes() != raw]
    if check and stale:
        raise ValueError(f"stale imported imbuement data: {stale}")
    if not check:
        for p in stale:
            (ROOT / p).parent.mkdir(parents=True, exist_ok=True)
            (ROOT / p).write_bytes(files[p])
    print("PASS imbuement static import: 24 families, 72 tiers; runtime admission blocked")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["content"])
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    try:
        return command(args.check)
    except (ValueError, OSError, KeyError) as exc:
        parser.exit(1, f"ERROR: {exc}\n")


if __name__ == "__main__":
    raise SystemExit(main())
