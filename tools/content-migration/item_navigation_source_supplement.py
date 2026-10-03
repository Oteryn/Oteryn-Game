"""Closed navigation qualification: 26 existing aliases, six A12 IDs and Old Rag."""

import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/content-schema/item-authoring"))
sys.path.insert(0, str(ROOT / "tools/content-census"))
from engine_items import CORPSE_FLAGS, GROUND_OR_BORDER_FLAGS, load_wiki_family_fallback
from item_id_alias_table import tibia_key
from item_official_navigation import qualification_inputs
from item_wiki_family_capture import parse_infobox_fields, resolve_infobox_fields

ALIAS_PATH = "imports/tibiawiki/facts/items-family-alias26-20261001.json"
ALIAS_SHA = "f2674c4a3074574fefa96d8ae5d5ef3ed5ce3e0b7e62b322791114a9ecc05a52"
SEVEN_PATH = "tools/content-migration/samples/navigation-seven-20261001.json"
SEVEN_SHA = "162eaf6462844f886bc4a730416b8f22a44004f700963692c506e6c5829567da"
SEVEN_IDS = frozenset({40522, 43946, 43947, 44048, 44432, 44433, 24415})


def digest(data):
    return hashlib.sha256(data).hexdigest()


def checked(root, path, expected):
    data = (root / path).read_bytes()
    if digest(data) != expected:
        raise ValueError(f"NAVIGATION_SOURCE_DIGEST:{path}")
    return data


def check_mapping(root, sources):
    checked(
        root,
        "tools/content-schema/item-authoring/engine_items.py",
        sources["engine_sha256"],
    )
    checked(
        root,
        "tools/content-census/item_wiki_family_capture.py",
        sources["helper_sha256"],
    )


def load_alias_fallback(root, identity):
    document = json.loads(checked(root, ALIAS_PATH, ALIAS_SHA))
    sources = document["source"]
    check_mapping(root, sources)
    for field in ("bindings", "stats"):
        checked(root, sources[field]["path"], sources[field]["sha256"])
    entries = load_wiki_family_fallback(root / ALIAS_PATH, identity)
    if len(entries) != 26:
        raise ValueError("NAVIGATION_ALIAS_SCOPE")
    for key, entry in entries.items():
        entry.update(
            snapshot_path=ALIAS_PATH,
            snapshot_sha256=document["snapshot_sha256"],
            snapshot_file_sha256=ALIAS_SHA,
            qualified_native_name=sources["name_guards"][key],
        )
    return entries


def name_agrees(definition, name):
    presentation = definition.get("semantics", {}).get("presentation", {})
    leaf = presentation.get("value", {}).get("name", {})
    if presentation.get("state") in (
        "BLOCKED",
        "CONFLICT",
        "NOT_APPLICABLE",
    ) or leaf.get("state") in ("BLOCKED", "CONFLICT", "NOT_APPLICABLE"):
        return False
    return (
        leaf.get("state") != "KNOWN"
        or leaf["value"].strip().casefold() == name.strip().casefold()
    )


def derive_seven(entry, definition, appearance, observations, member, binding, cutoff):
    """A closed source join; retain existing native semantics and admission."""
    iid, target = entry["appearance_id"], entry["target"]
    source = entry["wiki_source"]
    if (
        iid not in SEVEN_IDS
        or not definition
        or definition.get("identity") != target
        or target
        != {"family": "Item", "key": tibia_key(iid), "revision": "definition-r1"}
        or definition.get("materializable") is not False
        or definition.get("stack_class") != "Unknown"
        or not name_agrees(definition, entry["official_name"])
        or appearance.get("id") != iid
        or appearance.get("name") != entry["official_name"]
        or not member
        or member[0] != iid
        or source["revision_timestamp"] > cutoff
        or digest(source["content"].encode()) != source["content_sha256"]
    ):
        return None
    fields = parse_infobox_fields(source["content"])
    profile, field, value = resolve_infobox_fields(fields)
    names = re.findall(r"(?im)^\|\s*name\s*=\s*(.*?)\s*$", source["content"])
    if (
        fields.get("itemid") != str(iid)
        or len(names) != 1
        or names[0].strip().casefold() != entry["official_name"].strip().casefold()
        or fields.get("actualname", "").strip().casefold()
        != entry["official_name"].strip().casefold()
        or source["title"].strip().casefold()
        != entry["official_name"].strip().casefold()
        or profile != entry["family_profile"]
    ):
        return None
    flags = appearance.get("flags", {})
    if flags.get("flags.take") is not True or any(
        flags.get(f) is True
        for f in CORPSE_FLAGS + GROUND_OR_BORDER_FLAGS + ("flags.unmove",)
    ):
        return None
    coords = ("page_id", "revision_id", "revision_timestamp", "content_sha256")
    if len(observations) != 1 or any(
        observations[0].get(k) != source[k] for k in coords
    ):
        return None
    observed = observations[0]
    if observed.get("wiki_title") != source["title"]:
        return None
    if iid == 24415:
        if (
            entry["mode"] != "SAME_REVISION_STATUS_PROJECTION"
            or binding != entry["source_binding"]
            or not binding
            or binding.get("disposition") != "EXACT"
            or binding.get("external_id") != str(iid)
            or binding.get("target") != target
            or fields.get("primarytype") != "Clothing Accessories"
            or observed["fields"].get("primarytype") != fields["primarytype"]
            or "status" in observed["fields"]
            or (profile, field, value) != ("event_collectible", "status", "event")
        ):
            return None
        authority = "EXACT_CRYSTAL_BINDING_SAME_RAW_STATUS_PROJECTION"
    else:
        decoded = digest(
            json.dumps(
                appearance, sort_keys=True, ensure_ascii=False, separators=(",", ":")
            ).encode()
        )
        if (
            entry["mode"] != "OFFICIAL_A12_OWN_WIKI_PRIMARY"
            or entry["source_binding"] is not None
            or binding is not None
            or member[1:]
            != [entry["identity_projection_sha256"], entry["record_sha256"]]
            or decoded != entry["decoded_record_sha256"]
            or field != "primarytype"
            or observed["fields"].get("primarytype") != value
        ):
            return None
        authority = "A12-ITEM-IDENTITY-TIBIA-ID-V1"
    return {
        "target": target,
        "source_taxonomy": {"primary": value},
        "family_profile": profile,
        "source_evidence": {
            "classification": "DERIVED",
            "scope": "NAVIGATION_ONLY",
            "identity_authority": authority,
            "source_binding": entry["source_binding"],
            "qualification": SEVEN_PATH,
            "qualification_sha256": SEVEN_SHA,
            "appearance_id": iid,
            "appearance_name": appearance["name"],
            "wiki_source": {k: v for k, v in source.items() if k != "content"},
            "selected_field": field,
            "selected_value": value,
        },
    }


def build_seven(definitions, snapshot, client, excluded, root=ROOT):
    document = json.loads(checked(root, SEVEN_PATH, SEVEN_SHA))
    sources = document["sources"]
    check_mapping(root, sources)
    for field in ("bindings", "stats", "client_artifact", "admission_source"):
        checked(root, sources[field]["path"], sources[field]["sha256"])
    official, members = qualification_inputs()
    if any(
        sources[f] != official[f] for f in ("client_artifact", "membership_manifest")
    ):
        raise ValueError("NAVIGATION_OFFICIAL_SOURCE_GENERATION")
    entries = document["records"]
    if len(entries) != 7 or {e["appearance_id"] for e in entries} != SEVEN_IDS:
        raise ValueError("NAVIGATION_SEVEN_SCOPE")
    bindings = json.loads((root / sources["bindings"]["path"]).read_text())["bindings"]
    by_key = {}
    for binding in bindings:
        by_key.setdefault(binding["target"]["key"], []).append(binding)
    wiki = {r["item_id"]: r["observations"] for r in snapshot["records"].values()}
    out = []
    for entry in entries:
        iid, key = entry["appearance_id"], entry["target"]["key"]
        if key in excluded:
            continue
        bound = by_key.get(key, [])
        if len(bound) > 1:
            continue
        row = derive_seven(
            entry,
            definitions.get(key),
            client.get(iid, {}),
            wiki.get(iid, []),
            members.get(iid),
            bound[0] if bound else None,
            sources["qualification_cutoff"],
        )
        if row:
            row["source_evidence"]["source_inputs"] = sources
            out.append(row)
    return out
