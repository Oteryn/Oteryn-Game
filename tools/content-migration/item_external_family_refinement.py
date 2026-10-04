"""Closed18 navigation refinement; public names do not assert numeric identity."""

import html
import json
import re
import sys
from collections import Counter
from pathlib import Path

sys.path.insert(
    0, str(Path(__file__).resolve().parents[2] / "tools/content-schema/item-authoring")
)

from engine_items import CORPSE_FLAGS, GROUND_OR_BORDER_FLAGS, build_identity_index
from item_navigation_source_supplement import (
    ROOT,
    check_mapping,
    checked,
    digest,
    name_agrees,
)
from item_wiki_family_capture import split_template_params

PATH = "imports/tibiawiki/facts/items-external-family18-20261001.json"
SHA = "87813e341e88d8d46f28a1826a0536654e6a7894d5d98fdf43ff515032f36329"
IDS = frozenset(
    {
        51443,
        51702,
        51703,
        51704,
        52635,
        52637,
        52638,
        52743,
        52744,
        52758,
        52816,
        52817,
        52818,
        52819,
        52820,
        52821,
        52822,
        52823,
    }
)
NOTES = "Can be used on an imbued item to erase all applied [[imbuements]]."
COORDS = ("page_id", "revision_id", "revision_timestamp", "content_sha256")


def normalized(name):
    return name.strip().casefold()


def dom_fields(witness):
    """Read retained literal DOM excerpts, not a report's category declaration."""
    if (
        witness.get("http_status") != 200
        or witness.get("article_revision") != "UNKNOWN"
        or witness.get("article_revision_timestamp") != "UNKNOWN"
        or not witness.get("capture_utc")
        or not witness.get("source_url", "").startswith("https://tibiopedia.pl/items/")
    ):
        return None
    excerpts = witness["excerpts"]
    if set(excerpts) != {
        "document_title",
        "own_caption_prefix",
        "own_category_header",
        "own_category_values",
    }:
        return None
    if any(digest(e["raw"].encode()) != e["sha256"] for e in excerpts.values()):
        return None
    title = re.fullmatch(
        r"<title>Items: ([^<]+) - Tibia ~ Tibiopedia\.pl</title>",
        excerpts["document_title"]["raw"],
    )
    caption = re.fullmatch(
        r"<caption>\s*([^<]+)", excerpts["own_caption_prefix"]["raw"]
    )
    header = excerpts["own_category_header"]["raw"]
    links = re.findall(
        r'<a class="thinlink" href="([^"]+)">([^<]+)</a>',
        excerpts["own_category_values"]["raw"],
    )
    if (
        not title
        or not caption
        or re.findall(r"<strong>([^<]+)</strong>", header) != ["Class", "Subclass"]
        or len(links) != 2
    ):
        return None
    categories = tuple(html.unescape(text).strip() for _, text in links)
    if categories == ("Others", "Quest"):
        profile, suffix = "quest_item", "quest"
    elif categories == ("Others", "Taming"):
        profile, suffix = "tool", "taming"
    else:
        return None
    if [url for url, _ in links] != [
        "https://tibiopedia.pl/items/others",
        f"https://tibiopedia.pl/items/others#{suffix}",
    ]:
        return None
    return html.unescape(title[1]), html.unescape(caption[1]), profile, links


def own_fields(source):
    raw = source["raw_own_infobox"]
    if (
        digest(raw.encode()) != source["raw_own_infobox_sha256"]
        or source.get("balanced") is not True
        or source.get("inside_comment") is not False
        or source.get("positive_exact_infobox_object_match") is not True
        or not raw.startswith("{{Infobox Object|")
        or not raw.endswith("}}")
        or "<!--" in raw
    ):
        return None
    parts = split_template_params(raw[2:-2])
    fields = {}
    for part in parts[1:]:
        if "=" not in part:
            return None
        key, value = part.split("=", 1)
        key = key.strip().lower()
        if not re.fullmatch(r"[a-z]+", key) or key in fields:
            return None
        fields[key] = value.strip()
    if {k: [v] for k, v in fields.items()} != source["parameter_values"]:
        return None
    return fields


def derive_external(
    entry, definition, appearance, observations, binding, name_ids, witness, cutoff
):
    iid, target = entry["appearance_id"], entry["target"]
    name = entry["official_name"]
    if (
        iid not in IDS
        or not definition
        or definition.get("identity") != target
        or target.get("family") != "Item"
        or target.get("revision") != "definition-r1"
        or definition.get("materializable") is not False
        or definition.get("stack_class") != "Unknown"
        or not name_agrees(definition, name)
        or appearance.get("id") != iid
        or appearance.get("name") != name
        or name_ids != [iid]
        or binding != entry["source_binding"]
        or not binding
        or binding.get("disposition") != "EXACT"
        or binding.get("external_id") != str(iid)
        or binding.get("target") != target
        or binding.get("identity_namespace") != "ots/item_server_id"
        or binding.get("source_key") != "oteryn:source.crystalserver"
        or binding.get("source_revision") != "ff7ede593c69d4c658b382c97443e8155926924a"
        or digest(
            json.dumps(
                appearance, sort_keys=True, ensure_ascii=False, separators=(",", ":")
            ).encode()
        )
        != entry["decoded_record_sha256"]
    ):
        return None
    flags = appearance.get("flags", {})
    if flags.get("flags.take") is not True or any(
        flags.get(f) is True
        for f in CORPSE_FLAGS + GROUND_OR_BORDER_FLAGS + ("flags.unmove",)
    ):
        return None
    sources = entry["wiki_sources"]
    if not sources or len(observations) != len(sources):
        return None
    observed_by_page = {o["page_id"]: o for o in observations}
    if len(observed_by_page) != len(observations):
        return None
    for source in sources:
        fields = own_fields(source)
        coordinate = source["coordinates"]
        observed = observed_by_page.get(coordinate["page_id"], {})
        if (
            not fields
            or fields.get("itemid") != str(iid)
            or fields.get("primarytype") != "Others"
            or any(
                fields.get(k, "") for k in ("objectclass", "secondarytype", "status")
            )
            or fields.get("pickupable") != "yes"
            or any(
                normalized(fields.get(k, "")) != normalized(name)
                for k in ("name", "actualname")
            )
            or normalized(coordinate["title"]) != normalized(name)
            or coordinate["revision_timestamp"] > cutoff
            or any(observed.get(k) != coordinate[k] for k in COORDS)
            or observed.get("wiki_title") != coordinate["title"]
            or observed.get("fields", {}).get("primarytype") != "Others"
            or any(
                observed.get("fields", {}).get(k, "")
                for k in ("objectclass", "secondarytype", "status")
            )
        ):
            return None
    if iid == 51443:
        if (
            entry["mode"] != "OWN_WIKI_FUNCTION"
            or len(sources) != 1
            or own_fields(sources[0]).get("notes") != NOTES
            or entry["family_profile"] != "tool"
            or witness is not None
        ):
            return None
        refinement = {"field": "notes", "value": NOTES, "profile": "tool"}
    else:
        fields = dom_fields(witness or {})
        if (
            entry["mode"] != "EXTERNAL_CATEGORY_REFINEMENT"
            or not fields
            or any(normalized(n) != normalized(name) for n in fields[:2])
            or fields[2] != entry["family_profile"]
        ):
            return None
        refinement = {
            "field": "public_class_subclass",
            "links": [list(link) for link in fields[3]],
            "profile": fields[2],
            "public_source": witness,
        }
    return {
        "target": target,
        "source_taxonomy": {"primary": "Others"},
        "family_profile": entry["family_profile"],
        "source_evidence": {
            "classification": "DERIVED",
            "scope": "NAVIGATION_ONLY",
            "identity_authority": (
                "EXACT_CRYSTAL_BINDING_OWN_WIKI_ID_FUNCTION"
                if iid == 51443
                else "EXACT_CRYSTAL_BINDING_UNIQUE_PUBLIC_OFFICIAL_NATIVE_NAME"
            ),
            "source_binding": binding,
            "qualification": PATH,
            "qualification_sha256": SHA,
            "appearance_id": iid,
            "appearance_name": name,
            "decoded_record_sha256": entry["decoded_record_sha256"],
            "wiki_sources": [
                {
                    k: s[k]
                    for k in (
                        "coordinates",
                        "capture_coordinate",
                        "url",
                        "raw_own_infobox_sha256",
                        "revision_sha1",
                    )
                }
                for s in sources
            ],
            "refinement": refinement,
        },
    }


def build_external(definitions, snapshot, client, excluded, root=ROOT):
    document = json.loads(checked(root, PATH, SHA))
    sources = document["sources"]
    check_mapping(root, sources)
    for field in ("bindings", "stats", "client_artifact", "dom_witnesses"):
        checked(root, sources[field]["path"], sources[field]["sha256"])
    witnesses = json.loads((root / sources["dom_witnesses"]["path"]).read_text())[
        "records"
    ]
    entries = document["records"]
    if len(entries) != 18 or {e["appearance_id"] for e in entries} != IDS:
        raise ValueError("EXTERNAL_NAVIGATION_SCOPE")
    if set(witnesses) != {str(i) for i in IDS - {51443}}:
        raise ValueError("EXTERNAL_DOM_SCOPE")
    identity = build_identity_index()
    bindings = json.loads((root / sources["bindings"]["path"]).read_text())["bindings"]
    by_key, by_id = {}, {}
    for binding in bindings:
        by_key.setdefault(binding["target"]["key"], []).append(binding)
        by_id.setdefault(binding["external_id"], []).append(binding)
    names = Counter(normalized(o["name"]) for o in client.values() if o.get("name"))
    wiki = {r["item_id"]: r["observations"] for r in snapshot["records"].values()}
    out = []
    for entry in entries:
        iid, key = entry["appearance_id"], entry["target"]["key"]
        if (
            key in excluded
            or identity.get(iid, (None,))[0] != key
            or len(by_key.get(key, [])) != 1
            or len(by_id.get(str(iid), [])) != 1
        ):
            continue
        row = derive_external(
            entry,
            definitions.get(key),
            client.get(iid, {}),
            wiki.get(iid, []),
            by_key[key][0],
            [iid] if names[normalized(entry["official_name"])] == 1 else [],
            witnesses.get(str(iid)),
            sources["qualification_cutoff"],
        )
        if row:
            row["source_evidence"]["source_inputs"] = sources
            out.append(row)
    return out
