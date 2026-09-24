#!/usr/bin/env python3
"""Build a per-page documented-blocker ledger from the pinned G3 artifact.

This tool records why a source-family assignment remains unresolved. It never
assigns a source family, target identity, gameplay meaning, or runtime authority.
"""
from __future__ import annotations

import argparse
import collections
import hashlib
import json
import pathlib
import zipfile

ARTIFACT_ID = 10801778929
RUN_ID = 35986883931
HEAD_SHA = "20e8e5d99b7b42b6fd008e67029623ea7a4a8e3a"
ZIP_SHA256 = "a63635e4cdfcd237cf69b5e2fe0471c30723e45fe3ab2ccc9095730a9337d2e7"
UNIVERSE_SHA256 = "1562382c66ad471a46309eaa5ae2fe9e7b3b1e3c8c2d5fc047ee8faa65bd1666"
UNIVERSE_NAME = "source-family-classified-universe.json"
MANIFEST_NAME = "manifest.json"
SHAPES = {
    "STRUCTURED_PRIMARY", "NO_INFOBOX_ITEM", "STRUCTURED_ALTERNATE",
    "REDIRECT", "SOURCE_CLASSIFICATION_UNRESOLVED", "INFOBOX_ITEM_PARSE_ERROR",
}
TEMPLATES = {
    "OBJECT": "Predefinição:Infobox Object",
    "HUNTS": "Predefinição:Infobox Hunts",
    "WORLD_CHANGE": "Predefinição:Infobox World Change",
    "WORLD_QUEST": "Predefinição:Infobox World Quest",
}
EXPECTED_PAGE_SHAPES = {
    "STRUCTURED_PRIMARY": 3803,
    "NO_INFOBOX_ITEM": 1399,
    "STRUCTURED_ALTERNATE": 187,
    "REDIRECT": 104,
    "SOURCE_CLASSIFICATION_UNRESOLVED": 12,
    "INFOBOX_ITEM_PARSE_ERROR": 7,
}
EXPECTED_OBSERVATIONS = {
    "STRUCTURED_PRIMARY": 3803,
    "NO_INFOBOX_ITEM": 1436,
    "STRUCTURED_ALTERNATE": 187,
    "REDIRECT": 104,
    "SOURCE_CLASSIFICATION_UNRESOLVED": 12,
    "INFOBOX_ITEM_PARSE_ERROR": 7,
}


def canonical_bytes(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def load_g3(path: pathlib.Path) -> tuple[dict, dict]:
    archive_bytes = path.read_bytes()
    actual_zip_sha = sha256_bytes(archive_bytes)
    if actual_zip_sha != ZIP_SHA256:
        raise ValueError(f"G3 ZIP SHA256 mismatch: {actual_zip_sha}")
    with zipfile.ZipFile(path) as zf:
        if set(zf.namelist()) != {UNIVERSE_NAME, MANIFEST_NAME}:
            raise ValueError("G3 ZIP members differ from the pinned two-file artifact")
        manifest_bytes = zf.read(MANIFEST_NAME)
        universe_bytes = zf.read(UNIVERSE_NAME)
    if sha256_bytes(universe_bytes) != UNIVERSE_SHA256:
        raise ValueError("embedded classified universe SHA256 mismatch")
    manifest = json.loads(manifest_bytes)
    universe = json.loads(universe_bytes)
    if manifest.get("schema") != "OTERYN_SOURCE_FAMILY_CLASSIFICATION_MANIFEST/v1":
        raise ValueError("unexpected G3 manifest schema")
    if universe.get("schema") != "OTERYN_SOURCE_FAMILY_CLASSIFIED_UNIVERSE/v1":
        raise ValueError("unexpected G3 universe schema")
    if manifest.get("classified_universe_sha256") != UNIVERSE_SHA256:
        raise ValueError("manifest does not bind the pinned universe digest")
    if universe.get("authority", {}).get("source_family_classification") != "SOURCE_EVIDENCE_ONLY":
        raise ValueError("G3 authority boundary mismatch")
    if universe.get("authority", {}).get("gameplay_truth") != "NONE":
        raise ValueError("G3 unexpectedly claims gameplay truth")
    invariants = manifest.get("invariants", {})
    for key in ("identity_resolution_performed", "target_identity_selection_performed", "semantic_promotion_performed", "candidate_relations_resolved"):
        if invariants.get(key) is not False:
            raise ValueError(f"G3 invariant {key} is not false")
    if len(universe.get("pages", [])) != 15787 or manifest.get("classified_page_count") != 15787:
        raise ValueError("G3 page count mismatch")
    return manifest, universe


def primary_provenance(page: dict) -> dict:
    provenance = page.get("provenance")
    if not isinstance(provenance, list) or not provenance:
        raise ValueError("UNKNOWN page has no provenance")
    return provenance[0]


def blocker(page: dict) -> tuple[str, str, str]:
    p = primary_provenance(page)
    shape = p.get("source_shape")
    templates = set(p.get("templates", []))
    if shape == "STRUCTURED_PRIMARY":
        obj = TEMPLATES["OBJECT"] in templates
        quest = TEMPLATES["WORLD_QUEST"] in templates
        if obj and quest:
            return "OBJECT_WORLD_QUEST_OVERLAP", "Observed primary templates span object and world-quest source shapes; this overlap does not select a family or identity.", "VERIFY_PAGE_ID_19087_DIRECT_PRIMARY_ROLE_AND_TEMPLATE_SCOPE"
        if obj:
            return "OBJECT_SUBTYPE_UNRESOLVED", "Infobox Object is observed, but the source shape does not resolve WorldObject, LocalObject, Terrain, or placement.", "VERIFY_OBJECT_SUBTYPE_AND_DEFINITION_VS_PLACEMENT"
        if TEMPLATES["HUNTS"] in templates:
            return "HUNTS_EDITORIAL_SCOPE_UNRESOLVED", "Infobox Hunts is an editorial guide shape with Area, Creature, and Encounter references; it does not establish a blanket Area definition.", "VERIFY_HUNTS_GUIDE_VS_AREA_OR_RELATIONSHIP"
        if TEMPLATES["WORLD_CHANGE"] in templates:
            return "WORLD_CHANGE_DYNAMIC_SCOPE_UNRESOLVED", "Infobox World Change evidences dynamic world state or relationships, not an automatically static Encounter definition.", "VERIFY_WORLD_CHANGE_RUNTIME_STATE_VS_STATIC_DEFINITION"
        if quest:
            return "WORLD_QUEST_SCOPE_UNRESOLVED", "Infobox World Quest identifies an event/quest source shape but does not decide its source-family or gameplay representation.", "VERIFY_WORLD_QUEST_DEFINITION_VS_EVENT_RELATIONSHIP"
        roots = set(p.get("discovery_roots", []))
        root_actions = {
            "runes": ("RUNE_ROLE_UNRESOLVED", "VERIFY_RUNE_DEFINITION_VS_RELATIONSHIP"),
            "store": ("COMMERCE_ROLE_UNRESOLVED", "VERIFY_COMMERCE_PAGE_ROLE_AND_EXTERNAL_AUTHORITY"),
            "geography": ("GEOGRAPHY_ROLE_UNRESOLVED", "VERIFY_GEOGRAPHIC_DEFINITION_AND_PLACEMENT"),
            "daily-tasks": ("DAILY_TASK_ROLE_UNRESOLVED", "VERIFY_DAILY_TASK_RUNTIME_ROLE_AND_PRIMARY_DEFINITION"),
            "world-quests": ("WORLD_QUEST_ROOT_ROLE_UNRESOLVED", "VERIFY_WORLD_QUEST_DEFINITION_VS_EVENT_RELATIONSHIP"),
            "mini-world-changes": ("MINI_WORLD_CHANGE_ROLE_UNRESOLVED", "VERIFY_MINI_WORLD_CHANGE_STATE_VS_DEFINITION"),
            "world-changes": ("WORLD_CHANGE_ROOT_ROLE_UNRESOLVED", "VERIFY_WORLD_CHANGE_STATE_VS_DEFINITION"),
            "hunting-places": ("HUNTING_PLACE_ROLE_UNRESOLVED", "VERIFY_HUNTING_GUIDE_VS_AREA_DEFINITION"),
            "mounts": ("MOUNT_ROLE_UNRESOLVED", "VERIFY_MOUNT_SOURCE_ROLE_AND_PRIMARY_DEFINITION"),
            "cyclopedia": ("CYCLOPEDIA_ROLE_UNRESOLVED", "VERIFY_CYCLOPEDIA_PAGE_ROLE_AND_PRIMARY_DEFINITION"),
            "magical-archive": ("MAGIC_SOURCE_ROLE_UNRESOLVED", "VERIFY_MAGIC_SOURCE_ROLE_AND_PRIMARY_DEFINITION"),
            "tibiadrome": ("TIBIADROME_ROLE_UNRESOLVED", "VERIFY_TIBIADROME_PAGE_ROLE_AND_PRIMARY_DEFINITION"),
        }
        priority = ("runes", "store", "geography", "daily-tasks", "world-quests", "mini-world-changes", "world-changes", "hunting-places", "mounts", "cyclopedia", "magical-archive", "tibiadrome")
        selected = next((root for root in priority if root in roots), None)
        if selected is None:
            return "CURRENT_PAGE_ROLE_VERIFICATION_REQUIRED", "No registered direct primary-definition signature is observed; available discovery roots do not support a family assignment.", "VERIFY_CURRENT_PAGE_ROLE_AND_DIRECT_PRIMARY_DEFINITION"
        class_name, action = root_actions[selected]
        return class_name, f"No registered direct primary-definition signature is observed. Discovery root {selected!r} is retrieval context only; it does not establish source family or identity.", action
    reasons = {
        "NO_INFOBOX_ITEM": ("NO_INFOBOX_ITEM", "Pinned source-shape evidence records no parseable Infobox Item; no direct primary source-family signature resolves this page.", "VERIFY_CURRENT_PAGE_ID_FOR_DIRECT_PRIMARY_DEFINITION"),
        "STRUCTURED_ALTERNATE": ("ALTERNATE_SOURCE_ONLY", "The available structured observation is alternate/reference evidence, not a resolved direct primary definition.", "LOCATE_PAGE_SPECIFIC_PRIMARY_DEFINITION_AND_RECHECK_CURRENT_REVISION"),
        "REDIRECT": ("REDIRECT_TARGET_REQUIRES_DIRECT_EVIDENCE", "The pinned observation is a redirect; title or redirect target alone cannot establish the target page's source family or identity.", "RESOLVE_REDIRECT_BY_PAGE_ID_THEN_VERIFY_TARGET_PRIMARY_DEFINITION"),
        "SOURCE_CLASSIFICATION_UNRESOLVED": ("SOURCE_CLASSIFIER_UNRESOLVED", "The pinned classifier explicitly leaves source classification unresolved; no title-only inference is permitted.", "RECHECK_SOURCE_CLASSIFIER_EVIDENCE_FOR_EXACT_PAGE_ID"),
        "INFOBOX_ITEM_PARSE_ERROR": ("INFOBOX_ITEM_PARSE_ERROR", "Infobox Item was observed but parsing failed; parse error is distinct from absence of an infobox.", "REPAIR_OR_REPLACE_PAGE_SPECIFIC_INFOBOX_ITEM_PARSER_EVIDENCE"),
    }
    if shape not in reasons:
        raise ValueError(f"unsupported or missing primary source shape: {shape!r}")
    return reasons[shape]


def lane_observations(page: dict) -> list[dict]:
    # Keep every G3/G2 provenance field exactly as observed instead of projecting
    # only fields currently used by the blocker classifier.
    return [dict(p) for p in page.get("provenance", [])]


def build_ledger(manifest: dict, universe: dict) -> tuple[dict, dict]:
    pages = universe["pages"]
    unknown = [p for p in pages if p.get("source_family_classification", {}).get("state") == "UNKNOWN"]
    if len(unknown) != 5512:
        raise ValueError(f"expected 5512 UNKNOWN rows, got {len(unknown)}")
    ids = [p.get("page_id") for p in unknown]
    if None in ids or len(ids) != len(set(ids)):
        raise ValueError("UNKNOWN page IDs are missing or duplicated")

    page_shape_counts = collections.Counter()
    observation_counts = collections.Counter()
    blocker_counts = collections.Counter()
    ledger_rows = []
    for page in sorted(unknown, key=lambda p: p["page_id"]):
        primary = primary_provenance(page)
        shape = primary.get("source_shape")
        if shape not in SHAPES:
            raise ValueError(f"page {page['page_id']} has unsupported primary shape {shape!r}")
        page_shape_counts[shape] += 1
        for p in page["provenance"]:
            observation_counts[p.get("source_shape")] += 1
        blocker_class, reason, next_action = blocker(page)
        blocker_counts[blocker_class] += 1
        source_family = page.get("source_family_classification", {})
        ledger_rows.append({
            "page_id": page["page_id"],
            "title_observations": page.get("title_observations", []),
            "cross_lane_title_divergence": page.get("cross_lane_title_divergence", False),
            "source_family": "UNKNOWN",
            "source_family_classification_state": "UNKNOWN",
            "disposition_state": "DOCUMENTED_BLOCKER",
            "primary_source_shape": shape,
            "blocker_class": blocker_class,
            "blocker_reason": reason,
            "evidence_pointer": {
                "artifact_id": ARTIFACT_ID,
                "run_id": RUN_ID,
                "head_sha": HEAD_SHA,
                "universe_sha256": UNIVERSE_SHA256,
                "page_id": page["page_id"],
            },
            "lane_observations": lane_observations(page),
            "source_classification_evidence": source_family,
            "candidate_relationships_evidence_only": source_family.get("candidate_relationships_evidence_only", []),
            "candidate_relations_resolved": False,
            "target_identity": None,
            "gameplay_semantics_promoted": False,
            "revision_state": "PINNED_OBSERVATION_CURRENT_UNVERIFIED",
            "pinned_revision_ids": sorted({p["revision_id"] for p in page["provenance"] if isinstance(p.get("revision_id"), int)}),
            "next_action": next_action,
        })

    if dict(page_shape_counts) != EXPECTED_PAGE_SHAPES:
        raise ValueError(f"page-primary shape partition mismatch: {dict(page_shape_counts)}")
    if dict(observation_counts) != EXPECTED_OBSERVATIONS:
        raise ValueError(f"lane-observation shape counts mismatch: {dict(observation_counts)}")
    if sum(blocker_counts.values()) != 5512 or any(not row["blocker_reason"] or not row["next_action"] or not row["evidence_pointer"] or not row["lane_observations"] for row in ledger_rows):
        raise ValueError("ledger contains an undispositioned row")

    primary_rows = [r for r in ledger_rows if r["primary_source_shape"] == "STRUCTURED_PRIMARY"]
    template_presence = collections.Counter()
    for row in primary_rows:
        templates = set(row["lane_observations"][0].get("templates", []))
        for key, template in TEMPLATES.items():
            if template in templates:
                template_presence[key] += 1
    christmas_count = sum(row["page_id"] == 19087 and TEMPLATES["OBJECT"] in set(row["lane_observations"][0].get("templates", [])) and TEMPLATES["WORLD_QUEST"] in set(row["lane_observations"][0].get("templates", [])) for row in primary_rows)
    primary_union = template_presence["OBJECT"] + template_presence["HUNTS"] + template_presence["WORLD_CHANGE"] + template_presence["WORLD_QUEST"] - christmas_count
    primary_template_subtypes = {
        "object_template_observed": template_presence["OBJECT"],
        "object_only": template_presence["OBJECT"] - christmas_count,
        "object_world_quest_overlap": christmas_count,
        "hunts": template_presence["HUNTS"],
        "world_change": template_presence["WORLD_CHANGE"],
        "world_quest_only": template_presence["WORLD_QUEST"] - christmas_count,
        "residual_without_these_templates": len(primary_rows) - primary_union,
    }
    expected_template_subtypes = {"object_template_observed": 3122, "object_only": 3121, "object_world_quest_overlap": 1, "hunts": 521, "world_change": 12, "world_quest_only": 17, "residual_without_these_templates": 131}
    if primary_template_subtypes != expected_template_subtypes:
        raise ValueError(f"primary template subtype mismatch: {primary_template_subtypes}")
    actual_subtypes = collections.Counter(r["blocker_class"] for r in primary_rows)
    if sum(actual_subtypes.values()) != 3803:
        raise ValueError(f"primary blocker partition mismatch: {dict(actual_subtypes)}")
    christmas = next((r for r in primary_rows if r["page_id"] == 19087), None)
    if not christmas or christmas["blocker_class"] != "OBJECT_WORLD_QUEST_OVERLAP":
        raise ValueError("page_id 19087 Object x World Quest overlap counterexample changed")

    ledger = {
        "schema": "OTERYN_UNKNOWN_DISPOSITION_LEDGER/v1",
        "source_artifact": {"artifact_id": ARTIFACT_ID, "run_id": RUN_ID, "head_sha": HEAD_SHA, "zip_sha256": ZIP_SHA256, "universe_sha256": UNIVERSE_SHA256},
        "source_family_assignments": "NONE; every row remains UNKNOWN",
        "counts": {"unknown_pages": len(ledger_rows), "documented_blockers": len(ledger_rows), "undispositioned": 0, "page_primary_source_shapes": dict(sorted(page_shape_counts.items())), "all_lane_observations_by_shape": dict(sorted(observation_counts.items())), "blocker_classes": dict(sorted(blocker_counts.items())), "primary_template_subtypes": primary_template_subtypes},
        "invariants": {"exact_page_ids_only": True, "duplicate_page_ids": False, "candidate_relationships_resolved": False, "target_identity_selected": False, "gameplay_semantics_promoted": False, "current_revisions_verified": False, "current_revision_state_explicit": True, "all_unknown_rows_have_blocker_evidence_next_action": True},
        "pages": ledger_rows,
    }
    compact = {
        "schema": "OTERYN_UNKNOWN_DISPOSITION_LEDGER_MANIFEST/v1",
        "input": ledger["source_artifact"],
        "ledger_sha256": sha256_bytes(canonical_bytes(ledger)),
        "counts": ledger["counts"],
        "invariants": ledger["invariants"],
        "illustrative_blockers": [
            {"page_id": 19087, "title": "Christmas", "blocker_class": "OBJECT_WORLD_QUEST_OVERLAP"},
            {"page_id": 444, "title": "World Quests", "blocker_class": "PRIMARY_SIGNATURE_UNRESOLVED"},
            {"page_id": 3297, "title": "Updates/8.7", "blocker_class": "PRIMARY_SIGNATURE_UNRESOLVED"},
        ],
        "limitations": ["The ledger documents blockers; all 5,512 source families remain UNKNOWN.", "Title observations never assign family or target identity.", "Current source revisions remain unverified; revision IDs/timestamps are pinned observations only.", "The complete row ledger is retained only in the bounded workflow artifact, not committed."],
    }
    return ledger, compact


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input-zip", required=True, type=pathlib.Path)
    parser.add_argument("--ledger-output", required=True, type=pathlib.Path)
    parser.add_argument("--manifest-output", required=True, type=pathlib.Path)
    args = parser.parse_args()
    manifest, universe = load_g3(args.input_zip)
    ledger, compact = build_ledger(manifest, universe)
    args.ledger_output.write_bytes(canonical_bytes(ledger) + b"\n")
    args.manifest_output.write_bytes(canonical_bytes(compact) + b"\n")
    print(json.dumps(compact["counts"], sort_keys=True))
    print(f"ledger_sha256={compact['ledger_sha256']}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
