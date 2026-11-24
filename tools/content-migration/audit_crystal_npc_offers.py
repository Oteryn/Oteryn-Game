"""Census literal buy/sell offers in the Crystal 201189f8 Global NPC scripts."""
import argparse
import collections
import csv
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DECLARATIONS = ROOT / "content/world/definitions/declarations.json"
FIELDS = re.compile(r"\b(clientid|buy|sell)\s*=\s*(\d+)\s*(?=[,}])", re.I)
TABLE_ROW = re.compile(r"\{[^{}]+\}")
ITEM_NAME = re.compile(r'\bitemname\s*=\s*"([^"]+)"', re.I)
CONDITIONAL_FIELDS = re.compile(r"\b(?:count|subtype|actionid|storage)\s*=", re.I)
DIRECTIONS = {"buy": "SellToPlayer", "sell": "BuyFromPlayer"}


def source_offers(root):
    offers = collections.defaultdict(list)
    scripts = set()
    for path in sorted((root / "data-global/npc").rglob("*.lua")):
        script = re.sub(r"[^a-z0-9]+", "_", path.stem.lower()).strip("_")
        scripts.add(script)
        for line_number, line in enumerate(path.read_text(encoding="utf-8-sig").splitlines(), 1):
            for row in TABLE_ROW.findall(line.split("--", 1)[0]):
                fields = {key.lower(): int(value) for key, value in FIELDS.findall(row)}
                if "clientid" not in fields:
                    continue
                for side, direction in DIRECTIONS.items():
                    if fields.get(side, 0) > 0:
                        offers[script, fields["clientid"], direction].append(
                            (
                                fields[side],
                                f"{path.relative_to(root)}:{line_number}",
                                ITEM_NAME.search(row)[1].casefold()
                                if ITEM_NAME.search(row) and not CONDITIONAL_FIELDS.search(row)
                                else None,
                            )
                        )
    return offers, scripts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("crystal", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--import-exact", action="store_true",
                        help="admit uniquely named materializable offers to existing merchants")
    parser.add_argument("--normalize", action="store_true",
                        help="rewrite canonical service ordering without importing offers")
    args = parser.parse_args()
    donor, scripts = source_offers(args.crystal)
    document = json.loads(DECLARATIONS.read_bytes())
    declarations = document["records"]
    native = collections.defaultdict(list)
    item_shards = {}
    for shard in sorted((ROOT / "content/items/definitions").glob("items-*.json")):
        for row in json.loads(shard.read_bytes())["records"]:
            item_shards[row["definition"]["identity"]["key"]] = f"{shard.relative_to(ROOT)}:1"
    items = {
        row["definition"]["identity"]["key"]: row["definition"]
        for shard in (ROOT / "content/items/definitions").glob("items-*.json")
        for row in json.loads(shard.read_bytes())["records"]
    }
    npc_keys = {
        row["declaration"]["identity"]["key"].removeprefix("oteryn:npc.")
        for shard in (ROOT / "content/npcs/definitions").glob("npcs-*.json")
        for row in json.loads(shard.read_bytes())["records"]
    }
    services = {}
    gated_services = set()
    for row in declarations:
        service = row["identity"]["key"]
        if not service.startswith("oteryn:service.trade."):
            continue
        merchant = service.removeprefix("oteryn:service.trade.")
        services[merchant] = row
        if any(field["field_path"].startswith("oteryn:source.npc.trade.held_tuple_")
               for field in row.get("fields", [])):
            gated_services.add(merchant)
        for offer in row.get("offers", []):
            key = offer["item"]["key"]
            item = re.fullmatch(r"oteryn:item.tibia.i(\d+)", key)
            item_id = int(item[1]) if item else key
            native[merchant, item_id, offer["direction"]].append(offer["unit_price"])
    # The official source records Lily's blueberry purchase as dialogue-only.
    # Remove the plain shop offer admitted by the earlier literal-only import.
    lily = ("lily", 3588, "BuyFromPlayer")
    if args.import_exact and lily in native:
        services["lily"]["offers"] = [
            offer for offer in services["lily"]["offers"]
            if (offer["item"]["key"], offer["direction"])
            != ("oteryn:item.tibia.i3588", "BuyFromPlayer")
        ]
        native.pop(lily)
        removed = True
    else:
        removed = False
    counts = collections.Counter()
    with args.output.open("w", newline="") as output:
        writer = csv.writer(output, delimiter="\t", lineterminator="\n")
        writer.writerow(("class", "merchant", "item", "direction", "oteryn_gold", "crystal_gold", "source", "native_item_definition"))
        for key in sorted(native.keys() | donor.keys(), key=str):
            merchant, item, side = key
            ours, theirs = native.get(key, []), donor.get(key, [])
            if ours and theirs:
                continue
            if ours:
                reason = "oteryn_only_no_named_donor_script" if merchant not in scripts else "oteryn_only_no_literal_match"
            elif merchant == "grizzly_adams" or merchant in gated_services:
                reason = "crystal_only_dedicated_quest_gated_shop"
            elif key == lily:
                reason = "crystal_only_official_dialogue_quantity"
            elif merchant not in npc_keys:
                reason = "crystal_only_no_native_npc_definition"
            elif f"oteryn:item.tibia.i{item}" not in items:
                reason = "crystal_only_no_native_item"
            else:
                definition = items[f"oteryn:item.tibia.i{item}"]
                name = (definition.get("semantics", {}).get("presentation", {})
                        .get("value", {}).get("name", {}).get("value", ""))
                if not definition["materializable"]:
                    reason = "crystal_only_item_not_materializable"
                elif len(theirs) != 1 or not name or theirs[0][2] != name.casefold():
                    reason = "crystal_only_variant_or_name_mismatch"
                else:
                    reason = "crystal_only_exact_native_offer"
                    if args.import_exact:
                        if merchant not in services:
                            service = {
                                "kind": "Service",
                                "identity": {"key": f"oteryn:service.trade.{merchant}",
                                             "revision": "definition-r1"},
                                "fields": [],
                                "offers": [],
                            }
                            declarations.append(service)
                            services[merchant] = service
                            npc = next(record for record in declarations
                                       if record["identity"]["key"] == f"oteryn:npc.{merchant}")
                            npc["services"].append({"family": "Service", **service["identity"]})
                        services[merchant].setdefault("offers", []).append({
                            "item": definition["identity"],
                            "direction": side,
                            "unit_price": theirs[0][0],
                        })
            counts[reason] += 1
            writer.writerow((reason, merchant, item, side, ",".join(map(str, ours)),
                             ",".join(str(price) for price, _, _ in theirs),
                             ",".join(source for _, source, _ in theirs),
                             item_shards.get(f"oteryn:item.tibia.i{item}", "-")))
    if args.normalize or (args.import_exact and (removed or counts["crystal_only_exact_native_offer"])):
        from questpc7_world_seal import refresh
        from world_project_v2_to_tree import git_blob_sha, write, write_registry, write_services

        kind_order = {kind: index for index, kind in enumerate(
            dict.fromkeys(record["kind"] for record in declarations)
        )}
        declarations.sort(key=lambda record: (
            kind_order[record["kind"]], record["identity"]["key"], record["identity"]["revision"]
        ))
        for service in services.values():
            if "offers" in service and "fields" in service:
                service["fields"] = service.pop("fields")
            service.get("offers", []).sort(key=lambda offer: (
                offer["item"]["key"], offer["item"]["revision"], offer["direction"]
            ))
        DECLARATIONS.write_text(json.dumps(document, ensure_ascii=False, separators=(",", ":")) + "\n")
        refresh(ROOT, ROOT / "content/world")
        native_npcs = {r["identity"]["key"]: r for r in declarations if r["kind"] == "NPC"}
        for shard in sorted((ROOT / "content/npcs/definitions").glob("npcs-*.json")):
            payload = json.loads(shard.read_bytes())
            changed = False
            for row in payload["records"]:
                npc = row["declaration"]
                source = native_npcs.get(npc["identity"]["key"])
                if not source:
                    continue
                existing = npc.get("services", [])
                missing_trade = [
                    ref for ref in source.get("services", [])
                    if ref["key"].startswith("oteryn:service.trade.") and ref not in existing
                ]
                if missing_trade:
                    npc["services"] = sorted(existing + missing_trade, key=lambda ref: ref["key"])
                    changed = True
            if changed:
                write(shard.relative_to(ROOT).as_posix(), payload)
        index_path = "content/services/trade/index.json"
        prior = json.loads((ROOT / index_path).read_bytes())["shards"]
        declaration_digest = git_blob_sha(DECLARATIONS)
        service_counts, service_shards = write_services(document, declaration_digest)
        current = service_shards["Service.Trade"]
        obsolete = set(prior) - set(current)
        manifest_path = "content/manifest.json"
        manifest = json.loads((ROOT / manifest_path).read_bytes())
        manifest["families"]["Service.Trade"]["records"] = service_counts["Service.Trade"]
        managed = {entry["path"] for entry in manifest["managed_files"]} - obsolete
        manifest["managed_files"] = [{"path": path} for path in sorted(managed | set(current))]
        write_registry(manifest_path, manifest)
        lock_path = "content/content.lock.json"
        lock = json.loads((ROOT / lock_path).read_bytes())
        lock["family_counts"]["Service.Trade"] = service_counts["Service.Trade"]
        lock["legacy_blobs"]["declarations"] = declaration_digest
        write_registry(lock_path, lock)
        for path in obsolete:
            (ROOT / path).unlink()
    print(json.dumps(counts, sort_keys=True))


if __name__ == "__main__":
    main()
