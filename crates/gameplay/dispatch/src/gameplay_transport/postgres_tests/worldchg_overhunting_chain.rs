//! Counter receipt -> real save projection -> trap callback -> native Benevola -> save.
use super::TestResult;
use super::fixture::{Harness, debug};
use crate::content::NativeEntryMovementCells;
use crate::durability::world_event::changes::counters::{self, CounterKind, CounterState};
use crate::foundation::{ChannelRuntimeV1, WorldId};
use crate::gameplay_transport::ComposedFreshAdmission;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[test]
fn worldchg_overhunting_kill_save_trap_benevola_save_deer_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("worldchg-overhunting-chain", || {
        super::worldchg_benevola::scenario_with_chain(true)
    })
}

pub(super) async fn save(h: &Harness, world: WorldId, epoch: u64) -> TestResult {
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind([7u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("UPDATE game_world_reset_active SET reset_epoch=$2::text::numeric WHERE world_id=encode($1,'hex')::uuid")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&h.pool).await?;
    h.root
        .world_changes(world, epoch)
        .await?
        .ok_or("saved day")?;
    Ok(())
}

pub(super) async fn hunt_and_save(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    let world = owner.world_id;
    let kind = CounterKind::Overhunting;
    let bindings = counters::bindings()?;
    // Fixture starts one kill below departure; the final kill uses the durable
    // native damage commit, captured death queue and durable reward drain.
    let mut state = CounterState::new(kind, 1, false);
    state.counts[0] = bindings.parameters.deer_kills - 1;
    sqlx::query("UPDATE game_world_change_counters SET state=$2 WHERE world_id=encode($1,'hex')::uuid AND change_key='overhunting'")
        .bind(world.as_bytes().as_slice()).bind(serde_json::to_value(state)?).execute(&h.pool).await?;
    owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native policies")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    owner.admit_progression_session(admitted).await;
    // Keep the native kill helper's adjacent target clear of Benevola's fixed tile.
    super::autowalk::place(
        owner,
        *admitted,
        crate::foundation::MovementLocalPosition {
            x: 32594,
            y: 31745,
            floor: 7,
        },
    )
    .await?;
    for (creature, epoch, deaths) in [
        ("oteryn:creature.white_deer", 1u64, 1u32),
        ("oteryn:creature.starving_wolf", 2, 90),
    ] {
        for death in 0..deaths {
            super::qengine_kill_stories::death(
                owner,
                admitted,
                creature,
                20_000 + epoch * 100 + u64::from(death),
            )
            .await?;
        }
        let day = h
            .root
            .world_changes(world, epoch)
            .await?
            .ok_or("current day")?;
        assert_eq!(
            day.counter_stages[kind.index()],
            if epoch == 1 { 1 } else { 2 }
        );
        if epoch == 1 {
            save(h, world, 2).await?;
            owner.reset_epoch = 2;
        }
    }
    super::autowalk::place(
        owner,
        *admitted,
        crate::foundation::MovementLocalPosition {
            x: 32595,
            y: 31745,
            floor: 7,
        },
    )
    .await?;
    Ok(())
}

pub(super) fn capture(
    runtime: &mut ChannelRuntimeV1,
    fallback: &NativeEntryMovementCells,
) -> TestResult {
    use crate::map::boot::{BootPins, boot};
    use crate::map::overlay::TilePos;
    let (bytes, pins) = super::creature_push::bundle(Some(12368))?;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let world = boot(
        &bytes,
        &BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: TilePos {
                x: 2,
                y: 2,
                floor: -7,
            },
        },
        runtime.binding().world_id(),
        runtime.binding().channel_id().ok_or("channel scope")?,
        |key| {
            Some(crate::map::facts::ItemDefinition {
                reference: std::num::NonZeroU32::new(match key {
                    "oteryn:item.tibia.i3944" => 4,
                    "oteryn:item.tibia.i3945" => 5,
                    "oteryn:item.tibia.i12368" => 6,
                    "oteryn:item.tibia.i12369" => 7,
                    _ => return None,
                })?,
                solid: Some(false),
                blocks_projectile: false,
                pickupable: false,
                door: false,
            })
        },
    )
    .map_err(debug)?;
    let cells = world.movement_cells(fallback).map_err(debug)?;
    let at = crate::foundation::MovementLocalPosition {
        x: 4,
        y: 2,
        floor: 7,
    };
    let creature = "oteryn:creature.starving_wolf";
    let policy = runtime.companion_policy(creature).map_err(debug)?;
    let wolf = runtime
        .admit_encounter_actor(
            creature,
            policy.maximum_health,
            crate::foundation::MovementLocalPosition { x: 3, ..at },
        )
        .map_err(debug)?;
    runtime
        .install_creature_policy(wolf, creature)
        .map_err(debug)?;
    {
        let mut movement = runtime.borrow_movement_position();
        let before = movement.read(wolf).map_err(debug)?;
        movement.commit_cardinal(before, at).map_err(debug)?;
    }
    crate::movement::monster_map::apply_committed_step_in(runtime, &cells, wolf, at);
    assert!(!runtime.contains_live_creature(wolf));
    let tile = crate::map::view::compose_tile(
        world.overlay(),
        world.facts(),
        TilePos {
            x: 4,
            y: 2,
            floor: -7,
        },
    )
    .map_err(debug)?
    .ok_or("trap tile")?;
    assert!(
        tile.entries
            .iter()
            .any(|entry| entry.facts.appearance_id == 12369)
    );
    // Carried custody is a fixture boundary after this verified ground callback.
    // Benevola observes and debits real PostgreSQL custody in the caller.
    Ok(())
}
