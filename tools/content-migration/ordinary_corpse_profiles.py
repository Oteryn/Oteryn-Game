"""Lower the scoped Crystal corpse capture through the existing native Item owner."""

import copy
import hashlib
import json
import re
from itertools import pairwise

import qengine_reward_item_profiles as items
from boss_conditional_item_reference import definition
from boss_corpse_profiles import complete_source_top_order

CAPTURE = "imports/crystalserver/ordinary-corpses"
PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
# Native goal/Bank Robbery owns the two late Elf stages. Quest corpse policy
# owns the three butterfly roots and their terminal remains.
PUBLISHED_GROUND_PICKUP = {
    3994: False,
    3995: False,
    3996: False,
    4162: False,
    4163: False,
    4378: True,
    4991: True,
    4992: True,
    4993: True,
}
GROUND_CORPSE_ADMISSION = {
    "materializable": True,
    "stack_class": "NonStackable",
    "legal_destinations": ["Ground"],
}


def read(root, name):
    return json.loads((root / CAPTURE / f"{name}.json").read_bytes())


def item_ref(item):
    return {
        "family": "Item",
        "key": f"oteryn:item.tibia.i{item}",
        "revision": "definition-r1",
    }


def extend_definitions(root, definitions):
    present = {row["identity"]["key"]: row for row in definitions}
    for row in read(root, "creature-bindings"):
        key = item_ref(row["item"])["key"]
        expected = definition(key)
        prior = present.get(key)
        if prior is None:
            definitions.append(expected)
            present[key] = expected
        elif prior != expected:
            raise ValueError(
                f"Crystal corpse reference differs from the captured identity: {key}"
            )


def bind_creatures(root, creatures):
    by_name = {
        row["profile"]["target"]["key"]: row["profile"]["data"]["profile"]["details"]
        for row in creatures["records"]
    }
    for row in read(root, "creature-bindings"):
        details = by_name["oteryn:creature." + row["creature"]]
        body = item_ref(row["item"])
        if details.get("corpse_item") not in (None, body):
            raise ValueError(f"Crystal corpse conflicts with {row['creature']}")
        details["corpse_item"] = body


def preserve_projected_capacity(record, prior, capacity_projections):
    from quest_corpse_profiles import CORPSE_CAPACITY_MAX

    key = record["production_definition"]["production_key"]
    if key not in capacity_projections:
        return False

    known = items.source.K
    source = known({"capacity": known(capacity_projections[key])})
    published = known({"capacity": known(CORPSE_CAPACITY_MAX)})
    if (
        record["semantics"]["container"] != source
        or prior is None
        or prior["semantics"]["container"] != published
    ):
        raise ValueError(
            f"Crystal corpse capacity projection differs from its owner: {key}"
        )
    record["semantics"]["container"] = published
    return True


def preserve_published_ground_custody(item, prior, record):
    if item not in PUBLISHED_GROUND_PICKUP:
        return

    portable = {
        **GROUND_CORPSE_ADMISSION,
        "legal_destinations": ["CharacterInventory", "Ground"],
    }
    known = items.source.K
    physical = record["semantics"]["physical"]
    if (
        prior is None
        or prior["admission"] != GROUND_CORPSE_ADMISSION
        or record["admission"] != portable
    ):
        raise ValueError(
            f"Crystal corpse Ground custody differs from its owner: {item}"
        )
    if (
        physical["state"] != "KNOWN"
        or physical["value"]["movable"] != known(True)
        or physical["value"]["pickupable"] != known(True)
        or prior["semantics"]["physical"]
        != known(
            {**physical["value"], "pickupable": known(PUBLISHED_GROUND_PICKUP[item])}
        )
    ):
        raise ValueError(
            f"Crystal corpse physical custody differs from its owner: {item}"
        )
    record["admission"] = copy.deepcopy(GROUND_CORPSE_ADMISSION)
    record["semantics"]["physical"] = prior["semantics"]["physical"]


def admit(root, generated, path, manifest_path, order, canonical, capacity_projections):
    from quest_corpse_profiles import RAT_STAGES

    source = items.source
    raw = (root / CAPTURE / "items.xml").read_bytes()
    appearances = (root / items.CLIENT).read_bytes()
    if hashlib.sha256(appearances).hexdigest() != items.CLIENT_SHA:
        raise ValueError(
            "Crystal corpse appearance bank differs from the checked client bank"
        )
    xml = source.xml_index(raw)
    objects = source.engine_items.load_appearance_objects(appearances)
    ids = read(root, "item-ids")
    if ids != sorted(set(ids)) or not ids:
        raise ValueError(
            "Crystal corpse Item cohort must be nonempty, unique and ordered"
        )
    definitions = {
        row["definition"]["identity"]["key"]: row["definition"]
        for shard in (root / "content/items/definitions").glob("items-*.json")
        for row in json.loads(shard.read_bytes())["records"]
    }
    bindings = source.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    for binding in read(root, "bindings"):
        external = binding["external_id"]
        if (
            external in bindings
            or binding["source_key"] != "oteryn:source.crystalserver"
            or binding["source_revision"] != PIN
            or binding["target"] != item_ref(int(external))
            or binding["disposition"] != "EXACT"
        ):
            raise ValueError(f"Invalid exact Crystal corpse binding {external}")
        bindings[external] = binding
    # A physical-only admission can reference an existing terminal Item even
    # when the latter has no active factory binding. No custody is added to it.
    targets = dict(bindings)
    for key, definition_row in definitions.items():
        prefix = "oteryn:item.tibia.i"
        external = key.removeprefix(prefix)
        if key.startswith(prefix) and external.isdecimal():
            targets.setdefault(external, {"target": definition_row["identity"]})
    physical = {
        row["server_item_id"]: row
        for row in source.policies(appearances, raw, set(ids))
    }
    mandatory_yields = {
        item for loot in read(root, "scope")["mandatory_loot"].values() for item in loot
    }
    current = {
        record["production_binding"]["external_id"]: record
        for record in json.loads(generated[path])["records"]
    }
    records = []
    for item in ids:
        identity = item_ref(item)
        binding = bindings[str(item)]
        if (
            binding["target"] != identity
            or definitions[identity["key"]]["identity"] != identity
        ):
            raise ValueError(f"Crystal corpse Item identity drift: {item}")
        node = xml.get(item)
        target = source.attrs(node).get("decayto", "0") if node is not None else "0"
        unresolved = target != "0" and target not in targets
        if unresolved:
            node = copy.deepcopy(node)
            for child in list(node):
                if child.get("key", "").lower() in ("duration", "decayto"):
                    node.remove(child)
        record = source.profile(
            item, node, objects[item], physical[item], targets, None
        )
        complete_source_top_order(record, objects[item])
        record["production_binding"] = binding
        record["production_binding_qualification"] = None
        record["production_definition"] = {
            "family": "Item",
            "production_key": identity["key"],
            "revision_ref": identity["revision"],
        }
        # Keep the capacity of a body already materializable in the canonical
        # Item tree. This overlay contributes its source physics and decay.
        definition_row = definitions[identity["key"]]
        canonical_container = definition_row.get("semantics", {}).get("container", {})
        canonical_capacity = canonical_container.get("value", {}).get("capacity", {})
        owns_capacity = preserve_projected_capacity(
            record, current.get(str(item)), capacity_projections
        )
        if (
            not owns_capacity
            and definition_row["materializable"]
            and canonical_container.get("state") == "KNOWN"
            and canonical_capacity.get("state") == "KNOWN"
        ):
            record["semantics"]["container"] = canonical_container
        if (
            item not in mandatory_yields
            and record["admission"]["stack_class"] != "NonStackable"
        ):
            raise ValueError(f"Crystal corpse is not a single Item: {item}")
        if unresolved:
            record["semantics"]["temporal"] = source.U
        if item in RAT_STAGES:
            # The Global Rat owner qualifies the visible stages before this
            # Crystal overlay. Keep its floor-only custody and container facts.
            prior = current[str(item)]["semantics"]
            record["semantics"]["physical"]["value"]["weight"] = source.K(RAT_STAGES[item][3])
            record["semantics"]["container"] = copy.deepcopy(prior["container"])
            if item == 3995:
                record["semantics"]["classification"]["value"]["item_type"] = copy.deepcopy(
                    prior["classification"]["value"]["item_type"]
                )
        preserve_published_ground_custody(item, current.get(str(item)), record)
        # The donor's default zero for Lost Time and its 30-unit red gem differ
        # from the Global items. Keep the canonical weight in both custody paths.
        if item in (23729, 36706):
            items.reconcile_canonical_physical(item, definition_row, record)
            if str(item) in current:
                items.reconcile_active_physical(current[str(item)], record)
        if item in mandatory_yields and str(item) in current:
            prior = copy.deepcopy(current[str(item)])
            if (
                prior["admission"] != record["admission"]
                or prior["production_binding"] != binding
            ):
                raise ValueError(
                    f"Crystal required yield custody differs from the active owner: {item}"
                )
            # Waterskin and twig arms already have carried custody. Fill the
            # missing classification/charge facts needed by the loot producer.
            for field in ("classification", "charges"):
                if prior["semantics"][field] == source.U:
                    prior["semantics"][field] = record["semantics"][field]
            record = prior
        records.append(record)
    items.admit_records(root, generated, path, manifest_path, order, canonical, records)


def movable_stage_profiles(root):
    from ordinary_corpse_cohort import item_profile

    bindings = items.source.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    for item, facts in read(root, "movable-stages"):
        binding = bindings[str(item)]
        if binding["target"] != item_ref(item) or binding["disposition"] != "EXACT":
            raise ValueError(f"Unqualified movable corpse stage: {item}")
        record = item_profile(item, {**facts, "movable": True}, bindings)
        # The corpse's MINT registry owns its stage schedule.
        record["semantics"].pop("temporal", None)
        top = facts["always_on_top_order"]
        record["attributes"].update(always_on_top=top != 0, always_on_top_order=top)
        yield record


def inherited_portable_stage_ids(root):
    from ordinary_corpse_cohort import packet

    cohort = packet(root)
    stages = {item for chain in cohort["chains"].values() for item, _ in chain[1:]}
    return [
        item
        for item in sorted(stages)
        if item not in PUBLISHED_GROUND_PICKUP
        and (facts := cohort["items"].get(str(item)))
        and facts["movable"]
        and facts["pickupable"]
    ]


def qualify_inherited_portable_stages(root, records):
    active = {
        record["production_definition"]["production_key"]: record for record in records
    }
    for item, pickupable in PUBLISHED_GROUND_PICKUP.items():
        record = active[item_ref(item)["key"]]
        if record["admission"] != GROUND_CORPSE_ADMISSION or record["semantics"][
            "physical"
        ]["value"]["pickupable"] != items.source.K(pickupable):
            raise ValueError(f"Published Ground corpse custody changed: {item}")
    for item in inherited_portable_stage_ids(root):
        record = active[item_ref(item)["key"]]
        admission = record["admission"]
        if "CharacterInventory" in admission["legal_destinations"]:
            continue
        physical = record["semantics"]["physical"]["value"]
        if (
            admission
            != {
                "materializable": True,
                "stack_class": "NonStackable",
                "legal_destinations": ["Ground"],
            }
            or physical["movable"] != items.source.K(True)
            or physical["pickupable"] != items.source.K(False)
        ):
            raise ValueError(f"Unexpected inherited corpse admission: {item}")
        physical["pickupable"] = items.source.K(True)
        admission["legal_destinations"] = ["CharacterInventory", "Ground"]


def native_decay_edges(root, active):
    from fourth_quarter_corpses import source as fourth_quarter_source
    from ordinary_corpse_cohort import packet

    first = (zip(ids, durations, strict=True) for ids, durations in read(root, "decay"))
    slice_two = json.loads(
        (
            root
            / "tools/content-schema/native-gameplay/ordinary-corpse-decay-slice2.json"
        ).read_bytes()
    )
    second = (zip(row["chain"], row["durations_ms"], strict=True) for row in slice_two)
    third = packet(root)["chains"].values()
    fourth = fourth_quarter_source(root)["decay_chains"].values()

    materialized = {
        key for key, record in active.items() if record["admission"]["materializable"]
    }
    for stages in (*first, *second, *third, *fourth):
        chain = list(stages)
        if item_ref(chain[0][0])["key"] not in materialized:
            continue
        for (item, duration), (target, _) in pairwise(chain):
            key, successor = item_ref(item)["key"], item_ref(target)
            if key not in materialized or successor["key"] not in materialized:
                # A reference-only successor cannot acquire Item custody here.
                continue
            yield (
                key,
                items.source.K(
                    {
                        "consumption_mode": items.source.K("DURABLE_ABSOLUTE_DEADLINE"),
                        "duration": items.source.K(duration),
                        "stop_duration": items.source.K(False),
                        "decay_target": items.source.K(
                            {"key": successor["key"], "revision": successor["revision"]}
                        ),
                    }
                ),
            )


def qualify_native_decay_edges(root, records):
    active = {
        record["production_definition"]["production_key"]: record for record in records
    }
    for key, expected in native_decay_edges(root, active):
        semantics = active[key]["semantics"]
        prior = semantics.get("temporal")
        if prior is None or prior == items.source.U:
            semantics["temporal"] = expected
        elif decay_target(prior) != decay_target(expected):
            raise ValueError(f"Conflicting native corpse decay successor: {key}")


def decay_target(temporal):
    return (temporal or {}).get("value", {}).get("decay_target")


def check_movable_stages(root):
    profile_path = root / "content/items/definitions/spell-native-profiles.json"
    active = {
        record["production_definition"]["production_key"]: record
        for record in json.loads(profile_path.read_bytes())["records"]
    }
    for record in movable_stage_profiles(root):
        key = record["production_definition"]["production_key"]
        actual = active.get(key)
        if (
            actual is None
            or any(
                actual[field] != record[field]
                for field in ("admission", "production_binding")
            )
            or actual["semantics"].get("physical") != record["semantics"]["physical"]
        ):
            raise ValueError(f"Missing or inconsistent movable corpse admission: {key}")

    for key, temporal in native_decay_edges(root, active):
        actual = active[key]["semantics"].get("temporal")
        if decay_target(actual) != decay_target(temporal):
            raise ValueError(f"Missing captured native corpse decay successor: {key}")

    for item in inherited_portable_stage_ids(root):
        key = item_ref(item)["key"]
        actual = active[key]
        if "CharacterInventory" not in actual["admission"][
            "legal_destinations"
        ] or actual["semantics"]["physical"]["value"]["pickupable"] != items.source.K(
            True
        ):
            raise ValueError(f"Missing portable corpse admission: {key}")


def stage_portability_sql(root, path):
    portable = set(inherited_portable_stage_ids(root))
    portable.update(
        item for item, facts in read(root, "movable-stages") if facts["pickupable"]
    )
    start = "        -- Begin generated portable stages\n"
    end = "        -- End generated portable stages"
    sql = path.read_text()
    inherited = sql.split(end, 1)[1].split("]) AS id", 1)[0]
    portable.difference_update(map(int, re.findall(r"\b\d+\b", inherited)))
    body = "        " + ",".join(map(str, sorted(portable))) + ",\n"
    result, count = re.subn(
        re.escape(start) + r"(?s:.*?)" + re.escape(end),
        start + body + end,
        sql,
    )
    if count != 1:
        raise ValueError("Movable corpse SQL marker changed")
    return result


MIGRATION = "apps/game-server/migrations/3328_crystal_ordinary_corpse_registry.sql"


def migration(root):
    chains = read(root, "decay")
    values = ",\n".join(
        f"        ({chain[0]}, ARRAY[{','.join(map(str, chain))}], "
        f"ARRAY[{','.join(map(str, times))}]::BIGINT[])"
        for chain, times in chains
    )
    plain = ",".join(
        f"'oteryn:item.tibia.i{item}'"
        for item in sorted({item for _, item in read(root, "plain")})
    )
    return f"""-- Generated by ordinary_corpse_profiles.py from Crystal 201189f8.
-- Extend the installed corpse registries. The existing scheduler owns the
-- original mint proof, stage transforms and final retirement.
-- Existing receipts keep the policy that was in force at materialization,
-- including the deadline already frozen by a retirement reservation.
ALTER TABLE game_item_mint_receipts ADD COLUMN ordinary_corpse_policy BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE game_item_mint_receipts ALTER COLUMN ordinary_corpse_policy SET DEFAULT true;

CREATE FUNCTION game_corpse_policy_epoch_guard() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' AND NOT NEW.ordinary_corpse_policy THEN
        RAISE EXCEPTION 'New Item mints require the active corpse policy' USING ERRCODE='23514';
    ELSIF TG_OP = 'UPDATE' AND NEW.ordinary_corpse_policy IS DISTINCT FROM OLD.ordinary_corpse_policy THEN
        RAISE EXCEPTION 'The mint corpse policy is immutable' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_corpse_policy_epoch_guard() FROM PUBLIC;
CREATE TRIGGER game_corpse_policy_epoch_guard
    BEFORE INSERT OR UPDATE OF ordinary_corpse_policy ON game_item_mint_receipts
    FOR EACH ROW EXECUTE FUNCTION game_corpse_policy_epoch_guard();

DO $pre_ordinary_corpse_policy$
DECLARE
    predecessor TEXT;
    deadline TEXT := pg_get_functiondef('game_corpse_retirement_deadline(uuid)'::regprocedure);
    original CONSTANT TEXT := 'game_source_corpse_durations_v1(m.loot_table_production_key)';
BEGIN
    predecessor := pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
    EXECUTE replace(predecessor,'game_summer_corpse_chain(', 'game_pre_ordinary_corpse_chain(');
    predecessor := pg_get_functiondef('game_source_corpse_durations_v1(text)'::regprocedure);
    EXECUTE replace(replace(predecessor,
        'game_source_corpse_durations_v1(', 'game_pre_ordinary_corpse_durations('),
        'game_summer_corpse_chain(', 'game_pre_ordinary_corpse_chain(');
    IF (length(deadline)-length(replace(deadline,original,'')))/length(original) <> 1 THEN
        RAISE EXCEPTION 'Corpse retirement policy dispatcher changed';
    END IF;
    EXECUTE replace(deadline,original,'CASE WHEN m.ordinary_corpse_policy THEN '
        || original || ' ELSE game_pre_ordinary_corpse_durations(m.loot_table_production_key) END');
END
$pre_ordinary_corpse_policy$;
REVOKE ALL ON FUNCTION game_pre_ordinary_corpse_chain(TEXT),game_pre_ordinary_corpse_durations(TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_pre_ordinary_corpse_chain(TEXT),game_pre_ordinary_corpse_durations(TEXT)
    TO oteryn_game_runtime,oteryn_game_control;

DO $ordinary_crystal_corpses$
DECLARE
    body TEXT;
    anchor TEXT;
    source RECORD;
    item_key TEXT;
    installed_chain INTEGER[];
    installed_lifetimes BIGINT[];
    chain_cases TEXT := '';
    lifetime_cases TEXT := '';
BEGIN
    -- An installed source binding is reused if every stage and lifetime agrees.
    FOR source IN SELECT * FROM (VALUES
{values}
    ) AS policy(item, stages, lifetimes) LOOP
        item_key := 'oteryn:item.tibia.i' || source.item::text;
        installed_chain := game_summer_corpse_chain(item_key);
        installed_lifetimes := game_source_corpse_durations_v1(item_key);
        IF installed_chain = source.stages AND installed_lifetimes = source.lifetimes THEN
            CONTINUE;
        END IF;
        IF installed_chain IS NOT NULL OR installed_lifetimes IS NOT NULL THEN
            RAISE EXCEPTION 'Conflicting corpse lifecycle policy for %', item_key;
        END IF;
        chain_cases := chain_cases || format(E'\\n WHEN %L THEN %L::INTEGER[]',
                                             item_key, source.stages);
        lifetime_cases := lifetime_cases || format(E'\\n WHEN p_key=%L THEN %L::BIGINT[]',
                                                   item_key, source.lifetimes);
    END LOOP;
    body := pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
    anchor := 'WHEN ''oteryn:item.tibia.i54678'' THEN';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Crystal corpse chain dispatcher changed';
    END IF;
    EXECUTE replace(body, anchor, chain_cases || E'\\n ' || anchor);

    body := pg_get_functiondef('game_source_corpse_durations_v1(text)'::regprocedure);
    anchor := 'WHEN game_summer_corpse_chain(p_key) IS NOT NULL THEN';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Crystal corpse duration dispatcher changed';
    END IF;
    EXECUTE replace(body, anchor, lifetime_cases || E'\\n ' || anchor);

    body := pg_get_functiondef('game_item_plain_corpse_source(uuid)'::regprocedure);
    anchor := 'i.definition_production_key IN (';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Crystal plain corpse custody dispatcher changed';
    END IF;
    EXECUTE replace(body, anchor, anchor || $cases$ {plain}, $cases$);
END
$ordinary_crystal_corpses$;
"""


if __name__ == "__main__":
    import argparse
    from pathlib import Path

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    path = root / MIGRATION
    expected = migration(root)
    portable_sql = (
        root / "apps/game-server/migrations/3332_ordinary_corpse_move_lifecycle.sql"
    )
    portable_expected = stage_portability_sql(root, portable_sql)
    if args.check:
        check_movable_stages(root)
        if portable_sql.read_text() != portable_expected:
            raise SystemExit("Portable corpse stages differ from their source capture")
        if path.read_text() != expected:
            raise SystemExit(
                "Crystal ordinary corpse migration differs from its source capture"
            )
    else:
        path.write_text(expected)
        portable_sql.write_text(portable_expected)
