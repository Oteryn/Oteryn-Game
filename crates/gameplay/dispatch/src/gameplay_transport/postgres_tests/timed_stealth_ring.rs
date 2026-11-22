//! Crystal 201189f8 items.xml:6917-6926,7326-7337 through the durable
//! equipment move and the timed equipment clock.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelContentPin, ExactActorRef, GameSessionId};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::equipment_move::{move_equipment, observe_character_equipment};
use crate::gameplay_transport::item_move::{EquipmentMove, EquipmentMoveResult};
use crate::gameplay_transport::item_ref_admission::definition_ref;
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::item_view::EquipmentSlot;
use std::time::Duration;

const STEALTH_BASE: &str = "oteryn:item.tibia.i3049";
const STEALTH_ACTIVE: &str = "oteryn:item.tibia.i3086";
const FULL_DURATION_MS: i64 = 600_000;
const RING: ItemKey = ItemKey::Instance([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

#[derive(Clone, Copy)]
enum End {
    Expiry,
    Unequip,
}

#[test]
fn i05_stealth_ring_equip_projects_invisibility_until_expiry() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, End::Expiry)
}

#[test]
fn i05_stealth_ring_unequip_clears_invisibility_and_preserves_remaining_time() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, End::Unequip)
}

fn run(admin: String, end: End) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            runtime
                .block_on(Box::pin(scenario(admin, end)))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "stealth ring scenario thread")??;
    Ok(())
}

async fn remaining(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT remaining_ms FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(90).as_slice()).fetch_one(&h.pool).await?)
}

async fn invisible(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<bool> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    states
        .get(&runtime, actor, session)
        .ok_or("stealth ring actor state")?
        .owned_invisible_at(owner.owner_now().get())
        .map_err(debug)
}

async fn scenario(admin: String, end: End) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "timed_stealth_ring", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let active = controller.active().ok_or("active")?;
    let native = active.native_gameplay().ok_or("native")?;
    let index = native.item_index().ok_or("index")?;
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        pin.qualified_room().map_revision_digest(),
        (2, 0, 7),
    );
    let deps = Dependencies::with_content(
        channel_pin,
        crate::content::qualify_native_entry_room(world).map_err(debug)?,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    let session = deps.admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "fence")?
        .ok_or("current")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (seed, key) in [(70, "oteryn:item.tibia.i2854"), (90, STEALTH_BASE)] {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
            .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(key).bind(index.revision(key).ok_or("revision")?).bind(id(seed+1).as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(42).as_slice()).bind(id(72).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(90).as_slice()).bind(id(42).as_slice()).bind(id(92).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(active.identity().server_artifact_digest().as_slice()).bind(active.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        states
            .initialize(
                &runtime,
                deps.actor,
                session,
                crate::spell::cast::CharacterCastFacts {
                    vocation: crate::spell::Vocation::Knight,
                    level: 50,
                    magic_level: 10,
                    max_health: 1000,
                    max_mana: 101,
                    max_soul: 100,
                },
                (0, 0),
                owner.owner_now(),
            )
            .ok_or("initialize")?;
    }
    let actor = deps.actor;
    let ring = ItemKey::Instance(id(90));
    let equip = EquipmentMove::Equip {
        item: ring,
        to: EquipmentSlot::Ring,
    };
    let command = UseCommand {
        game_session_id: session,
        command_id: 1,
        item_fence: Some(fence.item_fence()),
    };
    assert!(!invisible(&owner, actor, session).await?);
    assert_eq!(
        move_equipment(&owner, actor, command, equip).await,
        EquipmentMoveResult::Committed
    );
    let worn = observe_character_equipment(&owner, session)
        .await
        .ok_or("worn ring view")?;
    let active_ref = definition_ref(
        index,
        &TypedDefinitionRef {
            family: "Item".into(),
            production_key: STEALTH_ACTIVE.into(),
            revision_ref: index
                .revision(STEALTH_ACTIVE)
                .ok_or("active revision")?
                .into(),
        },
    )
    .ok_or("active ring definition")?;
    assert_eq!(worn.len(), 1);
    assert_eq!(worn[0].0, EquipmentSlot::Ring);
    assert_eq!(worn[0].1.item_definition_ref, active_ref);
    assert!(invisible(&owner, actor, session).await?);

    owner.clock_origin -= Duration::from_secs(61);
    owner.tick_timed_items().await;
    let worn_remaining = remaining(&h).await?;
    assert!((FULL_DURATION_MS - 70_000..=FULL_DURATION_MS - 61_000).contains(&worn_remaining));
    assert!(invisible(&owner, actor, session).await?);

    let until_expiry = match end {
        End::Expiry => worn_remaining,
        End::Unequip => {
            assert_eq!(
                move_equipment(
                    &owner,
                    actor,
                    UseCommand {
                        command_id: 2,
                        ..command
                    },
                    EquipmentMove::Unequip {
                        item: ring,
                        from: EquipmentSlot::Ring
                    },
                )
                .await,
                EquipmentMoveResult::Committed
            );
            assert!(!invisible(&owner, actor, session).await?);
            let (base, paused): (String, i64) = sqlx::query_as(
                "SELECT i.definition_production_key,t.remaining_ms FROM game_item_container_entries e
                 JOIN game_item_instances i USING(item_instance_id)
                 JOIN game_item_timed_states t USING(item_instance_id)
                 WHERE e.item_instance_id=encode($1,'hex')::uuid",
            )
            .bind(id(90).as_slice())
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(base, STEALTH_BASE);
            assert!((worn_remaining - 10_000..=worn_remaining).contains(&paused));
            owner.clock_origin -= Duration::from_secs(600);
            owner.tick_timed_items().await;
            assert_eq!(remaining(&h).await?, paused);
            assert!(!invisible(&owner, actor, session).await?);

            assert_eq!(
                move_equipment(
                    &owner,
                    actor,
                    UseCommand {
                        command_id: 3,
                        ..command
                    },
                    equip
                )
                .await,
                EquipmentMoveResult::Committed
            );
            assert!(invisible(&owner, actor, session).await?);
            paused
        }
    };

    owner.clock_origin -= Duration::from_millis(u64::try_from(until_expiry)? + 1);
    owner.tick_timed_items().await;
    let lifecycle: i16 = sqlx::query_scalar(
        "SELECT lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(90).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(lifecycle, 2, "Crystal decayTo=0 removes the spent item");
    assert!(
        observe_character_equipment(&owner, session)
            .await
            .ok_or("expired ring view")?
            .is_empty()
    );
    assert!(!invisible(&owner, actor, session).await?);
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
