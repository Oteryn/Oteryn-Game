"""Pinned canonical proficiency profiles admitted by Item authoring."""

from copy import deepcopy

CANARY_PROFICIENCY_SOURCE = {
    "source_profile": "canary_47dfd51_item_definition_v1",
    "repository": "opentibiabr/canary",
    "revision": "47dfd51f45280a59a1d3e50ba7edd573d7234446",
    "path": "data/items/proficiencies.json",
    "digest_sha256": "1a915dffd9265cd1c18d39e55da7ede691b2e58add534bc186238ae028a73f22",
}

CRYSTAL_PROFICIENCY_SOURCE = {
    "source_profile": "crystal_ff7ede5_item_definition_v1",
    "repository": "zimbadev/crystalserver",
    "revision": "ff7ede593c69d4c658b382c97443e8155926924a",
    "path": "data/json/proficiencies.json",
    "digest_sha256": "1a915dffd9265cd1c18d39e55da7ede691b2e58add534bc186238ae028a73f22",
}

MAGIC_SWORD_PROFICIENCY_REF = {
    "family": "Proficiency",
    "key": "oteryn:proficiency.weapon.sword.magic-sword",
    "revision": "definition-r1",
}

INTENSE_WOUND_CLEANSING_REF = {
    "family": "Ability",
    "key": "oteryn:ability.spell.intense-wound-cleansing",
    "revision": "definition-r1",
}

BERSERK_REF = {
    "family": "Ability",
    "key": "oteryn:ability.spell.berserk",
    "revision": "definition-r1",
}

MAGIC_SWORD_PROFICIENCY_PAYLOAD = {
    "levels": [
        {
            "level": 1,
            "selection_count": 1,
            "perks": [
                {
                    "selection_slot": 1,
                    "key": "sword_skill_auto_attack_extra_damage",
                    "target": {
                        "kind": "skill_scaled_auto_attack",
                        "skill": "sword",
                    },
                    "value": {
                        "kind": "rational_percent",
                        "value": {"numerator": 7, "denominator": 1},
                    },
                }
            ],
        },
        {
            "level": 2,
            "selection_count": 1,
            "perks": [
                {
                    "selection_slot": 1,
                    "key": "intense_wound_cleansing_cooldown",
                    "target": {
                        "kind": "ability",
                        "ability": INTENSE_WOUND_CLEANSING_REF,
                    },
                    "value": {"kind": "signed_milliseconds", "value": -30000},
                },
                {
                    "selection_slot": 2,
                    "key": "weapon_shield_modifier",
                    "target": {"kind": "weapon_shield_modifier"},
                    "value": {"kind": "signed_points", "value": 1},
                },
                {
                    "selection_slot": 3,
                    "key": "sword_skill_spell_healing",
                    "target": {
                        "kind": "skill_scaled_spell_healing",
                        "skill": "sword",
                    },
                    "value": {
                        "kind": "rational_percent",
                        "value": {"numerator": 10, "denominator": 1},
                    },
                },
            ],
        },
        {
            "level": 3,
            "selection_count": 1,
            "perks": [
                {
                    "selection_slot": 1,
                    "key": "berserk_life_leech",
                    "target": {"kind": "ability", "ability": BERSERK_REF},
                    "value": {
                        "kind": "rational_percent",
                        "value": {"numerator": 10, "denominator": 1},
                    },
                },
                {
                    "selection_slot": 2,
                    "key": "berserk_mana_leech",
                    "target": {"kind": "ability", "ability": BERSERK_REF},
                    "value": {
                        "kind": "rational_percent",
                        "value": {"numerator": 5, "denominator": 1},
                    },
                },
            ],
        },
    ]
}

MAGIC_SWORD_PROFICIENCY_SOURCE_IDENTITIES = frozenset(
    {
        (CANARY_PROFICIENCY_SOURCE["source_profile"], "238", 3),
        (CRYSTAL_PROFICIENCY_SOURCE["source_profile"], "238", 3),
    }
)


def proficiency_crosswalk(source):
    """Return one exact source-to-canonical Magic Sword proficiency crosswalk."""

    return {
        **deepcopy(source),
        "external_id": "238",
        "source_version": 3,
        "target": deepcopy(MAGIC_SWORD_PROFICIENCY_REF),
    }


def magic_sword_proficiency():
    """Return the admitted static profile without player-owned progression state."""

    return {
        "profile_binding": deepcopy(MAGIC_SWORD_PROFICIENCY_REF),
        **deepcopy(MAGIC_SWORD_PROFICIENCY_PAYLOAD),
    }
