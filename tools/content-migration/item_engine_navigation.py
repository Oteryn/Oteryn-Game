"""Closed NAV-only import of existing own Crystal predicates, never native admission."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import defaultdict
from functools import lru_cache
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path[:0] = [
    str(ROOT / "tools/content-schema/item-authoring"),
    str(ROOT / "tools/content-census"),
]
import appearance_membership
import donor_census
import engine_items as engine
from item_id_alias_table import tibia_key
from item_wiki_family_capture import resolve_infobox_fields

QUALIFICATION = Path(__file__).parent / "samples/engine-family-navigation-265.json"
QUALIFICATION_SHA256 = (
    "09fd4b5faaf120ff2f49c65774f6e5322b4eb60a828f752114118dddbf5eabc3"
)


def digest(value):
    return hashlib.sha256(
        json.dumps(
            value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
        ).encode()
    ).hexdigest()


def qualification():
    raw = QUALIFICATION.read_bytes()
    if hashlib.sha256(raw).hexdigest() != QUALIFICATION_SHA256:
        raise ValueError("ENGINE_NAV_QUALIFICATION_SEAL")
    document = json.loads(raw)
    ids = [row["appearance_id"] for row in document["records"]]
    if (
        len(ids) != 265
        or ids != sorted(set(ids))
        or len(set(document["world49_exclusion"]["keys"])) != 49
    ):
        raise ValueError("ENGINE_NAV_CLOSED_SCOPE")
    return document


@lru_cache(maxsize=1)
def source_inputs():
    document = qualification()
    for spec in (
        document["engine_rules"],
        document["wiki_parser"],
        document["retained_stats"],
    ):
        if (
            hashlib.sha256((ROOT / spec["path"]).read_bytes()).hexdigest()
            != spec["sha256"]
        ):
            raise ValueError("ENGINE_NAV_PINNED_INPUT")
    index, manifests = appearance_membership.load_admitted()
    spec = document["source_pins"]["official"]
    if index["newest"] != "client-15.30" or index["files"][-1] != spec:
        raise ValueError("ENGINE_NAV_CURRENT_MEMBERSHIP")
    artifact = (
        ROOT / f"content/assets/files/appearances-{spec['appearances_sha256']}.dat"
    )
    raw = artifact.read_bytes()
    if (
        hashlib.sha256(raw).hexdigest() != spec["appearances_sha256"]
        or appearance_membership.manifest_entries(raw)
        != manifests[index["newest"]]["entries"]
    ):
        raise ValueError("ENGINE_NAV_FULL_CLIENT_ARTIFACT")
    members = {
        label: {row[0]: row for row in manifest["entries"]}
        for label, manifest in manifests.items()
    }
    bindings = defaultdict(list)
    reverse = defaultdict(list)
    for binding in json.loads(
        (ROOT / "imports/crystalserver/bindings/items.json").read_bytes()
    )["bindings"]:
        bindings[binding["target"]["key"]].append(binding)
        reverse[binding["external_id"]].append(binding)
    stats = json.loads((ROOT / document["retained_stats"]["path"]).read_bytes())
    retained = {
        row["item_id"]: row.get("observations", []) for row in stats["records"].values()
    }
    target_keys = {
        binding["target"]["key"]
        for entry in document["records"]
        for binding in entry["family_proof"].get("target_binding", [])
    }
    target_definitions = {}
    for path in (ROOT / "content/items/definitions").glob("items-*.json"):
        for row in json.loads(path.read_bytes())["records"]:
            definition = row["definition"]
            if definition["identity"]["key"] in target_keys:
                target_definitions[definition["identity"]["key"]] = definition
    return document, members, bindings, reverse, retained, target_definitions


def family(entry, flags):
    """Literal admitted order; no clothing-only inference or arbitrary owner table."""
    xml = entry["xml_record"]
    attrs = xml["attrs"]
    if digest(xml) != entry["xml_record_sha256"] or engine.non_item_route(
        xml, attrs, flags, entry["appearance_id"]
    ):
        raise ValueError("ENGINE_NAV_OWN_SOURCE_OR_WORLD_ROUTE")
    profile = engine.classify_family_profile(
        attrs, attrs.get("primarytype"), flags.get("clothes.slot")
    )
    if profile:
        if engine.classify_family_profile(attrs, attrs.get("primarytype")) is None and (
            attrs.get("slot"),
            flags.get("clothes.slot"),
        ) not in {("ring", 9), ("head", 1)}:
            raise ValueError("ENGINE_NAV_CLOTHING_ONLY")
        basis = "OWN_CRYSTAL_ATTRIBUTES_OR_CURRENT_CLOTHES"
    else:
        if engine.immovable_non_item_route(flags, attrs):
            raise ValueError("ENGINE_NAV_IMMOVABLE_PRECEDENCE")
        proof = entry["family_proof"]
        target = proof.get("target_own_record")
        if target:
            # The existing rule is exactly one hop, solely T's own primarytype.
            target_id = proof["wrap_target_id"]
            result = engine.resolve_wrap_target_profile({target_id: target}, attrs)
            if result != (entry["profile"], target_id, proof["target_primarytype"]):
                raise ValueError("ENGINE_NAV_SINGLE_HOP_TARGET")
            profile, basis = result[0], "EXISTING_SINGLE_HOP_OWN_WRAP_TARGET"
        else:
            result = engine.resolve_dead_item_route_or_profile(xml["name"], flags)
            if not result or result[0] != "profile":
                raise ValueError("ENGINE_NAV_EXACT_DEAD_RULE")
            profile, basis = result[1], "EXISTING_EXACT_REVIEWED_TAKEABLE_DEAD_NAME"
    if (profile, basis) != (entry["profile"], entry["basis"]):
        raise ValueError("ENGINE_NAV_REVIEWED_FAMILY_CHANGED")
    return profile


def wiki_priority(entry, retained):
    for source in entry["wiki_own_sources"]:
        values = defaultdict(list)
        for excerpt in source["parameter_excerpts"]:
            key, value = excerpt.split("=", 1)
            values[key.strip().lower()].append(value.strip())
        if (
            dict(values) != source["parameter_values"]
            or not source["strict_numeric_itemid"]
            or source["revision_timestamp"] > "2026-09-27T23:59:59Z"
        ):
            raise ValueError("ENGINE_NAV_WIKI_PROJECTION")
        if (
            any(
                len(values.get(key, [])) > 1
                for key in (
                    "itemid",
                    "primarytype",
                    "name",
                    "actualname",
                    "objectclass",
                    "status",
                )
            )
            or len(values.get("itemid", [])) != 1
        ):
            raise ValueError("ENGINE_NAV_WIKI_DUPLICATE")
        ids = values["itemid"][0]
        if not re.fullmatch(r"[1-9][0-9]*(?:\s*,\s*[1-9][0-9]*)*", ids) or entry[
            "appearance_id"
        ] not in [int(i.strip()) for i in ids.split(",")]:
            raise ValueError("ENGINE_NAV_OWN_WIKI_ID")
        fields = {key: values[0] for key, values in values.items()}
        primary = fields.get("primarytype", "")
        if (
            primary.strip()
            and engine.resolve_wiki_family_value("primarytype", primary)
            != entry["profile"]
        ):
            raise ValueError("ENGINE_NAV_WIKI_PRIMARY_PRECEDENCE")
        resolved = resolve_infobox_fields(fields)[0]
        if resolved is not None and resolved != entry["profile"]:
            raise ValueError("ENGINE_NAV_WIKI_POSITIVE_DISAGREEMENT")
    if any("primarytype" in row["fields"] for row in retained):
        raise ValueError("ENGINE_NAV_RETAINED_PRIMARY_PRECEDENCE")


def derive_row(entry, appearance, definition, inputs, excluded):
    document, members, bindings, reverse, retained, target_definitions = inputs
    item_id = entry["appearance_id"]
    target = entry["target"]
    key = target["key"]
    binding = entry["binding"]
    label = (
        "crystal-donor-00ce02a5"
        if binding["source_revision"] == donor_census.DONOR_COMMIT
        else "crystal-ff7ede5"
    )
    current = members["client-15.30"].get(item_id)
    old = members[label].get(item_id)
    if (
        entry not in document["records"]
        or key in excluded
        or key in document["world49_exclusion"]["keys"]
        or not current
        or current != entry["current_member"]
        or old != entry["old_member"]
        or old[1] != current[1]
        or target
        != {"family": "Item", "key": tibia_key(current[0]), "revision": "definition-r1"}
        or bindings.get(key) != [binding]
        or reverse.get(str(item_id)) != [binding]
        or binding
        != {
            "disposition": "EXACT",
            "external_id": str(item_id),
            "identity_namespace": "ots/item_server_id",
            "source_key": "oteryn:source.crystalserver",
            "source_revision": engine.ENGINES["crystal"]["revision"]
            if label == "crystal-ff7ede5"
            else donor_census.DONOR_COMMIT,
            "target": target,
        }
        or not definition
        or definition.get("identity") != target
        or definition.get("materializable") is not False
        or definition.get("stack_class") != "Unknown"
        or appearance.get("id") != item_id
        or digest(appearance) != entry["decoded_record_sha256"]
        or appearance.get("name") != entry["official_name"]
    ):
        raise ValueError("ENGINE_NAV_IDENTITY_OR_PRECEDENCE")
    presentation = definition.get("semantics", {}).get("presentation", {})
    name = presentation.get("value", {}).get("name", {})
    if (
        presentation != entry["native_presentation"]
        or presentation.get("state") in ("CONFLICT", "NOT_APPLICABLE")
        or name.get("state") in ("CONFLICT", "NOT_APPLICABLE")
    ):
        raise ValueError("ENGINE_NAV_NATIVE_PRESENTATION")
    names = [
        value.strip().casefold()
        for value in (
            appearance.get("name"),
            entry["xml_record"].get("name"),
            name.get("value") if name.get("state") == "KNOWN" else None,
        )
        if value
    ]
    if not names or len(set(names)) != 1:
        raise ValueError("ENGINE_NAV_CURRENT_NAME_DISAGREEMENT")
    proof = entry["family_proof"]
    if "wrap_target_id" in proof:
        target_id = proof["wrap_target_id"]
        target_binding = proof["target_binding"]
        if (
            target_binding != reverse.get(str(target_id))
            or len(target_binding) != 1
            or target_binding[0]
            != {
                **binding,
                "external_id": str(target_id),
                "target": {
                    "family": "Item",
                    "key": tibia_key(target_id),
                    "revision": "definition-r1",
                },
            }
            or members["client-15.30"].get(target_id) != proof["target_current_member"]
            or members[label].get(target_id) != proof["target_old_member"]
            or proof["target_current_member"][1] != proof["target_old_member"][1]
            or target_definitions.get(tibia_key(target_id), {}).get("identity")
            != target_binding[0]["target"]
        ):
            raise ValueError("ENGINE_NAV_WRAP_TARGET_IDENTITY")
    profile = family(entry, appearance.get("flags", {}))
    wiki_priority(entry, retained.get(item_id, []))
    source_primary = entry["xml_record"]["attrs"].get("primarytype")
    primary_known = isinstance(source_primary, str) and bool(source_primary.strip())
    return {
        "target": target,
        "source_taxonomy": {
            "primary": source_primary if primary_known else "Source primarytype absent"
        },
        "family_profile": profile,
        "source_evidence": {
            "classification": "DERIVED",
            "scope": "NAVIGATION_ONLY",
            "snapshot": QUALIFICATION.relative_to(ROOT).as_posix(),
            "snapshot_file_sha256": QUALIFICATION_SHA256,
            "engine_revision": binding["source_revision"],
            "engine_artifact_digests": document["source_pins"][
                "crystal_artifact_digests"
                if label == "crystal-ff7ede5"
                else "donor_artifact_digests"
            ],
            "family_basis": entry["basis"],
            "source_primarytype": {"state": "KNOWN", "value": source_primary}
            if primary_known
            else {"state": "UNKNOWN", "reason": "ABSENT_OR_EMPTY"},
            "own_record_sha256": entry["xml_record_sha256"],
            "official_decoded_record_sha256": entry["decoded_record_sha256"],
            "identity_binding": binding,
            "family_proof": proof,
        },
    }


def build_navigation(definitions, client, excluded):
    document = qualification()
    entries = [
        entry
        for entry in document["records"]
        if entry["target"]["key"] in definitions
        and entry["target"]["key"] not in excluded
    ]
    if not entries:
        return []
    inputs = source_inputs()
    return [
        derive_row(
            entry,
            client.get(entry["appearance_id"], {}),
            definitions[entry["target"]["key"]],
            inputs,
            excluded,
        )
        for entry in entries
    ]


def check_source_projections(source, donor_source):
    document = qualification()
    current = engine.load_engine_sources("crystal", source)
    donor = donor_census.load_donor_artifacts(donor_source)
    if (
        current["artifact_digests"]
        != document["source_pins"]["crystal_artifact_digests"]
        or donor["artifact_digests"]
        != document["source_pins"]["donor_artifact_digests"]
    ):
        raise ValueError("ENGINE_NAV_SOURCE_ARTIFACT_PINS")
    for entry in document["records"]:
        sources = (
            donor
            if entry["binding"]["source_revision"] == donor_census.DONOR_COMMIT
            else current
        )
        if sources["items"].get(entry["appearance_id"]) != entry["xml_record"]:
            raise ValueError("ENGINE_NAV_ACTUAL_OWN_XML_PROJECTION")
        if "wrap_target_id" in entry["family_proof"]:
            proof = entry["family_proof"]
            if (
                sources["items"].get(proof["wrap_target_id"])
                != proof["target_own_record"]
            ):
                raise ValueError("ENGINE_NAV_ACTUAL_TARGET_XML_PROJECTION")
    print(json.dumps({"source_projection_check": "PASS", "records": 265}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--donor-source", type=Path, required=True)
    args = parser.parse_args()
    check_source_projections(args.source, args.donor_source)
