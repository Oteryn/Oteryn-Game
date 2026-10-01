#!/usr/bin/env python3
"""Audit every client imbuement-slot item against pinned community/OTS facts.

Only names and observed slot/type/tier facts are retained from external sources.
This is an offline authoring census, never a runtime allowlist or Global proof.
Missing sources remain missing; OTS defaults never substitute for Global facts.
"""
from __future__ import annotations

import argparse
from collections import Counter
from datetime import date
import importlib.util
import json
from pathlib import Path

import binding_evidence as binding

HERE = Path(__file__).resolve().parent
OUTPUT = HERE / "samples/imbuement-eligibility.json"
TARGET = "global-tibia-observable-2026-07-28-post-server-save"
TARGET_DATE = date(2026, 7, 28)
WIKI_ALIASES = {"souleater (axe)": "souleater"}
CANONICAL_ALIASES = {6527: ("avenger", "the avenger"), 8024: ("devileye", "the devileye"),
                     8025: ("ironworker", "the ironworker"), 8101: ("stomper", "the stomper"),
                     8103: ("epiphany", "the epiphany")}
TYPES = tuple("bash blockade chop cloud_fabric demon_presence dragon_hide electrify epiphany "
              "featherweight frost lich_shroud precision punch quara_scale reap scorch slash "
              "snake_skin strike swiftness vampirism venom vibrancy void".split())
ENGINE_CATEGORIES = {
    "elemental damage": ["electrify", "frost", "reap", "scorch", "venom"],
    "life leech": ["vampirism"], "mana leech": ["void"], "critical hit": ["strike"],
    "elemental protection death": ["lich_shroud"], "elemental protection earth": ["snake_skin"],
    "elemental protection fire": ["dragon_hide"], "elemental protection ice": ["quara_scale"],
    "elemental protection energy": ["cloud_fabric"], "elemental protection holy": ["demon_presence"],
    "increase speed": ["swiftness"], "increase capacity": ["featherweight"],
    "skillboost axe": ["chop"], "skillboost sword": ["slash"], "skillboost club": ["bash"],
    "skillboost shielding": ["blockade"], "skillboost distance": ["precision"],
    "skillboost magic level": ["epiphany"], "skillboost fist": ["punch"],
    "paralysis deflection": ["vibrancy"], "paralysis removal": ["vibrancy"],
}


def normalize(name: str) -> str:
    return name.casefold().strip()


def primary_items() -> list[dict]:
    spec = importlib.util.spec_from_file_location(
        "slot_census", binding.ROOT / "tools/content-census/stage_proficiencies.py")
    assert spec and spec.loader
    parser = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(parser)
    raw = binding.APPEARANCES.read_bytes()
    if binding.sha256(raw) != binding.APPEARANCES_SHA256:
        raise ValueError("primary client appearances digest mismatch")
    result, ids = [], set()
    for field, wire, record in parser.parse(raw):
        if (field, wire) != (1, 2):
            continue
        fields = parser.parse(record)
        object_ids = [v for f, w, v in fields if (f, w) == (1, 0)]
        names = [v.decode("utf-8") for f, w, v in fields if (f, w) == (4, 2)]
        flags = [v for f, w, v in fields if (f, w) == (3, 2)]
        if len(object_ids) != 1 or object_ids[0] in ids or len(names) > 1 or len(flags) > 1:
            raise ValueError("duplicate/malformed primary object identity")
        ids.add(object_ids[0])
        slot_fields = [v for f, w, v in parser.parse(flags[0]) if (f, w) == (60, 2)] if flags else []
        if not slot_fields:
            continue
        if len(slot_fields) != 1:
            raise ValueError("duplicate slot field")
        values = parser.parse(slot_fields[0])
        if len(values) != 1 or values[0][:2] != (1, 0) or values[0][2] not in (1, 2, 3):
            raise ValueError("unsupported primary slot message")
        result.append({"id": object_ids[0], "name": names[0] if names else None,
                       "slots": values[0][2], "record_sha256": binding.sha256(record)})
    if len(result) != 663 or Counter(r["slots"] for r in result) != {1: 253, 2: 286, 3: 124}:
        raise ValueError("pinned primary slot census changed")
    names = [normalize(r["name"]) for r in result if r["name"]]
    if len(names) != len(set(names)):
        raise ValueError("ambiguous primary item names")
    return sorted(result, key=lambda r: r["id"])


def engine_types(categories: dict) -> dict:
    result = {}
    for category, tier in categories.items():
        if category not in ENGINE_CATEGORIES or type(tier) is not int or tier < 1:
            raise ValueError(f"unsupported engine category/tier {category}: {tier}")
        if tier not in (1, 2, 3):
            continue  # Preserve out-of-domain raw limits separately; never cap or guess.
        for key in ENGINE_CATEGORIES[category]:
            result[key] = tier
    return dict(sorted(result.items()))


def build(packet: dict) -> dict:
    sources = packet["normalized_sources"]
    primary = primary_items()
    if sources["primary_items"] != primary:
        raise ValueError("stored primary facts differ from pinned client asset")
    canonical = binding.canonical_items()
    identity_facts = json.loads(binding.ITEM_FACTS.read_bytes())["records"]
    release_dates = sources.get("release_dates", {})
    for release in release_dates.values():
        date.fromisoformat(release["date"])
        if release["source_id"] not in packet["source_registry"]:
            raise ValueError("release date lacks its pinned source")
        if release["date"] != packet["source_registry"][release["source_id"]]["release_date"]:
            raise ValueError("release date differs from pinned source metadata")
    amendments = {}
    for amendment in sources.get("type_amendments", []):
        if amendment["candidate_key"] not in TYPES or type(amendment["max_tier"]) is not int or amendment["max_tier"] not in (1, 2, 3):
            raise ValueError("invalid sourced type amendment")
        if any(s not in packet["source_registry"] for s in amendment["source_ids"]):
            raise ValueError("type amendment lacks source evidence")
        amendments.setdefault(amendment["client_id"], []).append(amendment)
    wiki_by_name = {}
    for source_id, table in sources["wiki_tables"].items():
        for row in table:
            allowed = row["allowed_types"]
            if not allowed or any(k not in TYPES or type(t) is not int or t not in (1, 2, 3)
                                  for k, t in allowed.items()):
                raise ValueError("invalid community allowed type/tier")
            if type(row["slots"]) is not int or row["slots"] not in (1, 2, 3):
                raise ValueError("invalid community slot count")
            wiki_name = normalize(row["name"])
            wiki_by_name.setdefault(WIKI_ALIASES.get(wiki_name, wiki_name), []).append((source_id, row))
    engines = {}
    for source_id, rows in sources["engines"].items():
        if len({r["id"] for r in rows}) != len(rows):
            raise ValueError("duplicate engine item identity")
        engines[source_id] = {r["id"]: r for r in rows}
    items = []
    for client in primary:
        name, item_id = client["name"], client["id"]
        key = f"oteryn:item.tibia.i{item_id}"
        definition = canonical.get(key, {}).get("definition")
        item_ref, binding_status = None, "CANONICAL_ITEM_ABSENT"
        identity_evidence = {}
        if definition is not None:
            identity = definition["identity"]
            presentation = definition.get("semantics", {}).get("presentation", {}).get("value", {}).get("name", {})
            canonical_name = presentation.get("value") if presentation.get("state") == "KNOWN" else None
            if identity != {"family": "Item", "key": key, "revision": "definition-r1"}:
                raise ValueError(f"unsupported canonical identity {key}")
            fact = identity_facts.get(key, {})
            wiki_identity = [o for o in fact.get("observations", []) if normalize(o["wiki_title"]) == normalize(name)]
            if wiki_identity and fact["item_id"] != item_id:
                raise ValueError("revisioned named identity fact has conflicting item ID")
            identity_evidence = {"canonical_path": canonical[key]["path"],
                                 "canonical_definition_sha256": canonical[key]["definition_sha256"],
                                 "canonical_name": canonical_name}
            if canonical_name is not None and normalize(canonical_name) != normalize(name):
                binding_status = "CANONICAL_CLIENT_NAME_DISPUTE"
                if CANONICAL_ALIASES.get(item_id) == (normalize(canonical_name), normalize(name)) and wiki_identity:
                    item_ref, binding_status = identity, "EXPLICIT_ARTICLE_ALIAS_AND_REVISIONED_WIKI"
            elif canonical_name is not None:
                item_ref, binding_status = identity, "CLIENT_AND_CANONICAL_NAME"
            else:
                binding_status = "CANONICAL_NAMED_IDENTITY_UNPROVEN"
                if wiki_identity:
                    item_ref, binding_status = identity, "CLIENT_AND_REVISIONED_WIKI_CANONICAL_EXISTS"
            if wiki_identity:
                identity_evidence["wiki_observations"] = [{k: o[k] for k in ("wiki_title", "page_id", "revision_id",
                    "revision_timestamp", "content_sha256")} for o in wiki_identity]
        wiki_rows = wiki_by_name.get(normalize(name), [])
        allowed, evidence, discrepancies = {}, [], []
        selected_source = None
        community_comparison = {source_id: {"slots": row["slots"], "allowed_types": row["allowed_types"]}
                                for source_id, row in wiki_rows}
        if wiki_rows:
            claims = {binding.canonical_bytes(row["allowed_types"]) for _, row in wiki_rows}
            if len(claims) != 1:
                discrepancies.append({"kind": "COMMUNITY_TYPE_TIER_DISPUTE", "sources": [s for s, _ in wiki_rows]})
            native = [(s, r) for s, r in wiki_rows if s.startswith("tibiopedia_i")]
            if len(native) > 1:
                raise ValueError("ambiguous direct native table selection")
            selected_source, selected_row = native[0] if native else wiki_rows[0]
            if native and selected_row["slots"] != client["slots"]:
                raise ValueError("direct item table slots differ from primary")
            allowed = dict(sorted(selected_row["allowed_types"].items()))
            evidence = [source_id for source_id, _ in wiki_rows]
            for source_id, row in wiki_rows:
                if row["slots"] != client["slots"]:
                    discrepancies.append({"kind": "PRIMARY_WIKI_SLOT_DISPUTE", "source": source_id,
                                          "primary_slots": client["slots"], "source_slots": row["slots"]})
        comparisons = {}
        selected_amendments = []
        for amendment in amendments.get(item_id, []):
            if normalize(amendment["item_name"]) != normalize(name):
                raise ValueError("type amendment has conflicting named identity")
            prior = allowed.get(amendment["candidate_key"])
            if prior is None:
                raise ValueError("partial type amendment cannot invent an unobserved item profile")
            allowed[amendment["candidate_key"]] = amendment["max_tier"]
            selected_amendments.append(amendment)
            discrepancies.append({"kind": "DATED_ITEM_TYPE_TIER_AMENDMENT", "candidate_key": amendment["candidate_key"],
                                  "helper_max_tier": prior, "selected_max_tier": amendment["max_tier"],
                                  "sources": amendment["source_ids"]})
        for source_id, rows in engines.items():
            row = rows.get(item_id)
            if row is None:
                comparisons[source_id] = {"status": "NO_ENGINE_ENTRY"}
                continue
            if normalize(row["name"]) != normalize(name):
                comparisons[source_id] = {"status": "ID_NAME_MISMATCH", "source_name": row["name"]}
                continue
            hypothesis = engine_types(row["categories"])
            comparisons[source_id] = {"status": "OTS_HYPOTHESIS_ONLY", "slots": row["slots"],
                                      "allowed_types": hypothesis}
            invalid = {k: t for k, t in row["categories"].items() if t not in (1, 2, 3)}
            if invalid:
                comparisons[source_id]["unqualified_raw_tier_limits"] = invalid
                discrepancies.append({"kind": "ENGINE_TIER_LIMIT_OUTSIDE_AUTHORING_DOMAIN", "source": source_id,
                                      "raw_tier_limits": invalid})
            if row.get("slot_attribute_count", 1) > 1:
                discrepancies.append({"kind": "ENGINE_DUPLICATE_SLOT_ATTRIBUTES", "source": source_id,
                                      "count": row["slot_attribute_count"], "slot_values": row["slot_values"]})
            if row["slots"] != client["slots"]:
                discrepancies.append({"kind": "PRIMARY_ENGINE_SLOT_DISPUTE", "source": source_id,
                                      "primary_slots": client["slots"], "source_slots": row["slots"]})
            if allowed and hypothesis != allowed:
                discrepancies.append({"kind": "WIKI_ENGINE_TYPE_TIER_DISPUTE", "source": source_id,
                                      "wiki_only": {k: t for k, t in allowed.items() if hypothesis.get(k) != t},
                                      "engine_only": {k: t for k, t in hypothesis.items() if allowed.get(k) != t}})
        status = "COMMUNITY_CORROBORATED" if allowed else (
            "OTS_HYPOTHESIS_ONLY" if any(c["status"] == "OTS_HYPOTHESIS_ONLY" for c in comparisons.values())
            else "MISSING_ELIGIBILITY_EVIDENCE")
        if allowed and not any(row["slots"] == client["slots"] for _, row in wiki_rows):
            status = "PRIMARY_WIKI_SLOT_DISPUTE"
        if allowed and selected_source.startswith("tibiopedia_i") and any(
                d["kind"] == "COMMUNITY_TYPE_TIER_DISPUTE" for d in discrepancies):
            status = "DERIVED_SELECTED_OVER_STALE_HELPER"
        elif allowed and not selected_source.startswith("tibiopedia_i"):
            status = "COMMUNITY_SINGLE_SOURCE"
        native_meta = packet["source_registry"].get(f"tibiopedia_i{item_id}", {})
        target_time_evidence = {}
        introduced = release_dates.get(native_meta.get("introduced_version"))
        withdrawn = release_dates.get(native_meta.get("withdrawn_version"))
        if withdrawn and date.fromisoformat(withdrawn["date"]) <= TARGET_DATE:
            target_time_status = "WITHDRAWN_BEFORE_TARGET_EXCLUDED"
            target_time_evidence = {"version": native_meta["withdrawn_version"], **withdrawn}
        elif introduced and date.fromisoformat(introduced["date"]) > TARGET_DATE:
            target_time_status = "POST_TARGET_RELEASE_EXCLUDED"
            target_time_evidence = {"version": native_meta["introduced_version"], **introduced}
        elif introduced:
            target_time_status = "PRE_TARGET_RELEASE_CONTINUITY_UNVERIFIED"
            target_time_evidence = {"version": native_meta["introduced_version"], **introduced}
        else:
            prior_observations = [o for o in identity_evidence.get("wiki_observations", [])
                                  if date.fromisoformat(o["revision_timestamp"][:10]) <= TARGET_DATE]
            if prior_observations:
                target_time_status = "PRE_TARGET_NAMED_OBSERVATION_CONTINUITY_UNVERIFIED"
                target_time_evidence = {"observations": prior_observations}
            else:
                target_time_status = "TARGET_RELEASE_DATE_UNPROVEN"
                historical_helpers = [(source_id, packet["source_registry"][source_id]) for source_id, _ in wiki_rows
                                      if packet["source_registry"][source_id].get("revision_date") and
                                      date.fromisoformat(packet["source_registry"][source_id]["revision_date"]) <= TARGET_DATE]
                if historical_helpers:
                    target_time_status = "PRE_TARGET_DERIVED_NAMED_RECORD_CONTINUITY_UNVERIFIED"
                    target_time_evidence = {"sources": [{"source_id": source_id, "revision": meta["revision"],
                                                       "date": meta["revision_date"]} for source_id, meta in historical_helpers]}
        if native_meta.get("withdrawn_version"):
            status = "RETIRED_SOURCE_ITEM_EXCLUDED"
        if selected_amendments and status == "COMMUNITY_SINGLE_SOURCE":
            status = "COMMUNITY_SINGLE_SOURCE_WITH_CORROBORATED_FIELD"
        evidence_status = status
        if item_ref is None:
            if status != "RETIRED_SOURCE_ITEM_EXCLUDED":
                status = binding_status
        if target_time_status == "POST_TARGET_RELEASE_EXCLUDED":
            status = "POST_TARGET_RELEASE_EXCLUDED"
        items.append({"client_id": item_id, "item_ref": item_ref, "name": name, "slots": client["slots"],
                      "slot_source": "primary_client", "binding_status": binding_status,
                      "identity_evidence": identity_evidence,
                      "allowed_types": allowed, "eligibility_sources": evidence, "status": status,
                      "evidence_status": evidence_status, "selected_eligibility_source": selected_source,
                      "selected_type_amendments": selected_amendments,
                      "target_time_status": target_time_status, "target_time_evidence": target_time_evidence,
                      "community_comparison": community_comparison,
                      "engine_comparison": comparisons, "discrepancies": discrepancies})
    primary_names = {normalize(r["name"]) for r in primary}
    summary = {"primary_items": len(items), "canonical_bound_items": sum(r["item_ref"] is not None for r in items),
               "community_table_rows": sum(len(t) for t in sources["wiki_tables"].values()),
               "wiki_tool_table_rows": sum(len(t) for s, t in sources["wiki_tables"].items() if s.startswith("wiki_")),
               "native_item_table_rows": sum(len(t) for s, t in sources["wiki_tables"].items() if s.startswith("tibiopedia_")),
               "community_named_matches": sum(bool(r["community_comparison"]) for r in items),
               "status_counts": dict(sorted(Counter(r["status"] for r in items).items())),
               "evidence_status_counts": dict(sorted(Counter(r["evidence_status"] for r in items).items())),
               "typed_items": sum(bool(r["allowed_types"]) for r in items),
               "target_time_status_counts": dict(sorted(Counter(r["target_time_status"] for r in items).items())),
               "target_candidate_typed_items": sum(bool(r["allowed_types"]) and r["target_time_status"] not in (
                    "POST_TARGET_RELEASE_EXCLUDED", "WITHDRAWN_BEFORE_TARGET_EXCLUDED") for r in items),
               "target_candidate_bound_items": sum(bool(r["allowed_types"]) and r["item_ref"] is not None and
                    r["target_time_status"] not in ("POST_TARGET_RELEASE_EXCLUDED", "WITHDRAWN_BEFORE_TARGET_EXCLUDED")
                    for r in items),
               "discrepancy_counts": dict(sorted(Counter(d["kind"] for r in items for d in r["discrepancies"]).items())),
               "qualified_items_per_type": {k: sum(r["item_ref"] is not None and r["evidence_status"] in (
                                                  "COMMUNITY_CORROBORATED", "DERIVED_SELECTED_OVER_STALE_HELPER",
                                                  "COMMUNITY_SINGLE_SOURCE", "COMMUNITY_SINGLE_SOURCE_WITH_CORROBORATED_FIELD")
                                                  and r["target_time_status"] not in ("POST_TARGET_RELEASE_EXCLUDED",
                                                  "WITHDRAWN_BEFORE_TARGET_EXCLUDED") and k in r["allowed_types"]
                                                  for r in items) for k in TYPES},
               "global_verified_items": 0}
    return {"schema": "OTERYN_IMBUEMENT_ELIGIBILITY_EVIDENCE/v1", "activation": "DRAFT_NOT_RUNTIME_READY",
            "target": TARGET, "source_registry": packet["source_registry"], "normalized_sources": sources,
            "normalized_sources_sha256": binding.sha256(binding.canonical_bytes(sources)),
            "qualification_policy": "Draft selection prefers explicit current per-item Tibiopedia tables matching primary "
                "client slots over older Imbuement Tool helper tables (21 stale slot counts, pre-Monk missing Punch and "
                "incorrect built-in-effect exclusions). Both claims and every discrepancy remain preserved. "
                "DERIVED_SELECTED_OVER_STALE_HELPER is a source candidate choice, never verified Global parity or runtime "
                "admission. BR-only fallback is COMMUNITY_SINGLE_SOURCE. ItemRefs independently require named identity "
                "and canonical existence. Global server eligibility and target-date continuity remain unverified; "
                "OTS-only hypotheses fail closed; empty allowed_types means unknown, never forbidden. "
                "Dated named-item amendments correct only their documented fields, without promoting the remaining "
                "profile. Post-target launches and pre-target withdrawals are excluded from target summaries while "
                "current client facts remain retained; a dated named observation establishes chronology, not server "
                "rule continuity at the target server save.",
            "items": items, "summary": summary,
            "rejected_sources": packet["rejected_sources"],
            "unmatched_wiki_items": sorted(name for name in wiki_by_name if name not in primary_names)}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--catalogue", type=Path, default=OUTPUT)
    args = parser.parse_args()
    packet = json.loads(args.catalogue.read_bytes())
    expected = build(packet)
    if args.check:
        if packet != expected:
            raise ValueError("eligibility census differs from deterministic offline evidence replay")
    else:
        args.catalogue.write_text(json.dumps(expected, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(expected["summary"], sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
