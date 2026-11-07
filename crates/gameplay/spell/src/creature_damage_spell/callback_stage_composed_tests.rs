use super::*;

#[test]
fn srx3_real_native_callback_stages_compile_for_existing_creature_and_player_combat()
-> Result<(), String> {
    use crate::content::native_monster_callbacks::{NativeCallbackList, qualify_native_callback};
    let mut world = [0x71; 16];
    world[6] = 0x70;
    world[8] = 0x80;
    let world = crate::foundation::WorldId::decode(&world).map_err(|error| format!("{error:?}"))?;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../content/spells.manifest.json");
    let native = crate::content::native_gameplay::retained_callback_native_fixture_at(world, &path);
    for (slug, attack_index, count) in [
        ("energy_pulse", 0, 1),
        ("foam_stalker", 4, 3),
        ("charging_outburst", 5, 1),
        ("eruption_of_destruction", 1, 1),
        ("gaz_haragoth", 10, 1),
        ("lady_tenebris", 4, 1),
        ("plagirath", 6, 1),
        ("rootthing_amber_shaper", 4, 2),
        ("rootthing_nutshell", 3, 2),
    ] {
        let creature = Ref {
            family: ProjectV2Family::Creature,
            key: format!("oteryn:creature.{slug}"),
            revision: "definition-r1".into(),
        };
        let parent =
            qualify_native_callback(&native, &creature, attack_index, NativeCallbackList::Attack)
                .map_err(|error| format!("{slug}: {error:?}"))?
                .ok_or("actual native parent missing")?;
        for index in 0..count {
            let stage = parent
                .stage(index)
                .map_err(|error| format!("{error:?}"))?
                .ok_or("actual closed stage missing")?;
            let compiled = ComposedSource::from_callback_stage(&native, &stage, None)
                .map_err(|error| format!("{slug} stage{index} cannot enter existing creature/player combat: {error:?}"))?;
            let primary = compiled
                .primary_source()
                .ok_or("source damage stage missing")?;
            assert_eq!(primary.minimum, stage.minimum());
            assert_eq!(primary.maximum, stage.maximum());
            assert_eq!(primary.element.as_deref(), Some(stage.element()));
            assert_eq!(primary.area.as_ref(), stage.area());
            assert_eq!(compiled.ability, parent.entry().ability);
        }
        assert!(
            parent
                .stage(count)
                .map_err(|error| format!("{error:?}"))?
                .is_none()
        );
    }
    Ok(())
}
