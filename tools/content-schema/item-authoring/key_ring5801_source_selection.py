"""Exact Item5801 weight/capacity qualifier; retains the retired Key Ring proof."""

import hashlib
import json
from pathlib import Path

KEY = "oteryn:item.tibia.i5801"
SCOPE = frozenset({"physical.weight", "container.capacity"})
MANIFEST = (
    "docs/agents/evidence/OTV2-20261001-item5801-temporal-source-qualification-v1.json"
)
MANIFEST_SHA256 = "07713ba72db6cb4106fc6f5b28bd27af93f3e9367676ccd6942dd6b8dc607898"


def require(condition, reason):
    if not condition:
        raise ValueError("Item5801 temporal qualifier: " + reason)


def native_field(definition, group, field):
    envelope = definition.get("semantics", {}).get(group, {"state": "UNKNOWN"})
    require(envelope.get("state") in {"UNKNOWN", "KNOWN"}, "blocked native group")
    return envelope.get("value", {}).get(field, {"state": "UNKNOWN"})


def qualify(manifest, record, definition, binding, appearance, *, routed=False):
    """Pure qualifier; input-byte digests are verified by load_context first."""
    require(not routed, "existing map owner")
    require(definition.get("identity", {}).get("key") == KEY, "native identity")
    require(binding == manifest["exact_binding"], "exact source binding drift")
    require(record.get("item_id") == 5801, "source id drift")
    require(
        record["observations"] == manifest["expected_snapshot_observations"],
        "snapshot observations drift",
    )
    require(
        manifest.get("item_key") == KEY and set(manifest["field_scope"]) == SCOPE,
        "scope drift",
    )
    current = manifest["required_current_source"]
    old = manifest["excluded_source"]
    raw = manifest["retained_raw_wiki_observations"]
    require(len(raw) == 2, "raw observation cardinality")
    for capture in raw:
        require(
            hashlib.sha256(capture["wikitext"].encode()).hexdigest()
            == capture["content_sha256"],
            "raw capture digest",
        )
    captures = {(c["page_id"], c["revision_id"], c["content_sha256"]): c for c in raw}
    old_key = (old["page_id"], old["revision_id"], old["content_sha256"])
    current_key = (
        current["page_id"],
        current["revision_id"],
        current["content_sha256"],
    )
    require(set(captures) == {old_key, current_key}, "raw observation identity")
    require(
        "| status        = deprecated" in captures[old_key]["wikitext"],
        "retired status",
    )
    official = manifest["official_retirement"]
    require(
        official["news_id"] == 1356 and official["article_date"] == "2010-06-09",
        "official retirement identity",
    )
    require(
        official["quote"]
        == "key rings will transform into a new container, the jewelled backpack, which will have 22 slots and weigh only 17 oz.",
        "official replacement facts",
    )
    require(
        appearance.get("id") == 5801 and appearance.get("name") == "jewelled backpack",
        "current client identity",
    )
    flags = appearance.get("flags", {})
    require(
        flags.get("flags.container") is True
        and flags.get("flags.take") is True
        and flags.get("market.category") == 4
        and flags.get("market.trade_as_object_id") == 5801
        and flags.get("market.show_as_object_id") == 5801,
        "affirmative client container identity",
    )
    capacity = native_field(definition, "container", "capacity")
    require(capacity == {"state": "KNOWN", "value": 22}, "native capacity precondition")
    weight = native_field(definition, "physical", "weight")
    require(
        weight == {"state": "UNKNOWN"} or weight == {"state": "KNOWN", "value": 1700},
        "native weight precondition",
    )
    name = native_field(definition, "presentation", "name")
    require(
        name.get("state") == "UNKNOWN"
        or (
            name.get("state") == "KNOWN"
            and name["value"].strip().lower() == "jewelled backpack"
        ),
        "native presentation hold or disagreement",
    )
    retained = [
        o
        for o in record["observations"]
        if (o["page_id"], o["revision_id"], o["content_sha256"]) == current_key
    ]
    require(
        len(retained) == 1
        and retained[0]["fields"].get("volume") == "22"
        and retained[0]["fields"].get("weight") == "17.00",
        "current source values",
    )
    return {
        "selected_observations": retained,
        "qualification": {
            "selector": {
                "path": "tools/content-schema/item-authoring/key_ring5801_source_selection.py",
                "sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            },
            "path": MANIFEST,
            "sha256": MANIFEST_SHA256,
            "classification": "DERIVED",
            "scope": sorted(SCOPE),
            "excluded_historical_observation": old,
            "required_current_observation": current,
        },
    }


def load_context(root, snapshot, definitions, routed_keys, appearances=None):
    """Load one immutable, input-digest-qualified selector; no general temporal rule."""
    root = Path(root)
    data = (root / MANIFEST).read_bytes()
    require(hashlib.sha256(data).hexdigest() == MANIFEST_SHA256, "manifest digest")
    manifest = json.loads(data)
    inputs = manifest["source_inputs"]
    digest = hashlib.sha256(
        json.dumps(
            snapshot["records"],
            sort_keys=True,
            ensure_ascii=False,
            separators=(",", ":"),
        ).encode()
    ).hexdigest()
    require(
        digest == snapshot["snapshot_sha256"] == inputs["snapshot_sha256"],
        "snapshot digest",
    )
    binding_bytes = (root / inputs["binding_path"]).read_bytes()
    require(
        hashlib.sha256(binding_bytes).hexdigest() == inputs["binding_sha256"],
        "binding digest",
    )
    bindings = [
        b
        for b in json.loads(binding_bytes)["bindings"]
        if b.get("target", {}).get("key") == KEY
    ]
    require(len(bindings) == 1, "binding cardinality")
    client_bytes = (root / inputs["client_path"]).read_bytes()
    require(
        hashlib.sha256(client_bytes).hexdigest() == inputs["client_sha256"],
        "client digest",
    )
    if appearances is None:
        from engine_items import load_appearance_objects

        appearances = load_appearance_objects(client_bytes)
    records = [r for r in snapshot["records"].values() if r["item_id"] == 5801]
    require(len(records) == 1, "source record cardinality")
    return qualify(
        manifest,
        records[0],
        definitions.get(KEY, {}),
        bindings[0],
        appearances.get(5801, {}),
        routed=KEY in routed_keys,
    )


def select(record, field_path, context=None):
    """Other IDs and fields keep every original page, including deprecated pages."""
    if context is None or record.get("item_id") != 5801 or field_path not in SCOPE:
        return record["observations"]
    return context["selected_observations"]
