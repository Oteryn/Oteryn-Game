//! Native action-id StepIn binding for Crystal's Unsafe Release summon.
use super::Callback;

pub(super) const LUCIFUGA: &str = "oteryn:quest-callback/dream-courts-lucifuga";

pub(super) fn lucifuga() -> Result<Callback, String> {
    let row = serde_json::json!({
        "key": LUCIFUGA,
        "quest": "oteryn:quest.the_dream_courts_quest",
        "trigger": "step_in",
        "selector": {"action": 23120},
        "reversible": false,
        "passages": [],
        "offerings": [],
        "return_on_failure": false,
        "storage": [{
            "track": "oteryn:quest-progress/quest/u12_00/the_dream_courts/unsafe_release/has_bait",
            "minimum": 1, "maximum": 1, "set": 1, "read_only": true
        }],
        "effects": {
            "step_summon": {
                "creature": "oteryn:creature.lucifuga_aranea",
                "cooldown_minutes": [1, 10]
            }
        },
        "sources": [
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/movements_spider_summon.lua:13-31"
        ]
    });
    super::parse(&serde_json::json!({"schema": 1, "callbacks": [row]}).to_string())?
        .pop()
        .ok_or_else(|| "missing Lucifuga step callback".into())
}
