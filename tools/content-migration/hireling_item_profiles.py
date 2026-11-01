"""Admit E12's complete native Item closure without replacing richer policies."""
import copy
import hashlib
import json

PATH = "tools/content-schema/native-gameplay/hireling-item-profiles.json"
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
CATALOGUE = "crates/gameplay/domain/src/domain/hireling/catalogue/generated.rs"
LAMP = "oteryn:item.tibia.i29432"
VALUE_GROUPS = ("physical", "stack", "container", "charges")
RICH_GROUPS = ("equipment", "weapon", "protection", "skill_modifiers", "imbuement",
               "use_transform", "trade_restrictions", "fluid", "readable_writeable")

# ANSWER Q14A (PR2783 comment6548596838): these shared native owners
# keep their already established weights. Raw Crystal capture remains unchanged.
# Every pair is (active native centi-ounces, Crystal centi-ounces, XML item line).
SHARED_NATIVE_WEIGHTS = {
    "oteryn:item.tibia.i283": (100, 160, 503),
    "oteryn:item.tibia.i284": (135, 180, 507),
    "oteryn:item.tibia.i285": (75, 150, 511),
    "oteryn:item.tibia.i3031": (10, 1, 6818),
    "oteryn:item.tibia.i23721": (150, 0, 47233),
}

# Package merge: main 149bf364f owns the rounded weights of these Task Board
# items. Preserve its exact known values; the raw Hireling capture stays unchanged.
MERGED_MAIN_NATIVE_WEIGHTS = {
    "oteryn:item.tibia.i10317": (113, 112, 26957),
    "oteryn:item.tibia.i10454": (115, 114, 27704),
    "oteryn:item.tibia.i18924": (510, 509, 38605),
    "oteryn:item.tibia.i27598": (230, 229, 52453),
}

# ANSWER Q15A (PR2783 comment6549542080): this one virtual Hireling
# operand keeps the existing shared GoldPouch owner and Inventory destination.
SHARED_NATIVE_MOVABLE = {
    "oteryn:item.tibia.i23721": (True, False, 47233),
}


def qualified_source_values(existing, source):
    """Closed approved leaf pairs; neither identity nor admission changes."""
    key = source["production_definition"]["production_key"]
    qualified = source
    for leaf, decisions, label in (("weight", SHARED_NATIVE_WEIGHTS | MERGED_MAIN_NATIVE_WEIGHTS, "WEIGHT"),
                                   ("movable", SHARED_NATIVE_MOVABLE, "MOVABLE")):
        decision = decisions.get(key)
        if decision is None:
            continue
        old, donor, _ = decision
        expected_old = {"state": "KNOWN", "value": old}
        expected_donor = {"state": "KNOWN", "value": donor}
        try:
            old_fact = existing["semantics"]["physical"]["value"][leaf]
            source_fact = source["semantics"]["physical"]["value"][leaf]
        except (KeyError, TypeError) as error:
            raise ValueError(f"HIRELING_SHARED_{label}_PROOF:{key}") from error
        if (not isinstance(old_fact, dict) or not isinstance(source_fact, dict)
                or old_fact != expected_old or source_fact != expected_donor
                or type(old_fact.get("value")) is not type(old)
                or type(source_fact.get("value")) is not type(donor)):
            raise ValueError(f"HIRELING_SHARED_{label}_DRIFT:{key}")
        if qualified is source:
            qualified = copy.deepcopy(source)
        qualified["semantics"]["physical"]["value"][leaf] = copy.deepcopy(old_fact)
    return qualified


def merge_fact(existing, source, path):
    """Fill unknown leaves; a disagreement between known facts is reported."""
    if existing == source or source.get("state") == "UNKNOWN":
        return copy.deepcopy(existing)
    if existing.get("state") == "UNKNOWN":
        return copy.deepcopy(source)
    if existing.get("state") == source.get("state") == "KNOWN":
        old, new = existing["value"], source["value"]
        if isinstance(old, dict) and isinstance(new, dict) and old.keys() == new.keys():
            return {"state": "KNOWN", "value": {
                key: merge_fact(old[key], new[key], f"{path}.{key}") for key in old}}
    raise ValueError(f"HIRELING_NATIVE_SOURCE_CONFLICT:{path}:{existing!r}!={source!r}")


def merge_record(existing, source):
    key = source["production_definition"]["production_key"]
    for field in ("production_definition", "production_binding"):
        if existing[field] != source[field]:
            raise ValueError(f"HIRELING_NATIVE_IDENTITY_CONFLICT:{key}:{field}")
    merged = copy.deepcopy(existing)
    qualified = qualified_source_values(existing, source)
    for group in VALUE_GROUPS:
        if group not in existing["semantics"]:
            merged["semantics"][group] = copy.deepcopy(qualified["semantics"][group])
            continue
        fact = merge_fact(existing["semantics"][group], qualified["semantics"][group], f"{key}:{group}")
        # Validate known conflicts in every group, but only enrich physical.
        # Existing owners retain even UNKNOWN stack, container and charge facts.
        if group == "physical":
            merged["semantics"][group] = fact
    old, new = existing["admission"], source["admission"]
    if old["stack_class"] != new["stack_class"]:
        raise ValueError(f"HIRELING_NATIVE_STACK_CLASS_CONFLICT:{key}")
    if key == LAMP:
        if old != new:
            raise ValueError("HIRELING_NATIVE_LAMP_DESTINATION_CONFLICT")
    elif not old["materializable"] or "CharacterInventory" not in old["legal_destinations"]:
        # Copy only proven source destinations. Source Ground-only furniture
        # and malformed sell rows never acquire Inventory by catalogue presence.
        # Preserve an existing inventory-only specialized owner (gold pouch).
        merged["admission"] = {"materializable": new["materializable"], "stack_class": new["stack_class"],
                               "legal_destinations": sorted(set(old["legal_destinations"]) | set(new["legal_destinations"]))}
    return merged


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay["records"]
    keys = [row["production_definition"]["production_key"] for row in records]
    expected = {f"oteryn:item.tibia.i{number}" for number in overlay["required_ids"]}
    catalogue = overlay["catalogue"]
    if (overlay["schema"] != "OTERYN_NATIVE_HIRELING_ITEM_PROFILES/v1"
            or overlay["source_revision"] != REVISION or len(keys) != len(expected)
            or set(keys) != expected or catalogue["path"] != CATALOGUE
            or catalogue["trade_rows"] != 1675 or catalogue["trader_items"] != 1409
            or catalogue["packages"] != 23 or catalogue["package_ingredients"] != 69
            or hashlib.sha256((root / CATALOGUE).read_bytes()).hexdigest() != catalogue["sha256"]
            or hashlib.sha256((root / "imports/crystalserver/bindings/items.json").read_bytes()).hexdigest()
                != overlay["binding_table_sha256"]):
        raise ValueError("HIRELING_NATIVE_ITEM_PROFILE_SET_OR_PIN")
    for key, (_, _, line) in (SHARED_NATIVE_WEIGHTS | MERGED_MAIN_NATIVE_WEIGHTS).items():
        number = key.removeprefix("oteryn:item.tibia.i")
        if overlay["source_lines"].get(number) != f"data/items/items.xml:{line}":
            raise ValueError(f"HIRELING_SHARED_WEIGHT_SOURCE_LINE:{key}")
    for key, (_, _, line) in SHARED_NATIVE_MOVABLE.items():
        number = key.removeprefix("oteryn:item.tibia.i")
        if overlay["source_lines"].get(number) != f"data/items/items.xml:{line}":
            raise ValueError(f"HIRELING_SHARED_MOVABLE_SOURCE_LINE:{key}")
    world = {row["identity"]["key"]: row for row in
             json.loads((root / "content/world/definitions/reference.json").read_bytes())["records"]}
    document = json.loads(generated[profile_path])
    existing = {row["production_definition"]["production_key"]: row for row in document["records"]}
    if len(existing) != len(document["records"]):
        raise ValueError("HIRELING_NATIVE_EXISTING_DUPLICATE")
    conflicts = []
    for source in records:
        key = source["production_definition"]["production_key"]
        if key in existing:
            try:
                existing[key] = merge_record(existing[key], source)
            except ValueError as error:
                conflicts.append(str(error))
            continue
        canonical_item = world.get(key)
        if canonical_item is None or canonical_item["identity"] != source["production_binding"]["target"]:
            raise ValueError(f"HIRELING_NATIVE_CANONICAL_IDENTITY:{key}")
        record = copy.deepcopy(source)
        # Existing qualified Global equipment/weapon facts remain the owner of
        # their groups. Native source import supplies physical/value facts only.
        # Many immutable World identities have no authored semantics yet;
        # readable/writable native importers use the same optional shape.
        # They contribute no richer fact and do not suppress pinned source facts.
        canonical_semantics = canonical_item.get("semantics", {})
        if not isinstance(canonical_semantics, dict):
            raise TypeError(f"HIRELING_NATIVE_CANONICAL_SEMANTICS:{key}")
        for group in RICH_GROUPS:
            value = canonical_semantics.get(group, {"state": "UNKNOWN"})
            if value["state"] == "KNOWN":
                record["semantics"][group] = copy.deepcopy(value)
        existing[key] = record
    if conflicts:
        raise ValueError("\n".join(conflicts))
    taxonomy = {row["target"]["key"]: row["source_taxonomy"] for row in
                json.loads((root / "content/items/taxonomy/items.json").read_bytes())["records"]}
    for key in expected:
        if key in taxonomy:
            existing[key]["authoring"]["taxonomy"] = copy.deepcopy(taxonomy[key])
    document["records"] = sorted(existing.values(), key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + "\n").encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
