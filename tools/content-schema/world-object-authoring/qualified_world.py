"""Closed static D93 authoring: 46 own-Wiki world primaries and three A12 fixtures.

Wiki supplies only its own primary owner; physical flags come from the complete
admitted official artifact. This adds no runtime or native Item admission rule.
"""

from __future__ import annotations

import hashlib
import json
import re
from collections import defaultdict
from functools import lru_cache
from pathlib import Path

# isort: off
# Existing official admission helper installs the shared source parsers.
import official_corpses as official
import world_objects
from item_wiki_family_capture import split_template_params
# isort: on

ROOT = official.ROOT
WIKI_PROFILE = "OTERYN_OFFICIAL_CLIENT_OWN_WIKI_WORLD/v1"
FIXED_PROFILE = "OTERYN_OFFICIAL_CLIENT_FIXED_GEOMETRY/v1"
PROFILES = {WIKI_PROFILE, FIXED_PROFILE}
QUALIFICATION = (
    Path(__file__).parent / "samples/qualified-world-wiki46-fixed3-15.30.json"
)
QUALIFICATION_SHA256 = (
    "6355ed079f1d73294e818f217827215fdfa3282edd33478e8ad193e6380d9472"
)
CENSUS = Path(__file__).parent / "samples/census-world-wiki46-fixed3-15.30.json"
WIKI_IDS = frozenset(
    {
        1019,
        1127,
        1128,
        5556,
        5557,
        5558,
        5559,
        6270,
        6271,
        6272,
        6273,
        10605,
        10986,
        10989,
        10991,
        10997,
        10998,
        10999,
        11000,
        11001,
        11002,
        11553,
        11554,
        12355,
        13883,
        13884,
        13885,
        18615,
        18616,
        18617,
        18618,
        18619,
        18620,
        20107,
        20888,
        20890,
        20891,
        20892,
        20893,
        20894,
        21772,
        21773,
        21774,
        22338,
        25101,
        25102,
    }
)
FIXED_IDS = frozenset({36929, 39949, 51302})
IDS = WIKI_IDS | FIXED_IDS
KEYS = frozenset(official.tibia_key(i) for i in IDS)


def qualification():
    raw = QUALIFICATION.read_bytes()
    if hashlib.sha256(raw).hexdigest() != QUALIFICATION_SHA256:
        raise ValueError("QUALIFIED_WORLD_SEAL")
    document = json.loads(raw)
    ids = [row["appearance_id"] for row in document["records"]]
    if len(ids) != 49 or set(ids) != IDS:
        raise ValueError("QUALIFIED_WORLD_SCOPE")
    return document


@lru_cache(maxsize=1)
def source_inputs():
    document = qualification()
    old, members, client, definitions, admitted, taxonomy, _, _ = (
        official.source_inputs()
    )
    if (
        document["official_source"]
        != {k: v for k, v in old["source"].items() if k != "profile"}
        or document["admission_source"] != old["admission_source"]
    ):
        raise ValueError("QUALIFIED_WORLD_OFFICIAL_SOURCE")
    wiki = document["wiki_source"]
    raw = (ROOT / wiki["path"]).read_bytes()
    if hashlib.sha256(raw).hexdigest() != wiki["sha256"]:
        raise ValueError("QUALIFIED_WORLD_WIKI_SOURCE")
    snapshot = json.loads(raw)
    bindings = defaultdict(list)
    reverse = defaultdict(list)
    for binding in json.loads(
        (ROOT / "imports/crystalserver/bindings/items.json").read_bytes()
    )["bindings"]:
        bindings[binding["target"]["key"]].append(binding)
        reverse[binding["external_id"]].append(binding)
    _index, manifests = official.appearance_membership.load_admitted()
    old_members = {row[0]: row for row in manifests["crystal-ff7ede5"]["entries"]}
    reviewed = {row["appearance_id"]: row for row in document["records"]}
    owners = set()
    generated_seen = set()
    for directory in ("objects", "terrain"):
        for path in (ROOT / "content/world" / directory).glob("*.json"):
            for record in json.loads(path.read_bytes()).get("records", []):
                entry = reviewed.get(record["provenance"]["source_item_id"])
                expected_shard = (
                    "terrain-08578-08611.json"
                    if entry and entry["owner"] == "Terrain"
                    else "objects-12917-12931.json"
                )
                if (
                    entry
                    and path.name == expected_shard
                    and entry["appearance_id"] not in generated_seen
                    and official.digest(record) == entry["validated_record_sha256"]
                ):
                    generated_seen.add(entry["appearance_id"])
                    continue
                owners.add(record["provenance"]["item_pointer"]["key"])
    return (
        document,
        members,
        client,
        definitions,
        admitted,
        taxonomy,
        owners,
        bindings,
        reverse,
        old_members,
        snapshot,
    )


def wiki_primary(page, item_id, cutoff):
    """Read duplicate-preserving own parameters, never page-wide lexical ItemID."""
    raw = page["raw_infobox"]
    parts = split_template_params(raw[2:-2])
    values = defaultdict(list)
    for part in parts[1:]:
        if "=" in part:
            key, value = part.split("=", 1)
            values[key.strip().lower()].append(value.strip())
    if (
        not raw.startswith("{{")
        or not raw.endswith("}}")
        or parts[0].strip().lower() != "infobox object"
        or dict(values) != page["parameter_values"]
        or hashlib.sha256(raw.encode()).hexdigest() != page["raw_infobox_sha256"]
        or page["revision_timestamp"] > cutoff
        or not re.fullmatch(r"[0-9a-f]{64}", page["content_sha256"])
        or any(
            len(values.get(key, [])) != 1
            for key in ("itemid", "primarytype", "pickupable")
        )
        or any(
            len(values.get(key, [])) > 1
            for key in ("actualname", "name", "objectclass", "status")
        )
        or not re.fullmatch(r"[1-9][0-9]*(?:\s*,\s*[1-9][0-9]*)*", values["itemid"][0])
        or item_id not in [int(i.strip()) for i in values["itemid"][0].split(",")]
        or values["pickupable"][0].lower() != "no"
    ):
        raise ValueError("QUALIFIED_WORLD_OWN_WIKI_ID_OR_PRIMARY")
    primary = values["primarytype"][0].lower()
    if primary not in official.engine_items.WORLD_OBJECT_PRIMARYTYPES:
        raise ValueError("QUALIFIED_WORLD_WIKI_OWNER")
    return primary


def observation(entry, appearance, source, page=None):
    """Reuse existing kind builders with honest primary input, no XML attributes."""
    item_id = entry["appearance_id"]
    attrs = {"primarytype": entry["primarytype"]} if page else {}
    # The builder's input adapter carries only the qualified Wiki primary. No XML
    # scalar is supplied or claimed; all generated source labels are rewritten below.
    primary_input = {"attrs": attrs, "name": ""} if page else None
    record = world_objects.build_record(
        entry["owner"],
        item_id,
        entry["target"]["key"],
        entry["reason"],
        primary_input,
        appearance,
        source,
    )
    fields = record["provenance"]["fields"]
    for field, labels in fields.items():
        fields[field] = [
            label.replace(
                "appearance:", f"official_client:{source['revision']}."
            ).replace("converter:", "derived:")
            for label in labels
        ]
    kind_flags = (
        ("flags.bank", "flags.fullbank", "flags.clip", "flags.bottom")
        if entry["owner"] == "Terrain"
        else (
            "flags.unpass",
            "flags.usable",
            "flags.forceuse",
            "flags.multiuse",
            "flags.container",
        )
    )
    fields.setdefault("kind", []).extend(
        f"official_client:{source['revision']}.{flag}" for flag in kind_flags
    )
    fields["route"] = [
        "derived:non_item_route" if page else "derived:immovable_non_item_route",
        f"official_client:{source['revision']}.flags.unmove",
    ]
    if page:
        label = f"tibiawiki:p{page['page_id']}.r{page['revision_id']}.{page['content_sha256']}.primarytype"
        fields["route"].append(label)
        fields["route"].append(
            f"tibiawiki:p{page['page_id']}.r{page['revision_id']}.{page['content_sha256']}.pickupable"
        )
        fields.setdefault("kind", []).append(label)
    return record


def build_record(
    entry,
    appearance,
    definition,
    member,
    admitted,
    taxonomy,
    owners,
    bindings,
    reverse,
    old_members,
    document,
    snapshot,
):
    item_id = entry["appearance_id"]
    target = entry["target"]
    key = official.tibia_key(member[0]) if member else None
    if (
        item_id not in IDS
        or member != entry["member"]
        or not member
        or member[0] != item_id
        or appearance.get("id") != item_id
        or official.digest(appearance) != entry["decoded_record_sha256"]
        or appearance.get("name") != entry["official_name"]
        or not definition
        or definition.get("identity") != target
        or target != {"family": "Item", "key": key, "revision": "definition-r1"}
        or definition.get("materializable") is not False
        or definition.get("stack_class") != "Unknown"
        or key in taxonomy
        or key in owners
    ):
        raise ValueError("QUALIFIED_WORLD_IDENTITY_OR_PRECEDENCE")
    presentation = definition.get("semantics", {}).get("presentation", {})
    name = presentation.get("value", {}).get("name", {})
    if (
        presentation.get("state") in ("CONFLICT", "NOT_APPLICABLE")
        or name.get("state") in ("CONFLICT", "NOT_APPLICABLE")
        or presentation != entry["native_presentation"]
        or appearance.get("name") is not None
        and name.get("state") == "KNOWN"
        and name.get("value") != appearance["name"]
    ):
        raise ValueError("QUALIFIED_WORLD_PRESENTATION")
    flags = appearance.get("flags", {})
    if flags.get("flags.unmove") is not True:
        raise ValueError("QUALIFIED_WORLD_POSITIVE_FIXED_FLAG")
    source = dict(document["official_source"])
    page = None
    if item_id in WIKI_IDS:
        binding = entry["binding"]
        if (
            not binding
            or bindings.get(key) != [binding]
            or reverse.get(str(item_id)) != [binding]
            or binding
            != {
                "disposition": "EXACT",
                "external_id": str(item_id),
                "identity_namespace": "ots/item_server_id",
                "source_key": "oteryn:source.crystalserver",
                "source_revision": "ff7ede593c69d4c658b382c97443e8155926924a",
                "target": target,
            }
            or old_members.get(item_id, [None, None])[1] != member[1]
        ):
            raise ValueError("QUALIFIED_WORLD_EXACT_REVERSE_BINDING")
        page = snapshot["pages"][entry["wiki_page_index"]]
        primary = wiki_primary(page, item_id, snapshot["qualification_cutoff"])
        if primary != entry["primarytype"] or official.engine_items.non_item_route(
            None, {"primarytype": primary}, flags, item_id
        ) != (entry["owner"], entry["reason"]):
            raise ValueError("QUALIFIED_WORLD_ACCEPTED_PRIMARY_ROUTE")
        source.update(
            profile=WIKI_PROFILE,
            wiki_source={
                **document["wiki_source"],
                **{k: page[k] for k in ("page_id", "revision_id", "content_sha256")},
            },
        )
    else:
        if (
            item_id not in admitted
            or entry["binding"] is not None
            or bindings.get(key)
            or entry["wiki_page_index"] is not None
            or flags.get("flags.unpass") is not True
            or "market.category" in flags
            or "clothes.slot" in flags
            or official.engine_items.classify_family_profile(
                {}, None, flags.get("clothes.slot")
            )
            is not None
            or official.engine_items.immovable_non_item_route(flags)
            != (entry["owner"], entry["reason"])
        ):
            raise ValueError("QUALIFIED_WORLD_NATIVE_FIXED_ROUTE")
        source["profile"] = FIXED_PROFILE
    record = observation(entry, appearance, source, page)
    if official.digest(record) != entry["validated_record_sha256"]:
        raise ValueError("QUALIFIED_WORLD_REVIEWED_RECORD")
    return record


def expected_record(item_id, inputs=None):
    (
        document,
        members,
        client,
        definitions,
        admitted,
        taxonomy,
        owners,
        bindings,
        reverse,
        old_members,
        snapshot,
    ) = inputs or source_inputs()
    entry = next(
        (row for row in document["records"] if row["appearance_id"] == item_id), None
    )
    if entry is None:
        raise ValueError("QUALIFIED_WORLD_UNREVIEWED_ID")
    return build_record(
        entry,
        client.get(item_id, {}),
        definitions.get(entry["target"]["key"]),
        members.get(item_id),
        admitted,
        taxonomy,
        owners,
        bindings,
        reverse,
        old_members,
        document,
        snapshot,
    )


def validate_record(record):
    try:
        if record != expected_record(record["provenance"]["source_item_id"]):
            return ["QUALIFIED_WORLD_SOURCE_OR_RECORD_CHANGED"]
    except (KeyError, ValueError, IndexError) as error:
        return [str(error)]
    return []


def outputs(existing_keys, starts):
    if existing_keys & KEYS:
        raise ValueError("QUALIFIED_WORLD_EXISTING_OWNER")
    inputs = source_inputs()
    records = [expected_record(i, inputs) for i in sorted(IDS)]
    files = {}
    for family, directory, prefix in (
        ("Terrain", "terrain", "terrain"),
        ("WorldObject", "objects", "objects"),
    ):
        rows = [record for record in records if record["identity"]["family"] == family]
        for row in rows:
            if errors := world_objects.validate_record(row):
                raise ValueError(f"QUALIFIED_WORLD_RECORD:{errors[:3]}")
        start = starts[family]
        files[
            f"content/world/{directory}/{prefix}-{start:05d}-{start + len(rows) - 1:05d}.json"
        ] = world_objects.canonical_bytes({"family": family, "records": rows}) + b"\n"
    census = {
        "schema": "OTERYN_QUALIFIED_WORLD49_CENSUS/v1",
        "qualification_sha256": QUALIFICATION_SHA256,
        "records": {
            "total": 49,
            "by_family": {"Terrain": 34, "WorldObject": 15},
            "digest_sha256": hashlib.sha256(
                b"".join(world_objects.canonical_bytes(row) + b"\n" for row in records)
            ).hexdigest(),
        },
    }
    files[CENSUS.relative_to(ROOT).as_posix()] = (
        world_objects.canonical_bytes(census) + b"\n"
    )
    return files
