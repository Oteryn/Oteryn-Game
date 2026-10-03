"""Closed, navigation-only A12 identity route for six existing official-only Items."""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/content-schema/item-authoring"))
sys.path.insert(0, str(ROOT / "tools/content-census"))

from appearance_membership import load_admitted
from engine_items import CORPSE_FLAGS, GROUND_OR_BORDER_FLAGS, resolve_wiki_family_value
from item_id_alias_table import tibia_key

QUALIFICATION = Path(__file__).parent / "samples/official-rule-only-navigation-six.json"
QUALIFICATION_SHA256 = (
    "33f8c2dc7e4c4d219ab499565c9326652c32b240e6d6761b1e83d32f22da6456"
)
REVIEWED_IDS = frozenset({51276, 53197, 53199, 53201, 53203, 53205})


def decoded_digest(appearance):
    return hashlib.sha256(
        json.dumps(
            appearance, sort_keys=True, ensure_ascii=False, separators=(",", ":")
        ).encode()
    ).hexdigest()


def admission_ids(text, expected):
    # Inspect the existing explicit native admission cohort, never infer it from a key.
    declaration = re.search(
        r"const APPEARANCE_ONLY_ITEM_IDS: \[u64; 60\] = \[([\d,\s]+)\];", text
    )
    if not declaration:
        raise ValueError("OFFICIAL_NAVIGATION_NATIVE_ADMISSION")
    ids = [int(value) for value in declaration[1].split(",") if value.strip()]
    digest = hashlib.sha256(json.dumps(ids, separators=(",", ":")).encode()).hexdigest()
    if (
        len(ids) != expected["explicit_rule_only_count"]
        or len(set(ids)) != len(ids)
        or digest != expected["explicit_ids_sha256"]
        or not REVIEWED_IDS.issubset(ids)
    ):
        raise ValueError("OFFICIAL_NAVIGATION_NATIVE_ADMISSION")
    return set(ids)


def qualification_inputs():
    raw = QUALIFICATION.read_bytes()
    if hashlib.sha256(raw).hexdigest() != QUALIFICATION_SHA256:
        raise ValueError("OFFICIAL_NAVIGATION_QUALIFICATION_DIGEST")
    document = json.loads(raw)
    admission = document["admission_source"]
    admission_ids((ROOT / admission["path"]).read_text(), admission)
    index, manifests = load_admitted()
    spec = index["files"][-1]
    expected = document["membership_manifest"]
    if (
        index["newest"] != "client-15.30"
        or spec["label"] != index["newest"]
        or spec["manifest_sha256"] != expected["sha256"]
        or spec["appearances_sha256"] != document["client_artifact"]["sha256"]
    ):
        raise ValueError("OFFICIAL_NAVIGATION_CURRENT_GENERATION")
    manifest = manifests[index["newest"]]
    if manifest["entries_sha256"] != expected["entries_sha256"]:
        raise ValueError("OFFICIAL_NAVIGATION_MEMBERSHIP_DIGEST")
    members = {row[0]: row for row in manifest["entries"]}
    records = document["records"]
    if len(records) != 6 or {row["appearance_id"] for row in records} != REVIEWED_IDS:
        raise ValueError("OFFICIAL_NAVIGATION_REVIEWED_SCOPE")
    for row in records:
        item_id = row["appearance_id"]
        member = members.get(item_id)
        if (
            not member
            or member[1:] != [row["identity_projection_sha256"], row["record_sha256"]]
            or row["target"]
            != {
                "family": "Item",
                "key": tibia_key(member[0]),
                "revision": "definition-r1",
            }
        ):
            raise ValueError("OFFICIAL_NAVIGATION_A12_IDENTITY")
    return document, members


def derive_row(
    entry,
    appearance,
    definition,
    observations,
    excluded_keys,
    bound_keys,
    member,
    market_profiles,
    market_slots,
):
    """Return only admitted positive navigation; conflicts and missing facts stay held."""
    item_id = entry["appearance_id"]
    key = tibia_key(member[0]) if member else None
    if (
        item_id not in REVIEWED_IDS
        or not member
        or member[0] != item_id
        or appearance.get("id") != item_id
        or decoded_digest(appearance) != entry["decoded_record_sha256"]
        or member[1:] != [entry["identity_projection_sha256"], entry["record_sha256"]]
        or not definition
        or definition.get("identity") != entry["target"]
        or entry["target"]["key"] != key
        or definition.get("materializable") is not False
        or definition.get("stack_class") != "Unknown"
        or key in excluded_keys
        or key in bound_keys
    ):
        return None
    presentation = definition.get("semantics", {}).get("presentation", {})
    canonical_name = presentation.get("value", {}).get("name", {})
    name = appearance.get("name")
    if (
        presentation.get("state") in ("CONFLICT", "NOT_APPLICABLE")
        or canonical_name.get("state") in ("CONFLICT", "NOT_APPLICABLE")
        or name != entry["official_name"]
        or canonical_name.get("state") == "KNOWN"
        and canonical_name["value"].strip().casefold() != name.strip().casefold()
    ):
        return None
    flags = appearance.get("flags", {})
    if (
        flags.get("flags.take") is not True
        or any(
            flags.get(field) is True for field in CORPSE_FLAGS + GROUND_OR_BORDER_FLAGS
        )
        or flags.get("flags.unmove") is True
    ):
        return None
    market = flags.get("market.category")
    expected_slot = market_slots.get(market)
    actual_slot = flags.get("clothes.slot")
    if (
        market in market_profiles
        and expected_slot is not None
        and actual_slot is not None
        and expected_slot == actual_slot
    ):
        profile = market_profiles[market]
    elif market == 24 and flags.get("flags.cumulative") is True:
        profile = "material_valuable"
    else:
        return None
    if profile != entry["family_profile"] or any(
        flags.get(field) != value for field, value in entry["official_flags"].items()
    ):
        return None
    for observation in observations:
        primary = observation["fields"].get("primarytype")
        if primary and resolve_wiki_family_value("primarytype", primary) != profile:
            return None
        if observation["wiki_title"].strip().casefold() != name.strip().casefold():
            return None
    return {
        "target": definition["identity"],
        "family_profile": profile,
        "source_taxonomy": {"primary": f"official client market category {market}"},
        "source_evidence": {
            "classification": "DERIVED",
            "scope": "NAVIGATION_ONLY",
            "source": "official_client",
            "identity_authority": "OTERYN_TIBIA_ID_KEY_RULE_V1",
            "qualification": QUALIFICATION.relative_to(ROOT).as_posix(),
            "qualification_sha256": QUALIFICATION_SHA256,
            "identity_bridge": {
                "appearance_id": item_id,
                "canonical_key": key,
                "identity_projection_sha256": member[1],
                "record_sha256": member[2],
                "basis": "A12 current official appearance and existing ITEM-ADD-1 definition",
            },
            "official_name": name,
            "official_flags": entry["official_flags"],
            "wiki_corroboration": [
                {
                    field: observation[field]
                    for field in (
                        "page_id",
                        "revision_id",
                        "content_sha256",
                        "url",
                        "wiki_title",
                    )
                }
                | {
                    "role": "name/category corroboration; no G4 or native stat authority"
                }
                for observation in observations
            ],
        },
    }


def build_official_navigation(
    definitions,
    snapshot,
    client,
    excluded_keys,
    bound_keys,
    market_profiles,
    market_slots,
):
    document, members = qualification_inputs()
    wiki = {row["item_id"]: row["observations"] for row in snapshot["records"].values()}
    out = []
    for entry in document["records"]:
        item_id = entry["appearance_id"]
        row = derive_row(
            entry,
            client.get(item_id, {}),
            definitions.get(entry["target"]["key"]),
            wiki.get(item_id, []),
            excluded_keys,
            bound_keys,
            members.get(item_id),
            market_profiles,
            market_slots,
        )
        if row:
            row["source_evidence"]["client_artifact"] = document["client_artifact"]
            row["source_evidence"]["membership_manifest"] = document[
                "membership_manifest"
            ]
            out.append(row)
    return out
