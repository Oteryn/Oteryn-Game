//! ADR-0019 gameplay crate `oteryn-gameplay-combat`: modules moved verbatim from `oteryn-game-server` (re-exported there at the old path).

pub mod ability;
pub mod combat;
pub mod creature_appearance_content;
pub mod creature_area_heal;

pub mod creature_auto_attack;
pub mod creature_chain_attack;
pub mod creature_condition_content;
pub mod creature_defense_presentation;
pub mod creature_spell_facts;
mod error_codes;
pub mod hazard;

fn invariant_violation(code: oteryn_error_codes::ErrorCode, detail: &str) {
    use std::io::Write as _;

    let line = oteryn_error_codes::Line::new(
        oteryn_error_codes::Level::Error,
        "combat",
        "invariant_violation",
    )
    .code(code)
    .detail(detail);
    if oteryn_error_codes::filter().allows(line.level, line.module, true) {
        let rendered = format!(
            "{}\n",
            line.render("oteryn-game-server", oteryn_error_codes::unix_ms())
        );
        let _ignored = std::io::stderr().lock().write_all(rendered.as_bytes());
    }
}

const fn record_variant_invariant_code() -> oteryn_error_codes::ErrorCode {
    oteryn_error_codes::ErrorCode::new(
        4080,
        "COMBAT_RECORD_VARIANT_INVARIANT",
        oteryn_error_codes::Category::InternalUnavailable,
        oteryn_error_codes::Progression::Terminal,
    )
}

#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_content::content;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_domain::domain;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_durability::character_bootstrap_intent;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_durability::durability;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_entities as foundation;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_world::house_scope;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_world::map;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_world::movement;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_world::world_clock;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_world::world_object_revert;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_world::world_runtime;

#[path = "combat/attack/mod.rs"]
pub mod combat_attack;
#[path = "combat/pickup.rs"]
pub mod combat_pickup;
#[allow(dead_code, reason = "PvP stays off until the PVP-0 go-live gate")]
#[path = "combat/pvp/mod.rs"]
pub mod combat_pvp;
#[path = "ability/player_lethal.rs"]
pub mod player_lethal;

use oteryn_gameplay_source_critical_baseline as source_critical_baseline;
