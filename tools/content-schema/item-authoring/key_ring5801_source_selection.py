"""Exact Item5801 weight/capacity qualifier; retains the retired Key Ring proof."""

import hashlib
import json
from pathlib import Path

KEY = "oteryn:item.tibia.i5801"
SCOPE = frozenset({"physical.weight", "container.capacity"})
MANIFEST = (
    "docs/agents/evidence/OTV2-20261001-item5801-temporal-source-qualification-v1.json"
)
MANIFEST_SHA256 = "7980b15e56dae1c23c15129b9ea85a7bac080a817d9b004ab65876884b7aacb9"


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


def refresh_input_pins(root):
    """Retain the exact5801 observation pair across reviewed snapshot re-keying."""
    import re
    import sys

    sys.path.insert(0, str(root / "tools/content-migration"))
    sys.path.insert(0, str(root / "tools/content-schema/native-gameplay"))
    from build_spell_item_identities import historical_binding_bytes
    from source_relocations import read_pinned

    path = root / MANIFEST
    original = path.read_bytes()
    require(hashlib.sha256(original).hexdigest() == MANIFEST_SHA256, "manifest digest")
    manifest = json.loads(original)
    inputs = manifest["source_inputs"]
    snapshot_raw = (root / inputs["snapshot_path"]).read_bytes()
    require(
        read_pinned(
            root,
            inputs["snapshot_path"],
            "24e3e57fddc6c14f00aa5c4976e91117d31bc2102bd3835b3d7891d9e41fbeae",
        )
        == snapshot_raw,
        "unreviewed snapshot change",
    )
    snapshot = json.loads(snapshot_raw)
    (record,) = [r for r in snapshot["records"].values() if r["item_id"] == 5801]
    require(
        record["observations"] == manifest["expected_snapshot_observations"],
        "5801 observations changed",
    )
    binding_raw = (root / inputs["binding_path"]).read_bytes()
    historical_binding_bytes(binding_raw, inputs["binding_sha256"])
    selected = [
        row
        for row in json.loads(binding_raw)["bindings"]
        if row["external_id"] == "5801"
    ]
    require(selected == [manifest["exact_binding"]], "5801 exact binding changed")
    require(
        hashlib.sha256((root / inputs["client_path"]).read_bytes()).hexdigest()
        == inputs["client_sha256"],
        "client digest",
    )
    records_digest = hashlib.sha256(
        json.dumps(
            snapshot["records"],
            sort_keys=True,
            ensure_ascii=False,
            separators=(",", ":"),
        ).encode()
    ).hexdigest()
    require(records_digest == snapshot["snapshot_sha256"], "snapshot records digest")
    inputs["snapshot_sha256"] = records_digest
    inputs["binding_sha256"] = hashlib.sha256(binding_raw).hexdigest()
    output = (
        json.dumps(manifest, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    ).encode()
    path.write_bytes(output)
    selector = Path(__file__)
    text, count = re.subn(
        r'(?m)^MANIFEST_SHA256 = "[0-9a-f]+"',
        'MANIFEST_SHA256 = "' + hashlib.sha256(output).hexdigest() + '"',
        selector.read_text(),
    )
    require(count == 1, "selector pin anchor")
    selector.write_text(text)


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--refresh-input-pins", action="store_true", required=True)
    parser.parse_args()
    refresh_input_pins(Path(__file__).resolve().parents[3])
