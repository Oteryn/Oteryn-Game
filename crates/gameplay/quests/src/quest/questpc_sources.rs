//! Local questpc content registrations; shared engine semantics remain unchanged.
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
pub(super) const CITIZEN_PROGRAM: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../content/dialogues/quest-rules-supplemental/citizen-mainland.json"
));
