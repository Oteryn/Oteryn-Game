//! Crystal 201189f8 data-global/scripts/quests/dark_trails/creaturescripts_think_the_ravager_heal.lua:3-13.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::content::native_gameplay::{
    NativeTileSelfHeal,
    test_support::{activated_loop_monster_nearby_heal, activated_loop_monster_tile_heal},
};
use crate::foundation::ChannelRuntimeV1;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[derive(Clone, Copy)]
pub(super) enum HealFixture {
    Tile,
    Nearby { present: bool },
}

pub(super) async fn exercise(h: &Harness, fixture: HealFixture) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let rat = "oteryn:creature.rat";
    let (controller, pin) = match fixture {
        HealFixture::Tile => activated_loop_monster_tile_heal(world, NativeTileSelfHeal {
        // The native lab's current stack at (2, 11, -7) has source item 3657.
        required_item_id: 3657,
        below_health_ppm: 999_900,
        minimum: 6,
        maximum: 7,
        speech: "The tile restores vitality.".into(),
        source: "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dark_trails/creaturescripts_think_the_ravager_heal.lua:3-13".into(),
        }),
        HealFixture::Nearby { .. } => activated_loop_monster_nearby_heal(world, rat),
    };
    let active = controller.active().ok_or("active content")?;
    let native = active.native_gameplay().ok_or("admitted native content")?;
    let mut deps = Dependencies::new()?;
    deps.room = pin.qualified_room().clone();
    deps.combat_room = Some(deps.room.clone());
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        deps.room.map_revision_digest(),
        (2, 11, -7),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        deps.admitted.channel_id,
        crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        channel_pin,
    )
    .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let healer = match fixture {
        HealFixture::Tile => rat,
        HealFixture::Nearby { .. } => "oteryn:creature.fugue",
    };
    let health = |key| -> TestResult<u64> {
        Ok(u64::try_from(
            runtime.companion_policy(key).map_err(debug)?.maximum_health,
        )?)
    };
    let mut placements = vec![((2, 11, -7), 60_000, healer.into(), health(healer)?, false)];
    if matches!(
        fixture,
        HealFixture::Tile | HealFixture::Nearby { present: true }
    ) {
        placements.push(((2, 12, -7), 60_000, rat.into(), health(rat)?, false));
    }
    let count = placements.len();
    runtime
        .begin_activation_spawn_sources(vec![(placements, 5_000)])
        .map_err(debug)?;
    assert_eq!(
        runtime
            .realize_activation_spawn_window(0)
            .map_err(debug)?
            .realized,
        count
    );
    let actors = runtime.spawn_point_creatures();
    let on = actors[0].2.ok_or("healer")?;
    let off = actors.get(1).and_then(|entry| entry.2);
    for actor in std::iter::once(on).chain(off) {
        runtime.set_test_creature_health(actor, 5).map_err(debug)?;
    }
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (deps.room.movement_cells()).into();
    owner.clock_origin -= std::time::Duration::from_secs(10);
    owner.drain_monster_melee().await;
    let runtime = deps.runtime.lock().await;
    let healed = runtime.live_creature_health(on).ok_or("wounded creature")?;
    match fixture {
        HealFixture::Tile => assert!((11..=12).contains(&healed)),
        HealFixture::Nearby { present: true } => assert_eq!(healed, 10_005),
        HealFixture::Nearby { present: false } => assert_eq!(healed, 5),
    }
    if let Some(off) = off {
        assert_eq!(runtime.live_creature_health(off), Some(5));
    }
    Ok(())
}
