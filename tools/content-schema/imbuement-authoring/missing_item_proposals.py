#!/usr/bin/env python3
"""Build validated portable Item proposals without admitting canonical identities."""
from __future__ import annotations

import argparse
from datetime import datetime
import hashlib
import json
from pathlib import Path
import re
import sys
from urllib.parse import parse_qs, unquote, urlsplit

from binding_evidence import APPEARANCES, APPEARANCES_SHA256, canonical_items, client_objects

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
ITEM_TOOL = ROOT / "tools/content-schema/item-authoring"
FACTS = HERE / "samples/missing-item-source-facts.json"
OUTPUT = HERE / "samples/missing-item-definitions.json"
TEST_IDS = (28464, 28465, 28478, 28479)


def digest(value: object) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=False,
                                     separators=(",", ":")).encode()).hexdigest()


def validate_source_provenance(source: dict) -> None:
    """Reject malformed links and evidence attached to a different named item."""
    title = source["source_name"].replace(" ", "_")
    for provider, evidence in source["sources"].items():
        if provider not in {"wiki_br", "tibiopedia", "tibiopedia_update"}:
            raise ValueError(f"{provider}: unsupported provenance provider")
        url = evidence["url"]
        if not isinstance(url, str) or re.search(r"[\s\[\]()<>{}]", url):
            raise ValueError(f"{provider}: malformed provenance URL")
        parts = urlsplit(url)
        if parts.scheme != "https" or parts.fragment or parts.username or parts.password:
            raise ValueError(f"{provider}: invalid provenance URL")
        expected_host = "www.tibiawiki.com.br" if provider == "wiki_br" else "tibiopedia.pl"
        if parts.netloc != expected_host or parts.query:
            raise ValueError(f"{provider}: unexpected provenance origin/query")
        expected_path = ("/wiki/" if provider == "wiki_br" else "/items/") + title
        if provider == "tibiopedia_update":
            expected_path = "/updates/" + source["facts"]["introduced_version"]
            observed_date = datetime.strptime(evidence["observed_publication_date"], "%d.%m.%Y").date().isoformat()
            if (evidence["published_on"] != source["facts"]["introduced_on"]
                    or observed_date != evidence["published_on"]):
                raise ValueError("update: introduction date disagrees with source")
            if unquote(evidence["observed_item_url"]) != "https://tibiopedia.pl/items/" + title:
                raise ValueError("update: named item row disagrees with proposal")
        if unquote(parts.path) != expected_path:
            raise ValueError(f"{provider}: source URL names another item/version")
        if not re.fullmatch(r"[0-9a-f]{64}", evidence["sha256"]):
            raise ValueError(f"{provider}: malformed provenance digest")
        if provider != "wiki_br":
            continue
        revision = evidence["revision_id"]
        revision_url = evidence["revision_url"]
        if type(revision) is not int or revision <= 0:
            raise ValueError("wiki_br: invalid revision identity")
        if not isinstance(revision_url, str) or re.search(r"[\s\[\]()<>{}]", revision_url):
            raise ValueError("wiki_br: malformed revision URL")
        revision_parts = urlsplit(revision_url)
        if (revision_parts.scheme != "https" or revision_parts.netloc != expected_host
                or revision_parts.path != "/index.php" or revision_parts.fragment
                or revision_parts.username or revision_parts.password
                or parse_qs(revision_parts.query, strict_parsing=True)
                != {"title": [title], "oldid": [str(revision)]}):
            raise ValueError("wiki_br: revision URL disagrees with named revision")


def build() -> dict:
    sys.path.insert(0, str(ITEM_TOOL))
    import engine_items
    import validate_item

    facts = json.loads(FACTS.read_bytes())
    clients = client_objects()  # Verifies the pinned primary client's exact bytes.
    appearances = engine_items.load_appearance_objects(APPEARANCES.read_bytes())
    # The owning engine reader preserves other known flags but currently leaves
    # field 60 unmapped. Record it directly; absence stays unknown, never zero.
    selected_ids = {r["client_id"] for r in facts["records"]} | set(TEST_IDS)
    for field, raw in engine_items.protobuf_fields(APPEARANCES.read_bytes()):
        if field != 1:
            continue
        fields = list(engine_items.protobuf_fields(raw))
        item_id = next(value for number, value in fields if number == 1)
        if item_id not in selected_ids:
            continue
        flag_raw = next(value for number, value in fields if number == 3)
        marked = [value for number, value in engine_items.protobuf_fields(flag_raw) if number == 60]
        slots = None
        if marked:
            inner = list(engine_items.protobuf_fields(marked[0]))
            if len(marked) != 1 or len(inner) != 1 or inner[0][0] != 1:
                raise ValueError(f"{item_id}: malformed primary imbuement slots")
            slots = inner[0][1]
        clients[item_id]["imbuement_slots"] = slots
    admitted = canonical_items()
    dependencies = {"definitions": [], "assets": [], "presentations": [], "proficiency_crosswalks": []}
    proposals = []
    for source in facts["records"]:
        validate_source_provenance(source)
        item_id, name = source["client_id"], source["source_name"]
        client, appearance = clients[item_id], appearances[item_id]
        if client["name"].casefold() != name.casefold():
            raise ValueError(f"{item_id}: named source/client identity disagrees")
        key = f"oteryn:item.tibia.i{item_id}"
        if key in admitted:
            raise ValueError(f"{key}: Item already admitted; proposal needs owning-lane reconciliation")
        flags, observed = appearance["flags"], source["facts"]
        if not flags.get("flags.container") or not flags.get("flags.take") or flags.get("clothes.slot") != 3:
            raise ValueError(f"{item_id}: primary client does not corroborate portable back-slot container")
        if not source["sources"]["wiki_br"]["revision_id"]:
            raise ValueError(f"{item_id}: missing revisioned public item evidence")
        authoring = {
            "identity": {"key": key, "revision": "definition-r1"},
            "display_name": name,
            "family_profile": "container_equipment",
            "delivery_task_eligible": False,
            "taxonomy": {"item_class": "container", "primary": "backpack"},
            "physical": {"weight": {"value": observed["weight_oz"], "unit": "oz"}, "pickupable": True},
            "stack": {"stackable": observed["stackable"], "max_count": 1},
            "equipment": engine_items.build_equipment({}, flags["clothes.slot"]),
            "container": {"capacity": observed["container_capacity"], "content_kind": "items"},
            "imbuement": {
                "slot_count": observed["imbuement_slots"],
                "allowed_family_tiers": [{"family": family, "tier": tier}
                                         for family, maximum in observed["allowed_imbuement_max_tiers"].items()
                                         for tier in range(1, maximum + 1)],
            },
            "trade": {"tradeable": observed["tradeable"], "marketable": observed["marketable"]},
        }
        if observed["description"]:
            authoring["presentation"] = {"inspection_description": observed["description"]}
        errors, warnings = validate_item.validate(authoring, dependencies)
        if errors:
            raise ValueError(f"{item_id}: owning Item validator rejected proposal: {errors}")
        introduced = observed["introduced_on"]
        proposals.append({
            "source_client_id": item_id,
            "proposed_item_ref": {"family": "Item", **authoring["identity"]},
            "identity_state": "PROPOSED_NOT_REGISTERED",
            "authoring_definition": authoring,
            "dependencies": dependencies,
            "primary_client": {**client, "flags": flags, "frame_groups": appearance["frame_groups"]},
            "source_facts": source,
            "source_facts_sha256": digest(source),
            "target_time_status": "INTRODUCED_AFTER_TARGET" if introduced and introduced > facts["target_date"]
                                  else "PREEXISTING_TARGET_SOURCE_VERSION",
            "validation": {"contract": "tools/content-schema/item-authoring/item.schema.json",
                           "validator": "tools/content-schema/item-authoring/validate_item.py",
                           "errors": errors, "warnings": warnings},
            "admission_requirements": [
                "Owning Item population must admit the proposed exact identity before any ItemRef is activated.",
                "Owning Presentation population must admit the primary-client appearance and sprite dependencies.",
                "delivery_task_eligible=false is a conservative draft authoring default, not an observed global rule.",
            ],
        })
    wiki_path = ROOT / "imports/tibiawiki/facts/items-stats.json"
    wiki = json.loads(wiki_path.read_bytes())["records"]
    tests = []
    for item_id in TEST_IDS:
        client, flags = clients[item_id], appearances[item_id]["flags"]
        item = wiki[f"oteryn:item.tibia.i{item_id}"]
        tests.append({
            "source_client_id": item_id, "primary_client": {**client, "flags": flags},
            "wiki_observations": item["observations"],
            "classification": "CLIENT_AND_PUBLIC_WIKI_TEST_LABEL; AVAILABILITY_UNPROVEN",
            "public_status": "Imported revisioned pages identify these as Test items but contain no availability status.",
            "flags_limit": "Pickupable/equipment flags and slot observations prove client records, not live release or obtainability.",
            "action": "Do not mint live global definitions or assign real-item eligibility from naming/classes.",
        })
    return {
        "schema": "OTERYN_MISSING_IMBUEMENT_ITEM_DEFINITION_PROPOSALS/v1",
        "activation": "REVIEWABLE_PROPOSALS_NOT_RUNTIME_DEFINITIONS",
        "target_date": facts["target_date"],
        "inputs": {
            "source_facts_sha256": hashlib.sha256(FACTS.read_bytes()).hexdigest(),
            "primary_appearances_sha256": APPEARANCES_SHA256,
            "owning_item_schema_sha256": hashlib.sha256((ITEM_TOOL / "item.schema.json").read_bytes()).hexdigest(),
            "test_wiki_facts_sha256": hashlib.sha256(wiki_path.read_bytes()).hexdigest(),
        },
        "proposals": proposals, "test_item_evidence": tests,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raw = (json.dumps(build(), ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_bytes() != raw:
            raise SystemExit("missing Item proposals are stale; run missing_item_proposals.py")
    else:
        OUTPUT.write_bytes(raw)
    print("Both missing Item proposals pass the owning Item validator; canonical content unchanged.")


if __name__ == "__main__":
    main()
