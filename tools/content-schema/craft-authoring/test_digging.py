"""Parity for every duration-backed digging branch and explicit source gaps."""

import craft_authoring as craft


def test_all_duration_backed_shovel_and_pick_rows_keep_the_xml_return():
    rows = [r["recipe"] for r in craft.build() if r["recipe"]["tool"] in ("shovel", "pick")]
    assert len(rows) == 12
    observed = set()
    for row in rows:
        used = row["operands"][0]
        assert row["operands"] == [craft.operand("USED", int(used["item_id"]["key"].split("i")[-1]), "KEEP")]
        assert row["outputs"] == craft.fixed()
        assert row["target"]["kind"] == "map_item"
        assert "target_effect" not in row and "tile_cooldown_ms" not in row
        assert "skill" not in row
        assert row["source"]["path"] == craft.source_snapshot()["sources"]["digging"]["path"]
        observed.add((row["input_item"]["key"], row["target"]["definition"]["key"],
                      row["opened_target"]["key"], row["revert_ms"]))
    expected = {
        (craft.item(tool)["key"], craft.item(closed)["key"], craft.item(opened)["key"], duration)
        for tool in (3457, 5710, 3456)
        for closed, opened, duration in [(593, 594, 60000), (606, 615 if tool == 3456 else 607, 300000),
                                         (608, 609, 300000), (867, 868, 600000)]
    }
    assert observed == expected


def test_missing_duration_transitions_are_fact_only_with_no_invented_timer():
    facts = craft.outputs()[craft.DIRECTORY + "map-facts.json"]
    assert facts["excluded_transitions"] == [
        {"closed": 7200, "opened": 7236, "reason": "pinned XML has no return duration"},
        {"closed": 21341, "opened": 21342, "reason": "pinned XML has no return duration"},
    ]
    for form in facts["opened_forms"]:
        if form["item_id"] in (7236, 21342):
            assert form["duration_ms"] is None and form["decay_to"] is None
    rows = [r["recipe"] for r in craft.build() if "opened_target" in r["recipe"]]
    assert all(r["opened_target"] not in [craft.item(7236), craft.item(21342)] for r in rows)
    assert facts["floor_ownership"] == "C5 preserves FLOOR-1 LocalObject transitions; these facts grant no runtime ownership"
