#!/usr/bin/env python3
"""No-network checks for `item_wiki_stats_capture.py` (ITEM-SEM-2a).

Synthetic pages cover the parser (nested templates, one page listing several ids, a
junk itemid, a page without an infobox), the key rule (an Item key only where it
resolves, the bare id otherwise) and the admitted parameter allowlist; the committed
snapshot is checked for its own digest, key rule, allowlist and attribution.
"""

from __future__ import annotations

import hashlib
import json

import item_wiki_stats_capture as capture

PAGE = """{{Infobox Object|List={{{1|}}}|GetValue={{{GetValue|}}}
| name          = Soulcrusher
| itemid        = 34086
| levelrequired = 400
| vocrequired   = knights
| attack        = 6
| ice_attack    = 46
| defensemod    = +3
| attrib        = club fighting +4, {{Link|magic level}} +1
| notes         = Part of the [[Soul Set]].
| droppedby     = {{Dropped By|}}
}}
Article text that is never stored."""


def page(content, page_id=1, revision_id=10):
    return {
        "content": content,
        "page_id": page_id,
        "revision_id": revision_id,
        "sha1": "0" * 40,
        "timestamp": "2026-01-01T00:00:00Z",
    }


KNOWN = {"oteryn:item.tibia.i34086", "oteryn:item.tibia.i100"}


def test_parser_keeps_only_admitted_stats():
    fields = capture.infobox_fields(PAGE)
    assert fields["attrib"] == "club fighting +4, {{Link|magic level}} +1", fields
    assert fields["notes"].startswith("Part of"), fields
    records, report = capture.build_records({"Soulcrusher": page(PAGE)}, KNOWN)
    record = records["oteryn:item.tibia.i34086"]
    stats = record["observations"][0]["fields"]
    assert set(stats) <= set(capture.STAT_PARAMS), stats
    assert "notes" not in stats and "droppedby" not in stats and "name" not in stats
    assert stats["defensemod"] == "+3" and stats["ice_attack"] == "46", stats
    assert report["records"] == 1 and report["no_infobox"] == 0, report
    text = json.dumps(records)
    assert "Article text" not in text and "Soul Set" not in text, "no article text"


def test_multi_id_bad_id_and_no_infobox():
    shared = "{{Infobox Object\n| itemid = 100, 101\n| weight = 1.00\n}}"
    junk = "{{Infobox Object\n| itemid = 12 (old)\n| weight = 2.00\n}}"
    records, report = capture.build_records(
        {
            "Shared": page(shared, page_id=2),
            "Other": page(shared, page_id=1),
            "Junk": page(junk, page_id=3),
            "Plain": page("no infobox here", page_id=4),
        },
        KNOWN,
    )
    assert list(records) == ["oteryn:item.tibia.i100", "101"], list(records)
    assert records["101"]["item_id"] == 101 and report["unbound_ids"] == 1, report
    ids = [row["page_id"] for row in records["oteryn:item.tibia.i100"]["observations"]]
    assert ids == [1, 2], ids
    assert report["bad_itemid"] == ["Junk"] and report["no_infobox"] == 1, report
    assert report["multi_page_ids"] == 2, report


def test_page_without_stats_is_skipped():
    records, _report = capture.build_records(
        {"Bare": page("{{Infobox Object\n| itemid = 7\n| name = x\n}}")}, KNOWN
    )
    assert records == {}, records


def test_committed_snapshot_is_consistent():
    document = json.loads(capture.DEFAULT_OUTPUT.read_text(encoding="utf-8"))
    assert (
        document["schema"] == capture.SCHEMA
        and document["batch_id"] == capture.BATCH_ID
    )
    assert document["source"]["stat_params"] == list(capture.STAT_PARAMS)
    assert document["source"]["attribution"] == capture.ATTRIBUTION
    known = capture.resolvable_item_keys()
    records = document["records"]
    digest = hashlib.sha256(capture.canonical_records_bytes(records)).hexdigest()
    assert digest == document["snapshot_sha256"], "snapshot digest"
    assert list(records) == sorted(records), "records are written in key order"
    for key, record in records.items():
        assert key == capture.record_key(record["item_id"], known), key
        pages = [row["page_id"] for row in record["observations"]]
        assert pages == sorted(pages) and len(set(pages)) == len(pages), key
        for row in record["observations"]:
            assert row["fields"] and set(row["fields"]) <= set(capture.STAT_PARAMS), key
    soulcrusher = records["oteryn:item.tibia.i34086"]["observations"][0]["fields"]
    assert soulcrusher["levelrequired"] == "400" and soulcrusher["hpleech_am"] == "2%"
    return len(records)


def main():
    test_parser_keeps_only_admitted_stats()
    test_multi_id_bad_id_and_no_infobox()
    test_page_without_stats_is_skipped()
    count = test_committed_snapshot_is_consistent()
    print(f"item_wiki_stats_capture self-test: PASS records={count}")


if __name__ == "__main__":
    main()
