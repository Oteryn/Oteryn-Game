use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::raid_schedule::RaidWrite;
use crate::foundation::{ChannelRuntimeV1, ScopeOwnershipGeneration};
use crate::map::boot::BundleCollisionIndexExt;
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_gameplay_encounter_rules::raid::{RaidCatalogue, RaidPosition};
use oteryn_gameplay_entities::carrier::RaidSpawnOutcome;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

const RAID: &str = "raid:venore.orcsambackpack";
const CREATURE: &str = "oteryn:creature.orc_sam_backpack";

#[test]
fn orc_sam_backpack_raid_postgres_firing_replay_does_not_spawn_twice() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let run = || -> TestResult {
                crate::bestiary_postgres_harness::runtime()?.block_on(async {
                    let h = Harness::create_with_admission_seed(
                        admin,
                        "orc_sam_backpack_replay",
                        false,
                        false,
                    )
                    .await?;
                    let result = scenario(&h).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup
                })
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Orc Sam Backpack replay thread failed")?
        .map_err(Box::<dyn std::error::Error>::from)
}

async fn scenario(h: &Harness) -> TestResult {
    let catalogue = RaidCatalogue::embedded().map_err(debug)?;
    let raid = catalogue
        .active()
        .find(|raid| raid.key == RAID)
        .ok_or("active Orc Sam Backpack raid")?;
    let key = ProductionKey::new(RAID).map_err(debug)?;
    let firing = id(91);
    let initial = raid.select([7; 32], firing).map_err(debug)?;
    let area = *initial
        .areas
        .values()
        .next()
        .ok_or("source spawn rectangle")?;
    let from = (u16::try_from(area.from.x)?, u16::try_from(area.from.y)?);
    let to = (u16::try_from(area.to.x)?, u16::try_from(area.to.y)?);
    let floor = i8::try_from(-area.from.floor)?;
    let map = crate::map::spell_tiles::tests::movement_arena::rectangle(from, to, floor);
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activated) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            deps.admitted.world_id,
            None,
        );
    let active = controller.active().ok_or("active raid generation")?;
    let native = active.native_gameplay().ok_or("native raid gameplay")?;
    deps.room = activated.qualified_room().clone();
    let cells = map
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        cells.index()
    else {
        return Err("bundle collision index".into());
    };
    let bundle = index.map().overlay.base().digest();
    let selected = raid.select(bundle, firing).map_err(debug)?;
    let room = deps
        .room
        .bundle_view(
            cells.clone(),
            map.frame_binding_digest(deps.admitted.world_id),
            map.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: area.from.x,
                y: area.from.y,
                floor: area.from.floor,
            },
        )
        .ok_or("raid combat view")?;
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        1,
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (area.from.x, area.from.y, area.from.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        h.node.fact().node_id(),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        pin.clone(),
    )
    .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    deps.admitted.runtime_actor = Some(deps.actor);
    deps.runtime = tokio::sync::Mutex::new(runtime);
    deps.combat_room = Some(room);
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    owner.reset_epoch = 1;

    // The scheduler case proves how the firing is written. Start this test at
    // the same durable handoff so both lab turns exercise the physical owner.
    let world = owner.world_id.as_bytes().as_slice();
    let channel = owner.channel_id.as_bytes().as_slice();
    let now = i64::from(
        selected
            .definition
            .waves
            .first()
            .ok_or("source wave")?
            .delay_ms,
    );
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world).bind(channel).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(world).bind(channel).bind(pin.server_artifact_digest().as_slice())
        .bind(pin.client_artifact_digest().as_slice()).bind(pin.frame_binding_digest().as_slice())
        .execute(&h.pool).await?;
    let map_revision = format!(
        "sha256:{}",
        pin.map_revision_digest()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,$3,$4,$5,$6,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
        .bind(world).bind(bundle.as_slice()).bind(map_revision).bind(pin.server_artifact_digest().as_slice())
        .bind(pin.client_artifact_digest().as_slice()).bind(pin.frame_binding_digest().as_slice())
        .execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world)
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO game_world_raid_schedules VALUES(encode($1,'hex')::uuid,$2,2,1,1,2,60000,0,NULL,false)")
        .bind(world).bind(key.as_str()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_firings VALUES(encode($1,'hex')::uuid,$2,1,encode($3,'hex')::uuid,1,0,$4,$5,1)")
        .bind(world).bind(key.as_str()).bind(firing.as_slice()).bind(now).bind(bundle.as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_targets VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(firing.as_slice()).bind(channel).execute(&h.pool).await?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
    let step = h
        .root
        .advance_raid_run(
            &h.node,
            firing,
            owner.channel_id,
            generation,
            selected.definition.clone(),
            now,
        )
        .await?
        .ok_or("committed raid wave")?;
    let placed = owner
        .apply_raid_wave(&step, &selected, bundle)
        .await
        .map_err(|_| "raid placement")?;
    assert_eq!(placed.len(), 1);
    assert!(placed[0].placed);
    let actor = {
        let runtime = deps.runtime.lock().await;
        let live = runtime.raid_creatures(firing).map_err(debug)?;
        assert_eq!(live.len(), 1);
        assert!(live[0].alive);
        assert!(!live[0].boss);
        let RaidSpawnOutcome::Placed(actor) = live[0].outcome else {
            return Err("expected placed Orc Sam Backpack".into());
        };
        let at = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        assert!(area.contains(RaidPosition {
            x: at.x,
            y: at.y,
            floor: at.floor
        }));
        assert_eq!(
            runtime
                .companion_snapshot(actor)
                .map_err(debug)?
                .state
                .policy
                .definition_key,
            CREATURE
        );
        actor
    };
    let replay = h
        .root
        .advance_raid_run(
            &h.node,
            firing,
            owner.channel_id,
            generation,
            selected.definition.clone(),
            now,
        )
        .await?
        .ok_or("replayed pending raid wave")?;
    assert!(replay.replayed);
    assert_eq!(step.identity, replay.identity);
    assert_eq!(
        owner
            .apply_raid_wave(&replay, &selected, bundle)
            .await
            .map_err(|_| "raid replay")?,
        placed
    );
    {
        let runtime = deps.runtime.lock().await;
        let live = runtime.raid_creatures(firing).map_err(debug)?;
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].outcome, RaidSpawnOutcome::Placed(actor));
    }
    assert_eq!(
        h.root
            .acknowledge_raid_effect(&h.node, step.identity, placed, now)
            .await?,
        RaidWrite::Applied
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_world_raid_creature_placements WHERE placed"
        )
        .fetch_one(&h.pool)
        .await?,
        1
    );
    settle_source_loot(h, &owner, deps.admitted, actor).await
}

async fn settle_source_loot(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    target: crate::foundation::ExactActorRef,
) -> TestResult {
    use crate::foundation::runtime_actor_spell_types::{
        OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
    };
    use crate::foundation::{CharacterId, CharacterLease, CommandId, CommandRef};
    use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_batch_kills};

    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native rewards")?;
    let actor = admitted.runtime_actor.ok_or("raid participant")?;
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(admitted.game_session_id)
            .await?
            .0;
    let fence =
        crate::gameplay_transport::item_fence_of(current).ok_or("current Character fence")?;
    admitted.item_fence = Some(fence);
    super::item_look_fixtures::seed_main_backpack(h, &[]).await?;
    super::srx3_activity_source_fixture::prepare_player(h, owner, &mut admitted, 1999).await?;
    let character = CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?;
    let now = owner.owner_now();
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .bind_attacker_lease(
                actor,
                admitted.game_session_id,
                CharacterLease::new(character, fence.character_lease_generation).map_err(debug)?,
            )
            .map_err(debug)?;
        let snapshot = runtime.companion_snapshot(target).map_err(debug)?;
        let batch = OwnerCombatBatch {
            caster: actor,
            attacker: character,
            current_lease_generation: fence.character_lease_generation,
            command: CommandRef::new(admitted.game_session_id, CommandId::new(91).map_err(debug)?),
            occurrence: SpellOccurrenceBinding {
                id: "orc-sam-backpack-raid-lethal".into(),
                revisions: ["r1"; 5].map(str::to_owned),
            },
            binding: b"orc-sam-backpack-raid-lethal".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1_000,
            effects: vec![OwnerCombatEffect {
                target,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(target).map_err(debug)?,
                    magnitude: snapshot.maximum_health,
                },
            }],
            deferred: None,
        };
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(receipt.applied);
        assert!(receipt.effects.iter().any(|effect| effect.target == target
            && effect.health.is_some_and(|health| health.health_after == 0)));
        let attack = owner.states.attack.lock().await;
        record_batch_kills(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: attack.kills(),
                rewards: native.reward_table(),
                charms: None,
            },
            &receipt,
            now.get() / 1_000,
        );
    }
    super::srx3_native_source_cast_fixtures::drain_source_death(owner, target, now).await?;
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    assert_eq!(sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM game_item_corpse_container_entries e JOIN game_item_instances child ON child.item_instance_id=e.item_instance_id JOIN game_item_instances corpse ON corpse.item_instance_id=e.parent_item_instance_id WHERE child.definition_production_key='oteryn:item.tibia.i3244' AND child.quantity=1 AND corpse.definition_production_key='oteryn:item.tibia.i5966'"
    ).fetch_one(&h.pool).await?, 1);
    Ok(())
}
