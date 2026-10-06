"""Build the canonical all-373 quest completion matrix.

This matrix joins the pinned wiki quest inventory to current canonical Oteryn
Quest definitions and donor/source binding evidence. It is an audit/work-queue
artifact only: it never promotes runtime or playability.
"""
from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path

CATALOGUE = "tools/content-schema/quest-authoring/samples/catalogue/catalogue.json"
COVERAGE = "tools/content-schema/quest-authoring/samples/quest-coverage-2026-09-27.json"
CROSSWALK = "tools/content-schema/quest-authoring/samples/binding_packets/source/crosswalk.json"
DEFINITIONS = "content/quests/definitions/index.json"
COMPLETION_CANDIDATE = "content/quests/missions/quest-state-completion-candidate.json"
COMPLETION_RECEIPT = "content/quests/missions/completion-candidate.json"
OUTPUT = "tools/content-schema/quest-authoring/samples/completion-matrix/all373.json"


def read(root: Path, path: str):
    return json.loads((root / path).read_text(encoding="utf-8"))


def compact(value) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def source_to_canonical(source_key: str) -> str | None:
    if ":quest/" not in source_key:
        return None
    return "oteryn:quest." + source_key.split(":quest/", 1)[1].replace("/", ".")


def load_definitions(root: Path):
    index = read(root, DEFINITIONS)
    records = []
    for shard in index["shards"]:
        records.extend(row["definition"] for row in read(root, shard)["records"])
    if len(records) != index["record_count"]:
        raise ValueError("Quest definition index count differs from shards")
    return index, records


def definition_titles(definition):
    titles = set()
    if definition.get("display_name"):
        titles.add(definition["display_name"])
    wiki = definition.get("wiki")
    if isinstance(wiki, dict) and wiki.get("title"):
        titles.add(wiki["title"])
    supplement = definition.get("oteryn_recipe")
    if isinstance(supplement, dict):
        payload = supplement.get("payload") or {}
        recipe = payload.get("recipe") or {}
        if recipe.get("wiki_title"):
            titles.add(recipe["wiki_title"])
        titles.update(payload.get("covered_wiki_titles") or [])
    return titles


def map_titles(catalogue, definitions):
    by_key = {d["identity"]["key"]: d for d in definitions}
    mapped = defaultdict(list)

    def add(title, definition):
        if definition not in mapped[title]:
            mapped[title].append(definition)

    for definition in definitions:
        for title in definition_titles(definition):
            add(title, definition)

    for row in catalogue:
        title = row["wiki_title"]
        for candidate in row.get("authored_candidates") or []:
            key = source_to_canonical(candidate["identity"]["key"])
            if key in by_key:
                add(title, by_key[key])
        family = row.get("family_representation")
        if family and family.get("target_key"):
            key = source_to_canonical(family["target_key"])
            if key in by_key:
                add(title, by_key[key])

    return mapped


def donor_mode(coverage):
    states = [coverage["canary"], coverage["crystalserver"]]
    if "IMPLEMENTED" in states:
        return "DONOR_IMPLEMENTATION_AVAILABLE"
    if "PARTIAL" in states:
        return "DONOR_PARTIAL_PLUS_REFERENCE"
    if states == ["ABSENT", "ABSENT"]:
        return "REFERENCE_AUTHORED_REQUIRED"
    return "DONOR_STATUS_REVIEW"


SOURCE_DATA_CODES = {
    "reported_source_gap",
    "requirement_unknown",
    "claim_item_semantics_missing",
    "claim_source_data_missing",
    "source_kind_log_flag_conflict",
}
NATIVE_LOWERING_CODES = {
    "quest_native_lowering_missing",
    "claim_native_lowering_missing",
}
NATIVE_BINDING_CODES = {
    "authored_trigger_and_delivery_bindings_missing",
}


def states(definitions, mapping_state, candidate_states, held_keys):
    codes = {
        issue.get("code", "UNKNOWN")
        for definition in definitions
        for issue in definition.get("missing_data") or []
    }
    keys = {definition["identity"]["key"] for definition in definitions}
    source_state = "SOURCE_HOLDS_PRESENT" if codes & SOURCE_DATA_CODES else "SOURCE_HOLDS_CLEAR"
    progress_states = {candidate_states.get(key, "NO_CANDIDATE") for key in keys}
    if mapping_state == "MULTIPLE":
        implementation = "MAPPING_REVIEW"
    elif progress_states & {"CHOSEN_TYPED_PROGRESS_ONLY", "CHOSEN_SOURCE_TYPED_PROGRESS_ONLY", "LOWERED"}:
        implementation = "NATIVE_BINDINGS_PENDING"
    elif progress_states & {"NOT_LOWERED_MULTI_TRACK", "NOT_LOWERED_NO_MISSIONS"}:
        implementation = "NATIVE_LOWERING_PENDING"
    elif keys & held_keys:
        implementation = "NATIVE_LOWERING_PENDING"
    elif codes & NATIVE_BINDING_CODES:
        implementation = "NATIVE_BINDINGS_PENDING"
    elif codes & NATIVE_LOWERING_CODES:
        implementation = "NATIVE_LOWERING_PENDING"
    else:
        readiness = {d.get("readiness", "UNKNOWN") for d in definitions}
        implementation = (
            "DEFINITION_READY_RUNTIME_UNKNOWN"
            if readiness == {"definition_ready"}
            else "REVIEW_REQUIRED"
        )
    progress_state = next(iter(progress_states)) if len(progress_states) == 1 else "MULTIPLE"
    return source_state, progress_state, implementation


def expected(root: Path):
    catalogue_obj = read(root, CATALOGUE)
    coverage_obj = read(root, COVERAGE)
    crosswalk_obj = read(root, CROSSWALK)
    completion_candidate = read(root, COMPLETION_CANDIDATE)
    completion_receipt = read(root, COMPLETION_RECEIPT)
    index, definitions = load_definitions(root)

    catalogue = catalogue_obj["quests"]
    coverage = {q["title"]: q for q in coverage_obj["quests"]}
    if len(catalogue) != 373 or len(coverage) != 373:
        raise ValueError("Pinned wiki inventory must contain exactly 373 quest titles")
    if {r["wiki_title"] for r in catalogue} != set(coverage):
        raise ValueError("Catalogue and coverage wiki-title sets differ")

    mapped = map_titles(catalogue, definitions)
    missing = [r["wiki_title"] for r in catalogue if not mapped[r["wiki_title"]]]
    if missing:
        raise ValueError("Canonical mapping missing for: " + ", ".join(missing))

    crosswalk = {row["quest_key"]: row for row in crosswalk_obj["quests"]}
    candidate_states = {}
    for quest in completion_candidate["quests"]:
        completion = quest["completion"]
        candidate_states[quest["quest"]] = completion if isinstance(completion, str) else completion["state"]
    held_keys = {row["quest"] for row in completion_receipt.get("chosen_source_progress_holds") or []}
    records = []
    used_definitions = set()
    for row in catalogue:
        title = row["wiki_title"]
        cover = coverage[title]
        defs = mapped[title]
        mapping_state = "SINGLE" if len(defs) == 1 else "MULTIPLE"
        canonical = []
        for definition in defs:
            key = definition["identity"]["key"]
            used_definitions.add(key)
            source = crosswalk.get(key)
            missing_codes = [item.get("code", "UNKNOWN") for item in definition.get("missing_data") or []]
            canonical.append({
                "key": key,
                "display_name": definition["display_name"],
                "kind": definition.get("kind"),
                "classification": definition.get("classification"),
                "readiness": definition.get("readiness"),
                "runtime_enabled": definition.get("runtime_enabled"),
                "missing_data_codes": missing_codes,
                "source_crosswalk": {
                    "present": source is not None,
                    "runtime_readiness": source.get("runtime_readiness") if source else None,
                    "stage_full_coverage": source.get("stage_full_coverage") if source else None,
                    "counts": source.get("counts") if source else None,
                },
            })
        source_state, progress_state, implementation_state = states(
            defs, mapping_state, candidate_states, held_keys
        )
        records.append({
            "wiki_title": title,
            "wiki": {
                "pageid": cover["pageid"],
                "revid": cover["revid"],
                "implemented": cover["implemented"],
                "in_quest_log": cover["in_quest_log"],
            },
            "donor": {
                "canary": cover["canary"],
                "crystalserver": cover["crystalserver"],
                "mode": donor_mode(cover),
            },
            "catalogue_coverage_state": row["coverage_state"],
            "canonical_mapping": mapping_state,
            "canonical": canonical,
            "source_fidelity_state": source_state,
            "typed_progress_state": progress_state,
            "implementation_state": implementation_state,
            "work_state": implementation_state,
            "playable_verification": "NOT_ASSESSED",
        })

    unused = [
        {"key": d["identity"]["key"], "display_name": d["display_name"], "readiness": d.get("readiness")}
        for d in definitions if d["identity"]["key"] not in used_definitions
    ]
    work_counts = Counter(r["work_state"] for r in records)
    source_counts = Counter(r["source_fidelity_state"] for r in records)
    progress_counts = Counter(r["typed_progress_state"] for r in records)
    implementation_counts = Counter(r["implementation_state"] for r in records)
    donor_counts = Counter(r["donor"]["mode"] for r in records)
    mapping_counts = Counter(r["canonical_mapping"] for r in records)
    unique_readiness = Counter(d.get("readiness") for d in definitions)

    return {
        "schema": "OTERYN_QUEST_COMPLETION_MATRIX/v1",
        "classification": "DERIVED_WORK_QUEUE_NOT_RUNTIME_AUTHORITY",
        "scope": "373 pinned wiki quest titles mapped to current canonical Oteryn Quest definitions",
        "inputs": {
            "catalogue": CATALOGUE,
            "coverage": COVERAGE,
            "crosswalk": CROSSWALK,
            "definitions": DEFINITIONS,
            "completion_candidate": COMPLETION_CANDIDATE,
            "completion_receipt": COMPLETION_RECEIPT,
        },
        "summary": {
            "wiki_titles": len(records),
            "canonical_definitions": len(definitions),
            "wiki_titles_mapped": sum(bool(mapped[r["wiki_title"]]) for r in catalogue),
            "unique_canonical_definitions_representing_wiki_titles": len(used_definitions),
            "canonical_definitions_not_in_wiki_title_inventory": len(unused),
            "mapping_state": dict(sorted(mapping_counts.items())),
            "donor_mode": dict(sorted(donor_counts.items())),
            "source_fidelity_state": dict(sorted(source_counts.items())),
            "typed_progress_state": dict(sorted(progress_counts.items())),
            "implementation_state": dict(sorted(implementation_counts.items())),
            "work_state": dict(sorted(work_counts.items())),
            "canonical_readiness": dict(sorted(unique_readiness.items())),
            "playable_verified": 0,
            "playable_assessment": "NOT_PERFORMED_BY_THIS_MATRIX",
        },
        "canonical_definitions_not_in_wiki_title_inventory": unused,
        "records": records,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    raw = compact(expected(root))
    target = root / OUTPUT
    if args.check:
        if not target.is_file() or target.read_bytes() != raw:
            raise ValueError("Quest completion matrix drift: " + OUTPUT)
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(raw)
    result = json.loads(raw)
    print(json.dumps(result["summary"], ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    main()
