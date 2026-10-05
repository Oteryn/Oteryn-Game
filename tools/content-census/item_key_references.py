#!/usr/bin/env python3
"""No-dangling-reference check for Item keys after the ITEM-ID-1 key switch (A12 §4.5, §5).

Scans `content/`, `imports/` and `apps/` for `oteryn:item.*` strings and fails closed on:

- a retired key (any key of `content/items/aliases.json`) outside the historical files
  listed in `HISTORY`, which name retired keys only to reproduce or translate immutable
  packets and never as a key of authored content;
- a Tibia key in `content/` with no Item record in `content/items/definitions`;
- a Tibia key in `imports/` that is neither an Item record nor a target of
  `imports/crystalserver/bindings/items.json` (which binds ids of the admitted CipSoft set,
  such as the epoch-2 donor ids, before a record is authored);
- a Tibia key in `apps/` that is neither an Item record nor in the admitted set, unless its
  id is above `MAX_APPEARANCE_ID` (65535): such an id can never be an appearance or Item
  id, so it is a synthetic key (the negative tests that assert an out-of-range id resolves
  to nothing) and names no content;
- any other `oteryn:item.*` string in `content/` or `imports/`, except the source-map alias
  form allowed in `SOURCE_ALIAS_FILES`.

`oteryn:item.source.canary.id<N>` is allowed only in the two pinned r25 spell source-world
files, and only when `<N>` has an Item record: the native spell world loader
(`native_spell_world.rs`) requires exactly that key and revision `source-map-r3` for every
non-native source binding, and both files are receipts of the pinned Canary revision.

Synthetic keys in `apps/` tests (for example `oteryn:item.chance.never`) are allowed: they
are not retired keys and name no content.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/content-schema/item-authoring"))

from appearance_membership import load_admitted

ALIAS_TABLE = "content/items/aliases.json"
DEFINITIONS_GLOB = "content/items/definitions/items-*.json"
CRYSTAL_BINDINGS = "imports/crystalserver/bindings/items.json"
KEY = re.compile(r"oteryn:item\.[a-z][a-z0-9_.-]*[a-z0-9_]")
TIBIA_KEY = re.compile(r"^oteryn:item\.tibia\.i([1-9][0-9]*)$")
# The §4.1 rule text (`oteryn:item.tibia.i<id>`) in the membership manifests and docs.
RULE_PREFIX = "oteryn:item.tibia.i"
SOURCE_ALIAS_KEY = re.compile(r"^oteryn:item\.source\.canary\.id([1-9][0-9]*)$")
SOURCE_ALIAS_FILES = {
    "content/test-packs/spells/r25/source_world.json",
    "imports/spells/r25/source-world.json",
}
# The appearance id is a u16, so a Tibia id above this can never name an appearance or Item.
MAX_APPEARANCE_ID = 65535
SUFFIXES = (".json", ".rs", ".toml", ".md")
SKIP_DIRS = ("content/assets/files",)
# Files that name retired keys as history: the historical importer and its tests reproduce
# the protected packets, the materializer applies its pre-switch overrides before the key
# switch, and the identity tests resolve retired keys through the alias table.
HISTORY = {
    ALIAS_TABLE,
    "apps/game-server/src/content/cw2_b1_import.rs",
    "apps/game-server/examples/materialize_content_world_project_v2.rs",
    "apps/game-server/tests/content_item_identity.rs",
    "apps/game-server/tests/content_world_cw2_b1_import.rs",
    "apps/game-server/tests/content_world_cw2_b1_promotion_lowering.rs",
}


def current_keys() -> dict[str, dict]:
    current: dict[str, dict] = {}
    table = json.loads((ROOT / ALIAS_TABLE).read_text(encoding="utf-8"))
    for entry in table["entries"]:
        current[entry["key"]] = entry
    return current


def record_keys() -> set[str]:
    keys = set()
    for path in sorted(ROOT.glob(DEFINITIONS_GLOB)):
        for record in json.loads(path.read_text(encoding="utf-8"))["records"]:
            keys.add(record["definition"]["identity"]["key"])
    return keys


def files(base: str):
    for path in sorted((ROOT / base).rglob("*")):
        relative = path.relative_to(ROOT).as_posix()
        if (
            path.is_file()
            and path.suffix in SUFFIXES
            and not relative.startswith(SKIP_DIRS)
        ):
            yield relative, path


def is_dangling_tibia(
    base: str,
    key: str,
    records: set[str],
    bound: set[str],
    admitted: set[int],
) -> bool:
    """Whether a well-formed Tibia key names no Item record, binding or admitted id."""
    if key in records:
        return False
    if base == "imports" and key in bound:
        return False
    if base == "apps":
        tibia_id = int(TIBIA_KEY.match(key).group(1))
        return tibia_id not in admitted and tibia_id <= MAX_APPEARANCE_ID
    return True


def check() -> dict[str, int]:
    retired = current_keys()
    records = record_keys()
    bindings = json.loads((ROOT / CRYSTAL_BINDINGS).read_text(encoding="utf-8"))
    bound = {row["target"]["key"] for row in bindings["bindings"]}
    _index, manifests = load_admitted()
    admitted = set().union(
        *({entry[0] for entry in doc["entries"]} for doc in manifests.values())
    )
    errors: list[str] = []
    counts = {"files": 0, "references": 0, "history_files": 0}
    for base in ("content", "imports", "apps"):
        for relative, path in files(base):
            text = path.read_text(encoding="utf-8")
            found = set(KEY.findall(text))
            if not found:
                continue
            counts["files"] += 1
            counts["references"] += len(found)
            if relative in HISTORY:
                counts["history_files"] += 1
                continue
            for key in sorted(found - {RULE_PREFIX}):
                if key in retired:
                    errors.append(f"RETIRED_KEY:{relative}:{key}")
                    continue
                tibia = TIBIA_KEY.match(key)
                if tibia is None:
                    alias = SOURCE_ALIAS_KEY.match(key)
                    if (
                        alias is not None
                        and relative in SOURCE_ALIAS_FILES
                        and f"oteryn:item.tibia.i{alias.group(1)}" in records
                    ):
                        continue
                    if base != "apps":
                        errors.append(f"NON_CANONICAL_KEY:{relative}:{key}")
                    continue
                if is_dangling_tibia(base, key, records, bound, admitted):
                    errors.append(f"DANGLING_KEY:{relative}:{key}")
    if errors:
        raise SystemExit("\n".join(errors[:50]) + f"\n{len(errors)} item key error(s)")
    return counts


def main() -> int:
    counts = check()
    print(f"item_key_references: PASS {json.dumps(counts, sort_keys=True)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
