"""Canonical inventory fields for the existing, complete Item range shards."""

from __future__ import annotations

import json
import re
from pathlib import Path

ITEM_SHARD_SIZE = 500
SHARD_DIRECTORY = "content/items/definitions"
SHARD_PATH = re.compile(r"content/items/definitions/items-([0-9]{5})-([0-9]{5})\.json")


def item_index_fields(root: Path, emitted_shards: list[str] | None = None) -> dict:
    """Read every actual range shard, or the converter's exact new output list.

    The converter retires superseded files later in its existing cleanup. Only
    that producer supplies a list; bootstrap must examine the full directory.
    No Item record, source pin or independently registered overlay is changed.
    """
    directory = root / SHARD_DIRECTORY
    if directory.is_symlink() or not directory.is_dir():
        raise ValueError("ITEM_INDEX_DIRECTORY")
    paths = emitted_shards if emitted_shards is not None else [
        path.relative_to(root).as_posix() for path in directory.iterdir()
        if path.name.startswith("items-")
    ]
    if (not paths or any(not isinstance(path, str) for path in paths)
            or len(set(paths)) != len(paths)):
        raise ValueError("ITEM_INDEX_SHARD_SET")
    paths = sorted(paths)
    count = 0
    identities = set()
    for ordinal, relative in enumerate(paths):
        match = SHARD_PATH.fullmatch(relative)
        path = root / relative
        if match is None or path.is_symlink() or not path.is_file():
            raise ValueError("ITEM_INDEX_SHARD_PATH:" + relative)
        start, end = map(int, match.groups())
        document = json.loads(path.read_bytes())
        if (not isinstance(document, dict)
                or document.get("schema") != "OTERYN_ITEM_AUTHORING_SHARD/v1"
                or document.get("family") != "Item"
                or document.get("source_legacy_role") != "content/world/definitions/reference.json"):
            raise ValueError("ITEM_INDEX_SHARD_SCHEMA:" + relative)
        rows = document.get("records")
        metadata = document.get("shard")
        if (not isinstance(rows, list) or not 0 < len(rows) <= ITEM_SHARD_SIZE
                or start != count or start != ordinal * ITEM_SHARD_SIZE
                or end != start + len(rows) - 1
                or (ordinal + 1 < len(paths) and len(rows) != ITEM_SHARD_SIZE)
                or not isinstance(metadata, dict)
                or any(type(value) is not int for value in metadata.values())
                or metadata != {"index": ordinal, "start": start, "end": end, "count": len(rows)}):
            raise ValueError("ITEM_INDEX_SHARD_RANGE:" + relative)
        for row in rows:
            definition = row.get("definition") if isinstance(row, dict) else None
            identity = definition.get("identity") if isinstance(definition, dict) else None
            if (not isinstance(identity, dict) or definition.get("kind") != "Item"
                    or set(identity) != {"family", "key", "revision"}
                    or identity["family"] != "Item"
                    or any(not isinstance(identity[field], str) or not identity[field]
                           for field in ("key", "revision"))):
                raise ValueError("ITEM_INDEX_IDENTITY:" + relative)
            key = (identity["family"], identity["key"], identity["revision"])
            if key in identities:
                raise ValueError("ITEM_INDEX_DUPLICATE_IDENTITY:" + identity["key"])
            identities.add(key)
        count += len(rows)
    return {"record_count": count, "shard_size": ITEM_SHARD_SIZE, "shards": paths}
