"""Preserve unresolved trade tuples as source evidence, never guessed native offers.

The OT fluid numbers are a source namespace. ProjectV2ServiceOffer.sub_type has
no accepted OT-to-native binding; its count is transaction quantity. A source
callback is likewise not a native quest predicate. This module deliberately
holds those tuples until the owning contracts provide their executable mapping.
"""

import copy
import hashlib
import json
import re
from collections import Counter
from pathlib import Path

CANARY_REVISION = "47dfd51f45280a59a1d3e50ba7edd573d7234446"
CRYSTAL_REVISION = "ff7ede593c69d4c658b382c97443e8155926924a"
FLUID_ENUM = dict(
    none=0,
    water=1,
    wine=2,
    beer=3,
    mud=4,
    blood=5,
    slime=6,
    oil=7,
    urine=8,
    milk=9,
    mana=10,
    life=11,
    lemonade=12,
    rum=13,
    **{"fruit juice": 14, "coconut milk": 15},
    mead=16,
    tea=17,
    ink=18,
    candy=19,
    chocolate=20,
)
# Identified container appearance IDs, not a mapping to native fluid subtypes.
FLUID_CONTAINERS = {2874, 2875, 2880, 2901, 9232}
ACCESS_CALLBACKS = {
    "rashid": (
        "TheTravellingTrader.Mission07",
        1,
        266,
        "9cc53e95dce796be39ff4c903b4806068cbb0d31aa13a700e335dfab0e642db1",
    ),
    "haroun": (
        "DjinnWar.MaridFaction.Mission03",
        3,
        133,
        "074bd59038d9627be2d384d53c85952fda9dba14835620081b0ba122ed346344",
    ),
    "alesar": (
        "DjinnWar.EfreetFaction.Mission03",
        3,
        198,
        "57ae144469c9532bf93ac1ed4b5b0af357db4b6203c55a1e2bc489adbe2f1db6",
    ),
    "nah_bob": (
        "DjinnWar.MaridFaction.Mission03",
        3,
        128,
        "d4294e61951becb49f0aac800ec597831999209271e67a4d4a21978265ccbdc4",
    ),
    "yaman": (
        "DjinnWar.EfreetFaction.Mission03",
        3,
        160,
        "29c74e7366f757943d795102f6023f43464dec690bc4cb3a8ffdb30112c6d9f4",
    ),
}


def access_fact(npc_slug):
    """A pinned source hypothesis requiring a native access predicate."""
    callback = ACCESS_CALLBACKS.get(npc_slug)
    if callback is None:
        return None
    storage, expected, line, sha = callback
    return {
        "source_callback": "CALLBACK_ON_TRADE_REQUEST",
        "source_predicate": {"storage": storage, "operator": "==", "value": expected},
        "source_proof": {
            "repository": "opentibiabr/canary",
            "revision": CANARY_REVISION,
            "path": f"data-otservbr-global/npc/{npc_slug}.lua",
            "sha256": sha,
            "line": line,
        },
        "authority": "OTS_HYPOTHESIS_ONLY",
        "native_predicate": None,
        "runtime_qualified": False,
    }


def source_fluid_fact(offer, source):
    """Recognize an explicit fluid row without turning its enum into quantity."""
    item_id = offer.get("client_id") or offer.get("server_item_id")
    if item_id not in FLUID_CONTAINERS:
        return None
    match = re.fullmatch(r".+ of (.+)", offer.get("item_name", "").casefold())
    fluid = match.group(1) if match else None
    if fluid not in FLUID_ENUM:
        return None
    encoded = (
        offer.get("sub_type")
        if offer.get("sub_type") is not None
        else offer.get("count")
    )
    return {
        "source_fluid_name": fluid,
        "source_fluid_enum": FLUID_ENUM[fluid],
        "source_encoded_value": encoded,
        "source_encoding_matches_enum": encoded == FLUID_ENUM[fluid],
        "old_source_offer": copy.deepcopy(offer),
        "source_proof": copy.deepcopy(source),
        "quantity_state": "UNKNOWN_SOURCE_FLUID_ENUM_NOT_TRANSACTION_UNITS",
        "native_sub_type": None,
        "runtime_qualified": False,
    }


def source_quotes(raw, source):
    """Read static shop rows from digest-checked pinned capture bytes only."""
    if hashlib.sha256(raw).hexdigest() != source.get("sha256"):
        raise ValueError("source capture SHA-256 mismatch")
    if (source.get("repository"), source.get("revision")) not in {
        ("opentibiabr/canary", CANARY_REVISION),
        ("zimbadev/crystalserver", CRYSTAL_REVISION),
    }:
        raise ValueError("unrecognized source pin")
    text = raw.decode("utf-8")
    start = re.search(r"npcConfig\.shop\s*=\s*\{", text)
    if start is None:
        return []
    # Static shop rows have no nested tables; a malformed/non-static row is
    # not assigned an inferred value. Capture locators stay in every fact.
    end = text.find("\n}", start.end())
    if end < 0:
        raise ValueError("unterminated static NPC shop")
    facts = []
    for match in re.finditer(r"\{([^{}]*)\}", text[start.end() : end]):
        row = match.group(1)
        name = re.search(r'\bitemName\s*=\s*"([^"\n]*)"', row)
        if name is None:
            continue
        nums = {
            key: int(value)
            for key, value in re.findall(
                r"\b(clientId|buy|sell|count|subType)\s*=\s*(\d+)\b(?=\s*[,}])",
                row + "}",
            )
        }
        if "clientId" not in nums:
            continue
        if any(
            re.search(r"\b" + field + r"\s*=", row) and field not in nums
            for field in ("buy", "sell", "count", "subType")
        ):
            continue  # A symbolic scalar is not a literal transaction fact.
        offer = {
            "item_name": name.group(1),
            "client_id": nums["clientId"],
            "server_item_id": None,
            "buy_price": nums.get("buy"),
            "sell_price": nums.get("sell"),
            "count": nums.get("count"),
            "sub_type": nums.get("subType"),
            "stock_gate": None,
        }
        position = start.end() + match.start()
        proof = dict(
            source, line=text.count("\n", 0, position) + 1, raw_shop_row=match.group(0)
        )
        fact = source_fluid_fact(offer, proof)
        if fact is not None:
            facts.append(fact)
    return facts


def capture_quotes(captures):
    """Load caller-selected custody records; no network or filesystem guessing."""
    facts, seen = [], set()
    for row in captures:
        if "path" not in row or "error" in row:
            continue
        identity = (row["url"], row["sha256"])
        if identity in seen:
            continue  # Duplicate custody records are not independent observations.
        seen.add(identity)
        url = row["url"].split("raw.githubusercontent.com/", 1)[-1].split("/")
        if len(url) < 4:
            raise ValueError("source capture URL has no pinned repository path")
        source = {
            "repository": "/".join(url[:2]),
            "revision": url[2],
            "path": "/".join(url[3:]),
            "sha256": row["sha256"],
        }
        for fact in source_quotes(Path(row["path"]).read_bytes(), source):
            fact["npc_slug"] = Path(source["path"]).stem
            facts.append(fact)
    return facts


def native_fluid_reason(offer):
    """Retain fluid variants until a native binding exists, including water=1."""
    key = offer.get("item", {}).get("key", "")
    match = re.fullmatch(r"oteryn:item\.tibia\.i(\d+)", key)
    if match is None or int(match.group(1)) not in FLUID_CONTAINERS:
        return None
    if offer.get("sub_type") is not None:
        return "SOURCE_FLUID_SUBTYPE_NATIVE_BINDING_UNQUALIFIED"
    if offer.get("count") is not None:
        return (
            "FLUID_SOURCE_ENUM_IS_NOT_NATIVE_TRANSACTION_QUANTITY"
            if offer["count"] != 1
            else "FLUID_WATER_VARIANT_NATIVE_BINDING_UNQUALIFIED"
        )
    return None


def matches_quote(offer, fact, npc_slug):
    source = fact["old_source_offer"]
    field = "buy_price" if offer["direction"] == "SellToPlayer" else "sell_price"
    return (
        fact.get("npc_slug") == npc_slug
        and offer["item"]["key"] == f"oteryn:item.tibia.i{source['client_id']}"
        and offer["unit_price"] == source[field]
        and offer.get("count") == source["count"]
        and offer.get("sub_type") == source["sub_type"]
    )


def repair_trade_services(services, quotes=(), retained_holds=()):
    """Return native source-only replacements and holds; never qualify a guess.

    Use after overlaying R4's existing Service corrections. `retained_holds`
    are the already committed R4 records, so a direct active-tree invocation
    can also remove their unsafe tuples without losing their source proofs.
    """
    replacements, inventory, counts = [], [], Counter()
    for original in services:
        service = copy.deepcopy(original)
        service_key = service["identity"]["key"]
        if not service_key.startswith("oteryn:service.trade."):
            continue
        npc_slug = service_key.rsplit(".", 1)[1]
        access = access_fact(npc_slug)
        keep, new_holds = [], []
        for offer in service.get("offers", []):
            existing = next(
                (
                    h
                    for h in retained_holds
                    if h["service"] == service_key and h["old_native_offer"] == offer
                ),
                None,
            )
            reason = (existing or {}).get("reason") or native_fluid_reason(offer)
            if access:
                reason = "SOURCE_TRADE_ACCESS_NATIVE_PREDICATE_UNAVAILABLE"
            if reason is None:
                keep.append(offer)
                continue
            hold = copy.deepcopy(existing) if existing else {}
            hold.update(
                service=service_key,
                npc="oteryn:npc." + npc_slug,
                old_native_offer=offer,
                reason=reason,
                admission_eligible=False,
                runtime_qualified=False,
                disposition="SOURCE_FACT_RETAINED_NOT_EXECUTABLE_OFFER",
            )
            if access:
                hold["access_fact"] = access
            observations = [q for q in quotes if matches_quote(offer, q, npc_slug)]
            if observations:
                hold["source_observations"] = observations
            elif native_fluid_reason(offer):
                hold["source_quote_state"] = (
                    "UNKNOWN_CAPTURE_OR_EXACT_TUPLE_UNAVAILABLE"
                )
            new_holds.append(hold)
            inventory.append(hold)
            counts[reason] += 1
        if not new_holds:
            continue
        service["offers"] = keep
        fields = service.setdefault("fields", [])
        for hold in new_holds:
            encoded = json.dumps(
                hold, sort_keys=True, separators=(",", ":"), ensure_ascii=False
            )
            # Content-addressed field names make a repeated repair idempotent.
            field = (
                "oteryn:source.npc.trade.held_tuple_"
                + hashlib.sha256(encoded.encode()).hexdigest()[:16]
            )
            fields.append(
                {"field_path": field, "value": {"type": "Text", "value": encoded}}
            )
        fields.sort(key=lambda f: f["field_path"])
        replacements.append(service)
    return {
        "services": replacements,
        "held_offers": inventory,
        "counts": dict(counts),
        "runtime_qualified": False,
        "native_fluid_mapping": None,
        "unresolved": [
            "Accepted legacy fluid -> native Service subtype binding and native fluid variants.",
            "Native merchant quest/access predicates for the preserved source callbacks.",
            "Full count/subtype/currency/access tuple proof for retained R4 charge variants.",
            "Official exact prices for parity_pending rows.",
        ],
    }
