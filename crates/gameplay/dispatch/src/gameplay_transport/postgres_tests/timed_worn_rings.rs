use super::fixture::{Harness, debug, id, run};
use super::house_transfer_fixtures::{RecoverySource, prepare_owned_session};
use super::{Dependencies, TestResult};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{AuthenticatedTransportRefV1, ChannelContentPin, ChannelId, WorldId};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{
    ControlLossResult, FirstEntryOutcome, FreshAdmissionAuthority, ResumeAttempt, UseCommand,
};
use crate::gameplay_transport::equipment_move::{move_equipment, observe_character_equipment};
use crate::gameplay_transport::item_move::{EquipmentMove, EquipmentMoveResult};
use crate::gameplay_transport::item_ref_admission::definition_ref;
use crate::gameplay_transport::item_view::ItemKey;
use crate::map::boot::tests::{floor_items, floor_pins, teleport_bundle};
use oteryn_protocol_oteryn::item_view::EquipmentSlot;
use std::time::Duration;

#[test]
fn stealth_ring_equip_hides_until_expiry_postgres() -> TestResult {
    run_ring("stealth_ring", 3049, 3086, 600_000)
}

#[test]
fn life_ring_regen_pauses_and_resumes_postgres() -> TestResult {
    run_ring("life_ring", 3052, 3089, 1_200_000)
}

fn run_ring(name: &'static str, base: u32, worn: u32, full_ms: i64) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |admin| {
                let h = Harness::create_with_admission_seed(admin, name, true, false).await?;
                eprintln!("POSTGRES_STARTED {name}");
                let result = exercise_ring(&h, base, worn, full_ms).await;
                let cleanup = h.cleanup().await;
                result.and(cleanup)
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "worn ring test thread")??;
    Ok(())
}

async fn exercise_ring(h: &Harness, base: u32, worn: u32, full_ms: i64) -> TestResult {
    let recovery = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let active = controller.active().ok_or("active generation")?;
    let index = active
        .native_gameplay()
        .ok_or("native gameplay")?
        .item_index()
        .ok_or("item index")?;
    let channel = ChannelId::decode(&id(43)).map_err(debug)?;
    let (map_bytes, map_pins) = teleport_bundle();
    let map = crate::map::boot::boot(
        &map_bytes,
        &floor_pins(map_pins),
        world,
        channel,
        floor_items,
    )
    .map_err(debug)?;
    let cells = map
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let frame = map.frame_binding_digest(world);
    let deps = Dependencies::with_content(
        ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            frame,
            map.map_revision_digest(),
            (6, 1, 8),
        ),
        pin.qualified_room().clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    owner.evidence = &recovery.evidence;
    let actor = deps.actor;
    let session = deps.admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "fence")?
        .ok_or("current fence")?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = Some(fence.item_fence());
    owner.states.command_ingress.admit(session);

    let base_key = format!("oteryn:item.tibia.i{base}");
    seed_custody(
        h,
        &[
            (
                "oteryn:item.tibia.i2854",
                index
                    .revision("oteryn:item.tibia.i2854")
                    .ok_or("backpack revision")?,
            ),
            (&base_key, index.revision(&base_key).ok_or("ring revision")?),
        ],
    )
    .await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(active.identity().server_artifact_digest().as_slice())
        .bind(active.identity().client_artifact_digest().as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;

    deps.initialize(&owner).await;
    {
        let mut runtime = owner.runtime.lock().await;
        assert_eq!(
            crate::gameplay_transport::actor_spell::qualified_protection_zone(
                &mut runtime,
                owner.movement_cells(),
                actor,
            ),
            Some(false),
            "the fixture entry is outside protection zones"
        );
        let mut states = owner.states.spell_states.lock().await;
        let state = states
            .get_mut(&runtime, actor, session)
            .ok_or("actor spell state")?;
        state.restore_saved_vitals(25, 0);
    }
    let invisible = base == 3049;
    let starting = ring_effects(&owner, &deps).await?;
    assert_eq!(starting, (false, 0, 25, 0));
    let item = ItemKey::Instance(id(90));
    let command = |admitted: super::AdmittedSession, command_id| UseCommand {
        game_session_id: admitted.game_session_id,
        command_id,
        item_fence: admitted.item_fence,
    };
    let equip = EquipmentMove::Equip {
        item,
        to: EquipmentSlot::Ring,
    };
    let unequip = EquipmentMove::Unequip {
        item,
        from: EquipmentSlot::Ring,
    };
    let moved = move_equipment(&owner, actor, command(admitted, 1), equip).await;
    if moved != EquipmentMoveResult::Committed {
        return Err(format!("equip {base} through the durable equipment policy: {moved:?}").into());
    }
    let worn_key = format!("oteryn:item.tibia.i{worn}");
    let worn_ref = definition_ref(
        index,
        &TypedDefinitionRef {
            family: "Item".into(),
            revision_ref: index.revision(&worn_key).ok_or("worn revision")?.into(),
            production_key: worn_key,
        },
    )
    .ok_or("worn definition ref")?;
    let view = observe_character_equipment(&owner, session)
        .await
        .ok_or("worn view")?;
    assert_eq!(view.len(), 1);
    assert_eq!(
        (view[0].0, view[0].1.key, view[0].1.item_definition_ref),
        (EquipmentSlot::Ring, item, worn_ref)
    );
    assert_eq!(
        ring_effects(&owner, &deps).await?,
        (invisible, usize::from(!invisible), 25, 0)
    );

    tick_regen_pulse(&mut owner, &deps).await;
    owner.clock_origin -= Duration::from_secs(54);
    owner.tick_timed_items().await;
    assert!((full_ms - 62_000..=full_ms - 60_000).contains(&budget(h).await?));
    let (_, _, health, mana) = ring_effects(&owner, &deps).await?;
    // The hungry monk also gains one mana on the first six native ticks.
    assert_eq!((health, mana), if invisible { (25, 1) } else { (27, 9) });

    assert_eq!(
        move_equipment(&owner, actor, command(admitted, 2), unequip).await,
        EquipmentMoveResult::Committed
    );
    let paused = budget(h).await?;
    assert_eq!(ring_effects(&owner, &deps).await?, (false, 0, health, mana));
    assert_eq!(
        owner.lose_control(admitted, Duration::ZERO, None).await,
        ControlLossResult::Recorded
    );
    owner.clock_origin -= Duration::from_secs(30);
    owner.tick_timed_items().await;
    let token = RecoverySource::token(2)?;
    let resumed = owner
        .resume(ResumeAttempt {
            game_session_id: session,
            recovery_material: token.as_bytes(),
            last_applied_server_sequence: admitted.continuity.server_sequence,
            transport: AuthenticatedTransportRefV1::decode(&[2; 16]).map_err(debug)?,
            supported_capabilities: &[],
        })
        .await
        .map_err(debug)?;
    assert_eq!(resumed.continuity.connection_generation, 2);
    assert_eq!(budget(h).await?, paused);
    let _ = owner.tick_spell_periodic(actor, session).await;
    assert_eq!(ring_effects(&owner, &deps).await?, (false, 0, health, mana));

    assert_eq!(
        move_equipment(&owner, actor, command(resumed, 3), equip).await,
        EquipmentMoveResult::Committed
    );
    assert_eq!(
        budget(h).await?,
        paused,
        "re-equip retains the partial budget"
    );
    tick_regen_pulse(&mut owner, &deps).await;
    owner.clock_origin -= Duration::from_secs(54);
    owner.tick_timed_items().await;
    let (hidden, regen, health, mana) = ring_effects(&owner, &deps).await?;
    assert_eq!(
        (hidden, regen, health, mana),
        if invisible {
            (true, 0, 26, 3)
        } else {
            (false, 1, 30, 19)
        }
    );
    let remaining = budget(h).await?;
    assert!((paused - 62_000..=paused - 60_000).contains(&remaining));

    owner.clock_origin -= Duration::from_millis(u64::try_from(remaining - 2_000)?);
    owner.tick_timed_items().await;
    assert_eq!(ring_effects(&owner, &deps).await?.0, invisible);
    assert!((1..=2_000).contains(&budget(h).await?));
    owner.clock_origin -= Duration::from_secs(3);
    owner.tick_timed_items().await;
    assert_eq!(ring_effects(&owner, &deps).await?, (false, 0, health, mana));
    assert!(
        observe_character_equipment(&owner, session)
            .await
            .ok_or("expired view")?
            .is_empty()
    );
    let lifecycle: i16 = sqlx::query_scalar(
        "SELECT lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(90).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(lifecycle, 2, "expiry consumes the ring");
    Ok(())
}

async fn tick_regen_pulse(owner: &mut ComposedFreshAdmission<'_, '_, '_>, deps: &Dependencies) {
    for _ in 0..6 {
        owner.clock_origin -= Duration::from_secs(1);
        let _ = owner
            .tick_spell_periodic(deps.actor, deps.admitted.game_session_id)
            .await;
    }
}

async fn ring_effects(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
) -> TestResult<(bool, usize, u32, u32)> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let state = states
        .get(&runtime, deps.actor, deps.admitted.game_session_id)
        .ok_or("actor spell state")?;
    let vitals = state.vitals();
    Ok((
        state
            .owned_invisible_at(owner.owner_now().get())
            .map_err(debug)?,
        state.equipped_regeneration().len(),
        vitals.health,
        vitals.mana,
    ))
}

async fn budget(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT remaining_ms FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(90).as_slice()).fetch_one(&h.pool).await?)
}

async fn seed_custody(h: &Harness, items: &[(&str, &str); 2]) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (seed, (key, revision)) in [70, 90].into_iter().zip(items) {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
            .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(*key).bind(*revision).bind(id(seed + 1).as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(42).as_slice()).bind(id(72).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(90).as_slice()).bind(id(42).as_slice()).bind(id(92).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_build_state(character_id,vocation,magic_level,mana_spent,fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries,committed_character_revision,last_build_occurrence_id) VALUES(encode($1,'hex')::uuid,'monk',10,0,10,0,10,0,10,0,10,0,10,0,10,0,10,0,2,encode($2,'hex')::uuid) ON CONFLICT(character_id) DO UPDATE SET vocation='monk'")
        .bind(id(41).as_slice()).bind(id(30).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
