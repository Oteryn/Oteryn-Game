//! PKG6 P17: the owner decision of 2026-10-30 applies privacy and war rules to every world.
use super::TestResult;
use super::parameter_spell_fixtures::scenario_in_world;

macro_rules! native_case {
    ($test:ident, $world_type:literal, $case:literal) => {
        #[test]
        fn $test() -> TestResult {
            scenario_in_world($case, Some($world_type))
        }
    };
}

// Each isolated fixture also proves replay and refusal after the permission or active war is removed.
native_case!(p17_exiva_rules_enforced_pvp_postgres_explicit_denial, 3, 12);
native_case!(p17_exiva_rules_enforced_pvp_postgres_guild_list, 3, 9);
native_case!(p17_exiva_rules_enforced_pvp_postgres_own_guild, 3, 13);
native_case!(p17_exiva_rules_enforced_pvp_postgres_party, 3, 10);
native_case!(p17_exiva_rules_enforced_pvp_postgres_player_list, 3, 8);
native_case!(p17_exiva_rules_enforced_pvp_postgres_public, 3, 7);
native_case!(p17_exiva_rules_enforced_pvp_postgres_vip, 3, 11);
native_case!(
    p17_exiva_rules_enforced_pvp_postgres_war_caster_declares,
    3,
    14
);
native_case!(
    p17_exiva_rules_enforced_pvp_postgres_war_target_declares,
    3,
    16
);

native_case!(p17_exiva_rules_open_pvp_postgres_explicit_denial, 2, 12);
native_case!(p17_exiva_rules_open_pvp_postgres_guild_list, 2, 9);
native_case!(p17_exiva_rules_open_pvp_postgres_own_guild, 2, 13);
native_case!(p17_exiva_rules_open_pvp_postgres_party, 2, 10);
native_case!(p17_exiva_rules_open_pvp_postgres_player_list, 2, 8);
native_case!(p17_exiva_rules_open_pvp_postgres_public, 2, 7);
native_case!(p17_exiva_rules_open_pvp_postgres_vip, 2, 11);
native_case!(p17_exiva_rules_open_pvp_postgres_war_caster_declares, 2, 14);
native_case!(p17_exiva_rules_open_pvp_postgres_war_target_declares, 2, 16);

native_case!(p17_exiva_rules_optional_pvp_postgres_explicit_denial, 1, 12);
native_case!(p17_exiva_rules_optional_pvp_postgres_guild_list, 1, 9);
native_case!(p17_exiva_rules_optional_pvp_postgres_own_guild, 1, 13);
native_case!(p17_exiva_rules_optional_pvp_postgres_party, 1, 10);
native_case!(p17_exiva_rules_optional_pvp_postgres_player_list, 1, 8);
native_case!(p17_exiva_rules_optional_pvp_postgres_public, 1, 7);
native_case!(p17_exiva_rules_optional_pvp_postgres_vip, 1, 11);
native_case!(
    p17_exiva_rules_optional_pvp_postgres_war_caster_declares,
    1,
    14
);
native_case!(
    p17_exiva_rules_optional_pvp_postgres_war_target_declares,
    1,
    16
);
