"""C5 source-grounded digging rows and XML return facts; no map execution."""


def append_rows(row, snapshot, xml, item, operand, fixed):
    # The common shovel branch opens target + 1; the pick's 606 branch uses 615.
    for tool, tools, transitions, lines in [
        ("shovel", (3457, 5710), [(i, i + 1) for i in snapshot["facts"]["digging"]["tables"]["holes"]], (401, 410)),
        ("pick", (3456,), [(593, 594), (606, 615), (608, 609), (867, 868)], (606, 625)),
    ]:
        for used in tools:
            for closed, opened in transitions:
                attributes = xml[opened]["attributes"]
                if "duration" not in attributes:
                    if (closed, opened) != (21341, 21342):
                        raise ValueError(f"unreviewed digging duration gap: {closed}->{opened}")
                    continue
                if int(attributes["decayTo"]) != closed:
                    raise ValueError(f"digging return mismatch: {closed}->{opened}")
                row(f"dig-{used}-{closed}", "digging", lines, used,
                    {"kind": "map_item", "definition": item(closed)},
                    [operand("USED", used, "KEEP")], fixed(), tool=tool,
                    opened_target=item(opened), revert_ms=int(attributes["duration"]) * 1000)


def map_facts(snapshot, xml, xml_sha256, xml_revision):
    timer_ids = [4612, 7237, 594, 607, 609, 615, 868, 7236, 21342]
    return {
        "schema": "OTERYN_CRAFT_MAP_FACTS/v1",
        "source": {"path": "imports/canary/items-xml/items.xml", "sha256": xml_sha256,
                   "revision": xml_revision},
        "opened_forms": [{"item_id": i, "lines": xml[i]["lines"],
                          "duration_ms": int(xml[i]["attributes"]["duration"]) * 1000 if "duration" in xml[i]["attributes"] else None,
                          "decay_to": int(xml[i]["attributes"]["decayTo"]) if "decayTo" in xml[i]["attributes"] else None}
                         for i in timer_ids],
        "excluded_transitions": [
            {"closed": closed, "opened": opened, "reason": "pinned XML has no return duration"}
            for closed, opened in [(7200, 7236), (21341, 21342)]
        ],
        "digging_source": snapshot["sources"]["digging"],
        "floor_ownership": "C5 preserves FLOOR-1 LocalObject transitions; these facts grant no runtime ownership",
    }
