//! PostgreSQL-backed durability and native world adapters.

include!("core_modules.rs");

// Keep sorted by module name; insert in order, never append.
pub mod bed_regen;
pub mod bed_sleep;
pub mod creature_source_items;
pub mod daily_reward;
pub mod daily_reward_claim;
pub mod daily_reward_delivery;
mod daily_reward_jokers;
pub mod exercise_training;
pub mod guild_chat;
#[cfg(feature = "test-support")]
pub mod guild_chat_fixture;
pub mod house_management;
pub mod hunting_stamina;
pub mod hunting_xp;
pub mod mail_text_policy;
#[allow(dead_code)]
pub mod native_world_items;
#[allow(dead_code)]
pub mod native_world_read;
pub mod party_kill_xp;
pub mod player_report;
pub mod prey;
pub mod raid_schedule;
pub mod staff;
pub mod vip;
// End sorted module declarations.

#[cfg(test)]
#[test]
fn module_registration_block_is_sorted_and_complete() -> Result<(), &'static str> {
    fn module_name(line: &str) -> Option<&str> {
        let line = line.trim();
        let declaration = if let Some(rest) = line.strip_prefix("pub(") {
            rest.split_once(')')?.1.trim_start()
        } else {
            line.strip_prefix("pub ").unwrap_or(line)
        };
        declaration.strip_prefix("mod ").map(|name| {
            assert!(name.ends_with(';'), "one module declaration per line");
            name.trim_end_matches(';')
        })
    }

    // `core_modules.rs` is reached through `include!`, so rustfmt never formats
    // the modules it declares; they keep their own sorted block there. Outside
    // either block only `#[cfg(test)]` mounts may appear in `core_modules.rs`.
    for (source, test_mounts_outside) in [
        (include_str!("mod.rs"), false),
        (include_str!("core_modules.rs"), true),
    ] {
        let (before, rest) = source
            .split_once("// Keep sorted by module name; insert in order, never append.\n")
            .ok_or("module registration block header")?;
        let (block, after) = rest
            .split_once("// End sorted module declarations.\n")
            .ok_or("module registration block footer")?;
        let mut test_only = false;
        let mut in_attribute = false;
        for line in before.lines().chain(after.lines()) {
            let trimmed = line.trim();
            if in_attribute || trimmed.starts_with("#[") {
                test_only |= trimmed == "#[cfg(test)]";
                in_attribute = !trimmed.ends_with(']');
                continue;
            }
            if module_name(line).is_some() {
                assert!(
                    test_mounts_outside && test_only,
                    "module outside block: {line}"
                );
            }
            if !trimmed.is_empty() && !trimmed.starts_with("//") {
                test_only = false;
            }
        }
        let names: Vec<_> = block.lines().filter_map(module_name).collect();
        assert!(!names.is_empty(), "module registration block is empty");
        for pair in names.windows(2) {
            assert!(pair[0] < pair[1], "modules out of order: {pair:?}");
        }
    }
    Ok(())
}
