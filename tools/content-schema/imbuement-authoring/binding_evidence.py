#!/usr/bin/env python3
"""Reproduce imbuement Item references from pinned client and canonical evidence.

Numeric IDs are lookup candidates only. A reference requires a live canonical
definition and a matching named identity, independently checked against the
client appearance and/or the imported revisioned item-page observation.
No definitions, assets, or runtime tables are modified by this tool.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
APPEARANCES_SHA256 = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
APPEARANCES = ROOT / f"content/assets/files/appearances-{APPEARANCES_SHA256}.dat"
ITEM_FACTS = ROOT / "imports/tibiawiki/facts/items-stats.json"
SOURCES = HERE / "samples/imbuement-sources-2026-10-01.json"
OUTPUT = HERE / "samples/imbuement-bindings.json"
ALIASES = {"Sabretooth (Item)": "Sabretooth"}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def normalized(name: str) -> str:
    return name.casefold()


def canonical_bytes(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode()


def client_objects() -> dict[int, dict]:
    """Reuse the strict 15.30 census protobuf reader; hash raw object messages."""
    spec = importlib.util.spec_from_file_location(
        "stage_proficiencies", ROOT / "tools/content-census/stage_proficiencies.py"
    )
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    raw = APPEARANCES.read_bytes()
    if sha256(raw) != APPEARANCES_SHA256:
        raise ValueError("primary client appearances digest mismatch")
    result = {}
    for field, wire, record in module.parse(raw):
        if (field, wire) != (1, 2):
            continue
        fields = module.parse(record)
        ids = [value for f, w, value in fields if (f, w) == (1, 0)]
        names = [value.decode("utf-8") for f, w, value in fields if (f, w) == (4, 2)]
        if len(ids) != 1 or len(names) > 1 or ids[0] in result:
            raise ValueError("malformed/duplicate client object identity")
        result[ids[0]] = {
            "id": ids[0], "name": names[0] if names else None,
            "record_sha256": sha256(record),
        }
    return result


def canonical_items() -> dict[str, dict]:
    result = {}
    for path in sorted((ROOT / "content/items/definitions").glob("items-*.json")):
        for record in json.loads(path.read_bytes())["records"]:
            definition = record["definition"]
            key = definition["identity"]["key"]
            if key in result:
                raise ValueError(f"duplicate canonical Item {key}")
            result[key] = {
                "definition": definition, "path": str(path.relative_to(ROOT)),
                "definition_sha256": sha256(canonical_bytes(definition)),
            }
    return result


def bind(source_name: str, client: dict, definitions: dict, item_facts: dict) -> dict:
    item_id = client["id"]
    key = f"oteryn:item.tibia.i{item_id}"
    canonical = definitions.get(key)
    if canonical is None:
        raise ValueError(f"{source_name}: no canonical Item for client object {item_id}")
    definition = canonical["definition"]
    identity = definition["identity"]
    if identity != {"family": "Item", "key": key, "revision": "definition-r1"}:
        raise ValueError(f"{source_name}: unsupported canonical identity")
    expected = normalized(ALIASES.get(source_name, source_name))
    name_record = definition.get("semantics", {}).get("presentation", {}).get("value", {}).get("name", {})
    canonical_name = name_record.get("value") if name_record.get("state") == "KNOWN" else None
    if canonical_name is not None and normalized(canonical_name) != expected:
        raise ValueError(f"{source_name}: conflicting canonical name {canonical_name}")
    if client["name"] is not None and normalized(client["name"]) != expected:
        raise ValueError(f"{source_name}: conflicting client name {client['name']}")
    fact = item_facts.get(key, {})
    wiki = [obs for obs in fact.get("observations", []) if normalized(obs["wiki_title"]) == expected]
    if wiki and fact["item_id"] != item_id:
        raise ValueError(f"{source_name}: wiki observation points at different item")
    if not ((client["name"] and (canonical_name or wiki)) or (canonical_name and wiki)):
        raise ValueError(f"{source_name}: numeric coincidence without independent named identity")
    # Unknown presentation (e.g. Basic scrolls) is qualified by the imported
    # wiki observation keyed to the canonical Item, not by inventing its name.
    return {
        "source_name": source_name,
        "item_ref": identity,
        "evidence": {
            "primary_client": client,
            "canonical": {
                "path": canonical["path"], "name": canonical_name,
                "definition_sha256": canonical["definition_sha256"],
            },
            "wiki_observations": [{
                k: obs[k] for k in ("wiki_title", "url", "page_id", "revision_id",
                                    "revision_sha1", "revision_timestamp", "content_sha256")
            } for obs in wiki],
            "identity_basis": "CLIENT_AND_WIKI" if wiki and client["name"] else (
                "CANONICAL_AND_WIKI_CLIENT_OBJECT_EXISTS" if wiki else "CLIENT_AND_CANONICAL_NAME"
            ),
        },
    }


def build() -> dict:
    objects = client_objects()
    definitions = canonical_items()
    facts_raw = ITEM_FACTS.read_bytes()
    facts = json.loads(facts_raw)["records"]
    source = json.loads(SOURCES.read_bytes())
    recipes = source["sources"]["wiki_br"]["records"]
    by_name = {}
    for client in objects.values():
        if client["name"]:
            by_name.setdefault(normalized(client["name"]), []).append(client)

    def named(name: str) -> dict:
        matches = by_name.get(normalized(ALIASES.get(name, name)), [])
        if len(matches) != 1:
            raise ValueError(f"{name}: expected unique named primary client object; got {len(matches)}")
        return bind(name, matches[0], definitions, facts)

    materials = {
        name: named(name) for name in sorted({m["name"] for r in recipes for m in r["incremental_materials"]})
    }
    scrolls = {
        r["name"]: {tier: named(f"{tier.title()} {r['name']} Scroll")
                    for tier in ("basic", "intricate", "powerful")}
        for r in sorted(recipes, key=lambda row: row["name"])
    }
    shrine_variants = []
    # Client-named house variants and revisioned world variants; abandoned
    # shrines are deliberately excluded because a decoration is not a tool.
    for item_id, client in sorted(objects.items()):
        names = {obs["wiki_title"] for obs in facts.get(f"oteryn:item.tibia.i{item_id}", {}).get("observations", [])}
        if client["name"] in ("imbuing shrine", "gilded imbuing shrine"):
            shrine_variants.append(bind(client["name"].title(), client, definitions, facts))
        elif "Imbuing Shrine" in names:
            shrine_variants.append(bind("Imbuing Shrine", client, definitions, facts))
    utilities = {
        "blank_scroll": named("Blank Imbuement Scroll"),
        "etcher": named("Etcher"),
        "heavy_old_tome": named("Heavy Old Tome"),
        "gold_token": named("Gold Token"),
        "shrine": shrine_variants,
    }
    return {
        "schema": "OTERYN_IMBUEMENT_ITEM_BINDINGS/v1",
        "authority": "DRAFT_SOURCE_QUALIFIED_REFERENCES; no runtime activation",
        "inputs": {
            "appearances": {"path": str(APPEARANCES.relative_to(ROOT)), "sha256": APPEARANCES_SHA256},
            "item_facts": {"path": str(ITEM_FACTS.relative_to(ROOT)), "sha256": sha256(facts_raw)},
            "recipe_facts": {
                "path": str(SOURCES.relative_to(ROOT)),
                "digest_scope": "sources.wiki_br.records canonical compact JSON",
                "sha256": sha256(canonical_bytes(recipes)),
            },
        },
        "counts": {"materials": len(materials), "scrolls": sum(len(t) for t in scrolls.values()),
                   "shrine_variants": len(shrine_variants)},
        "material_bindings": materials,
        "scroll_bindings": scrolls,
        "utility_bindings": utilities,
        "findings": [],
        "notes": [
            "Sabretooth (Item) is the BR disambiguated label for client/wiki Sabretooth; alias is explicit.",
            "Basic, Intricate and Powerful scrolls all exist in this pinned client; none are inferred from ID ranges.",
            "Primary client name absence for world shrines is qualified by canonical and revisioned wiki names.",
            "Abandoned Imbuing Shrine items 25101/25102 are constructions and excluded from usable shrine variants.",
            "Item materializable=false is preserved; this evidence binds identity without activating gameplay definitions.",
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="reject stale bindings instead of writing")
    args = parser.parse_args()
    output = (json.dumps(build(), ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_bytes() != output:
            raise SystemExit("imbuement bindings are stale; run binding_evidence.py")
    else:
        OUTPUT.write_bytes(output)
    print("Imbuement material, scroll and utility bindings verified.")


if __name__ == "__main__":
    main()
