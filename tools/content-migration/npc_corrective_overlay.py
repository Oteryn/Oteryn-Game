"""Apply bounded NPC audit corrections to declarations; regenerate derived files separately.

The caller supplies an exact digest-bound baseline and the R4/R5 evidence files.
No source-only NPC, dialogue program, alias, runtime gate or Quest is admitted.
Output retains unrelated declarations and fields. Target facts must match either
the baseline or the corrected value, so reruns are safe and drift fails closed.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
from pathlib import Path


class OverlayError(ValueError):
    pass


def encode(value):
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode("utf-8")


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def index(records):
    result = {}
    for record in records:
        key = record["identity"]["key"]
        if key in result:
            raise OverlayError(f"duplicate declaration: {key}")
        result[key] = record
    return result


def field(path, value):
    return {
        "field_path": path,
        "value": {"type": "Text", "value": encode(value).decode().rstrip("\n")},
    }


def fields(records):
    result = {r["field_path"]: r for r in records}
    if len(result) != len(records):
        raise OverlayError("duplicate field_path")
    return result


def merge_fields(target, incoming):
    current = fields(target["fields"])
    fields(incoming)
    for addition in incoming:
        current[addition["field_path"]] = copy.deepcopy(addition)
    target["fields"] = list(current.values())


def lookup(record, pointer):
    parts = pointer.lstrip("/").split("/")
    parent = record
    for part in parts[:-1]:
        parent = parent[int(part)] if isinstance(parent, list) else parent[part]
    return parent, int(parts[-1]) if isinstance(parent, list) else parts[-1]


def validate_operation(row):
    """A supplied plan cannot broaden the audited native mutation boundary."""
    allowed = {
        "NPC": {"presentation"},
        "Service": {"offers", "routes"},
        "Dialogue": set(),
    }
    kind = row.get("kind")
    if kind not in allowed:
        raise OverlayError("only NPC/Dialogue/Service corrections are admitted")
    if set(row) != {"key", "kind", "identity", "values", "pointers", "fields"}:
        raise OverlayError("malformed correction operation")
    if row["identity"].get("key") != row["key"] or set(row["values"]) - allowed[kind]:
        raise OverlayError("identity or unsupported native field mutation")
    for name, change in row["values"].items():
        if set(change) != {"before", "after"}:
            raise OverlayError("malformed before/after fact")
        before, after = change["before"], change["after"]
        if kind == "NPC" and after is not None:
            raise OverlayError("presentation correction may only hold a reference")
        if kind == "Service":
            if not isinstance(before, list) or not isinstance(after, list):
                raise OverlayError("Service correction requires arrays")
            if name == "offers" and any(offer not in before for offer in after):
                raise OverlayError("Service correction may only remove existing offers")
            if name == "routes":
                for route in after:
                    price = route.get("price")
                    immutable = {k: v for k, v in route.items() if k != "price"}
                    if (
                        not isinstance(price, int)
                        or isinstance(price, bool)
                        or not 0 <= price < 2**64
                        or not any(
                            immutable == {k: v for k, v in old.items() if k != "price"}
                            for old in before
                        )
                    ):
                        raise OverlayError(
                            "only existing route removals and base-price changes are admitted"
                        )
    seen = set()
    for change in row["pointers"]:
        pointer = change.get("pointer")
        if (
            kind != "Dialogue"
            or not isinstance(pointer, str)
            or not re.fullmatch(r"(?:/keywords/\d+/reply|/send_trade)", pointer)
            or pointer in seen
        ):
            raise OverlayError("only existing Dialogue reply/message fields may change")
        if set(change) != {"pointer", "before", "after"}:
            raise OverlayError("malformed Dialogue change")
        for value in (change["before"], change["after"]):
            if not isinstance(value, list) or not all(
                isinstance(line, str) for line in value
            ):
                raise OverlayError("Dialogue correction requires text arrays")
        seen.add(pointer)
    seen = set()
    for change in row["fields"]:
        path = change.get("path")
        addition = change.get("after") or {}
        value = addition.get("value") or {}
        if (
            set(change) != {"path", "before", "after"}
            or not isinstance(path, str)
            or not path.startswith("oteryn:source.npc.")
            or path in seen
            or set(addition) != {"field_path", "value"}
            or addition.get("field_path") != path
            or set(value) != {"type", "value"}
            or value.get("type") != "Text"
            or not isinstance(value.get("value"), str)
        ):
            raise OverlayError("only typed NPC source evidence fields may change")
        seen.add(path)


def build_plan(
    baseline, r4, trade, transport, inventory, static, quest=None, custody=None
):
    """Prepare a reviewable before/after plan from the qualified correction inputs."""
    custody = custody or {}
    original = index(baseline["records"])
    target = {}
    pointers = {}

    def get(key, kind):
        if key not in original or original[key]["kind"] != kind:
            raise OverlayError(f"missing {kind}: {key}")
        return target.setdefault(key, copy.deepcopy(original[key]))

    def service_overlay(rows):
        for row in rows:
            record = get(row["identity"]["key"], "Service")
            if record["identity"] != row["identity"]:
                raise OverlayError("Service revision changed")
            allowed = {"kind", "identity", "fields", "offers", "routes"}
            if set(row) - allowed:
                raise OverlayError("unsupported Service correction field")
            for name in ("offers", "routes"):
                if name in row:
                    record[name] = copy.deepcopy(row[name])
            merge_fields(record, row["fields"])

    service_overlay(r4["records"])
    service_overlay(trade["services"])
    service_overlay(transport["services"])
    # Route facts stay in source fields, never in executable gate/discount slots.
    for key, record in list(target.items()):
        if "routes" not in record:
            continue
        observations = {
            name: [row for row in transport.get(name, []) if row.get("service") == key]
            for name in (
                "held_routes",
                "conditional_discounts",
                "route_access_observations",
                "access_route_holds",
                "scripted_route_holds",
                "base_fare_changes",
            )
        }
        if any(observations.values()):
            merge_fields(
                record,
                [
                    field(
                        "oteryn:source.npc.travel.audit_holds",
                        {
                            "runtime_qualified": False,
                            "native_quest_binding": None,
                            "custody": custody.get("transport"),
                            **observations,
                        },
                    )
                ],
            )
    held_by_npc = {}
    for row in transport.get("held_routes", []):
        held_by_npc.setdefault(row["npc"], []).append(row)
    for key, held in held_by_npc.items():
        merge_fields(
            get(key, "NPC"),
            [
                field(
                    "oteryn:source.npc.travel.unadmitted_routes",
                    {
                        "runtime_qualified": False,
                        "native_gate": None,
                        "custody": custody.get("transport"),
                        "held_routes": held,
                    },
                )
            ],
        )
    for row in inventory["presentation_corrections"]:
        record = get(row["npc_key"], "NPC")
        if (
            record.get("presentation") != row["source_reference"]
            or row["replacement_appearance"] is not None
        ):
            raise OverlayError(f"unexpected presentation: {row['npc_key']}")
        record["presentation"] = None
        merge_fields(
            record,
            [
                field(
                    "oteryn:source.npc.presentation_reference_hold",
                    {k: v for k, v in row.items() if k != "corrected_declaration"},
                )
            ],
        )
    for row in static["records"]:
        declaration = row["native_declaration"]
        key = declaration["identity"]["key"]
        record = get(key, "Dialogue")
        if (
            row["classification"] != "PROVEN"
            or digest(encode(original[key])) != row["active_dialogue_sha256"]
        ):
            raise OverlayError(f"unqualified or stale static Dialogue: {key}")
        changes = []
        for change in row["changes"]:
            pointer = change["pointer"]
            if not (
                pointer == "/send_trade"
                or (
                    pointer.startswith("/keywords/")
                    and pointer.endswith("/reply")
                    and len(pointer.split("/")) == 4
                    and pointer.split("/")[2].isdigit()
                )
            ):
                raise OverlayError(
                    "only existing static reply/message fields may change"
                )
            if change["kind"] not in (
                "STATIC_REPLY_REPLACEMENT",
                "RECORDED_NATIVE_MESSAGE_EXPANSION",
            ):
                raise OverlayError("unsupported static transformation")
            parent, part = lookup(record, pointer)
            if parent[part] != change["before"] or not change.get("proof"):
                raise OverlayError(f"unexpected static reply: {key}{pointer}")
            parent[part] = copy.deepcopy(change["after"])
            changes.append(
                {
                    "pointer": pointer,
                    "before": change["before"],
                    "after": change["after"],
                }
            )
        if record != declaration:
            raise OverlayError(
                f"static correction changes unrelated Dialogue facts: {key}"
            )
        pointers[key] = changes
        merge_fields(
            record,
            [
                field(
                    "oteryn:source.npc.static_speech_corrections",
                    {
                        "runtime_qualified": False,
                        "custody": custody.get("static"),
                        "changes": row["changes"],
                    },
                )
            ],
        )
    if quest is not None:
        if quest.get("runtime_eligible") is not False or quest.get(
            "native_quest_declarations"
        ):
            raise OverlayError("Quest input may contain only source associations")
        for position, row in enumerate(quest["records"]):
            key = row["npc_key"]
            record = get(key, "NPC")
            merge_fields(
                record,
                [
                    field(
                        "oteryn:source.npc.quest_bindings",
                        {
                            "locator": custody.get("quest", {}).get("locator"),
                            "sha256": custody.get("quest", {}).get("sha256"),
                            "record_pointer": f"/records/{position}",
                            "runtime_eligible": False,
                        },
                    )
                ],
            )
    planned = []
    for key in sorted(target):
        before, after = original[key], target[key]
        old_fields = fields(before["fields"])
        new_fields = fields(after["fields"])
        values = {
            name: {"before": before.get(name), "after": value}
            for name, value in after.items()
            if name not in ("fields", "keywords", "send_trade")
            and value != before.get(name)
        }
        if before["kind"] == "Service" and "offers" in values:
            if any(offer not in before["offers"] for offer in after["offers"]):
                raise OverlayError(f"trade correction adds or reprices an offer: {key}")
            held = [
                json.loads(f["value"]["value"])
                for f in new_fields.values()
                if f["value"]["type"] == "Text"
                and (
                    "held_offer_" in f["field_path"] or "held_tuple_" in f["field_path"]
                )
            ]
            if any(
                not any(h.get("old_native_offer") == offer for h in held)
                for offer in before["offers"]
                if offer not in after["offers"]
            ):
                raise OverlayError(
                    f"removed trade offer has no exact custody hold: {key}"
                )
        planned.append(
            {
                "key": key,
                "kind": before["kind"],
                "identity": before["identity"],
                "values": values,
                "pointers": pointers.get(key, []),
                "fields": [
                    {"path": path, "before": old_fields.get(path), "after": value}
                    for path, value in new_fields.items()
                    if value != old_fields.get(path)
                ],
            }
        )
    return {
        "schema": "oteryn.npc-corrective-overlay.v1",
        "records": planned,
        "runtime_qualified": False,
        "global_complete": False,
        "custody": custody,
    }


def apply_plan(document, plan):
    if plan.get("schema") != "oteryn.npc-corrective-overlay.v1":
        raise OverlayError("unexpected correction plan schema")
    keys = [row.get("key") for row in plan["records"]]
    if len(set(keys)) != len(keys):
        raise OverlayError("duplicate correction operation")
    for row in plan["records"]:
        validate_operation(row)
    result = copy.deepcopy(document)
    records = index(result["records"])
    for row in plan["records"]:
        record = records.get(row["key"])
        if (
            record is None
            or record["kind"] != row["kind"]
            or record["identity"] != row["identity"]
        ):
            raise OverlayError(f"declaration identity drift: {row['key']}")
        for name, change in row["values"].items():
            # Native serde omits empty Service arrays after canonical roundtrip.
            default = (
                []
                if row["kind"] == "Service" and name in ("offers", "routes")
                else None
            )
            if record.get(name, default) not in (change["before"], change["after"]):
                raise OverlayError(f"unexpected original fact: {row['key']}/{name}")
            record[name] = copy.deepcopy(change["after"])
        for change in row["pointers"]:
            parent, part = lookup(record, change["pointer"])
            if parent[part] not in (change["before"], change["after"]):
                raise OverlayError(
                    f"unexpected original reply: {row['key']}{change['pointer']}"
                )
            parent[part] = copy.deepcopy(change["after"])
        current = fields(record["fields"])
        for change in row["fields"]:
            if current.get(change["path"]) not in (change["before"], change["after"]):
                raise OverlayError(
                    f"unexpected source field: {row['key']}/{change['path']}"
                )
            current[change["path"]] = copy.deepcopy(change["after"])
        record["fields"] = list(current.values())
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in (
        "input",
        "baseline",
        "r4",
        "trade",
        "transport",
        "inventory",
        "static",
        "out",
        "plan-out",
        "baseline-out",
        "native-out",
    ):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--baseline-sha256", required=True)
    parser.add_argument("--quest", type=Path)
    parser.add_argument(
        "--evidence-root",
        required=True,
        help="Repository-relative R5 custody directory for proof locators",
    )
    args = parser.parse_args()
    raw = args.baseline.read_bytes()
    if digest(raw) != args.baseline_sha256:
        raise OverlayError("baseline SHA-256 mismatch")
    baseline = json.loads(raw)
    inputs, custody = {}, {}
    for name in ("r4", "trade", "transport", "inventory", "static", "quest"):
        path = getattr(args, name)
        if path is None:
            continue
        raw = path.read_bytes()
        inputs[name] = json.loads(raw)
        locator = (
            str(path)
            if name == "r4"
            else args.evidence_root.rstrip("/") + "/" + path.name
        )
        custody[name] = {"sha256": digest(raw), "locator": locator}
    plan = build_plan(baseline, **inputs, custody=custody)
    result = apply_plan(json.loads(args.input.read_bytes()), plan)
    baseline_index = index(baseline["records"])
    original_targets = {
        "records": [baseline_index[row["key"]] for row in plan["records"]]
    }
    corrected_index = index(result["records"])
    native = {
        "schema": "OTERYN_NPC_NATIVE_DECLARATION_REPAIRS/v1",
        "project_revision": "g4-npc-source-repairs-r10",
        "repairs": [
            {
                "identity": row["identity"],
                "before": baseline_index[row["key"]],
                "after": corrected_index[row["key"]],
            }
            for row in sorted(
                plan["records"],
                key=lambda row: (row["kind"], row["key"], row["identity"]["revision"]),
            )
        ],
    }
    for path, value in (
        (args.out, result),
        (args.plan_out, plan),
        (args.baseline_out, original_targets),
        (args.native_out, native),
    ):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(encode(value))
    print(
        json.dumps(
            {
                "corrected_records": len(plan["records"]),
                "baseline_targets_sha256": digest(encode(original_targets)),
                "native_repairs_sha256": digest(encode(native)),
                "static_speech_changes": sum(
                    len(row["pointers"]) for row in plan["records"]
                ),
                "routes_on_corrected_services": sum(
                    len(record.get("routes", []))
                    for record in result["records"]
                    if record["identity"]["key"] in {r["key"] for r in plan["records"]}
                ),
            }
        )
    )


if __name__ == "__main__":
    main()
