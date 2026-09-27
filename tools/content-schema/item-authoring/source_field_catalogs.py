"""Pinned source-field inventories and formal Item dispositions.

The catalogs deliberately record source coverage separately from the Item schema.
Every inventoried field is either mapped to one or more typed authoring paths or
is explicitly assigned to another owner.  Unknown fields never inherit a generic
attribute bag.
"""

from __future__ import annotations

CANARY_PROFILE = "canary_47dfd51_item_definition_v1"
CRYSTAL_PROFILE = "crystal_ff7ede5_item_definition_v1"
FANDOM_PROFILE = "tibia_fandom_merge_items_objects_1035268_v1"
BR_PROFILE = "tibiawiki_br_infobox_item_424807_v1"
BR_REAL_ITEM_PROFILE = "tibiawiki_br_real_item_pages_20260927_v1"
FANDOM_REAL_ITEM_PROFILE = "tibia_fandom_real_item_pages_20260927_v1"
REAL_ITEM_PAGE_CAPTURED_AT = "2026-09-27T07:08:47.439Z"


COMMON_PARSER_FIELDS = (
    "absorbpercentall",
    "absorbpercentallelements",
    "absorbpercentdeath",
    "absorbpercentdrown",
    "absorbpercentearth",
    "absorbpercentelements",
    "absorbpercentenergy",
    "absorbpercentfire",
    "absorbpercenthealing",
    "absorbpercentholy",
    "absorbpercentice",
    "absorbpercentlifedrain",
    "absorbpercentmagic",
    "absorbpercentmanadrain",
    "absorbpercentphysical",
    "absorbpercentpoison",
    "allowdistread",
    "allowpickupable",
    "ammotype",
    "armor",
    "attack",
    "augments",
    "bedpart",
    "bedpartof",
    "blocking",
    "blockprojectile",
    "charges",
    "cleavepercent",
    "containersize",
    "criticalhitchance",
    "criticalhitdamage",
    "deathmagiclevelpoints",
    "decayto",
    "defense",
    "description",
    "destroyto",
    "duration",
    "earthmagiclevelpoints",
    "effect",
    "elementalbond",
    "elementdeath",
    "elementearth",
    "elementenergy",
    "elementfire",
    "elementholy",
    "elementice",
    "energymagiclevelpoints",
    "extradef",
    "femalesleeper",
    "femaletransformto",
    "field",
    "fieldabsorbpercentearth",
    "fieldabsorbpercentenergy",
    "fieldabsorbpercentfire",
    "fieldabsorbpercentpoison",
    "firemagiclevelpoints",
    "floorchange",
    "fluidsource",
    "healingmagiclevelpoints",
    "healthgain",
    "healthticks",
    "hitchance",
    "holymagiclevelpoints",
    "icemagiclevelpoints",
    "imbuementslot",
    "invisible",
    "leveldoor",
    "lifeleechamount",
    "lifeleechchance",
    "loottype",
    "magiclevelpoints",
    "magicpoints",
    "magicpointspercent",
    "magicshieldcapacityflat",
    "magicshieldcapacitypercent",
    "malesleeper",
    "maletransformto",
    "managain",
    "manaleechamount",
    "manaleechchance",
    "manashield",
    "manaticks",
    "mantra",
    "maxhitchance",
    "maxhitpoints",
    "maxhitpointspercent",
    "maxmanapoints",
    "maxmanapointspercent",
    "maxtextlen",
    "movable",
    "partnerdirection",
    "perfectshotdamage",
    "perfectshotrange",
    "physicalmagiclevelpoints",
    "pickupable",
    "primarytype",
    "range",
    "readable",
    "reflectdamage",
    "reflectpercentall",
    "replaceable",
    "rotateto",
    "runespellname",
    "script",
    "shoottype",
    "showattributes",
    "showcharges",
    "showcount",
    "showduration",
    "skillaxe",
    "skillclub",
    "skilldist",
    "skillfish",
    "skillfist",
    "skillshield",
    "skillsword",
    "slottype",
    "speed",
    "stacksize",
    "stopduration",
    "suppresscurse",
    "suppressdazzle",
    "suppressdrown",
    "suppressdrunk",
    "suppressenergy",
    "suppressfire",
    "suppressfreeze",
    "suppressphysical",
    "suppresspoison",
    "transformdeequipto",
    "transformequipto",
    "transformonuse",
    "type",
    "unwrapableto",
    "usedbyhouseguests",
    "walkstack",
    "weapontype",
    "weight",
    "wrapableto",
    "wrapcontainer",
    "writeable",
    "writeonceitemid",
)

NESTED_DEFINITION_FIELDS = (
    "slot",
    "level",
    "vocation",
    "premium",
    "action",
    "breakchance",
    "mana",
    "unproperly",
    "fromdamage",
    "todamage",
    "wandtype",
    "chain",
    "eventtype",
    "weapontype",
)

ROOT_DEFINITION_FIELDS = (
    "name",
    "article",
    "plural",
    "id",
    "fromid",
    "toid",
    "editorsuffix",
)

BAG_RELATION_FIELDS = (
    "bags.itemid",
    "bags.name",
    "bags.chance",
    "bags.minAmount",
    "bags.maxAmount",
    "bags.class",
    "bags.raceId",
    "bags.bossOnly",
)

COMMON_APPEARANCE_FIELDS = (
    "appearance.id",
    "appearance.frame_group",
    "appearance.name",
    "appearance.description",
    "flags.bank",
    "flags.clip",
    "flags.bottom",
    "flags.top",
    "flags.container",
    "flags.cumulative",
    "flags.usable",
    "flags.forceuse",
    "flags.multiuse",
    "flags.write",
    "flags.write_once",
    "flags.liquidpool",
    "flags.unpass",
    "flags.unmove",
    "flags.unsight",
    "flags.avoid",
    "flags.no_movement_animation",
    "flags.take",
    "flags.liquidcontainer",
    "flags.hang",
    "flags.hook",
    "flags.rotate",
    "flags.light",
    "flags.dont_hide",
    "flags.translucent",
    "flags.shift",
    "flags.height",
    "flags.lying_object",
    "flags.animate_always",
    "flags.automap",
    "flags.lenshelp",
    "flags.fullbank",
    "flags.ignore_look",
    "flags.clothes",
    "flags.default_action",
    "flags.market",
    "flags.wrap",
    "flags.unwrap",
    "flags.topeffect",
    "flags.npcsaledata",
    "flags.changedtoexpire",
    "flags.corpse",
    "flags.player_corpse",
    "flags.cyclopediaitem",
    "flags.ammo",
    "flags.show_off_socket",
    "flags.reportable",
    "flags.upgradeclassification",
    "flags.reverse_addons_east",
    "flags.reverse_addons_west",
    "flags.reverse_addons_south",
    "flags.reverse_addons_north",
    "flags.wearout",
    "flags.clockexpire",
    "flags.expire",
    "flags.expirestop",
    "flags.wrapkit",
    "flags.skillwheel_gem",
    "flags.dual_wielding",
    "flags.proficiency",
    "bank.waypoints",
    "write.max_text_length",
    "write_once.max_text_length_once",
    "light.brightness",
    "light.color",
    "height.elevation",
    "shift.x",
    "shift.y",
    "clothes.slot",
    "default_action.action",
    "market.category",
    "market.trade_as_object_id",
    "market.show_as_object_id",
    "market.restrict_to_profession",
    "market.minimum_level",
    "npcsaledata.name",
    "npcsaledata.location",
    "npcsaledata.sale_price",
    "npcsaledata.buy_price",
    "npcsaledata.currency_object_type_id",
    "npcsaledata.currency_quest_flag_display_name",
    "automap.color",
    "hook.direction",
    "lenshelp.id",
    "changedtoexpire.former_object_typeid",
    "cyclopediaitem.cyclopedia_type",
    "skillwheel_gem.gem_quality_id",
    "skillwheel_gem.vocation_id",
    "proficiency.proficiency_id",
    "upgradeclassification.upgrade_classification",
)

CRYSTAL_APPEARANCE_ONLY_FIELDS = (
    "flags.imbueable",
    "flags.restrict_to_vocation",
    "flags.minimum_level",
    "flags.weapon_type",
    "imbueable.slot_count",
)


def _row(disposition, kind, status, destinations=(), reason=""):
    return {
        "disposition": disposition,
        "kind": kind,
        "status": status,
        "allowed_destinations": list(destinations),
        "reason": reason,
    }


def _mapped(
    *destinations,
    disposition="ITEM_TYPED",
    kind="definition",
    reason="typed portable Item definition fact",
):
    return _row(disposition, kind, "mapped", destinations, reason)


def _outside(disposition, kind, reason):
    status = {
        "PROVENANCE": "provenance_only",
        "RELATIONSHIP": "reverse_relation",
        "APPROVED_OMISSION": "approved_omission",
        "UNRESOLVED": "unresolved_semantics",
    }.get(disposition, "external_domain")
    return _row(disposition, kind, status, (), reason)


def _no_effect(reason):
    return _row("PINNED_NO_EFFECT", "source_defect", "pinned_no_effect", (), reason)


ENGINE_EXACT_RULES = {
    "description": _mapped(
        "/item/presentation/inspection_description", kind="presentation"
    ),
    "runespellname": _mapped("/item/use/ability"),
    "weight": _mapped("/item/physical/weight", "/item/presentation/display_weight"),
    "showcount": _mapped(
        "/item/presentation/display_flags/stack_count", kind="presentation"
    ),
    "armor": _mapped("/item/protection/armor"),
    "defense": _mapped("/item/weapon/defense"),
    "extradef": _mapped("/item/weapon/extra_defense"),
    "attack": _mapped("/item/weapon/attack"),
    "mantra": _mapped("/item/modifiers/mantra"),
    "movable": _mapped("/item/physical/movable"),
    "allowpickupable": _mapped("/item/physical/pickupable"),
    "pickupable": _mapped("/item/physical/pickupable"),
    "containersize": _mapped("/item/container/capacity"),
    "readable": _mapped("/item/readable/readable"),
    "writeable": _mapped("/item/readable/writable", "/item/readable/write_policy"),
    "maxtextlen": _mapped("/item/readable/max_characters"),
    "writeonceitemid": _mapped("/item/readable/write_once_target"),
    "allowdistread": _mapped("/item/readable/distance_readable"),
    "weapontype": _mapped("/item/weapon/weapon_type"),
    "slottype": _mapped("/item/equipment/slot", "/item/equipment/patterns/*/slot"),
    "ammotype": _mapped("/item/weapon/ammunition_kind"),
    "shoottype": _mapped("/item/presentation/projectile_effect", kind="presentation"),
    "effect": _mapped("/item/presentation/effects", kind="presentation"),
    "range": _mapped("/item/weapon/range_cells"),
    "stopduration": _mapped("/item/temporal/stop_duration_while_unequipped"),
    "decayto": _mapped("/item/temporal/decay_target", "/item/lifecycle/transforms"),
    "transformequipto": _mapped("/item/lifecycle/transforms"),
    "transformdeequipto": _mapped("/item/lifecycle/transforms"),
    "transformonuse": _mapped("/item/lifecycle/transforms"),
    "destroyto": _mapped("/item/lifecycle/transforms"),
    "duration": _mapped("/item/temporal/duration_ms"),
    "showduration": _mapped(
        "/item/presentation/display_flags/duration", kind="presentation"
    ),
    "charges": _mapped("/item/charges/count"),
    "showcharges": _mapped("/item/charges/show_count", kind="presentation"),
    "showattributes": _mapped(
        "/item/presentation/display_flags/attributes", kind="presentation"
    ),
    "hitchance": _mapped("/item/weapon/hit_chance_modifier_percent"),
    "maxhitchance": _mapped("/item/weapon/max_hit_chance_percent"),
    "invisible": _mapped("/item/modifiers/invisibility_enabled"),
    "speed": _mapped("/item/modifiers/movement_speed"),
    "healthgain": _mapped("/item/modifiers/regeneration"),
    "healthticks": _mapped("/item/modifiers/regeneration"),
    "managain": _mapped("/item/modifiers/regeneration"),
    "manaticks": _mapped("/item/modifiers/regeneration"),
    "manashield": _mapped("/item/modifiers/mana_shield_enabled"),
    "criticalhitchance": _mapped("/item/modifiers/critical_chance_percent"),
    "criticalhitdamage": _mapped("/item/modifiers/critical_damage_percent"),
    "lifeleechchance": _mapped("/item/modifiers/leech"),
    "lifeleechamount": _mapped("/item/modifiers/leech"),
    "manaleechchance": _mapped("/item/modifiers/leech"),
    "manaleechamount": _mapped("/item/modifiers/leech"),
    "maxhitpoints": _mapped("/item/modifiers/resource_capacity"),
    "maxhitpointspercent": _mapped("/item/modifiers/resource_capacity"),
    "maxmanapoints": _mapped("/item/modifiers/resource_capacity"),
    "maxmanapointspercent": _mapped("/item/modifiers/resource_capacity"),
    "magiclevelpoints": _mapped("/item/modifiers/magic_level"),
    "magicpoints": _mapped("/item/modifiers/magic_level"),
    "imbuementslot": _mapped("/item/imbuement/slot_count"),
    "stacksize": _mapped("/item/stack/max_count"),
    "magicshieldcapacityflat": _mapped("/item/modifiers/magic_shield_capacity"),
    "magicshieldcapacitypercent": _mapped("/item/modifiers/magic_shield_capacity"),
    "perfectshotdamage": _mapped("/item/modifiers/perfect_shot_bonus"),
    "perfectshotrange": _mapped("/item/modifiers/perfect_shot_bonus"),
    "cleavepercent": _mapped("/item/modifiers/cleave_percent"),
    "reflectdamage": _mapped("/item/modifiers/reflection"),
    "reflectpercentall": _mapped("/item/modifiers/reflection"),
    "primarytype": _mapped("/item/taxonomy/primary"),
    "augments": _mapped("/item/proficiency/augments"),
    "elementalbond": _mapped("/item/modifiers/elemental_bond"),
    "proficiency": _mapped("/item/proficiency/profile_binding"),
    "meleeattackeffect": _mapped(
        "/item/presentation/attack_effect", kind="presentation"
    ),
    "wrapableto": _mapped("/item/lifecycle/wrapping/wrap_target"),
    "wrapcontainer": _mapped("/item/lifecycle/wrapping/preserve_contents"),
    "loottype": _mapped("/item/taxonomy/tags"),
    "field": _outside(
        "TERRAIN",
        "terrain",
        "placed magic-field behavior belongs to Terrain/Interaction",
    ),
    "fluidsource": _outside(
        "WORLD_OBJECT",
        "world_object",
        "fluid source is a placed WorldObject/Terrain fact",
    ),
    "leveldoor": _outside(
        "WORLD_OBJECT", "world_object", "door gating belongs to WorldObject/Interaction"
    ),
    "usedbyhouseguests": _outside(
        "WORLD_OBJECT",
        "world_object",
        "house access belongs to WorldObject/Interaction ACL",
    ),
}

WORLD_FIELDS = {
    "rotateto",
    "blockprojectile",
    "floorchange",
    "replaceable",
    "partnerdirection",
    "bedpart",
    "bedpartof",
    "malesleeper",
    "maletransformto",
    "femalesleeper",
    "femaletransformto",
    "walkstack",
    "blocking",
}


PINNED_NO_EFFECT_COMMON = {
    "unwrapableto": "registered, but parseWrapableTo accepts only wrapableto",
    "magicpointspercent": "registered, but the handler checks the unregistered magiclevelpointspercent spelling",
    "fieldabsorbpercentearth": "registered, but the handler accepts only energy, fire and poison spellings",
    "absorbpercentallelements": "registered, but the handler expects absorbpercentelements",
}

ENGINE_VALUE_ROUTES = {
    "flags.unmove": {
        False: {
            **_mapped(
                "/item/physical/movable",
                reason="appearance unmove=false normalizes to movable=true",
            ),
            "destination_value_transform": "boolean_not_source",
        },
        True: {
            **_mapped(
                "/item/physical/movable",
                reason="appearance unmove=true normalizes to movable=false",
            ),
            "destination_value_transform": "boolean_not_source",
        },
    },
    "type": {
        **{
            value: {
                **_mapped("/item/taxonomy/item_class"),
                "destination_value_equals": value,
            }
            for value in (
                "key",
                "container",
                "rune",
                "supply",
                "creatureproduct",
                "food",
                "valuable",
                "potion",
                "soulcore",
            )
        },
        **{
            value: _outside(
                "EXTERNAL_DOMAIN",
                "external_domain",
                "this type value belongs to WorldObject, Terrain or Interaction",
            )
            for value in (
                "magicfield",
                "depot",
                "rewardchest",
                "carpet",
                "mailbox",
                "trashholder",
                "teleport",
                "door",
                "bed",
                "ladder",
                "dummy",
            )
        },
    },
    "script": {
        "weapon": _mapped("/item/weapon"),
        "moveevent": _row(
            "VALUE_DEPENDENT",
            "template_control",
            "approved_omission",
            (),
            "nested eventtype supplies the exact portable or external owner route",
        ),
    },
    "eventtype": {
        "equip": {
            **_mapped("/item/equipment/activation_events"),
            "destination_array_contains": "equip",
        },
        "deequip": {
            **_mapped(
                "/item/equipment/activation_events",
                reason="source deequip normalizes to the unequip activation event",
            ),
            "destination_array_contains": "unequip",
        },
        "stepin": _outside(
            "EXTERNAL_BEHAVIOR",
            "external_domain",
            "Interaction owns movement-event behavior",
        ),
        "stepout": _outside(
            "EXTERNAL_BEHAVIOR",
            "external_domain",
            "Interaction owns movement-event behavior",
        ),
        "additem": _outside(
            "EXTERNAL_BEHAVIOR",
            "external_domain",
            "Interaction owns movement-event behavior",
        ),
        "removeitem": _outside(
            "EXTERNAL_BEHAVIOR",
            "external_domain",
            "Interaction owns movement-event behavior",
        ),
    },
    "action": {
        "removecharge": {
            **_mapped("/item/weapon/consumption_mode"),
            "destination_value_equals": "consume_charge",
        },
        "removecount": {
            **_mapped("/item/weapon/consumption_mode"),
            "destination_value_equals": "consume_item",
        },
        "move": {
            **_mapped("/item/weapon/consumption_mode"),
            "destination_value_equals": "move_item_to_impact_tile",
        },
    },
    "weapontype": {
        source: {
            **_mapped("/item/weapon/weapon_type"),
            "destination_value_equals": target,
        }
        for source, target in {
            "sword": "sword",
            "axe": "axe",
            "club": "club",
            "fist": "fist",
            "distance": "distance_launcher",
            "ammo": "ammunition",
            "missile": "thrown_missile",
            "wand": "wand",
            "rod": "rod",
            "shield": "shield",
            "spellbook": "spellbook",
        }.items()
    },
}

ENGINE_NORMALIZATION_NOTES = {
    "movable": "registered twice; both registrations dispatch to the same handler",
    "allowpickupable": "exact alias group pickupable",
    "pickupable": "exact alias group pickupable",
    "magicpoints": "alias of magiclevelpoints",
    "fieldabsorbpercentpoison": "source spelling normalizes damage type to earth",
    "absorbpercentpoison": "source spelling normalizes damage type to earth",
    "absorbpercentearth": "canonical earth spelling",
    "absorbpercentall": "expands to every combat type except healing, undefined and mana drain",
    "absorbpercentmagic": "expands to energy, fire, earth and ice",
    "absorbpercentelements": "expands to energy, fire, earth and ice",
    "firemagiclevelpoints": "wrong pinned enum token, but string dispatch remains effective",
    "malesleeper": "Crystal alias of maletransformto; pinned Canary registration is ineffective",
    "femalesleeper": "Crystal alias of femaletransformto; pinned Canary registration is ineffective",
    "weapontype": "distance, ammo and missile normalize to distinct canonical authoring kinds; spellbook remains distinct from shield",
}


def engine_rule(field, profile):
    if field in ENGINE_VALUE_ROUTES:
        return _row(
            "VALUE_DEPENDENT",
            "raw_text",
            "unresolved_semantics",
            (),
            "source_value is required to select a pinned route",
        )
    if field in PINNED_NO_EFFECT_COMMON:
        return _no_effect(PINNED_NO_EFFECT_COMMON[field])
    if profile == CANARY_PROFILE and field in {"malesleeper", "femalesleeper"}:
        return _no_effect("registered alias has no effective pinned Canary handler")
    if field in ENGINE_EXACT_RULES:
        return ENGINE_EXACT_RULES[field]
    if field in WORLD_FIELDS:
        return _outside(
            "WORLD_OBJECT",
            "world_object",
            "placed collision, traversal, bed or transform behavior is not portable Item truth",
        )
    if field.startswith(("absorbpercent", "fieldabsorbpercent")):
        return _mapped("/item/protection/resistances")
    if field.startswith("suppress"):
        return _mapped("/item/protection/condition_suppressions")
    if field.startswith("skill") and field not in {"skillwheel_gem"}:
        return _mapped("/item/modifiers/skill_boost")
    if field.endswith("magiclevelpoints"):
        return _mapped("/item/modifiers/magic_level")
    if field.startswith("element") and field != "elementalbond":
        return _mapped("/item/weapon/elemental_attack")
    raise ValueError("missing engine field disposition: " + field)


NESTED_RULES = {
    "slot": _mapped("/item/equipment/slot", "/item/equipment/patterns/*/slot"),
    "level": _mapped(
        "/item/requirements/min_level", "/item/equipment/patterns/*/min_level"
    ),
    "vocation": _mapped(
        "/item/requirements/vocations", "/item/equipment/patterns/*/vocations"
    ),
    "premium": _mapped("/item/requirements/premium_only"),
    "action": _row(
        "VALUE_DEPENDENT",
        "raw_text",
        "unresolved_semantics",
        (),
        "source_value is required to select a pinned action normalization",
    ),
    "breakchance": _mapped("/item/weapon/break_chance_percent"),
    "mana": _mapped("/item/use/mana_cost"),
    "unproperly": _mapped("/item/requirements/level_magic_shortfall"),
    "fromdamage": _mapped("/item/weapon/damage_range"),
    "todamage": _mapped("/item/weapon/damage_range"),
    "wandtype": _mapped("/item/weapon/damage_type"),
    "chain": _mapped("/item/weapon/chain"),
    "eventtype": _row(
        "VALUE_DEPENDENT",
        "raw_text",
        "unresolved_semantics",
        (),
        "source_value is required to select a pinned route",
    ),
    "weapontype": _row(
        "VALUE_DEPENDENT",
        "raw_text",
        "unresolved_semantics",
        (),
        "source_value is required to select a canonical weapon kind",
    ),
}


ROOT_RULES = {
    "name": _mapped("/item/display_name"),
    "article": _mapped("/item/presentation/grammar/article", kind="presentation"),
    "plural": _mapped("/item/presentation/grammar/plural", kind="presentation"),
    "id": _outside(
        "PROVENANCE", "provenance", "numeric source ID is crosswalk provenance only"
    ),
    "fromid": _outside(
        "PROVENANCE",
        "provenance",
        "numeric source ID range is crosswalk provenance only",
    ),
    "toid": _outside(
        "PROVENANCE",
        "provenance",
        "numeric source ID range is crosswalk provenance only",
    ),
    "editorsuffix": _outside(
        "APPROVED_OMISSION",
        "editor",
        "present in source XML but ignored by the pinned loader",
    ),
}


APPEARANCE_EXACT_RULES = {
    "appearance.id": _outside(
        "PROVENANCE",
        "provenance",
        "numeric source appearance ID is crosswalk provenance",
    ),
    "appearance.frame_group": _mapped(
        "/item/presentation/appearance_binding", kind="presentation"
    ),
    "appearance.name": _mapped("/item/display_name"),
    "appearance.description": _mapped(
        "/item/presentation/inspection_description", kind="presentation"
    ),
    "flags.container": _mapped("/item/container"),
    "flags.cumulative": _mapped("/item/stack/stackable"),
    "flags.usable": _mapped("/item/use/usable"),
    "flags.forceuse": _outside(
        "UNRESOLVED",
        "raw_text",
        "loaded by the pinned engines but no downstream behavior was proven",
    ),
    "flags.multiuse": _mapped("/item/use/use_with"),
    "flags.write": _mapped("/item/readable/writable", "/item/readable/write_policy"),
    "flags.write_once": _mapped(
        "/item/readable/writable", "/item/readable/write_policy"
    ),
    "flags.take": _mapped("/item/physical/pickupable"),
    "flags.unmove": _row(
        "VALUE_DEPENDENT",
        "raw_text",
        "unresolved_semantics",
        (),
        "source_value is required to apply the pinned boolean inversion",
    ),
    "flags.liquidcontainer": _mapped("/item/fluid/role"),
    "flags.light": _mapped("/item/light", kind="presentation"),
    "flags.default_action": _mapped("/item/use/default_action"),
    "flags.market": {
        **_mapped("/item/trade/marketable"),
        "destination_value_equals": True,
    },
    "flags.wrap": _mapped("/item/lifecycle/wrapping/wrap_enabled"),
    "flags.unwrap": _mapped("/item/lifecycle/wrapping/unwrap_enabled"),
    "flags.changedtoexpire": _mapped(
        "/item/temporal/decay_target", "/item/lifecycle/transforms"
    ),
    "flags.corpse": _outside(
        "WORLD_OBJECT",
        "world_object",
        "corpse container behavior belongs to WorldObject/Interaction",
    ),
    "flags.player_corpse": _outside(
        "WORLD_OBJECT",
        "world_object",
        "player-corpse behavior belongs to WorldObject/Interaction",
    ),
    "flags.ammo": _mapped("/item/weapon/weapon_type"),
    "flags.upgradeclassification": _mapped("/item/forge/classification"),
    "upgradeclassification.upgrade_classification": _mapped(
        "/item/forge/classification"
    ),
    "flags.wearout": _mapped(
        "/item/presentation/display_flags/client_wear_counter", kind="presentation"
    ),
    "flags.clockexpire": _mapped(
        "/item/presentation/display_flags/client_expiry_timer", kind="presentation"
    ),
    "flags.expire": _mapped(
        "/item/presentation/display_flags/client_expiry_timer", kind="presentation"
    ),
    "flags.expirestop": _mapped(
        "/item/presentation/display_flags/client_expiry_timer", kind="presentation"
    ),
    "flags.wrapkit": _mapped("/item/lifecycle/wrapping/is_wrap_kit"),
    "flags.dual_wielding": _mapped("/item/equipment/dual_wielding"),
    "flags.proficiency": _outside(
        "PROVENANCE",
        "provenance",
        "source proficiency presence requires an admitted canonical crosswalk",
    ),
    "flags.imbueable": _mapped("/item/imbuement/slot_count"),
    "flags.restrict_to_vocation": _mapped(
        "/item/trade/vocations", "/item/equipment/patterns/*/vocations"
    ),
    "flags.minimum_level": _mapped(
        "/item/requirements/min_level", "/item/equipment/patterns/*/min_level"
    ),
    "flags.weapon_type": _mapped("/item/weapon/weapon_type"),
    "write.max_text_length": _mapped("/item/readable/max_characters"),
    "write_once.max_text_length_once": _mapped("/item/readable/max_characters"),
    "light.brightness": _mapped("/item/light/intensity", kind="presentation"),
    "light.color": _mapped("/item/light/color_binding", kind="presentation"),
    "default_action.action": _mapped("/item/use/default_action"),
    "market.category": _mapped("/item/trade/market_category"),
    "market.trade_as_object_id": _outside(
        "PROVENANCE", "provenance", "source appearance crosswalk only"
    ),
    "market.show_as_object_id": _outside(
        "PROVENANCE", "provenance", "source appearance crosswalk only"
    ),
    "market.restrict_to_profession": _mapped("/item/trade/vocations"),
    "market.minimum_level": _mapped("/item/requirements/min_level"),
    "npcsaledata.name": _outside(
        "RELATIONSHIP", "relationship", "NPC Service owns Item offers"
    ),
    "npcsaledata.location": _outside(
        "RELATIONSHIP", "relationship", "NPC Service owns Item offers"
    ),
    "npcsaledata.sale_price": _outside(
        "RELATIONSHIP", "relationship", "NPC Service owns Item offers"
    ),
    "npcsaledata.buy_price": _outside(
        "RELATIONSHIP", "relationship", "NPC Service owns Item offers"
    ),
    "npcsaledata.currency_object_type_id": _outside(
        "RELATIONSHIP", "relationship", "NPC Service owns Item offers"
    ),
    "npcsaledata.currency_quest_flag_display_name": _outside(
        "RELATIONSHIP", "relationship", "NPC Service owns Item offers"
    ),
    "changedtoexpire.former_object_typeid": _mapped(
        "/item/temporal/decay_target", "/item/lifecycle/transforms"
    ),
    "proficiency.proficiency_id": _mapped(
        "/item/proficiency/profile_binding",
        reason="numeric source identity maps only through an admitted exact proficiency crosswalk",
    ),
    "imbueable.slot_count": _mapped("/item/imbuement/slot_count"),
}

APPEARANCE_PRESENTATION_BINDING = {
    "flags.clip",
    "flags.bottom",
    "flags.top",
    "flags.no_movement_animation",
    "flags.dont_hide",
    "flags.translucent",
    "flags.shift",
    "flags.lying_object",
    "flags.animate_always",
    "flags.topeffect",
    "flags.reverse_addons_east",
    "flags.reverse_addons_west",
    "flags.reverse_addons_south",
    "flags.reverse_addons_north",
    "shift.x",
    "shift.y",
}

APPEARANCE_WORLD_FIELDS = {
    "flags.bank",
    "flags.liquidpool",
    "flags.unpass",
    "flags.unsight",
    "flags.avoid",
    "flags.hang",
    "flags.hook",
    "flags.rotate",
    "flags.fullbank",
    "flags.ignore_look",
    "flags.height",
    "height.elevation",
    "flags.show_off_socket",
    "bank.waypoints",
    "hook.direction",
}


def appearance_rule(field):
    if field in APPEARANCE_EXACT_RULES:
        return APPEARANCE_EXACT_RULES[field]
    if field in APPEARANCE_PRESENTATION_BINDING:
        return _mapped("/item/presentation/appearance_binding", kind="presentation")
    if field in APPEARANCE_WORLD_FIELDS:
        return _outside(
            "WORLD_OBJECT",
            "world_object",
            "placed geometry, collision or traversal belongs to WorldObject/Terrain",
        )
    if field in {"flags.automap", "automap.color"}:
        return _outside(
            "TERRAIN",
            "terrain",
            "automap presentation belongs to Terrain/world presentation",
        )
    if field in {"flags.clothes", "clothes.slot"}:
        return _mapped("/item/equipment/slot", "/item/presentation/appearance_binding")
    if field in {
        "flags.lenshelp",
        "lenshelp.id",
        "flags.cyclopediaitem",
        "cyclopediaitem.cyclopedia_type",
        "flags.reportable",
    }:
        return _outside(
            "PROVENANCE",
            "provenance",
            "client catalog/help/report metadata is provenance, not gameplay truth",
        )
    if field == "flags.npcsaledata":
        return _outside("RELATIONSHIP", "relationship", "NPC Service owns Item offers")
    if field in {
        "flags.skillwheel_gem",
        "skillwheel_gem.gem_quality_id",
        "skillwheel_gem.vocation_id",
    }:
        return _outside(
            "EXTERNAL_DOMAIN", "external_domain", "Skill Wheel owns gem semantics"
        )
    raise ValueError("missing appearance field disposition: " + field)


FANDOM_FIELDS = (
    "name",
    "article",
    "actualname",
    "plural",
    "flavortext",
    "sounds",
    "itemid",
    "implemented",
    "itemclass",
    "primarytype",
    "secondarytype",
    "pickupable",
    "lightcolor",
    "lightradius",
    "immobile",
    "blockspath",
    "walkable",
    "walkingspeed",
    "unshootable",
    "rotatable",
    "hangable",
    "mapcolor",
    "volume",
    "holdsliquid",
    "usable",
    "writable",
    "rewritable",
    "writechars",
    "levelrequired",
    "vocrequired",
    "mlrequired",
    "hands",
    "type",
    "attack",
    "fire_attack",
    "earth_attack",
    "ice_attack",
    "energy_attack",
    "death_attack",
    "defense",
    "defensemod",
    "imbueslots",
    "range",
    "atk_mod",
    "hit_mod",
    "crithit_ch",
    "critextra_dmg",
    "manaleech_ch",
    "manaleech_am",
    "hpleech_ch",
    "hpleech_am",
    "mana",
    "damagetype",
    "damage",
    "attrib",
    "charges",
    "armor",
    "resist",
    "words",
    "weight",
    "stackable",
    "duration",
    "value",
    "storevalue",
    "npcvalue",
    "npcprice",
    "npcvaluerook",
    "npcpricerook",
    "buyfrom",
    "sellto",
    "fansite",
    "consumable",
    "regenseconds",
    "imbuements",
    "enchanted",
    "enchantable",
    "destructible",
    "marketable",
    "droppedby",
    "location",
    "notes",
    "notes2",
    "history",
    "status",
)

FANDOM_EXACT_RULES = {
    "name": _mapped("/item/display_name"),
    "article": _mapped("/item/presentation/grammar/article", kind="presentation"),
    "actualname": _outside(
        "UNRESOLVED",
        "raw_text",
        "historical alias/override semantics conflict with name; preserve and fail closed",
    ),
    "plural": _mapped("/item/presentation/grammar/plural", kind="presentation"),
    "flavortext": _mapped("/item/presentation/flavor_text", kind="presentation"),
    "sounds": _mapped("/item/presentation/sounds", kind="presentation"),
    "itemid": _outside("PROVENANCE", "provenance", "source numeric ID only"),
    "implemented": _outside("PROVENANCE", "provenance", "source history metadata"),
    "itemclass": _mapped("/item/taxonomy/item_class"),
    "primarytype": _mapped("/item/taxonomy/primary"),
    "secondarytype": _mapped("/item/taxonomy/secondary"),
    "pickupable": _mapped("/item/physical/pickupable"),
    "lightcolor": _mapped("/item/light/color_binding", kind="presentation"),
    "lightradius": _mapped("/item/light/radius_cells", kind="presentation"),
    "volume": _mapped("/item/container/capacity"),
    "holdsliquid": _mapped("/item/container/content_kind", "/item/fluid/role"),
    "usable": _mapped("/item/use/usable"),
    "writable": _mapped("/item/readable/writable", "/item/readable/write_policy"),
    "rewritable": _mapped("/item/readable/write_policy"),
    "writechars": _mapped("/item/readable/max_characters"),
    "levelrequired": _mapped(
        "/item/requirements/min_level", "/item/equipment/patterns/*/min_level"
    ),
    "vocrequired": _mapped(
        "/item/requirements/vocations", "/item/equipment/patterns/*/vocations"
    ),
    "mlrequired": _mapped("/item/requirements/min_magic_level"),
    "hands": _mapped("/item/equipment/hands", "/item/equipment/patterns/*/hands"),
    "type": _mapped("/item/weapon/weapon_type"),
    "attack": _mapped("/item/weapon/attack"),
    "defense": _mapped("/item/weapon/defense"),
    "defensemod": _mapped("/item/weapon/extra_defense"),
    "imbueslots": _mapped("/item/imbuement/slot_count"),
    "range": _mapped("/item/weapon/range_cells"),
    "atk_mod": _mapped("/item/weapon/attack_modifier"),
    "hit_mod": _mapped("/item/weapon/hit_chance_modifier_percent"),
    "crithit_ch": _mapped("/item/modifiers/critical_chance_percent"),
    "critextra_dmg": _mapped("/item/modifiers/critical_damage_percent"),
    "manaleech_ch": _mapped("/item/modifiers/leech"),
    "manaleech_am": _mapped("/item/modifiers/leech"),
    "hpleech_ch": _mapped("/item/modifiers/leech"),
    "hpleech_am": _mapped("/item/modifiers/leech"),
    "mana": _mapped("/item/use/mana_cost"),
    "damagetype": _mapped(
        "/item/use/damage_type",
        "/item/weapon/damage_type",
        reason="family context must select exactly one destination",
    ),
    "damage": _mapped(
        "/item/use/damage",
        "/item/weapon/damage_range",
        reason="family context must select exactly one destination",
    ),
    "attrib": _mapped(
        "/item/source_observations/attributes_text",
        disposition="SOURCE_TEXT_PRESERVE_AND_PARSE",
        kind="raw_text",
    ),
    "charges": _mapped("/item/charges/count"),
    "armor": _mapped("/item/protection/armor"),
    "resist": _mapped("/item/protection/resistances"),
    "words": _outside(
        "EXTERNAL_BEHAVIOR",
        "external_domain",
        "Ability/Interaction owns spell incantation and behavior",
    ),
    "weight": _mapped("/item/physical/weight", "/item/presentation/display_weight"),
    "stackable": _mapped("/item/stack/stackable"),
    "duration": _mapped("/item/temporal/duration_ms"),
    "value": _mapped(
        "/item/editor/estimated_value", disposition="PRESENTATION_EDITOR", kind="editor"
    ),
    "storevalue": _outside(
        "EXTERNAL_DOMAIN", "external_domain", "Platform/Commerce owns commercial price"
    ),
    "fansite": _outside(
        "UNRESOLVED",
        "raw_text",
        "historical value shape is ambiguous; preserve and fail closed",
    ),
    "consumable": _outside(
        "APPROVED_OMISSION",
        "template_control",
        "historical migration table marks this parameter removed/unnecessary",
    ),
    "regenseconds": _mapped("/item/consumable/regeneration_seconds"),
    "imbuements": _outside(
        "UNRESOLVED",
        "raw_text",
        "could mean applicability, reverse association or instance state; preserve and fail closed",
    ),
    "enchanted": _mapped("/item/lifecycle/enchanted_variant"),
    "enchantable": _mapped("/item/lifecycle/enchantable"),
    "destructible": _mapped("/item/lifecycle/destructible"),
    "marketable": _mapped("/item/trade/marketable"),
    "location": _outside(
        "EXTERNAL_DOMAIN",
        "external_domain",
        "World/acquisition owner; never intrinsic portable Item truth",
    ),
    "notes": _mapped(
        "/item/editor/notes", disposition="PRESENTATION_EDITOR", kind="editor"
    ),
    "notes2": _mapped(
        "/item/editor/notes", disposition="PRESENTATION_EDITOR", kind="editor"
    ),
    "history": _outside("PROVENANCE", "provenance", "source article history metadata"),
    "status": _outside(
        "PROVENANCE",
        "provenance",
        "historical publication status is not runtime lifecycle",
    ),
}

FANDOM_ELEMENT_FIELDS = {
    "fire_attack",
    "earth_attack",
    "ice_attack",
    "energy_attack",
    "death_attack",
}
FANDOM_WORLD_FIELDS = {
    "blockspath",
    "walkable",
    "walkingspeed",
    "unshootable",
    "rotatable",
    "hangable",
    "mapcolor",
}
FANDOM_RELATION_FIELDS = {
    "npcvalue",
    "npcprice",
    "npcvaluerook",
    "npcpricerook",
    "buyfrom",
    "sellto",
    "droppedby",
}

FANDOM_VALUE_ROUTES = {
    "immobile": {
        False: {
            **_mapped(
                "/item/physical/movable",
                reason="portable false value normalizes to physical.movable=true",
            ),
            "destination_value_transform": "boolean_not_source",
        },
        True: _outside(
            "WORLD_OBJECT",
            "world_object",
            "immobile source object is not admitted as a portable Item definition",
        ),
    }
}


def fandom_rule(field):
    if field in FANDOM_VALUE_ROUTES:
        return _row(
            "VALUE_DEPENDENT",
            "raw_text",
            "unresolved_semantics",
            (),
            "source_value is required to select a pinned route",
        )
    if field in FANDOM_EXACT_RULES:
        return FANDOM_EXACT_RULES[field]
    if field in FANDOM_ELEMENT_FIELDS:
        return _mapped("/item/weapon/elemental_attack")
    if field in FANDOM_WORLD_FIELDS:
        return _outside(
            "WORLD_OBJECT",
            "world_object",
            "placed collision, traversal or presentation belongs outside portable Item",
        )
    if field in FANDOM_RELATION_FIELDS:
        return _outside(
            "RELATIONSHIP",
            "relationship",
            "NPC Service or Creature/Loot owns the reverse relation",
        )
    raise ValueError("missing Fandom field disposition: " + field)


def build_engine_catalog(profile):
    if profile == CANARY_PROFILE:
        parser_fields = (*COMMON_PARSER_FIELDS, "proficiency")
        appearance_fields = COMMON_APPEARANCE_FIELDS
        repository = "opentibiabr/canary"
        revision = "47dfd51f45280a59a1d3e50ba7edd573d7234446"
    elif profile == CRYSTAL_PROFILE:
        parser_fields = (*COMMON_PARSER_FIELDS, "meleeattackeffect")
        appearance_fields = (*COMMON_APPEARANCE_FIELDS, *CRYSTAL_APPEARANCE_ONLY_FIELDS)
        repository = "zimbadev/crystalserver"
        revision = "ff7ede593c69d4c658b382c97443e8155926924a"
    else:
        raise ValueError("unknown engine source profile: " + profile)

    origins = {
        "xml_item_root": tuple(sorted(ROOT_DEFINITION_FIELDS)),
        "xml_item_attribute": tuple(sorted(parser_fields)),
        "nested_script_attribute": tuple(sorted(NESTED_DEFINITION_FIELDS)),
        "appearance": tuple(sorted(appearance_fields)),
        "reverse_bag_relation": tuple(sorted(BAG_RELATION_FIELDS)),
    }
    fields = sorted(set().union(*map(set, origins.values())))
    rows = []
    for field in fields:
        if field in ROOT_DEFINITION_FIELDS:
            rule = ROOT_RULES[field]
        elif field in parser_fields:
            rule = engine_rule(field, profile)
        elif field in NESTED_DEFINITION_FIELDS:
            rule = NESTED_RULES[field]
        elif field in BAG_RELATION_FIELDS:
            rule = _outside(
                "RELATIONSHIP",
                "relationship",
                "Creature/Loot owns reverse surprise-bag membership",
            )
        else:
            rule = appearance_rule(field)
        metadata = {}
        if field in ENGINE_VALUE_ROUTES:
            metadata["source_value_routes"] = [
                {"source_value": value, **route}
                for value, route in ENGINE_VALUE_ROUTES[field].items()
            ]
        if field == "weight":
            metadata["source_value_router"] = "signed_weight"
        if field == "chain":
            metadata["source_value_router"] = "chain_mode"
        if field in parser_fields:
            metadata["parser_registration_count"] = 2 if field == "movable" else 1
        if field in ENGINE_NORMALIZATION_NOTES:
            metadata["normalization_note"] = ENGINE_NORMALIZATION_NOTES[field]
        rows.append(
            {
                "source_field": field,
                "origins": [
                    name for name, values in origins.items() if field in values
                ],
                **rule,
                **metadata,
            }
        )
    return {
        "schema": "OTERYN_ITEM_AUTHORING_ENGINE_FIELD_DISPOSITIONS/candidate-2",
        "source_profile": profile,
        "repository": repository,
        "revision": revision,
        "parser_registry": {
            "entry_count": 144,
            "unique_key_count": 143,
            "duplicate_registrations": {"movable": 2},
        },
        "unknown_field_policy": "unsupported_source_field",
        "origins": [
            {"name": name, "fields": list(values)} for name, values in origins.items()
        ],
        "fields": rows,
    }


def build_fandom_catalog():
    return {
        "schema": "OTERYN_ITEM_AUTHORING_FANDOM_FIELD_DISPOSITIONS/candidate-2",
        "source_profile": FANDOM_PROFILE,
        "authority": "HISTORICAL_CORROBORATION_ONLY",
        "source_url": "https://tibia.fandom.com/wiki/TibiaWiki:Projects/Merge_Items_and_Objects",
        "revision_id": 1035268,
        "revision_timestamp": "2023-08-12T17:54:38Z",
        "revision_sha1": "97d3b59ab6a838c6ffbd6a9d68de16f03a7d3cec",
        "coverage": "exact 84 legacy Item Param rows in the pinned migration table; not a current Fandom template census",
        "unknown_field_policy": "unsupported_source_field",
        "fields": [
            {
                "source_field": field,
                **fandom_rule(field),
                **(
                    {
                        "source_value_routes": [
                            {"source_value": value, **route}
                            for value, route in FANDOM_VALUE_ROUTES[field].items()
                        ]
                    }
                    if field in FANDOM_VALUE_ROUTES
                    else {}
                ),
                **(
                    {"source_value_router": "signed_weight"}
                    if field == "weight"
                    else {}
                ),
            }
            for field in FANDOM_FIELDS
        ],
    }


BR_REAL_ITEM_PAGE_PINS = (
    (
        7109,
        "Backpack",
        432220,
        "2026-01-27T00:35:10Z",
        "f74a5e4c2bea507c5c2c766f311323073324b521",
        "75816d7708357163fc7b007641fa9e703f1a5c6d7dc3fb14df26dc9f3d12b19d",
        (
            "buyfrom",
            "droppedby",
            "imbuement",
            "implemented",
            "itemclass",
            "name",
            "notes",
            "npcprice",
            "npcvalue",
            "primarytype",
            "sellto",
            "stackable",
            "volume",
            "weight",
        ),
    ),
    (
        6527,
        "Demon Armor",
        379817,
        "2022-12-18T19:26:13Z",
        "1aafcabfd8fd058c16d5f6a07250641518a021a7",
        "c5254032ea340db36ec8564515df8f01500d1a748c7a61a8061dbd85f6dc3029",
        (
            "armor",
            "buyfrom",
            "classificacao",
            "imbuement",
            "itemclass",
            "max_tier",
            "name",
            "notes",
            "npcprice",
            "npcvalue",
            "primarytype",
            "sellto",
            "stackable",
            "value",
            "weight",
        ),
    ),
    (
        5810,
        "Magic Sword",
        436743,
        "2026-06-16T15:36:49Z",
        "fa042e6cac18e57d36b535cfd6cac46bfc644b92",
        "31bc9a48ea263fa02e1e85a82316e9810ae0602183ef52c7da217508da3535a9",
        (
            "attack",
            "buyfrom",
            "classificacao",
            "defense",
            "defensemod",
            "droppedby",
            "enchantable",
            "flavortext",
            "hands",
            "imbuement",
            "itemclass",
            "levelrequired",
            "max_tier",
            "name",
            "notes",
            "npcprice",
            "npcvalue",
            "perk1",
            "perk2",
            "perk3",
            "primarytype",
            "sellto",
            "stackable",
            "type",
            "value",
            "weight",
        ),
    ),
    (
        8230,
        "Red Apple",
        381934,
        "2022-12-18T20:00:31Z",
        "974d6697071e73c455e505edab05f8d54e30f4d7",
        "f0f46d71956b08626126d519778ea6aa510d2f56978540758729f166c5bde656",
        (
            "buyfrom",
            "droppedby",
            "droppedEventby",
            "edible",
            "flavortext",
            "itemclass",
            "name",
            "notes",
            "npcprice",
            "npcvalue",
            "primarytype",
            "regenseconds",
            "secondarytype",
            "sellto",
            "sounds",
            "stackable",
            "value",
            "weight",
        ),
    ),
    (
        10250,
        "Sudden Death Rune",
        435712,
        "2026-06-03T03:41:42Z",
        "e3dbb273bb387789a1c7b53d0a061c2c0fa8b569",
        "7bae31e2dbd4eb80969b98e411397603faffdc06dd56647f950eff24a5ba5a25",
        (
            "buyfrom",
            "cooldowngrupo",
            "cooldownproprio",
            "damagetype",
            "droppedby",
            "droppedRaidby",
            "effect",
            "history",
            "levelrequired",
            "makelvl",
            "makemana",
            "makeqty",
            "makevoc",
            "mlrequired",
            "name",
            "notes",
            "npcprice",
            "premium",
            "soul",
            "storevalue",
            "Subclass",
            "weight",
            "words",
        ),
    ),
    (
        7288,
        "Vial",
        382884,
        "2022-12-18T20:16:04Z",
        "2caba430cecd7be590e2fd88f3adcd1e0af92229",
        "b1390d881313a0b17b1c5ccff510fb001de926130244031f6bf9137aaeb20dbe",
        (
            "attrib",
            "buyfrom",
            "droppedby",
            "itemclass",
            "name",
            "notes",
            "npcprice",
            "npcvalue",
            "primarytype",
            "sellto",
            "stackable",
            "value",
            "weight",
        ),
    ),
)


FANDOM_REAL_ITEM_PAGE_PINS = (
    (
        1591,
        "Backpack",
        1147105,
        "2025-12-08T14:20:09Z",
        "c7057ac6b95e3b63ea1e5b832ca1836dde9ecb2c",
        "1c564f86bf3ddfc848499bd2bab637de01708594180a3c8403911755b8c567f0",
        (
            "actualname",
            "article",
            "buyfrom",
            "droppedby",
            "history",
            "imbueslots",
            "immobile",
            "implemented",
            "itemid",
            "marketable",
            "name",
            "notes",
            "npcprice",
            "npcpricerook",
            "npcvalue",
            "npcvaluerook",
            "objectclass",
            "pickupable",
            "primarytype",
            "secondarytype",
            "sellto",
            "slot",
            "stackable",
            "usable",
            "value",
            "volume",
            "walkable",
            "weight",
        ),
    ),
    (
        1619,
        "Demon Armor",
        1147324,
        "2025-12-08T14:29:11Z",
        "abc09722ed0519f1b9c3cf8ee3033c0b158e3cea",
        "b8036ddc56b00e2c3986d2b85975fd705fcdc023e8c935bce2debec28b5c9f6e",
        (
            "actualname",
            "armor",
            "article",
            "buyfrom",
            "droppedby",
            "imbueslots",
            "immobile",
            "implemented",
            "itemid",
            "marketable",
            "name",
            "notes",
            "npcprice",
            "npcvalue",
            "objectclass",
            "pickupable",
            "plural",
            "primarytype",
            "sellto",
            "slot",
            "stackable",
            "upgradeclass",
            "value",
            "walkable",
            "weight",
        ),
    ),
    (
        1402,
        "Magic Sword",
        1147223,
        "2025-12-08T14:27:08Z",
        "518be1259d3c036b0cf40ea0074f754855fa478e",
        "c4800c24eed0114d308946603be6c89c8f6a2c8727173e9d14cdc57e40525c5b",
        (
            "actualname",
            "article",
            "attack",
            "buyfrom",
            "defense",
            "defensemod",
            "droppedby",
            "enchantable",
            "flavortext",
            "hands",
            "history",
            "imbueslots",
            "immobile",
            "implemented",
            "itemid",
            "levelrequired",
            "marketable",
            "name",
            "notes",
            "npcprice",
            "npcvalue",
            "objectclass",
            "pickupable",
            "plural",
            "primarytype",
            "sellto",
            "slot",
            "stackable",
            "upgradeclass",
            "usable",
            "value",
            "walkable",
            "weapontype",
            "weight",
        ),
    ),
    (
        1250,
        "Red Apple",
        1200917,
        "2026-08-16T17:40:11Z",
        "b1da8f370a728b95088e32900d8c9805a51c36d7",
        "ae2cbafd3e7835d91b3c6a19e72a573b4bcd82922cd082f7e4a8d6e376aae05a",
        (
            "actualname",
            "article",
            "buyfrom",
            "consumable",
            "droppedby",
            "immobile",
            "implemented",
            "itemid",
            "marketable",
            "name",
            "notes",
            "npcprice",
            "npcpricerook",
            "npcvalue",
            "npcvaluerook",
            "objectclass",
            "pickupable",
            "plural",
            "primarytype",
            "regenseconds",
            "sellto",
            "sounds",
            "stackable",
            "usable",
            "value",
            "walkable",
            "weight",
        ),
    ),
    (
        2216,
        "Sudden Death Rune",
        1189790,
        "2026-06-16T20:15:20Z",
        "a596bff1fe50fbc5e11998029f4fe1294f61cdb4",
        "15e969af7af151448d23c126e9c0e7a73ec2b0de4b61b4c3a3cfb547b0fe648e",
        (
            "actualname",
            "article",
            "basepower",
            "buyfrom",
            "damagetype",
            "droppedby",
            "history",
            "immobile",
            "implemented",
            "itemid",
            "levelrequired",
            "marketable",
            "mlrequired",
            "name",
            "notes",
            "npcprice",
            "npcvalue",
            "objectclass",
            "pickupable",
            "plural",
            "primarytype",
            "sellto",
            "stackable",
            "storevalue",
            "usable",
            "value",
            "vocrequired",
            "walkable",
            "weight",
            "words",
        ),
    ),
    (
        2725,
        "Vial",
        1098584,
        "2025-04-10T13:29:32Z",
        "7d45c4734e713967ae7195c24dd1d6646b311db8",
        "833c0ab7bf1c13678e90e184d8e8034b5558ff3fe01cc34a2259e62d3eafaae4",
        (
            "actualname",
            "article",
            "buyfrom",
            "droppedby",
            "holdsliquid",
            "immobile",
            "implemented",
            "itemid",
            "marketable",
            "name",
            "notes",
            "npcprice",
            "npcpricerook",
            "npcvalue",
            "npcvaluerook",
            "objectclass",
            "pickupable",
            "plural",
            "primarytype",
            "sellto",
            "stackable",
            "usable",
            "value",
            "walkable",
            "weight",
        ),
    ),
)


def _page_pin(base_url, values):
    page_id, title, revision, timestamp, sha1, digest, fields = values
    return {
        "page_id": page_id,
        "title": title,
        "url": (
            base_url
            + "index.php?title="
            + title.replace(" ", "_")
            + "&oldid="
            + str(revision)
        ),
        "revision": str(revision),
        "revision_timestamp": timestamp,
        "captured_at": REAL_ITEM_PAGE_CAPTURED_AT,
        "revision_sha1": sha1,
        "content_sha256": digest,
        "raw_fields": list(fields),
    }


def build_br_real_item_supplement():
    external_ability = _outside(
        "EXTERNAL_BEHAVIOR",
        "external_domain",
        "Ability/Effect authoring owns executable spell and rune behavior",
    )
    rules = {
        "max_tier": _mapped("/item/forge/max_tier", disposition="ITEM_AUTHORING"),
        "Subclass": {
            **_row("VALUE_DEPENDENT", "raw_text", "unresolved_semantics"),
            "source_value_routes": [
                {
                    "source_value": "Ataque",
                    **_mapped("/item/taxonomy/secondary"),
                    "destination_value_equals": "attack",
                }
            ],
        },
        "mlrequired": _mapped("/item/requirements/min_magic_level"),
        "premium": {
            **_row("VALUE_DEPENDENT", "raw_text", "unresolved_semantics"),
            "source_value_routes": [
                {
                    "source_value": "não",
                    **_mapped("/item/requirements/premium_only"),
                    "destination_value_equals": False,
                }
            ],
        },
        **{
            field: external_ability
            for field in (
                "cooldowngrupo",
                "cooldownproprio",
                "effect",
                "words",
                "makelvl",
                "makemana",
                "makeqty",
                "makevoc",
                "soul",
            )
        },
        "history": _outside(
            "PROVENANCE", "provenance", "community history is evidence, not Item truth"
        ),
    }
    return {
        "schema": "OTERYN_ITEM_AUTHORING_REAL_ITEM_FIELD_SUPPLEMENT/candidate-3",
        "source_profile": BR_REAL_ITEM_PROFILE,
        "base_profile": BR_PROFILE,
        "authority": "PINNED_REAL_ITEM_PAGE_FIXTURE_SUPPLEMENT",
        "unknown_field_policy": "unsupported_source_field",
        "pages": [
            _page_pin("https://www.tibiawiki.com.br/", values)
            for values in BR_REAL_ITEM_PAGE_PINS
        ],
        "fields": [{"source_field": field, **rules[field]} for field in sorted(rules)],
    }


def build_fandom_real_item_supplement():
    approved_taxonomy_omission = _outside(
        "APPROVED_OMISSION",
        "template_control",
        "coarse community class does not select a portable Item capability",
    )
    rules = {
        "objectclass": {
            **_row("VALUE_DEPENDENT", "raw_text", "unresolved_semantics"),
            "source_value_routes": [
                {
                    "source_value": source,
                    **_mapped("/item/taxonomy/item_class"),
                    "destination_value_equals": target,
                }
                for source, target in (
                    ("Weapons", "weapon"),
                    ("Body Equipment", "equipment"),
                    ("Runes", "rune"),
                )
            ]
            + [
                {"source_value": source, **approved_taxonomy_omission}
                for source in (
                    "Household Items",
                    "Plants, Animal Products, Food and Drink",
                )
            ],
        },
        "slot": {
            **_row("VALUE_DEPENDENT", "raw_text", "unresolved_semantics"),
            "source_value_routes": [
                {
                    "source_value": "Body",
                    **_mapped("/item/equipment/slot"),
                    "destination_value_equals": "armor",
                },
                {
                    "source_value": "Weapon Hand",
                    **_outside(
                        "UNRESOLVED",
                        "raw_text",
                        "source does not select left_hand or right_hand",
                    ),
                },
                {
                    "source_value": "Container",
                    **_outside(
                        "APPROVED_OMISSION",
                        "template_control",
                        "container classification is not an equipment slot",
                    ),
                },
            ],
        },
        "upgradeclass": _mapped("/item/forge/classification"),
        "weapontype": {
            **_row("VALUE_DEPENDENT", "raw_text", "unresolved_semantics"),
            "source_value_routes": [
                {
                    "source_value": "Sword",
                    **_mapped("/item/weapon/weapon_type"),
                    "destination_value_equals": "sword",
                }
            ],
        },
        "basepower": _outside(
            "EXTERNAL_BEHAVIOR",
            "external_domain",
            "Ability combat formula owns rune base power",
        ),
    }
    return {
        "schema": "OTERYN_ITEM_AUTHORING_REAL_ITEM_FIELD_SUPPLEMENT/candidate-3",
        "source_profile": FANDOM_REAL_ITEM_PROFILE,
        "base_profile": FANDOM_PROFILE,
        "authority": "PINNED_REAL_ITEM_PAGE_FIXTURE_SUPPLEMENT",
        "unknown_field_policy": "unsupported_source_field",
        "pages": [
            _page_pin("https://tibia.fandom.com/", values)
            for values in FANDOM_REAL_ITEM_PAGE_PINS
        ],
        "fields": [{"source_field": field, **rules[field]} for field in sorted(rules)],
    }


assert len(COMMON_PARSER_FIELDS) == 142
assert len(set(COMMON_PARSER_FIELDS)) == 142
assert len(FANDOM_FIELDS) == 84
assert len(set(FANDOM_FIELDS)) == 84
assert len(build_br_real_item_supplement()["fields"]) == 14
assert len(build_fandom_real_item_supplement()["fields"]) == 5
