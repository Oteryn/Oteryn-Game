//! NPC-TRADE-SELLALL-1: command 97 bytes through `serve_admitted`, the composed NPC trade owner
//! and runtime-role PostgreSQL: LOOK writes nothing, SELL_ALL sells every carried stack once.
use super::npc_shop_fixtures::{ShopScenario, run_shop_scenario, scenario_for};

use super::TestResult;

#[test]
fn npc_shop_postgres_real_wire_look_sell_all_replay_and_nothing_left() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "npc shop PostgreSQL thread panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run_shop_scenario(false)
}

#[test]
fn npc_shop_quotes_remain_available_without_an_item_profile() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_for(ShopScenario::MissingProfile).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "npc shop PostgreSQL thread panicked")??;
    Ok(())
}

#[test]
fn npc_shop_postgres_authoritative_side_bounds_and_bulk_quantities() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_for(ShopScenario::Bulk).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "bulk NPC trade thread panicked")??;
    Ok(())
}

#[test]
fn npc_shop_postgres_buy_checks_capacity_before_debit_or_split_mints() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("npc-buy-capacity", || {
        if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
            return Ok(());
        }
        scenario_for(ShopScenario::WeightLimit)
    })
}
