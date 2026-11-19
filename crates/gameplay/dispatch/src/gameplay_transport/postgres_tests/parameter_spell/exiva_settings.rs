use super::*;

#[test]
fn exiva_settings_allow_all_postgres() -> TestResult {
    scenario(7)
}

#[test]
fn exiva_settings_player_list_postgres() -> TestResult {
    scenario(8)
}

#[test]
fn exiva_settings_guild_postgres() -> TestResult {
    scenario(9)
}

#[test]
fn exiva_settings_party_postgres() -> TestResult {
    scenario(10)
}

#[test]
fn exiva_settings_vip_postgres() -> TestResult {
    scenario(11)
}

#[test]
fn exiva_settings_denied_postgres() -> TestResult {
    scenario(12)
}

#[test]
fn exiva_settings_own_guild_postgres() -> TestResult {
    scenario(13)
}

#[test]
fn exiva_settings_active_war_optional_pvp_postgres() -> TestResult {
    scenario(14)
}

#[test]
fn exiva_settings_active_war_open_pvp_postgres() -> TestResult {
    scenario(15)
}

#[test]
fn exiva_settings_active_war_reverse_optional_pvp_postgres() -> TestResult {
    scenario(16)
}

#[test]
fn exiva_settings_named_client_list_postgres() -> TestResult {
    scenario(17)
}
