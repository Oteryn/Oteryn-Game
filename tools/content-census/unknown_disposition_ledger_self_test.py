#!/usr/bin/env python3
"""Focused regressions for the pinned G3 blocker ledger."""
from __future__ import annotations
import pathlib
import sys
import tempfile

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import unknown_disposition_ledger as ledger

ARTIFACT = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path("/tmp/g3-input.zip")


def must_fail(fn, fragment: str) -> None:
    try:
        fn()
    except (ValueError, OSError, KeyError) as exc:
        assert fragment in str(exc), (fragment, str(exc))
    else:
        raise AssertionError(f"expected failure containing {fragment!r}")


def main() -> None:
    manifest, universe = ledger.load_g3(ARTIFACT)
    built, compact = ledger.build_ledger(manifest, universe)
    assert ledger.sha256_bytes(ledger.canonical_bytes(built) + b"\n") == compact["ledger_sha256"]
    rows = {row["page_id"]: row for row in built["pages"]}
    assert len(rows) == 5512 and built["counts"]["undispositioned"] == 0
    assert all(row["source_family"] == "UNKNOWN" for row in rows.values())
    assert all(row["target_identity"] is None and not row["gameplay_semantics_promoted"] for row in rows.values())
    assert all(row["revision_state"] == "PINNED_OBSERVATION_CURRENT_UNVERIFIED" for row in rows.values())

    # Preserve both lanes but count unique page-primary shapes only once.
    dual = [row for row in rows.values() if len(row["lane_observations"]) == 2]
    assert len(dual) == 37
    assert all({p["source_shape"] for p in row["lane_observations"]} == {"STRUCTURED_PRIMARY", "NO_INFOBOX_ITEM"} for row in dual)
    assert built["counts"]["page_primary_source_shapes"]["NO_INFOBOX_ITEM"] == 1399
    assert built["counts"]["all_lane_observations_by_shape"]["NO_INFOBOX_ITEM"] == 1436

    # Infobox Object on already classified Creature rows is not included among UNKNOWN.
    creature_object = [p for p in universe["pages"] if p["source_family_classification"].get("primary_definition_family") == "Creature" and any("Infobox Object" in t for src in p.get("provenance", []) for t in src.get("templates", []))]
    assert len(creature_object) == 6
    assert sum(1 for row in rows.values() if any("Infobox Object" in t for src in row["lane_observations"] for t in src.get("templates", []))) == 3122

    assert rows[19087]["blocker_class"] == "OBJECT_WORLD_QUEST_OVERLAP"
    winterlight = rows[45642]
    assert winterlight["source_family"] == "UNKNOWN"
    assert any("Infobox Mount/List" in t for src in winterlight["lane_observations"] for t in src.get("templates", []))
    assert winterlight["blocker_class"] == "WORLD_QUEST_SCOPE_UNRESOLVED"
    assert built["counts"]["blocker_classes"]["REDIRECT_TARGET_REQUIRES_DIRECT_EVIDENCE"] == 104
    assert built["counts"]["blocker_classes"]["ALTERNATE_SOURCE_ONLY"] == 187
    assert built["counts"]["blocker_classes"]["INFOBOX_ITEM_PARSE_ERROR"] == 7
    root_blocker_classes = {"RUNE_ROLE_UNRESOLVED", "COMMERCE_ROLE_UNRESOLVED", "GEOGRAPHY_ROLE_UNRESOLVED", "DAILY_TASK_ROLE_UNRESOLVED", "WORLD_QUEST_ROOT_ROLE_UNRESOLVED", "MINI_WORLD_CHANGE_ROLE_UNRESOLVED", "WORLD_CHANGE_ROOT_ROLE_UNRESOLVED", "HUNTING_PLACE_ROLE_UNRESOLVED", "MOUNT_ROLE_UNRESOLVED", "CYCLOPEDIA_ROLE_UNRESOLVED", "MAGIC_SOURCE_ROLE_UNRESOLVED", "TIBIADROME_ROLE_UNRESOLVED", "CURRENT_PAGE_ROLE_VERIFICATION_REQUIRED"}
    root_rows = [row for row in rows.values() if row["blocker_class"] in root_blocker_classes]
    assert len(root_rows) == 131
    assert all(row["evidence_pointer"] and row["next_action"] != "VERIFY_CURRENT_PAGE_ROLE_AND_DIRECT_PRIMARY_DEFINITION" for row in root_rows)
    assert all(row["lane_observations"][0].get("discovery_roots") for row in root_rows)

    duplicated = dict(universe)
    duplicated["pages"] = list(universe["pages"])
    unknown_indexes = [i for i, p in enumerate(duplicated["pages"]) if p["source_family_classification"].get("state") == "UNKNOWN"]
    dup_page = dict(duplicated["pages"][unknown_indexes[1]])
    dup_page["page_id"] = duplicated["pages"][unknown_indexes[0]]["page_id"]
    duplicated["pages"][unknown_indexes[1]] = dup_page
    must_fail(lambda: ledger.build_ledger(manifest, duplicated), "duplicated")

    with tempfile.TemporaryDirectory() as td:
        bad = pathlib.Path(td) / "corrupt.zip"
        bad.write_bytes(ARTIFACT.read_bytes()[:-1] + b"x")
        must_fail(lambda: ledger.load_g3(bad), "ZIP SHA256 mismatch")

    assert compact["invariants"]["current_revisions_verified"] is False
    print("unknown disposition ledger focused self-test: PASS")

if __name__ == "__main__":
    main()
