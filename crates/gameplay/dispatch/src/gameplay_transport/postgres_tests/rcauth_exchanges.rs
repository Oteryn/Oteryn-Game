//! RCAUTH: literal NPC talk exchanges and plain authored reward custody.

#[test]
fn rcauth_lee_delle_exchanges_honey_flowers_for_studded_legs_twice() -> super::TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::qengine_rewards::scenario("qengine-rewards-rcauth-lee-delle")
}

#[test]
fn rcauth_plain_container_and_written_parchment_are_in_inventory() -> super::TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::quest_container_chests::plain_container_scenario()
}

#[test]
fn rcauth_willie_exchanges_a_banana_for_a_studded_shield_once() -> super::TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::qengine_rewards::scenario("qengine-rewards-rcauth-willie")
}

#[test]
fn rcauth_wyda_exchanges_a_blood_herb_for_a_witchesbroom_once() -> super::TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::qengine_rewards::scenario("qengine-rewards-rcauth-wyda")
}
