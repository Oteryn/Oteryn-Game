//! Item-currency trades through NPC_TALK and NPC_TRADE_INTENT on the admitted wire path.
use super::TestResult;
use super::npc_shop_fixtures::{ShopScenario, scenario_for};

#[test]
fn npc_item_currency_buy_spends_only_matching_tokens_and_refuses_short_payment() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(ShopScenario::ItemCurrencyBuy)
}

#[test]
fn npc_item_currency_sell_mints_token_stacks_and_refuses_missing_goods() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(ShopScenario::ItemCurrencySell)
}

fn run(case: ShopScenario) -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario_for(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "item-currency shop PostgreSQL thread panicked")??;
    Ok(())
}
