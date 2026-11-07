//! Local questpc dialogue offers, interpreted by the shared dispatcher.
pub(super) const OFFERS: &[&str] = &[
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/quests/questpc-draconia-reward-offers.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/quests/questpc-citizen-reward-offers.json"
    )),
];
