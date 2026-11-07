"""Preserve NPC travel source semantics without treating Lua evidence as native gates.

The native travel DTO currently has no gate/discount/effect/keyword fields. This tool
repairs source authoring evidence and base fares; it does not execute NPC travel or
assert Global completeness. Network captures must be supplied as pinned local inputs.
"""

import argparse
import copy
import hashlib
import json
import re
from pathlib import Path

SCHEMA = "OTERYN_NPC_TRAVEL_SOURCE_REPAIR/v1"
MAX_FARE = 1_000_000
POSTMAN_AMOUNT_PROOF = {
    "repository": "opentibiabr/canary",
    "revision": "47dfd51f45280a59a1d3e50ba7edd573d7234446",
    "path": "data/npclib/npc_system/custom_modules.lua",
    "source_line": 19,
    "sha256": "7eb3f3b2e3246bcccbd9447cd7461287e8635af004bf89ae465ce1c04a3a4abf",
    "amount_gold": 10,
    "source_storage": "Storage.Quest.ExampleQuest",
    "source_storage_minimum": 1,
    "native_mapping": None,
    "qualification": "OTS_PLACEHOLDER_NOT_GLOBAL_QUEST_GATE",
}


def canonical_keyword(value):
    return re.sub(r"[^a-z0-9]+", "_", value.lower()).strip("_")


def route_hold_reason(row):
    """Only an explicitly ungated, effect-free static source route may be promoted."""
    if row.get("gate") not in ("NONE", "LUA_PREDICATE") or row.get("effect") not in (
        "NONE",
        "LUA_ACTION",
    ):
        return "ROUTE_SEMANTICS_UNKNOWN"
    if row["effect"] != "NONE":
        return "SCRIPTED_ROUTE"
    if row["gate"] != "NONE":
        return "GATED_ROUTE"
    if not isinstance(row.get("destination"), dict) or set(row["destination"]) != {
        "x",
        "y",
        "z",
    }:
        return "ROUTE_SEMANTICS_UNKNOWN"
    return None


def source_route_signature(row):
    """Agreement includes effects and discounts; equal prices alone lose behavior."""
    return tuple(
        json.dumps(row.get(k), sort_keys=True, separators=(",", ":"))
        for k in (
            "destination",
            "price",
            "premium",
            "min_level",
            "gate",
            "effect",
            "discount",
        )
    )


def held_source_semantics(bundles, routes):
    return {
        source: {
            "source": copy.deepcopy(bundles[source].get("source", {})),
            "route": copy.deepcopy(row),
        }
        for source, row in sorted(routes.items())
    }


def discounted_fare(base, amounts):
    """A discount changes the payable fare, never the authored undiscounted base."""
    if type(base) is not int or not 0 <= base <= MAX_FARE:
        raise ValueError("base fare outside accepted bounds")
    if any(type(a) is not int or not 0 < a <= MAX_FARE for a in amounts):
        raise ValueError("discount amount outside accepted bounds")
    return max(0, base - sum(amounts))


def _lua_mask(lua):
    """Blank comments and strings, retaining character offsets and newlines."""
    output = list(lua)
    token = re.compile(
        r'--\[(=*)\[.*?\]\1\]|--[^\n]*|\[(=*)\[.*?\]\2\]|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'',
        re.DOTALL,
    )
    for match in token.finditer(lua):
        for i in range(match.start(), match.end()):
            if output[i] != "\n":
                output[i] = " "
    return "".join(output)


def source_call_evidence(snapshot, keyword):
    """Retain complete matching source calls as opaque evidence, never Lua lowering.

    Multiple same-key calls are all retained. Their ordering, surrounding branches
    and helper definitions still require qualification; no expression is executed.
    """
    if snapshot.get("state") != "EXACT_PINNED_SOURCE":
        return {"state": "SOURCE_UNAVAILABLE", "calls": [], "native_gate": None}
    lua = snapshot["lua"]
    if hashlib.sha256(lua.encode()).hexdigest() != snapshot["sha256"]:
        raise ValueError("pinned source digest does not match supplied Lua")
    mask = _lua_mask(lua)
    calls = []
    previous_call = None
    for match in re.finditer(
        r"\b(?:addTravelKeyword|addKeyword|addChildKeyword|addDestination|addAliasKeyword)\s*\(",
        mask,
    ):
        start, opening = match.start(), mask.index("(", match.start(), match.end())
        depth, end = 1, opening + 1
        while end < len(mask) and depth:
            depth += (mask[end] == "(") - (mask[end] == ")")
            end += 1
        if depth:
            continue
        first = re.match(
            r'\s*(?:\{\s*)?(["\'])(.*?)\1', lua[opening + 1 : end], re.DOTALL
        )
        code = lua[start:end]
        previous = previous_call
        previous_call = {
            "source_line": lua.count("\n", 0, start) + 1,
            "source_call": code,
        }
        if not first or first.group(2).lower() != keyword.lower():
            continue
        calls.append(
            {
                "source_previous_call": previous
                if "addAliasKeyword" in mask[start:opening]
                else None,
                "source_line": lua.count("\n", 0, start) + 1,
                "source_end_line": lua.count("\n", 0, end) + 1,
                "source_call_sha256": hashlib.sha256(code.encode()).hexdigest(),
                "source_call": code,
                "storage_symbols": sorted(
                    set(re.findall(r"\bStorage(?:\.[A-Za-z0-9_]+)+", code))
                ),
                "source_kind": "OPAQUE_LUA_CALL_NOT_NATIVE_GATE",
            }
        )
    helpers = []
    for helper in re.finditer(
        r"(?m)^local function addTravelKeyword\b[^\n]*\n(?:.*\n)*?^end\b[^\n]*", lua
    ):
        helpers.append(
            {
                "source_line": lua.count("\n", 0, helper.start()) + 1,
                "source_code": helper.group(),
                "source_code_sha256": hashlib.sha256(
                    helper.group().encode()
                ).hexdigest(),
                "scope": "OPAQUE_SHARED_HELPER_NOT_NATIVE_GATE",
            }
        )
    return {
        "state": "SOURCE_CALLS_RETAINED" if calls else "ROUTE_CALL_NOT_RESOLVED",
        "shared_helper_definitions": helpers,
        "source": {
            k: snapshot[k] for k in ("repository", "revision", "path", "sha256")
        },
        "calls": calls,
        "native_gate": None,
        "runtime_qualified": False,
    }


def repair_overlay(travel_records, candidates, correction_records, snapshots=None):
    """Build a deterministic overlay with explicit holds, prices and discount custody."""
    snapshots = snapshots or {}
    services = [copy.deepcopy(r.get("declaration", r)) for r in travel_records]
    by_service = {r["identity"]["key"]: r for r in services}
    fare_changes, quote_observations, discounts, held, extra_effects = (
        [],
        {},
        [],
        [],
        [],
    )
    access_observations, access_holds = [], []
    for ri, record in enumerate(correction_records):
        if not record.get("routes"):
            continue
        service = record["identity"]["key"]
        if service not in by_service:
            raise ValueError("correction names unknown travel Service")
        for field_index, source_field in enumerate(record.get("fields", [])):
            if source_field["field_path"] != "oteryn:source.npc.travel.conditions":
                continue
            for observation_index, observation in enumerate(
                json.loads(source_field["value"]["value"])["observations"]
            ):
                access_observations.append(
                    {
                        "service": service,
                        "route": observation["route_key"],
                        "source_observation": copy.deepcopy(observation),
                        "evidence_pointer": f"/records/{ri}/fields/{field_index}/value/value",
                        "decoded_pointer": f"/observations/{observation_index}",
                        "native_gate": None,
                        "runtime_qualified": False,
                    }
                )
        quote_fields = [
            (fi, field)
            for fi, field in enumerate(record.get("fields", []))
            if field["field_path"] == "oteryn:source.npc.travel.price_quotes"
        ]
        if len(quote_fields) != 1:
            raise ValueError(
                "travel correction needs exactly one base/conditional quote field"
            )
        fi, field = quote_fields[0]
        quotes = json.loads(field["value"]["value"])["quotes"]
        by_route = {r["key"]: r for r in by_service[service]["routes"]}
        proposed = {r["key"]: r for r in record["routes"]}
        if set(proposed) != set(by_route):
            raise ValueError("fare correction changes route coverage")
        for qi, quote in enumerate(quotes):
            route = quote["route_key"]
            proof = quote.get("proof_refs", {})
            if set(proof) != {"br", "fandom"} or any(
                not proof[source]
                or any(type(rev) is not int or rev <= 0 for rev in proof[source])
                for source in proof
            ):
                raise ValueError("base fare needs both pinned wiki revision claims")
            if (
                quote.get("currency") != "GOLD_GP"
                or quote["base_price"] != proposed[route]["price"]
            ):
                raise ValueError("quoted undiscounted base must equal proposed fare")
            if discounted_fare(quote["base_price"], [10]) != quote["postman_price"]:
                raise ValueError("conditional Postman quote must remain base minus10")
            quote_observations[(service, route)] = copy.deepcopy(quote)
            current = by_route[route]
            if {k: v for k, v in current.items() if k != "price"} != {
                k: v for k, v in proposed[route].items() if k != "price"
            }:
                raise ValueError(
                    "base price repair cannot change destination or eligibility"
                )
            if current["price"] != proposed[route]["price"]:
                fare_changes.append(
                    {
                        "service": service,
                        "route": route,
                        "old_base_price": current["price"],
                        "base_price": proposed[route]["price"],
                        "conditional_postman_price": quote["postman_price"],
                        "evidence_pointer": f"/records/{ri}/fields/{fi}/value/value",
                        "decoded_pointer": f"/quotes/{qi}",
                        "wiki_revision_claims": copy.deepcopy(proof),
                        "source_recaptured": False,
                        "runtime_qualified": False,
                    }
                )
                current["price"] = proposed[route]["price"]
        if any(by_route[key]["price"] != proposed[key]["price"] for key in proposed):
            raise ValueError("changed fare lacks a qualified base quote")
    for ci, candidate in enumerate(candidates):
        npc = candidate["identity"]["key"]
        for hi, hold in enumerate(candidate.get("left_out", [])):
            if not hold["fact"].startswith("travel."):
                continue
            word = hold["fact"][7:]
            sources = {
                source: source_call_evidence(snapshots.get(meta["key"], {}), word)
                for source, meta in sorted(candidate.get("provenance", {}).items())
            }
            held.append(
                {
                    "npc": npc,
                    "source_keyword": word,
                    "reason": hold["reason"],
                    "promotion_pointer": f"/candidates/{ci}/left_out/{hi}",
                    "source_constraints": sources,
                    "native_gate": None,
                    "admission_eligible": False,
                    "runtime_qualified": False,
                }
            )
        service = candidate.get("travel_service")
        if not service:
            continue
        sk = service["identity"]["key"]
        native = by_service[sk]
        for si, source_row in enumerate(service["routes"]):
            key = canonical_keyword(source_row["destination_keyword"])
            native_row = next(r for r in native["routes"] if r["key"] == key)
            if source_row.get("discount") == "postman":
                quote = quote_observations.get((sk, key))
                discounts.append(
                    {
                        "service": sk,
                        "route": key,
                        "source_discount_key": "postman",
                        "amount_gold": 10,
                        "amount_source_proof": copy.deepcopy(POSTMAN_AMOUNT_PROOF),
                        "quote_source_proof": copy.deepcopy(quote),
                        "base_price": native_row["price"],
                        "conditional_price": discounted_fare(native_row["price"], [10]),
                        "eligibility": {
                            "kind": "SOURCE_POSTMAN_PRIVILEGE",
                            "rank": "Grand Postman or higher" if quote else None,
                            "milestone": "Mission06 reported and Grand Postman privilege granted"
                            if quote
                            else None,
                            "native_track": None,
                        },
                        "source_qualification": "R4_TWO_WIKI_QUOTE_CLAIMS"
                        if quote
                        else "OTS_HYPOTHESIS_ONLY",
                        "promotion_pointer": f"/candidates/{ci}/travel_service/routes/{si}/discount",
                        "native_discount": None,
                        "runtime_qualified": False,
                    }
                )
            # Two source callbacks previously disappeared during promotion.
            if (sk, key) in {
                ("oteryn:service.travel.captain_bluebear", "carlin"),
                ("oteryn:service.travel.captain_seahorse", "venore"),
            }:
                constraints = {
                    src: source_call_evidence(
                        snapshots.get(meta["key"], {}),
                        source_row["destination_keyword"],
                    )
                    for src, meta in sorted(candidate["provenance"].items())
                }
                extra_effects.append(
                    {
                        "npc": npc,
                        "service": sk,
                        "route": copy.deepcopy(native_row),
                        "reason": "SCRIPTED_ROUTE",
                        "source_effect": "LUA_ACTION",
                        "source_constraints": constraints,
                        "native_effect": None,
                        "admission_eligible": False,
                        "runtime_qualified": False,
                    }
                )
                native["routes"].remove(native_row)
    for observation in access_observations:
        access = observation["source_observation"]["other_access"]
        if access["value"] != "OTHER_ACCESS_UNKNOWN":
            native = by_service[observation["service"]]
            row = next(r for r in native["routes"] if r["key"] == observation["route"])
            access_holds.append(
                {
                    "service": observation["service"],
                    "route": copy.deepcopy(row),
                    "reason": "GATED_ROUTE",
                    "source_access": copy.deepcopy(access),
                    "source_observation": copy.deepcopy(observation),
                    "native_gate": None,
                    "admission_eligible": False,
                    "runtime_qualified": False,
                }
            )
            native["routes"].remove(row)
    services.sort(key=lambda r: r["identity"]["key"])
    discounts.sort(key=lambda r: (r["service"], r["route"]))
    held.sort(key=lambda r: (r["npc"], r["source_keyword"]))
    return {
        "schema": SCHEMA,
        "evidence": "SOURCE_AUTHORING_ONLY_NOT_EXECUTABLE_TRAVEL",
        "runtime_qualified": False,
        "global_complete": False,
        "services": services,
        "base_fare_changes": fare_changes,
        "conditional_discounts": discounts,
        "held_routes": held,
        "scripted_route_holds": extra_effects,
        "route_access_observations": access_observations,
        "access_route_holds": access_holds,
        "counts": {
            "services": len(services),
            "routes_after_semantic_holds": sum(len(s["routes"]) for s in services),
            "base_fare_changes": len(fare_changes),
            "conditional_discounts": len(discounts),
            "previous_held_routes": len(held),
            "scripted_route_holds": len(extra_effects),
            "access_route_holds": len(access_holds),
            "held_routes_with_exact_source_calls": sum(
                any(s["calls"] for s in h["source_constraints"].values()) for h in held
            ),
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--packet", type=Path, required=True)
    parser.add_argument("--sources", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    def read(path):
        return json.loads(path.read_text(encoding="utf-8"))

    source_rows = read(args.sources)["records"] if args.sources else []
    result = repair_overlay(
        read(args.root / "content/services/travel/travel-00000-00055.json")["records"],
        read(
            args.root
            / "tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json"
        )["candidates"],
        read(args.packet / "service-corrections.json")["records"],
        {r["key"]: r for r in source_rows},
    )
    result["input_sha256"] = {
        str(path.relative_to(args.root))
        if path.is_relative_to(args.root)
        else path.name: hashlib.sha256(path.read_bytes()).hexdigest()
        for path in [
            args.root / "content/services/travel/travel-00000-00055.json",
            args.root
            / "tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json",
            args.packet / "service-corrections.json",
        ]
        + ([args.sources] if args.sources else [])
    }
    args.out.write_text(
        json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(result["counts"], sort_keys=True))


if __name__ == "__main__":
    main()
