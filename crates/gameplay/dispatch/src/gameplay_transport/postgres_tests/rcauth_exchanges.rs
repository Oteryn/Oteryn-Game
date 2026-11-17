//! RCAUTH: Willie, Lee'Delle and Wyda exchange one carried item through the real NPC talk path.
//! Each case refuses without the input (no receipt, no item change), then consumes it and mints
//! the source reward in one fenced transaction; a literal replay changes nothing.

#[test]
fn rcauth_lee_delle_exchanges_a_honey_flower_for_studded_legs() -> super::TestResult {
    super::qengine_rewards::scenario("qengine-rewards-rcauth-lee-delle")
}

#[test]
fn rcauth_willie_exchanges_a_banana_for_a_studded_shield() -> super::TestResult {
    super::qengine_rewards::scenario("qengine-rewards-rcauth-willie")
}

#[test]
fn rcauth_wyda_exchanges_a_blood_herb_for_a_witchesbroom_once() -> super::TestResult {
    super::qengine_rewards::scenario("qengine-rewards-rcauth-wyda")
}
