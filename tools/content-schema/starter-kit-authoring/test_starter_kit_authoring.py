#!/usr/bin/env python3
"""No-network tests for starter_kit_authoring.py. Run with `python test_starter_kit_authoring.py`."""

from __future__ import annotations

import copy

import starter_kit_authoring as ska

BACKPACK = "oteryn:item.tibia.i2854"
COIN = "oteryn:item.tibia.i3031"
LABEL = "oteryn:starter-template.main"


def known(value) -> dict:
    return {"state": "KNOWN", "value": value}


def backpack() -> dict:
    return {
        "materializable": True,
        "stack_class": "NonStackable",
        "semantics": {
            "container": known({"capacity": known(20)}),
            "equipment": known(
                {
                    "patterns": known(
                        [{"pattern_id": 1, "primary_slot": known("CONTAINER")}]
                    )
                }
            ),
        },
    }


def items() -> dict:
    return {
        BACKPACK: backpack(),
        COIN: {"materializable": True, "stack_class": "StackCapable", "semantics": {}},
    }


def source(*rows: dict, label: str = LABEL) -> dict:
    return {
        "schema": ska.SOURCE_SCHEMA,
        "templates": [{"label": label, "records": list(rows)}],
    }


def row(
    item: str = BACKPACK, quantity: int = 1, key: str = "oteryn:starter.main_backpack"
) -> dict:
    return {
        "destination": "container_slot",
        "item": {"key": item, "revision": ska.ITEM_REVISION},
        "key": key,
        "quantity": quantity,
    }


def sealed(records: list) -> dict:
    return {
        label: {"record_count": len(rows), "records_sha256": ska.template_digest(rows)}
        for label, rows in ska.by_label(records).items()
    }


def errors_for(
    records: list, item_table: dict | None = None, seals: dict | None = None
) -> list:
    return ska.validate(
        records, item_table or items(), sealed(records) if seals is None else seals
    )


def test_record_shape() -> None:
    records = ska.build_records(source(row()))
    assert records == [
        {
            "definition": {
                "destination": "container_slot",
                "identity": {
                    "key": "oteryn:starter.main_backpack",
                    "revision": ska.REVISION,
                },
                "item": {
                    "family": "Item",
                    "key": BACKPACK,
                    "revision": ska.ITEM_REVISION,
                },
                "quantity": 1,
                "template": LABEL,
            }
        }
    ]
    assert errors_for(records) == []
    print("ok test_record_shape")


def test_admission_fails_closed() -> None:
    cases = []
    unadmitted = items()
    unadmitted[BACKPACK]["materializable"] = False
    cases.append((ska.build_records(source(row())), unadmitted))
    no_slot = items()
    no_slot[BACKPACK]["semantics"]["equipment"] = {"state": "UNKNOWN"}
    cases.append((ska.build_records(source(row())), no_slot))
    no_capacity = items()
    no_capacity[BACKPACK]["semantics"]["container"] = {"state": "UNKNOWN"}
    cases.append((ska.build_records(source(row())), no_capacity))
    cases.append((ska.build_records(source(row(item=COIN))), items()))
    cases.append((ska.build_records(source(row(quantity=2))), items()))
    cases.append((ska.build_records(source(row(item="oteryn:item.tibia.i1"))), items()))
    unknown_revision = ska.build_records(source(row()))
    unknown_revision[0]["definition"]["item"]["revision"] = "definition-r9"
    cases.append((unknown_revision, items()))
    bad_destination = ska.build_records(source(row()))
    bad_destination[0]["definition"]["destination"] = "store_inbox"
    cases.append((bad_destination, items()))
    cases.append(
        (ska.build_records(source(row(), row(key="oteryn:starter.second"))), items())
    )
    cases.append((ska.build_records(source(row(), row())), items()))
    cases.append((ska.build_records(source(row(key="Starter.Backpack"))), items()))
    cases.append((ska.build_records(source(row(), label="starter-1")), items()))
    for records, item_table in cases:
        assert errors_for(records, item_table), records
    print("ok test_admission_fails_closed")


def test_labels_are_immutable() -> None:
    records = ska.build_records(source(row()))
    seals = sealed(records)
    # Unsealed records.
    assert errors_for(records, seals={})
    # A changed record set under a sealed label.
    changed = copy.deepcopy(records)
    changed[0]["definition"]["identity"]["key"] = "oteryn:starter.backpack"
    assert errors_for(changed, seals=seals)
    # A sealed label that lost its records.
    assert ska.seal_errors([], seals)
    # A new label next to the sealed one is fine once sealed itself.
    both = records + ska.build_records(
        source(row(), label="oteryn:starter-template.second")
    )
    assert errors_for(both, seals=seals)
    assert errors_for(both) == []
    print("ok test_labels_are_immutable")


def test_committed_family() -> None:
    assert ska.committed_errors() == [], ska.committed_errors()
    assert ska.content_command(check=True) == 0
    print("ok test_committed_family")


if __name__ == "__main__":
    test_record_shape()
    test_admission_fails_closed()
    test_labels_are_immutable()
    test_committed_family()
