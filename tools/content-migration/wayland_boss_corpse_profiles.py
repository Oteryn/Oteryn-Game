"""Replay the exact Crystal 201189f8 corpse chains for Wayland's scrap bosses."""

import argparse
import json
from pathlib import Path

import boss_corpse_profiles as crystal
import qengine_reward_item_profiles as items

PATH = "tools/content-schema/native-gameplay/wayland-boss-corpse-profiles.json"
CHAINS = ((54346, 54345, 54344, 54343), (54433, 54434, 54435))
IDS = sorted(item for chain in CHAINS for item in chain)
BINDINGS = "imports/crystalserver/bindings/items.json"


def capture(root, cache):
    blobs = crystal.load_source_blobs(cache)
    xml_raw = blobs["data/items/items.xml"]
    dat = blobs["data/items/appearances.dat"]
    source = crystal.source
    xml = source.xml_index(xml_raw)
    appearances = source.engine_items.load_appearance_objects(dat)
    physics = {
        row["server_item_id"]: row for row in source.policies(dat, xml_raw, set(IDS))
    }
    bindings = crystal.binding_index((root / BINDINGS).read_bytes())
    records = []
    for item in IDS:
        appearance = appearances[item]
        if not appearance["flags"].get("flags.corpse"):
            raise ValueError(f"Wayland boss corpse appearance drift: {item}")
        row = source.profile(item, xml[item], appearance, physics[item], bindings, None)
        crystal.complete_source_top_order(row, appearance)
        row["authoring"]["item"]["revision"] = "wayland-boss-corpses-crystal-201189f"
        records.append(row)
    records = source.bind_profiles(
        {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": records},
        bindings,
        set(IDS),
        independent_crystal_source=(crystal.PIN, xml_raw),
    )["records"]
    return {
        "source_revision": crystal.PIN,
        "source_hashes": crystal.SOURCE_HASHES,
        "xml_lines": crystal.xml_source_lines(xml_raw, set(IDS)),
        "records": records,
    }


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root / PATH).read_bytes())
    records = packet["records"]
    if (
        packet["source_revision"] != crystal.PIN
        or packet["source_hashes"] != crystal.SOURCE_HASHES
        or [int(row["production_binding"]["external_id"]) for row in records] != IDS
    ):
        raise ValueError("Wayland boss corpse capture drift")
    records_by_id = dict(zip(IDS, records, strict=True))
    bindings = crystal.binding_index((root / BINDINGS).read_bytes())
    source = crystal.source
    for chain in CHAINS:
        for stage, item in enumerate(chain):
            row = records_by_id[item]
            binding = bindings[str(item)]
            target = binding["target"]
            successor = (
                bindings[str(chain[stage + 1])]["target"]
                if stage + 1 < len(chain)
                else None
            )
            duration = 10_000 if stage == 0 else (300_000 if successor else 60_000)
            if (
                row["production_binding"] != binding
                or binding["disposition"] != "EXACT"
                or row["production_definition"]
                != {
                    "family": target["family"],
                    "production_key": target["key"],
                    "revision_ref": target["revision"],
                }
                or row["admission"] != crystal.corpse_admission(item)
                or row["semantics"]["container"]
                != (source.K({"capacity": source.K(30)}) if successor else source.NA)
                or row["semantics"]["temporal"]
                != source.K(
                    {
                        "consumption_mode": source.K("DURABLE_ABSOLUTE_DEADLINE"),
                        "duration": source.K(duration),
                        "stop_duration": source.K(False),
                        "decay_target": source.K(
                            {"key": successor["key"], "revision": successor["revision"]}
                        )
                        if successor
                        else source.NA,
                    }
                )
            ):
                raise ValueError(f"Wayland boss corpse custody or decay drift: {item}")
    items.admit_records(
        root, generated, profile_path, manifest_path, order, canonical, records
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-cache", type=Path, required=True)
    args = parser.parse_args()
    packet = capture(crystal.ROOT, args.source_cache)
    print(crystal.encode(packet).decode(), end="")
