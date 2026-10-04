"""Closed six quest refinements and ID4290 structural container; navigation only."""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter, defaultdict
from functools import lru_cache
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path[:0] = [
    str(ROOT / "tools/content-schema/item-authoring"),
    str(ROOT / "tools/content-census"),
]
import appearance_membership
import engine_items as engine
from item_external_family_refinement import dom_fields, normalized, own_fields
from item_id_alias_table import tibia_key
from item_navigation_source_supplement import checked, name_agrees
from item_official_navigation import admission_ids, decoded_digest

PATH = "imports/tibiawiki/facts/items-bounded7-navigation-20261002.json"
SHA = "119ffb187fd4716b4e5d2e66f7e2bd4c5a40f61d98ec78159b9bf116343aae91"
IDS = frozenset({4290, 39571, 43778, 43779, 43780, 43781, 43782})
A12_IDS = IDS - {4290, 39571}
COORDS = ("page_id", "revision_id", "revision_timestamp", "content_sha256")


@lru_cache(maxsize=1)
def source_inputs(root=ROOT):
    document = json.loads(checked(root, PATH, SHA))
    records, sources = document["records"], document["sources"]
    if len(records) != 7 or {e["appearance_id"] for e in records} != IDS:
        raise ValueError("BOUNDED7_CLOSED_SCOPE")
    for name in (
        "engine_rules",
        "wiki_helper",
        "dom_helper",
        "a12_helper",
        "bindings",
        "retained_stats",
        "native_admission",
        "dom_witnesses",
    ):
        checked(root, sources[name]["path"], sources[name]["sha256"])
    admission = sources["native_admission"]
    if not A12_IDS.issubset(
        admission_ids((root / admission["path"]).read_text(), admission)
    ):
        raise ValueError("BOUNDED7_EXISTING_A12_ADMISSION")
    index, manifests = appearance_membership.load_admitted()
    pin = sources["current_artifact"]
    raw = checked(
        root,
        f"content/assets/files/appearances-{pin['appearances_sha256']}.dat",
        pin["appearances_sha256"],
    )
    if (
        index["newest"] != "client-15.30"
        or index["files"][-1] != pin
        or manifests[index["newest"]]["entries_sha256"]
        != sources["current_membership_entries_sha256"]
        or appearance_membership.manifest_entries(raw)
        != manifests[index["newest"]]["entries"]
    ):
        raise ValueError("BOUNDED7_FULL_CURRENT_MEMBERSHIP")
    members = {label: {r[0]: r for r in m["entries"]} for label, m in manifests.items()}
    forward, reverse = defaultdict(list), defaultdict(list)
    for b in json.loads((root / sources["bindings"]["path"]).read_bytes())["bindings"]:
        forward[b["target"]["key"]].append(b)
        reverse[b["external_id"]].append(b)
    retained = {
        r["item_id"]: r["observations"]
        for r in json.loads((root / sources["retained_stats"]["path"]).read_bytes())[
            "records"
        ].values()
    }
    witnesses = json.loads((root / sources["dom_witnesses"]["path"]).read_bytes())[
        "records"
    ]
    if set(witnesses) != {str(i) for i in IDS - {4290}}:
        raise ValueError("BOUNDED7_DOM_SCOPE")
    return document, members, forward, reverse, retained, witnesses


def derive_row(entry, definition, appearance, inputs, name_count, excluded):
    document, members, forward, reverse, retained, witnesses = inputs
    iid, target = entry["appearance_id"], entry["target"]
    key, name = target["key"], entry["official_name"]
    member = members["client-15.30"].get(iid)
    if (
        entry not in document["records"]
        or iid not in IDS
        or key in excluded
        or key in document["sources"]["world49_exclusion"]["keys"]
        or not member
        or member != entry["current_member"]
        or target
        != {"family": "Item", "key": tibia_key(member[0]), "revision": "definition-r1"}
        or not definition
        or definition.get("identity") != target
        or definition.get("materializable") is not False
        or definition.get("stack_class") != "Unknown"
        or appearance.get("id") != iid
        or appearance.get("name") != name
        or decoded_digest(appearance) != entry["decoded_record_sha256"]
        or definition.get("semantics", {}).get("presentation", {})
        != entry["native_presentation_projection"]
        or not name_agrees(definition, name)
    ):
        raise ValueError("BOUNDED7_IDENTITY_OR_PRECEDENCE")
    binding, xml = entry["source_binding"], entry["own_bound_crystal_record"]
    if iid in A12_IDS:
        if (
            binding is not None
            or xml is not None
            or forward.get(key)
            or reverse.get(str(iid))
        ):
            raise ValueError("BOUNDED7_A12_NO_FAKE_BINDING")
    elif (
        not binding
        or forward.get(key) != [binding]
        or reverse.get(str(iid)) != [binding]
        or binding
        != {
            "disposition": "EXACT",
            "external_id": str(iid),
            "identity_namespace": "ots/item_server_id",
            "source_key": "oteryn:source.crystalserver",
            "source_revision": engine.ENGINES["crystal"]["revision"],
            "target": target,
        }
        or members["crystal-ff7ede5"].get(iid) != entry["old_member"]
        or entry["old_member"][1] != member[1]
        or not xml
        or normalized(xml["name"]) != normalized(name)
    ):
        raise ValueError("BOUNDED7_EXACT_BINDING")
    flags, attrs = appearance.get("flags", {}), (xml or {}).get("attrs", {})
    if (
        flags.get("flags.take") is not True
        or any(
            flags.get(f) is True
            for f in engine.CORPSE_FLAGS
            + engine.GROUND_OR_BORDER_FLAGS
            + ("flags.unmove",)
        )
        or engine.non_item_route(xml or {"name": name}, attrs, flags, iid)
        or engine.immovable_non_item_route(flags, attrs)
    ):
        raise ValueError("BOUNDED7_WORLD_ROUTE_OR_PICKUP")
    source = entry["wiki_source"]
    fields = own_fields(source)
    coordinate = source["coordinates"]
    observations = retained.get(iid, [])
    if (
        not fields
        or fields.get("itemid") != str(iid)
        or coordinate["revision_timestamp"]
        > document["sources"]["qualification_cutoff"]
        or fields.get("pickupable") != "yes"
        or fields.get("immobile") != "no"
        or any(
            normalized(fields.get(k, "")) != normalized(name)
            for k in ("name", "actualname")
        )
        or normalized(coordinate["title"]) != normalized(name)
        or any(
            any(o.get(k) != coordinate[k] for k in COORDS)
            or normalized(o.get("wiki_title", "")) != normalized(name)
            for o in observations
        )
        or any(fields.get(k, "") for k in ("secondarytype", "status"))
    ):
        raise ValueError("BOUNDED7_OWN_WIKI_SOURCE")
    if iid == 4290:
        if (
            entry["mode"] != "ID4290_STRUCTURAL_CONTAINER_PRIMARY_QUALIFICATION"
            or entry["family_profile"] != "container"
            or flags.get("flags.container") is not True
            or fields.get("primarytype") != "Corpses"
            or fields.get("objectclass") != "Decoration"
            or fields.get("volume") != "8"
            or attrs.get("containersize") != "10"
            or engine.classify_family_profile(attrs, attrs.get("primarytype"))
            != "container"
            or len(observations) != 1
            or observations[0]["fields"].get("primarytype") != "Corpses"
            or observations[0]["fields"].get("objectclass") != "Decoration"
            or not entry["scalar_holds"]
        ):
            raise ValueError("BOUNDED7_ID4290_CLOSED_PRIMARY_QUALIFICATION")
        basis = {
            "kind": "ID4290_ONLY_EXISTING_STRUCTURAL_CONTAINER",
            "source_primary_qualification": "Literal Corpses retained; exact own storage and portable current container qualify NAV only.",
            "source_objectclass": "Decoration",
            "own_crystal_record_sha256": decoded_digest(xml),
            "scalar_disagreements_held": entry["scalar_holds"],
        }
        primary = {
            "state": "KNOWN",
            "value": "Corpses",
            "source": "own_wiki_primarytype",
        }
    else:
        dom = dom_fields(witnesses[str(iid)])
        if (
            entry["mode"] != "OWN_QUEST_PURPOSE_CURRENT_EXTERNAL_CATEGORY"
            or entry["family_profile"] != "quest_item"
            or name_count != 1
            or fields.get("primarytype", "")
            or fields.get("objectclass", "")
            or fields.get("notes") != entry["quest_purpose_quote"]
            or not entry["quest_purpose_quote"]
            or any(
                o["fields"].get(k, "")
                for o in observations
                for k in ("primarytype", "objectclass", "secondarytype", "status")
            )
            or not dom
            or any(normalized(n) != normalized(name) for n in dom[:2])
            or dom[2] != "quest_item"
        ):
            raise ValueError("BOUNDED7_EXACT_QUEST_REFINEMENT")
        basis = {
            "kind": "EXACT_OWN_QUEST_PURPOSE_AND_CURRENT_PUBLIC_CATEGORY",
            "own_notes": entry["quest_purpose_quote"],
            "public_source": witnesses[str(iid)],
        }
        primary = {"state": "UNKNOWN", "reason": "ABSENT_OR_EMPTY"}
    return {
        "target": target,
        "family_profile": entry["family_profile"],
        "source_taxonomy": {
            "primary": "Corpses" if iid == 4290 else "Source primarytype absent"
        },
        "source_evidence": {
            "classification": "DERIVED",
            "scope": "NAVIGATION_ONLY",
            "qualification": PATH,
            "qualification_sha256": SHA,
            "source_primarytype": primary,
            "source_binding": binding,
            "identity_authority": "EXPLICIT_EXISTING_A12_CURRENT_MEMBERSHIP"
            if iid in A12_IDS
            else "ACTUAL_EXACT_CRYSTAL_CURRENT_IDENTITY",
            "current_member": member,
            "official_name": name,
            "decoded_record_sha256": entry["decoded_record_sha256"],
            "wiki_source": {
                k: source[k]
                for k in (
                    "coordinates",
                    "capture_coordinate",
                    "url",
                    "raw_own_infobox_sha256",
                    "fullarticle_sha256_role",
                )
            },
            "family_basis": basis,
        },
    }


def build_bounded7(definitions, client, excluded):
    inputs = source_inputs()
    names = Counter(normalized(o["name"]) for o in client.values() if o.get("name"))
    return [
        derive_row(
            e,
            definitions.get(e["target"]["key"]),
            client.get(e["appearance_id"], {}),
            inputs,
            names[normalized(e["official_name"])],
            excluded,
        )
        for e in inputs[0]["records"]
        if e["target"]["key"] in definitions and e["target"]["key"] not in excluded
    ]


def check_source_projections(source):
    document = json.loads(checked(ROOT, PATH, SHA))
    actual = engine.load_engine_sources("crystal", source)
    if (
        actual["artifact_digests"]
        != document["sources"]["actual_Crystal_artifact_digests"]
    ):
        raise ValueError("BOUNDED7_FULL_CRYSTAL_SOURCE_PINS")
    for e in document["records"]:
        if (
            e["source_binding"]
            and actual["items"].get(e["appearance_id"]) != e["own_bound_crystal_record"]
        ):
            raise ValueError("BOUNDED7_ACTUAL_OWN_XML")
    print(json.dumps({"source_projection_check": "PASS", "bound_records": 2}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    args = parser.parse_args()
    check_source_projections(args.source)
