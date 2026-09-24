#!/usr/bin/env python3
"""CW2-B1 deterministic Crystal item identity/source catalogue.

This importer is deliberately candidate-only. It verifies the exact pinned
Crystal Git objects, expands admitted source identities without minting Oteryn
keys, classifies source fields against GAME-ITEM semantic families, and emits
deterministic evidence suitable for CW3 review/promotion.
"""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import subprocess
from typing import Any, Iterable
import xml.etree.ElementTree as ET

SCHEMA = "OTERYN_CW2_ITEM_IDENTITY_CATALOG_SOURCE_BATCH/v1"
MAPPER_PROFILE = "OTERYN_CW2_ITEM_IDENTITY_CATALOG_MAPPER/v1"
SOURCE_REPOSITORY = "zimbadev/crystalserver"
SOURCE_REVISION = "ff7ede593c69d4c658b382c97443e8155926924a"
SOURCE_CLASSIFICATION = "CRYSTAL_OTS / OTS_HYPOTHESIS_ONLY"
CLOSURE = "CANDIDATE_ONLY"

SOURCE_FILES = {
    "data/items/items.xml": {
        "blob": "0b1dc3ba1a49094d9c83b90ab399bd2a9dd7a17f",
        "size": 3819874,
        "role": "PRIMARY_ITEM_CATALOGUE",
    },
    "src/items/functions/item/item_parse.cpp": {
        "blob": "d95e2c83f44a81e7da7dc0d066c3b6f3a3302426",
        "size": 59234,
        "role": "INTERPRETER_REFERENCE",
    },
    "src/items/functions/item/item_parse.hpp": {
        "blob": "e22ede24f58aac0be6f85cd8d8167a6aa4341974",
        "size": 20529,
        "role": "INTERPRETER_REFERENCE",
    },
    "src/items/items_definitions.hpp": {
        "blob": "c09a62b585766909045e0fff5cf770e5550d333c",
        "size": 16234,
        "role": "SCHEMA_REFERENCE",
    },
    "LICENSE": {
        "blob": "d159169d1050894d3ea3b98e1c965c4058208fe1",
        "size": 18092,
        "role": "SOURCE_RIGHTS_LOCATOR",
    },
}

MAPPER_REL = "tools/reference-world-corridor-census/item_identity_catalog.py"

ADMITTED_NATIVE_BINDINGS: tuple[dict[str, Any], ...] = ()

PROVENANCE_ONLY = {
    "description", "showcount", "showattributes", "showcharges",
    "shoottype", "effect", "primarytype", "loottype", "runespellname",
}
EXCLUDED_BY_POLICY = {"script"}
UNSUPPORTED = {
    "bedpart", "partnerdirection", "bedpartof", "floorchange", "leveldoor",
    "usedbyhouseguests", "blocking", "blockprojectile", "walkstack",
    "replaceable", "field",
}

DIRECT_GAME_ITEM_FIELDS = {
    "type": ("classification", "item_type"),
    "weight": ("physical", "weight"),
    "movable": ("physical", "movable"),
    "allowpickupable": ("physical", "pickup_eligibility"),
    "containersize": ("container", "capacity"),
    "weapontype": ("weapon_use", "weapon_type"),
    "slottype": ("equipment", "slot_claim"),
    "ammotype": ("weapon_use", "ammo_type"),
    "range": ("weapon_use", "range"),
    "hitchance": ("weapon_use", "hit_chance"),
    "maxhitchance": ("weapon_use", "max_hit_chance"),
    "attack": ("weapon_use", "attack"),
    "defense": ("protection", "defense"),
    "extradef": ("protection", "extra_defense"),
    "armor": ("protection", "armor"),
    "charges": ("charges", "charge_count"),
    "duration": ("temporal", "duration"),
    "stopduration": ("temporal", "stop_duration"),
    "decayto": ("temporal", "decay_target_source_id"),
    "rotateto": ("transform", "rotate_target_source_id"),
    "wrapableto": ("transform", "wrap_target_source_id"),
    "transformonuse": ("transform", "use_target_source_id"),
    "transformequipto": ("transform", "equip_target_source_id"),
    "transformdeequipto": ("transform", "deequip_target_source_id"),
    "maletransformto": ("transform", "male_transform_target_source_id"),
    "femaletransformto": ("transform", "female_transform_target_source_id"),
    "destroyto": ("transform", "destroy_target_source_id"),
    "fluidsource": ("fluid", "fluid_source"),
    "readable": ("read_write", "readable"),
    "writeable": ("read_write", "writeable"),
    "maxtextlen": ("read_write", "max_text_length"),
    "allowdistread": ("read_write", "allow_distance_read"),
    "writeonceitemid": ("read_write", "write_once_target_source_id"),
    "imbuementslot": ("imbuement", "slot_and_allowed_family_tier"),
    "augments": ("modifier", "augment_binding"),
    "elementalbond": ("modifier", "elemental_bond"),
    "mantra": ("modifier", "mantra"),
    "invisible": ("modifier", "invisibility"),
    "manashield": ("modifier", "mana_shield"),
    "suppressdrunk": ("modifier", "suppress_drunk"),
    "suppressdrown": ("modifier", "suppress_drown"),
    "meleeattackeffect": ("presentation_binding", "melee_attack_effect"),
}

MODIFIER_PREFIXES = (
    "absorbpercent", "fieldabsorbpercent", "skill", "element",
    "criticalhit", "lifeleech", "manaleech", "perfectshot",
)
MODIFIER_EXACT = {
    "speed", "magiclevelpoints", "firemagiclevelpoints",
    "earthmagiclevelpoints", "healingmagiclevelpoints",
    "energymagiclevelpoints", "holymagiclevelpoints",
    "icemagiclevelpoints", "deathmagiclevelpoints",
    "healthticks", "healthgain", "manaticks", "managain",
    "cleavepercent", "reflectdamage", "magicshieldcapacityflat",
    "magicshieldcapacitypercent",
}
PRESENTATION_ONLY = {"showduration", "showattributes", "showcharges", "showcount"}

class CatalogError(RuntimeError):
    pass

def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")

def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()

def _git(repo: Path, *args: str, binary: bool = False) -> bytes | str:
    result = subprocess.run(
        ("git", "-C", str(repo), *args),
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if binary:
        return result.stdout
    return result.stdout.decode("utf-8").strip()

def _normalize_remote(url: str) -> str:
    value = url.strip().lower().replace("\\", "/")
    if value.endswith(".git"):
        value = value[:-4]
    if value.startswith("git@github.com:"):
        value = "https://github.com/" + value.split(":", 1)[1]
    return value.rstrip("/")

def verify_source_repository(source_repo: Path) -> tuple[dict[str, Any], dict[str, bytes]]:
    source_repo = source_repo.resolve()
    try:
        top = Path(str(_git(source_repo, "rev-parse", "--show-toplevel"))).resolve()
        remote = _normalize_remote(str(_git(source_repo, "remote", "get-url", "origin")))
        resolved_revision = str(_git(source_repo, "rev-parse", SOURCE_REVISION))
    except (OSError, subprocess.CalledProcessError) as exc:
        raise CatalogError("SOURCE_REPOSITORY_UNVERIFIABLE") from exc
    if top != source_repo:
        raise CatalogError(f"SOURCE_REPOSITORY_ROOT_MISMATCH: {top}")
    if remote != f"https://github.com/{SOURCE_REPOSITORY}".lower():
        raise CatalogError(f"SOURCE_REPOSITORY_REMOTE_MISMATCH: {remote}")
    if resolved_revision != SOURCE_REVISION:
        raise CatalogError("SOURCE_REVISION_MISMATCH")

    snapshot: dict[str, Any] = {
        "repository": SOURCE_REPOSITORY,
        "revision": SOURCE_REVISION,
        "classification": SOURCE_CLASSIFICATION,
        "files": [],
        "rights": {"license_locator": "LICENSE", "license_text": "GNU GPL v2"},
    }
    raw: dict[str, bytes] = {}
    for path, expected in SOURCE_FILES.items():
        try:
            blob = str(_git(source_repo, "rev-parse", f"{SOURCE_REVISION}:{path}"))
            payload = _git(source_repo, "cat-file", "blob", blob, binary=True)
        except (OSError, subprocess.CalledProcessError) as exc:
            raise CatalogError(f"SOURCE_OBJECT_UNAVAILABLE: {path}") from exc
        assert isinstance(payload, bytes)
        if blob != expected["blob"]:
            raise CatalogError(f"SOURCE_BLOB_MISMATCH: {path}: {blob}")
        if len(payload) != expected["size"]:
            raise CatalogError(
                f"SOURCE_SIZE_MISMATCH: {path}: expected {expected['size']}, got {len(payload)}"
            )
        raw[path] = payload
        snapshot["files"].append(
            {
                "path": path,
                "blob": blob,
                "size": len(payload),
                "sha256": sha256_bytes(payload),
                "role": expected["role"],
            }
        )
    if b"GNU GENERAL PUBLIC LICENSE" not in raw["LICENSE"] or b"Version 2, June 1991" not in raw["LICENSE"]:
        raise CatalogError("SOURCE_LICENSE_TEXT_MISMATCH")
    return snapshot, raw

def field_disposition(source_key: str) -> dict[str, str | None]:
    normalized = source_key.casefold()
    if normalized in EXCLUDED_BY_POLICY:
        return {
            "disposition": "EXCLUDED_BY_POLICY",
            "semantic_family": None,
            "native_field": None,
            "reason": "runtime/script authority is outside CW2 candidate data",
        }
    if normalized in UNSUPPORTED:
        return {
            "disposition": "UNSUPPORTED",
            "semantic_family": None,
            "native_field": None,
            "reason": "field belongs to world/interaction/housing semantics, not current GAME-ITEM B1",
        }
    if normalized in PROVENANCE_ONLY or normalized in PRESENTATION_ONLY:
        return {
            "disposition": "PROVENANCE_ONLY",
            "semantic_family": "presentation_or_source_taxonomy",
            "native_field": None,
            "reason": "not canonical item identity or authoritative GAME-ITEM semantics",
        }
    direct = DIRECT_GAME_ITEM_FIELDS.get(normalized)
    if direct is not None:
        return {
            "disposition": "GAME_ITEM_CANDIDATE",
            "semantic_family": direct[0],
            "native_field": direct[1],
            "reason": "typed candidate only; source classification remains OTS_HYPOTHESIS_ONLY",
        }
    if normalized in MODIFIER_EXACT or any(normalized.startswith(prefix) for prefix in MODIFIER_PREFIXES):
        return {
            "disposition": "GAME_ITEM_CANDIDATE",
            "semantic_family": "modifier",
            "native_field": f"modifier.{normalized}",
            "reason": "typed modifier candidate only; formula/units are not promoted",
        }
    return {
        "disposition": "UNKNOWN",
        "semantic_family": None,
        "native_field": None,
        "reason": "no admitted B1 semantic mapping",
    }

def _canonical_node(node: ET.Element) -> dict[str, Any]:
    outer = {key: node.attrib[key] for key in sorted(node.attrib)}
    attributes = []
    for child in node.findall("attribute"):
        attributes.append(
            {
                "attributes": {key: child.attrib[key] for key in sorted(child.attrib)},
                "children": sorted(
                    ({key: grand.attrib[key] for key in sorted(grand.attrib)} for grand in list(child)),
                    key=lambda value: canonical_bytes(value),
                ),
            }
        )
    attributes.sort(key=canonical_bytes)
    return {"outer": outer, "attributes": attributes}

def _node_ids(node: ET.Element) -> tuple[list[int], dict[str, Any] | None]:
    if "id" in node.attrib:
        if "fromid" in node.attrib or "toid" in node.attrib:
            raise CatalogError("SOURCE_ID_AND_RANGE_BOTH_PRESENT")
        try:
            value = int(node.attrib["id"])
        except ValueError as exc:
            raise CatalogError("SOURCE_ID_NOT_INTEGER") from exc
        if not 0 <= value <= 65535:
            raise CatalogError(f"SOURCE_ID_OUT_OF_RANGE: {value}")
        return [value], None

    if "fromid" not in node.attrib or "toid" not in node.attrib:
        raise CatalogError("SOURCE_NODE_WITHOUT_COMPLETE_IDENTITY")
    try:
        start = int(node.attrib["fromid"])
        end = int(node.attrib["toid"])
    except ValueError as exc:
        raise CatalogError("SOURCE_RANGE_NOT_INTEGER") from exc
    if not (0 <= start <= 65535 and 0 <= end <= 65535):
        raise CatalogError(f"SOURCE_RANGE_OUT_OF_RANGE: {start}..{end}")
    if start > end:
        digest = sha256_bytes(canonical_bytes(_canonical_node(node)))
        return [], {
            "kind": "REVERSED_RANGE_EXCLUDED",
            "fromid": start,
            "toid": end,
            "source_node_digest": digest,
            "reason": "range direction is invalid for deterministic ascending expansion; no repair is inferred",
        }
    return list(range(start, end + 1)), None

def _observation_signature(child: ET.Element) -> str:
    value = {
        "attributes": {key: child.attrib[key] for key in sorted(child.attrib)},
        "children": sorted(
            ({key: grand.attrib[key] for key in sorted(grand.attrib)} for grand in list(child)),
            key=lambda item: canonical_bytes(item),
        ),
    }
    return sha256_bytes(canonical_bytes(value))

def resolve_native_mapping(
    source_item_id: int,
    binding_candidates: Iterable[dict[str, Any]],
) -> dict[str, Any]:
    candidates = [value for value in binding_candidates if value.get("source_item_id") == source_item_id]
    if not candidates:
        return {
            "disposition": "UNRESOLVED",
            "content_key": None,
            "evidence_refs": [],
            "reason": "no admitted Game-owned explicit source-to-native binding",
        }

    validated = []
    for value in candidates:
        target = value.get("content_key")
        evidence_ref = value.get("evidence_ref")
        state = value.get("state", "ASSERTED")
        if not isinstance(target, str) or not target.startswith("oteryn:"):
            raise CatalogError("BINDING_TARGET_INVALID")
        if not isinstance(evidence_ref, str) or not evidence_ref:
            raise CatalogError("BINDING_EVIDENCE_REF_REQUIRED")
        if state not in {"ASSERTED", "CANDIDATE"}:
            raise CatalogError("BINDING_STATE_INVALID")
        validated.append((target, evidence_ref, state))

    targets = sorted({target for target, _, _ in validated})
    refs = sorted({ref for _, ref, _ in validated})
    if len(targets) == 1 and all(state == "ASSERTED" for _, _, state in validated):
        return {
            "disposition": "RESOLVED",
            "content_key": targets[0],
            "evidence_refs": refs,
            "reason": "explicit admitted Game-owned binding",
        }
    if any(state == "ASSERTED" for _, _, state in validated):
        return {
            "disposition": "CONFLICT",
            "content_key": None,
            "candidate_content_keys": targets,
            "evidence_refs": refs,
            "reason": "admitted binding evidence disagrees",
        }
    return {
        "disposition": "AMBIGUOUS",
        "content_key": None,
        "candidate_content_keys": targets,
        "evidence_refs": refs,
        "reason": "multiple candidate targets remain without authoritative selection",
    }

def build_semantic_catalog(
    xml_bytes: bytes,
    binding_candidates: Iterable[dict[str, Any]] = ADMITTED_NATIVE_BINDINGS,
) -> dict[str, Any]:
    try:
        root = ET.fromstring(xml_bytes)
    except ET.ParseError as exc:
        raise CatalogError("ITEMS_XML_PARSE_FAILED") from exc
    if root.tag != "items":
        raise CatalogError(f"ITEMS_XML_ROOT_INVALID: {root.tag}")

    records: list[dict[str, Any]] = []
    exclusions: list[dict[str, Any]] = []
    seen_ids: dict[int, str] = {}
    name_members: dict[str, set[int]] = defaultdict(set)
    raw_key_spellings: dict[str, set[str]] = defaultdict(set)
    field_source_counts: Counter[str] = Counter()
    field_expanded_counts: Counter[str] = Counter()
    nested_source_counts: Counter[str] = Counter()
    disposition_expanded_counts: Counter[str] = Counter()
    conflict_records: list[dict[str, Any]] = []
    semantic_node_records: dict[str, dict[str, Any]] = {}
    direct_nodes = 0
    range_nodes = 0
    reversed_ranges = 0

    nodes = list(root.findall("item"))
    for node in nodes:
        ids, exclusion = _node_ids(node)
        if "id" in node.attrib:
            direct_nodes += 1
        else:
            range_nodes += 1
        if exclusion is not None:
            reversed_ranges += 1
            exclusions.append(exclusion)
            continue

        node_data = _canonical_node(node)
        node_digest = sha256_bytes(canonical_bytes(node_data))
        normalized_name = node.attrib.get("name", "").strip().casefold()
        if normalized_name:
            name_members[normalized_name].update(ids)

        field_children = list(node.findall("attribute"))
        per_native: dict[str, set[str]] = defaultdict(set)
        field_summary = Counter()
        field_keys = set()
        field_key_counts: Counter[str] = Counter()
        candidate_observations: list[dict[str, Any]] = []

        for child in field_children:
            source_key = child.attrib.get("key")
            if not isinstance(source_key, str) or not source_key:
                disposition = {
                    "disposition": "UNKNOWN",
                    "semantic_family": None,
                    "native_field": None,
                    "reason": "attribute node lacks key",
                }
                normalized_key = "<missing-key>"
            else:
                normalized_key = source_key.casefold()
                disposition = field_disposition(source_key)
                raw_key_spellings[normalized_key].add(source_key)
            field_keys.add(normalized_key)
            field_key_counts[normalized_key] += 1
            field_source_counts[normalized_key] += 1
            nested_source_counts[normalized_key] += len(list(child))
            if disposition["native_field"]:
                per_native[str(disposition["native_field"])].add(_observation_signature(child))
            if disposition["disposition"] == "GAME_ITEM_CANDIDATE":
                candidate_observations.append(
                    {
                        "source_key": source_key,
                        "normalized_source_key": normalized_key,
                        "semantic_family": disposition["semantic_family"],
                        "native_field": disposition["native_field"],
                        "source_value": child.attrib.get("value"),
                        "nested_values": sorted(
                            (
                                {key: grand.attrib[key] for key in sorted(grand.attrib)}
                                for grand in list(child)
                            ),
                            key=canonical_bytes,
                        ),
                    }
                )
            field_summary[str(disposition["disposition"])] += 1

        candidate_observations.sort(key=canonical_bytes)
        if candidate_observations:
            semantic_record = {
                "source_node_digest": node_digest,
                "candidate_fields": candidate_observations,
            }
            previous_semantic = semantic_node_records.get(node_digest)
            if previous_semantic is not None and previous_semantic != semantic_record:
                raise CatalogError("SOURCE_NODE_SEMANTIC_PROFILE_CONFLICT")
            semantic_node_records[node_digest] = semantic_record

        conflicting_native_fields = {
            native_field for native_field, values in per_native.items() if len(values) > 1
        }
        if conflicting_native_fields:
            for source_item_id in ids:
                for native_field in sorted(conflicting_native_fields):
                    conflict_records.append(
                        {
                            "source_item_id": source_item_id,
                            "native_field": native_field,
                            "reason": "multiple distinct source observations map to one B1 native candidate field",
                        }
                    )

        expansion_count = len(ids)
        for normalized_key, count in field_key_counts.items():
            field_expanded_counts[normalized_key] += count * expansion_count
        for disposition_name, count in field_summary.items():
            disposition_expanded_counts[disposition_name] += count * expansion_count

        for offset, source_item_id in enumerate(ids):
            previous = seen_ids.get(source_item_id)
            if previous is not None:
                raise CatalogError(
                    f"DUPLICATE_SOURCE_ID_AFTER_RANGE_EXPANSION: {source_item_id}: {previous} vs {node_digest}"
                )
            seen_ids[source_item_id] = node_digest
            mapping = resolve_native_mapping(source_item_id, binding_candidates)
            record_dispositions = dict(sorted(field_summary.items()))
            if conflicting_native_fields:
                record_dispositions["CONFLICT"] = len(conflicting_native_fields)
            records.append(
                {
                    "source_item_id": source_item_id,
                    "source_identity": {
                        "source_snapshot_ref": "source_snapshot.files[data/items/items.xml]",
                        "source_numeric_identity": source_item_id,
                    },
                    "source_node_digest": node_digest,
                    "range_member_offset": offset if len(ids) > 1 else None,
                    "field_keys": sorted(field_keys),
                    "field_disposition_counts": record_dispositions,
                    "native_mapping": mapping,
                }
            )

    records.sort(key=lambda value: value["source_item_id"])
    exclusions.sort(key=canonical_bytes)
    conflict_records.sort(key=canonical_bytes)

    mapping_counts = Counter(record["native_mapping"]["disposition"] for record in records)
    name_collisions = []
    for name, members in name_members.items():
        if len(members) <= 1:
            continue
        ordered = sorted(members)
        name_collisions.append(
            {
                "normalized_name_sha256": sha256_bytes(name.encode("utf-8")),
                "member_count": len(ordered),
                "sample_source_item_ids": ordered[:16],
            }
        )
    name_collisions.sort(key=canonical_bytes)

    field_records = []
    for normalized_key in sorted(field_source_counts):
        disposition = field_disposition(normalized_key) if normalized_key != "<missing-key>" else {
            "disposition": "UNKNOWN",
            "semantic_family": None,
            "native_field": None,
            "reason": "attribute node lacks key",
        }
        field_records.append(
            {
                "normalized_source_key": normalized_key,
                "source_key_spellings": sorted(raw_key_spellings.get(normalized_key, set())),
                "source_xml_observation_count": field_source_counts[normalized_key],
                "expanded_identity_observation_count": field_expanded_counts[normalized_key],
                "nested_source_observation_count": nested_source_counts[normalized_key],
                **disposition,
            }
        )

    emitted = len(records)
    classified = sum(mapping_counts[name] for name in ("RESOLVED", "UNRESOLVED", "AMBIGUOUS", "CONFLICT"))
    if classified != emitted:
        raise CatalogError("NATIVE_MAPPING_PARTITION_INVARIANT_FAILED")

    counts = {
        "source_xml_item_nodes": len(nodes),
        "source_direct_id_nodes": direct_nodes,
        "source_range_nodes": range_nodes,
        "source_reversed_range_nodes_excluded": reversed_ranges,
        "expanded_source_item_identities": emitted,
        "total_emitted_source_identity_records": emitted,
        "native_RESOLVED": mapping_counts["RESOLVED"],
        "native_UNRESOLVED": mapping_counts["UNRESOLVED"],
        "native_AMBIGUOUS": mapping_counts["AMBIGUOUS"],
        "native_CONFLICT": mapping_counts["CONFLICT"],
        "unsupported_field_observations": disposition_expanded_counts["UNSUPPORTED"],
        "unknown_field_observations": disposition_expanded_counts["UNKNOWN"],
        "excluded_by_policy_field_observations": disposition_expanded_counts["EXCLUDED_BY_POLICY"],
        "field_conflict_records": len(conflict_records),
        "name_collision_groups": len(name_collisions),
        "semantic_candidate_node_records": len(semantic_node_records),
    }

    return {
        "identity_records": records,
        "semantic_candidate_node_records": [
            semantic_node_records[key] for key in sorted(semantic_node_records)
        ],
        "field_disposition_records": field_records,
        "range_exclusion_records": exclusions,
        "field_conflict_records": conflict_records,
        "name_collision_records": name_collisions,
        "counts": counts,
        "count_invariants": {
            "emitted_equals_mapping_partition": emitted == classified,
            "no_duplicate_expanded_source_ids": len(seen_ids) == emitted,
            "range_exclusions_are_explicit": reversed_ranges == len(exclusions),
            "source_node_partition": len(nodes) == direct_nodes + range_nodes,
        },
        "field_family_coverage": {
            "identity": "SOURCE_NUMERIC_ID_ONLY_PROVENANCE; NO_NATIVE_KEY_MINTING",
            "presentation_appearance_binding": "NO_QUALIFIED_EXPLICIT_CROSSWALK_IN_B1",
            "stack": "NOT_OBSERVED_AS_AUTHORITATIVE_FIELD_IN_PINNED_XML",
            "charges": "MAPPED_WHEN_PRESENT",
            "temporal_decay": "MAPPED_WHEN_PRESENT",
            "container": "MAPPED_WHEN_PRESENT",
            "equipment_slot_claims": "MAPPED_WHEN_PRESENT",
            "equipment_requirements": "SCRIPT_OR_DOWNSTREAM_SEMANTICS_NOT_PROMOTED",
            "weapon_use": "MAPPED_WHEN_PRESENT",
            "protection_modifiers_resists": "MAPPED_AS_OTS_CANDIDATES_WHEN_PRESENT",
            "binding_transfer_restrictions": "NOT_OBSERVED_IN_PINNED_SOURCE",
            "imbuement_slots_allowed_family_tier": "MAPPED_AS_NESTED_OTS_CANDIDATE_WHEN_PRESENT",
            "light": "NOT_OBSERVED_IN_PINNED_SOURCE",
            "readable_fluid_wrap_rotate": "MAPPED_WHEN_PRESENT",
            "field_world_collision_housing": "UNSUPPORTED_IN_B1; OWNER_SEMANTICS_NOT_GUESSED",
            "materialization_pickup": "MAPPED_WHEN_PRESENT",
        },
    }

def compact_semantic_catalog_for_evidence(semantic: dict[str, Any]) -> dict[str, Any]:
    profiles: dict[str, dict[str, Any]] = {}
    compact_records: list[dict[str, Any]] = []
    for record in semantic["identity_records"]:
        profile = {
            "field_keys": record["field_keys"],
            "field_disposition_counts": record["field_disposition_counts"],
        }
        profile_id = sha256_bytes(canonical_bytes(profile))
        profiles.setdefault(profile_id, {"profile_id": profile_id, **profile})
        mapping = record["native_mapping"]
        compact_record: dict[str, Any] = {
            "source_item_id": record["source_item_id"],
            "source_node_digest": record["source_node_digest"],
            "range_member_offset": record["range_member_offset"],
            "field_profile_id": profile_id,
            "native_disposition": mapping["disposition"],
        }
        detail = {
            key: value
            for key, value in mapping.items()
            if key not in {"disposition", "reason"} and value not in (None, [], {})
        }
        if detail:
            compact_record["native_mapping_detail"] = detail
        compact_records.append(compact_record)

    return {
        "identity_record_binding": {
            "source_snapshot_ref": "source_snapshot.files[data/items/items.xml]",
            "source_item_id_semantics": "exact source numeric identity; never canonical Oteryn identity",
            "field_profile_reference": "field_profile_id -> field_profiles[].profile_id",
        },
        "identity_records": compact_records,
        "field_profiles": [profiles[key] for key in sorted(profiles)],
        "semantic_candidate_node_records": semantic["semantic_candidate_node_records"],
        "field_disposition_records": semantic["field_disposition_records"],
        "range_exclusion_records": semantic["range_exclusion_records"],
        "field_conflict_records": semantic["field_conflict_records"],
        "name_collision_records": semantic["name_collision_records"],
        "counts": semantic["counts"],
        "count_invariants": semantic["count_invariants"],
        "field_family_coverage": semantic["field_family_coverage"],
    }

def verify_game_mapper(game_root: Path) -> dict[str, Any]:
    game_root = game_root.resolve()
    mapper_path = game_root / MAPPER_REL
    try:
        top = Path(str(_git(game_root, "rev-parse", "--show-toplevel"))).resolve()
        head = str(_git(game_root, "rev-parse", "HEAD"))
        status = str(_git(game_root, "status", "--porcelain=v1", "--", MAPPER_REL))
        tracked = str(_git(game_root, "ls-files", "--error-unmatch", "--", MAPPER_REL))
        blob = str(_git(game_root, "rev-parse", f"HEAD:{MAPPER_REL}"))
        commit = str(_git(game_root, "log", "-1", "--format=%H", "--", MAPPER_REL))
    except (OSError, subprocess.CalledProcessError) as exc:
        raise CatalogError("GAME_MAPPER_REVISION_UNVERIFIABLE") from exc
    if top != game_root:
        raise CatalogError("GAME_REPOSITORY_ROOT_MISMATCH")
    if status:
        raise CatalogError("GAME_MAPPER_PATH_DIRTY")
    if tracked != MAPPER_REL:
        raise CatalogError("GAME_MAPPER_PATH_UNTRACKED")
    content = mapper_path.read_bytes()
    return {
        "profile": MAPPER_PROFILE,
        "path": MAPPER_REL,
        "blob": blob,
        "code_commit": commit,
        "sha256": sha256_bytes(content),
        "game_candidate_generation_head": head,
        "final_pr_head": "RECORDED_EXTERNALLY_BY_LIVE_PR_READBACK",
        "final_pr_head_note": "a tracked Git object cannot contain its own final commit SHA without self-reference",
    }

def build_evidence(source_repo: Path, game_root: Path) -> dict[str, Any]:
    source_snapshot, raw = verify_source_repository(source_repo)
    semantic = build_semantic_catalog(raw["data/items/items.xml"])
    semantic_evidence = compact_semantic_catalog_for_evidence(semantic)
    mapper = verify_game_mapper(game_root)
    value: dict[str, Any] = {
        "schema": SCHEMA,
        "task": "CW2-B1 ITEM_IDENTITY_CATALOG_SOURCE_BATCH",
        "source_snapshot": source_snapshot,
        "mapper": mapper,
        "classification": {
            "source": SOURCE_CLASSIFICATION,
            "evidence_status": "OTS_HYPOTHESIS_ONLY",
            "production_authority": "NONE",
            "reference_parity_claim": "NONE",
        },
        "closure": CLOSURE,
        "native_identity_rule": {
            "target_family": "ItemType / ContentKey",
            "legacy_numeric_id_is_canonical": False,
            "display_name_is_canonical": False,
            "path_hash_appearance_id_is_canonical": False,
            "auto_mint_native_key": False,
        },
        "semantic_catalog": semantic_evidence,
        "loss_unsupported_unknown_ambiguous_conflict": {
            "range_exclusions": semantic_evidence["range_exclusion_records"],
            "unsupported_unknown_field_records": [
                record for record in semantic_evidence["field_disposition_records"]
                if record["disposition"] in {"UNSUPPORTED", "UNKNOWN", "EXCLUDED_BY_POLICY"}
            ],
            "native_ambiguous_or_conflict_records": [
                record for record in semantic_evidence["identity_records"]
                if record["native_disposition"] in {"AMBIGUOUS", "CONFLICT"}
            ],
            "field_conflict_records": semantic_evidence["field_conflict_records"],
        },
    }
    digest_basis = canonical_bytes(value)
    value["product_digest_sha256"] = sha256_bytes(digest_basis)
    value["product_digest_scope"] = "canonical JSON with product_digest fields omitted"
    return value

def write_evidence(path: Path, evidence: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical_bytes(evidence))

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-repo", type=Path, required=True)
    parser.add_argument("--game-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    evidence = build_evidence(args.source_repo, args.game_root)
    write_evidence(args.output, evidence)
    counts = evidence["semantic_catalog"]["counts"]
    print(
        "item-identity-catalog: PASS "
        f"nodes={counts['source_xml_item_nodes']} "
        f"identities={counts['total_emitted_source_identity_records']} "
        f"resolved={counts['native_RESOLVED']} "
        f"unresolved={counts['native_UNRESOLVED']} "
        f"digest={evidence['product_digest_sha256']}"
    )
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
