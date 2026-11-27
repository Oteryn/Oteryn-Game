"""Report corpse admission holds and capture deliberate regression exceptions."""
from __future__ import annotations

import argparse
from collections import Counter
import json

import verified_loot_corpses as source

LOOT = "tools/content-schema/native-gameplay/loot-tables.json"
ITEMS = "content/items/definitions/spell-native-profiles.json"
EXCEPTIONS = "tools/content-schema/native-gameplay/verified-loot-corpse-exceptions.json"
PLAIN = {
    ("glitterscale", 11589, True),
    ("symbol_of_hatred", 33792, False),
    ("hateful_soul", 33793, False),
    ("weeping_soul", 33876, False),
    ("poor_soul", 33891, False),
    ("aspect_of_power", 33949, False),
    ("necromantic_focus", 33984, False),
    ("purple_butterfly", 4992, False),
}


def known(field):
    return field.get("value") if field.get("state") == "KNOWN" else None


def native_capacity(row, ref):
    if (row is None or ref["key"] != source.key(5984)
            or row["production_binding"]["disposition"] != "EXACT"):
        return None
    semantics = row["semantics"]
    admission = row["admission"]
    container = known(semantics["container"]) or {}
    physical = known(semantics["physical"]) or {}
    classification = known(semantics["classification"]) or {}
    stack = known(semantics["stack"]) or {}
    if (row["production_binding"]["target"] == ref
            and admission == {"materializable": True, "stack_class": "NonStackable",
                              "legal_destinations": ["Ground"]}
            and known(classification.get("item_type", {})) == "CONTAINER"
            and known(physical.get("pickupable", {})) is False
            and known(stack.get("stackable", {})) is False
            and known(container.get("capacity", {})) == 56):
        return 56
    return None


def audit(root=source.ROOT):
    creatures = source.read(root, source.quest.CREATURES)["records"]
    loot = source.read(root, LOOT)
    facts = {row["item"]["key"]: row for row in loot["items"]}
    native = {row["production_definition"]["production_key"]: row
              for row in source.read(root, ITEMS)["records"]}
    tables = {row["identity"]["key"]: row for row in loot["tables"]}
    bindings = {row["creature"]["key"]: row["loot"] for row in loot["creature_loot"]}
    primary, secondary = source.source_rows(root)
    proof_holds = {}
    source.plan(root, source.canonical_definitions(root), {"records": creatures}, holds=proof_holds)
    exceptions = {"no_body": {}, "held_body": {}}
    held = {}
    reasons = Counter()
    for record in creatures:
        target = record["profile"]["target"]["key"]
        details = record["profile"]["data"]["profile"].get("details") or {}
        ref = details.get("corpse_item")
        if ref is None:
            exceptions["no_body"][target] = "Served Creature profile has no authored corpse Item."
            continue
        key = ref["key"]
        match = source.ITEM.fullmatch(key)
        number = int(match[1]) if match else None
        row = native.get(key)
        private = details.get("system_eligibility", {}).get("reward_boss", False)
        table = bindings.get(target)
        if ((target.removeprefix("oteryn:creature."), number, private) in PLAIN
                and row and row["admission"]["materializable"]
                and (private or not (table and tables[table["key"]]["entries"]))):
            continue
        fact = facts.get(key)
        capacity = fact and fact["container_capacity"]
        rejected = []
        if not fact or fact["item"] != ref:
            rejected.append("missing_exact_item_facts")
        if not fact or not fact["materializable"]:
            rejected.append("not_materializable")
        if private and fact and fact.get("ordinary_corpse_only"):
            rejected.append("ordinary_only")
        if not capacity:
            rejected.append("no_capacity")
        elif capacity > 48 and not private and native_capacity(row, ref) != capacity:
            rejected.append("large_capacity_without_native_proof")
        if not rejected:
            continue
        if number is None:
            evidence = "Source-only Item identity has no canonical Tibia Item and WorldObject binding."
        else:
            attributes = source.selected_source(number, primary, secondary)
            if "containersize" not in attributes:
                evidence = "Pinned donor row supplies no loot-container size."
            elif "ordinary_only" in rejected:
                evidence = "Existing quest corpse proof authorizes ordinary loot only."
            elif number in proof_holds:
                evidence = proof_holds[number].capitalize() + "."
            else:
                evidence = "The registered Item owner takes precedence and has no admissible Ground corpse policy."
        exceptions["held_body"][target] = f"{key}: {evidence}"
        held[target] = {"item": key, "predicates": rejected, "evidence": evidence}
        reasons.update(rejected)
    return {
        "profiles": len(creatures),
        "declared_corpses": len(creatures) - len(exceptions["no_body"]),
        "held_creatures": len(held),
        "held_item_ids": len({row["item"] for row in held.values()}),
        "predicate_counts": dict(reasons),
        "held": held,
        "exceptions": exceptions,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--record-reviewed-exceptions", action="store_true")
    args = parser.parse_args()
    result = audit()
    if args.record_reviewed_exceptions:
        (source.ROOT / EXCEPTIONS).write_text(
            json.dumps(result["exceptions"], indent=2, sort_keys=True) + "\n")
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
