//! Local questpc source identities and literal greetings.
pub(super) mod greetings;
pub(super) const CALLBACKS: &[&str] = &[
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/quests/questpc-draconia-reward-callbacks.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/quests/questpc-citizen-reward-callbacks.json"
    )),
];

pub(super) const GREETINGS: &[&str] = &[
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/dialogues/quests/questpc-draconia-reward-greetings.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/dialogues/quests/questpc-citizen-reward-greetings.json"
    )),
];
