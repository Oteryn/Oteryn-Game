"""Generate the pinned SRX-1 Item updates and encode the qualified base."""

import copy
import hashlib
import json

PATH = "content/items/definitions/srx1-item-updates.json"
MAX_ITEM_PROFILES = 16 * 1024 * 1024  # native_gameplay::MAX_ITEM_PROFILES


def compact_item_record(row):
    """Omit only facts that the native decoder defaults to the same value."""
    attributes = row["attributes"]
    for field in ("field_condition", "field_replaceable", "rune_consumption"):
        if attributes.get(field) is None:
            attributes.pop(field, None)
    if row.get("production_binding_qualification") is None:
        row.pop("production_binding_qualification", None)
    # First-party constructor receipts bind the explicit raw semantics.
    if row["production_binding"]["source_key"] != "oteryn:source.oteryn_authored":
        row["semantics"] = {
            field: fact
            for field, fact in row["semantics"].items()
            if fact != {"state": "UNKNOWN"}
        }
    return row


def compact_item_provider(document, canonical, exact_keys):
    for row in document["records"]:
        if row["production_definition"]["production_key"] not in exact_keys:
            compact_item_record(row)
    return canonical(document)


OMIT_IF_UNKNOWN = ('field_condition', 'rune_consumption', 'field_replaceable')


def omit_unknown_attributes(row):
    attributes = row["attributes"]
    for name in OMIT_IF_UNKNOWN:
        if attributes.get(name) is None:
            attributes.pop(name, None)


def exact_source_keys(packet):
    from floor_assembly_profiles import IDS as floor_assembly_ids
    from lion_trial_item_profiles import SOURCE_RECORDS as lion_trial_ids

    return {
        row["production_definition"]["production_key"]
        for group in ("records", "custody_promotions")
        for row in packet[group]
    } | {
        f"oteryn:item.tibia.i{item_id}"
        for item_id in floor_assembly_ids | lion_trial_ids.keys()
    }


def coalesce_dispenser_top_order(document, source_records, house_overlay):
    """Coalesce only exact dispenser owner facts plus verified source top flags.

    Crystal items.cpp:203-208 enriches the generic House admission first. The
    strict dispenser compiler must still compare every other source fact.
    """
    import house_decoration_actions

    expected = copy.deepcopy(source_records)
    house_decoration_actions.apply_top_order(expected, house_overlay)
    enriched = {row["production_definition"]["production_key"]: row for row in expected}
    original = {
        row["production_definition"]["production_key"]: row for row in source_records
    }
    for index, row in enumerate(document["records"]):
        key = row["production_definition"]["production_key"]
        if key not in enriched:
            continue
        candidate = compact_item_record(copy.deepcopy(row))
        for expected in (original[key], enriched[key]):
            if candidate == compact_item_record(copy.deepcopy(expected)):
                document["records"][index] = copy.deepcopy(original[key])
                break


def generate(root, generated, profile_path, manifest_path, order, canonical):
    import dispenser_item_profiles
    import equipment_eligibility
    import house_decoration_actions
    import qengine_reward_item_profiles
    import srx1_potion_variants
    import writing_item_admission

    base = generated[profile_path]
    overlay = {profile_path: base, manifest_path: generated[manifest_path]}
    house_overlay = json.loads((root / house_decoration_actions.PROFILES).read_bytes())
    document = json.loads(overlay[profile_path])
    dispenser_source = json.loads((root / dispenser_item_profiles.PATH).read_bytes())
    coalesce_dispenser_top_order(document, dispenser_source["records"], house_overlay)
    overlay[profile_path] = canonical(document)
    for module in (
        dispenser_item_profiles,
        srx1_potion_variants,
        writing_item_admission,
    ):
        module.admit(root, overlay, profile_path, manifest_path, order, canonical)
    # W18 source top-order facts enrich the final specialized rows only. Reapplying
    # generic furniture transforms here would overwrite the writing owner's policy.
    enriched = json.loads(overlay[profile_path])
    house_decoration_actions.apply_top_order(
        enriched["records"],
        json.loads((root / house_decoration_actions.PROFILES).read_bytes()),
    )
    original_document = json.loads(base)
    # Qualify both provider versions after the source owners' exact comparisons
    # so updated records cannot restore unknown eligibility.
    eligibility = equipment_eligibility.load_facts(root)
    equipment_eligibility.complete(original_document, eligibility)
    equipment_eligibility.complete(enriched, eligibility)
    exact_keys = exact_source_keys(
        json.loads(generated[qengine_reward_item_profiles.PROOF])
    )
    # Omit redundant nulls after the exact source comparisons. Rust reads either
    # representation as Unknown and keeps the composed provider within 16 MiB.
    # The QENGINE, Lion Trial and floor assembly proofs compare complete source rows.
    for profile_document in (original_document, enriched):
        for row in profile_document["records"]:
            if row["production_definition"]["production_key"] not in exact_keys:
                omit_unknown_attributes(row)
    original = {
        row["production_definition"]["production_key"]: row
        for row in original_document["records"]
    }
    updates = [
        row
        for row in enriched["records"]
        if original.get(row["production_definition"]["production_key"]) != row
    ]
    if not updates or len(updates) > 128:
        raise ValueError("SRX1_ITEM_UPDATE_BOUNDS")
    # Ground-only decoration records can omit unknown field conditions.
    # Keep the separate source records used by exact policy witnesses intact.
    decoration_keys = {
        row["production_definition"]["production_key"]
        for row in house_overlay["records"]
    }
    for row in original_document["records"]:
        if (
            row["production_definition"]["production_key"] in decoration_keys
            and row["admission"]["legal_destinations"] == ["Ground"]
            and row["attributes"].get("field_condition") is None
        ):
            row["attributes"].pop("field_condition", None)
    base = canonical(original_document)
    if len(base) > MAX_ITEM_PROFILES:
        # Preserve the exact profiles while omitting absent optional fields on
        # the same Ground decoration cohort.
        for row in original_document["records"]:
            if (
                row["production_definition"]["production_key"] in decoration_keys
                and row["admission"]["legal_destinations"] == ["Ground"]
            ):
                for field in ("field_replaceable", "rune_consumption"):
                    if row["attributes"].get(field) is None:
                        row["attributes"].pop(field, None)
                if row.get("production_binding_qualification") is None:
                    row.pop("production_binding_qualification", None)
        base = canonical(original_document)
    if len(base) > MAX_ITEM_PROFILES:
        base = compact_item_provider(original_document, canonical, exact_keys)
    if len(base) > MAX_ITEM_PROFILES:
        raise ValueError("SRX1_ITEM_PROVIDER_SIZE")
    generated[profile_path] = base
    document = {
        "schema": "OTERYN_NATIVE_ITEM_UPDATES/v1",
        "base_sha256": hashlib.sha256(base).hexdigest(),
        "records": sorted(updates, key=order),
    }

    def encode_updates():
        return (
            json.dumps(document, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
            + "\n"
        ).encode()

    data = encode_updates()
    if len(data) > 200_000:
        # The provider and its supplements use the same native Item decoder.
        # Omit decode-equivalent defaults and keep only rows that still change
        # the base provider after that normalization.
        records = []
        for row in document["records"]:
            key = row["production_definition"]["production_key"]
            predecessor = copy.deepcopy(original.get(key))
            if key not in exact_keys:
                compact_item_record(row)
                if predecessor is not None:
                    compact_item_record(predecessor)
            if predecessor != row:
                records.append(row)
        document["records"] = records
        data = encode_updates()
    if len(data) > 200_000:
        raise ValueError("SRX1_ITEM_UPDATE_SIZE")
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(base).hexdigest()
    manifest["item_profile_updates"] = {
        "path": "items/definitions/srx1-item-updates.json",
        "sha256": hashlib.sha256(data).hexdigest(),
    }
    generated[PATH] = data
    generated[manifest_path] = canonical(manifest)
