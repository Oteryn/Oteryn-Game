"""Loads the committed Crystal id -> Item key identity bindings
(`imports/crystalserver/bindings/items.json`, `OTERYN_SOURCE_IDENTITY_BINDINGS/v1`,
38,157 `EXACT` `ots/item_server_id` bindings). Both engines share this one Crystal id
space (`tools/content-schema/item-authoring/engine_items.py::build_identity_index`), so
a Canary `itemtype` id is looked up in the same table as a Crystal one. Every row's
`disposition`/`identity_namespace`/`source_key`/`target.family`/`target.revision` is
validated fail-closed, mirroring the item package's own loader; this is a small,
store-package-local copy rather than an import of that 100k-line module, per
minimum-sufficient scope."""

from __future__ import annotations

import json
from pathlib import Path

SCHEMA = "OTERYN_SOURCE_IDENTITY_BINDINGS/v1"
NAMESPACE = "ots/item_server_id"
SOURCE_KEY = "oteryn:source.crystalserver"
DEFINITION_REVISION = "definition-r1"


def load_item_bindings(repo_root: Path) -> dict[int, str]:
    path = repo_root / "imports" / "crystalserver" / "bindings" / "items.json"
    catalog = json.loads(path.read_text(encoding="utf-8"))

    def fail(reason: str):
        raise SystemExit(f"{path}: {reason}")

    if catalog.get("schema") != SCHEMA:
        fail(f"unexpected schema {catalog.get('schema')!r}")
    if catalog.get("family") != "Item":
        fail(f"unexpected family {catalog.get('family')!r}")
    bindings = catalog.get("bindings")
    if not isinstance(bindings, list) or not bindings:
        fail("bindings must be a non-empty list")
    index: dict[int, str] = {}
    for binding in bindings:
        if binding.get("disposition") != "EXACT":
            fail(f"unexpected disposition {binding.get('disposition')!r}")
        if binding.get("identity_namespace") != NAMESPACE:
            fail(f"unexpected identity_namespace {binding.get('identity_namespace')!r}")
        if binding.get("source_key") != SOURCE_KEY:
            fail(f"unexpected source_key {binding.get('source_key')!r}")
        target = binding.get("target")
        if not isinstance(target, dict) or target.get("family") != "Item":
            fail(f"unexpected target.family in binding {binding!r}")
        if target.get("revision") != DEFINITION_REVISION:
            fail(f"unexpected target.revision {target.get('revision')!r}")
        key = target.get("key")
        external_id = binding.get("external_id")
        if (
            not key
            or not isinstance(key, str)
            or not external_id
            or not isinstance(external_id, str)
        ):
            fail(f"missing/invalid key or external_id in binding {binding!r}")
        index[int(external_id)] = key
    return index


def item_ref(index: dict[int, str], item_id: int) -> dict | None:
    key = index.get(item_id)
    if key is None:
        return None
    return {"family": "Item", "key": key, "revision": DEFINITION_REVISION}
