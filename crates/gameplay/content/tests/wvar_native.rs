use oteryn_gameplay_content::content::{
    native_gameplay::{NativeGameplayInput, decode},
    native_source_world_carrier, qualify_selected_native_gameplay_room,
};
use std::path::Path;

#[test]
fn wvar_mamma_loads_real_native_ai_and_rewards() -> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new()
        .name("wvar-mamma-native".into())
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
            let input =
                NativeGameplayInput::from_manifest(&root.join("content/spells.manifest.json"))
                    .map_err(|error| format!("{error:?}"))?;
            let world = oteryn_protocol_oteryn::WorldId::decode(&[
                1, 144, 0, 0, 0, 8, 112, 0, 128, 0, 0, 0, 0, 0, 0, 8,
            ])
            .map_err(|error| format!("{error:?}"))?;
            let room = qualify_selected_native_gameplay_room(world, &input)
                .map_err(|error| format!("{error:?}"))?;
            let compiled = room.compiled();
            let source = native_source_world_carrier::decode(
                &compiled.server_artifact,
                &compiled.client_artifact,
            )
            .map_err(|error| format!("{error:?}"))?;
            let decoded = decode(source.baseline_server).map_err(|error| format!("{error:?}"))?;
            let native = &decoded.state;
            let policy = native
                .policies()
                .policies()
                .iter()
                .find(|policy| policy.definition_key == "oteryn:creature.mamma_longlegs")
                .ok_or("Mamma native policy")?;
            assert_eq!(policy.display_name, "Mamma Longlegs");
            assert_eq!(policy.maximum_health, 1800);
            assert_eq!(policy.outfit_look_type, 208);
            assert_eq!(policy.preferred_distance, Some(1));
            let reward = native
                .reward_table()
                .row("oteryn:creature.mamma_longlegs")
                .map_err(|error| format!("{error:?}"))?;
            assert_eq!(reward.xp_amount.get(), 2200);
            assert!(reward.loot_table.entries.is_empty());
            assert!(reward.race.is_none());
            let record = native
                .creature_profiles()
                .records
                .iter()
                .find(|record| record.profile.target.key == "oteryn:creature.mamma_longlegs")
                .ok_or("Mamma loaded source")?;
            assert!(
                native.qualifies_current_creature_profile(native.source_digest(), &record.profile)
            );
            assert!(!native.qualifies_current_creature_profile([0; 32], &record.profile));
            assert!(record.attack_definitions.is_some());
            println!(
                "WVAR_NATIVE_SOURCE_EXECUTED mamma_hp=1800 xp=2200 outfit=208 empty_loot=true giant_spider_settlement={:?}",
                native.reward_table().row("oteryn:creature.giant_spider").map(|row| row.xp_amount.get())
            );
            Ok(())
        })?
        .join()
        .map_err(|_| "Mamma native test thread failed")??;
    Ok(())
}
