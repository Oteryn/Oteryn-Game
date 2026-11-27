//! P08 Proficiency Catalysts through the production use-with owner: one unit burns with one
//! `catalyst` receipt, a replay burns nothing, and a non-weapon target spends nothing.
use super::fixture::{Harness, debug, id, run};
use super::use_holder_fixtures::{assert_holder_refusal, contend_use, holder_timeout_delay, send_use};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_proficiency::ProficiencyDefinitions;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, UseCommand,
    serve_admitted,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::item_view::ItemKey;
use crate::gameplay_transport::weapon_proficiency::ProductionProficiencyCatalogue;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::{self as wire, MessageType, world_object};
use std::{sync::Arc, time::Duration};
use tokio::io::AsyncWriteExt;

const SWORD: &str = "oteryn:item.tibia.i3264";
const CATALYST: &str = "oteryn:item.tibia.i51588";
const GREATER_CATALYST: &str = "oteryn:item.tibia.i51589";
const GOLD: &str = "oteryn:item.tibia.i3031";
const BAG: u8 = 160;
const UNITS: u8 = 162;
const WEAPON: u8 = 163;
const COIN: u8 = 164;

#[test]
fn proficiency_catalyst_burns_one_unit_with_one_receipt() -> TestResult {
    on_stack("proficiency_catalyst", None)
}

#[test]
fn proficiency_catalyst_use_retries_busy_holder_on_wire() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    on_stack("catalyst_busy", Some(Duration::from_millis(120)))
}

#[test]
fn proficiency_catalyst_use_holder_timeout_refuses_and_keeps_session_open() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    on_stack("catalyst_timeout", Some(holder_timeout_delay()))
}

fn on_stack(name: &'static str, hold: Option<Duration>) -> TestResult {
    std::thread::Builder::new()
        .name(name.into())
        .stack_size(64 << 20)
        .spawn(move || scenario(name, hold).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "proficiency catalyst scenario panicked")?
        .map_err(Into::into)
}

fn scenario(name: &str, hold: Option<Duration>) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, name, false, false)
                .await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("active")?;
        let room = pin.qualified_room().clone();
        let frame_digest = pin.frame_binding().digest();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let catalogue = ProductionProficiencyCatalogue::embedded().ok_or("catalogue")?;
        let cap = u64::from(
            catalogue
                .resolve_weapon(SWORD)
                .ok_or("sword definition")?
                .shape
                .track_shape()
                .final_progress(),
        );
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
        owner.proficiency_catalogue = Some(Arc::clone(&catalogue));
        deps.initialize(&owner).await;
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        let binding = owner.progression_binding(session).ok_or("progression")?;
        let definitions =
            catalogue.for_content(&binding.context.content) as Arc<dyn ProficiencyDefinitions>;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(frame_digest.as_slice()).execute(&harness.pool).await?;
        let catalyst_key = if hold.is_some() { GREATER_CATALYST } else { CATALYST };
        seed_bag(&harness, catalyst_key).await?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?
            .item_fence();
        let command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(fence),
        };
        let pool = &harness.pool;
        let quantity = |seed: u8| async move {
            sqlx::query_scalar::<_, i64>(
                "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
            )
            .bind(id(seed).as_slice())
            .fetch_one(pool)
            .await
        };
        let counts = || async {
            sqlx::query_as::<_, (i64, i64)>("SELECT (SELECT count(*) FROM game_character_proficiency_receipts WHERE cause='catalyst'),(SELECT count(*) FROM game_item_use_records WHERE cause_kind=494)")
                .fetch_one(pool)
                .await
        };
        if let Some(delay) = hold {
            admitted.item_fence = Some(fence);
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[4, 6, 7, world_object::CAPABILITY_ITEM_USE_V1, oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1],
            ).ok_or("targeted USE capabilities")?;
            let index = generation.native_gameplay().ok_or("native gameplay")?
                .item_index().ok_or("item index")?;
            let catalyst = index.definition_ref("Item", catalyst_key, "definition-r1").ok_or("catalyst definition")?;
            let weapon = index.definition_ref("Item", SWORD, "definition-r1").ok_or("weapon definition")?;
            catalyst_on_wire(&owner, admitted, &harness, catalyst, weapon, delay).await?;
            assert_eq!(quantity(UNITS).await?, 1);
            assert_eq!(counts().await?, (1, 1));
            eprintln!("PROFICIENCY_CATALYST_POSTGRES_CASE_COMPLETED {name}");
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        let use_with = async |command_id, target| {
            FreshAdmissionAuthority::use_proficiency_catalyst(
                &owner,
                actor,
                command(command_id),
                ItemKey::Instance(id(UNITS)),
                ItemKey::Instance(id(target)),
            )
            .await
            .expect("known catalyst outcome")
            .map(|outcome| outcome.disposition)
        };

        // A coin is not a weapon with a proficiency: nothing is spent or written.
        assert_eq!(use_with(1, COIN).await, Some(UseDisposition::NotPossible));
        assert_eq!(quantity(UNITS).await?, 2);
        assert_eq!(counts().await?, (0, 0));

        // One unit burns and the sword's track gains 25,000 (TibiaWiki; Crystal
        // proficiency_catalysts.lua:3) in the same transaction as the burn.
        assert_eq!(use_with(2, WEAPON).await, Some(UseDisposition::Committed));
        assert_eq!(quantity(UNITS).await?, 1);
        assert_eq!(counts().await?, (1, 1));
        let paired: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_use_records u JOIN game_character_proficiency_receipts r ON r.proficiency_occurrence_id=u.transaction_id JOIN game_character_proficiency_receipt_lines l USING(proficiency_occurrence_id) WHERE u.cause_kind=494 AND u.shape=1 AND r.cause='catalyst' AND l.progress_after-l.progress_before=$1 AND u.source_definition_production_key=$2")
            .bind(i64::try_from(cap.min(25_000))?)
            .bind(CATALYST)
            .fetch_one(pool)
            .await?;
        assert_eq!(paired, 1);

        // The same command again answers from its receipt without a second burn.
        assert_eq!(use_with(2, WEAPON).await, Some(UseDisposition::Committed));
        assert_eq!(quantity(UNITS).await?, 1);
        assert_eq!(counts().await?, (1, 1));

        let character = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?
            .character_id;
        let (tracks, _) = harness
            .root
            .read_character_proficiency_with_definitions(&authority, character, None, definitions)
            .await
            .map_err(debug)?;
        let stored = tracks
            .iter()
            .find(|track| track.state.item_key() == SWORD)
            .ok_or("durable track")?;
        assert_eq!(stored.state.progress(), cap.min(25_000));
        let projected = owner
            .states
            .proficiencies
            .lock()
            .map_err(|_| "states")?
            .get(&session)
            .and_then(|state| state.track(SWORD).map(|track| u64::from(track.progress)));
        assert_eq!(projected, Some(cap.min(25_000)));
        eprintln!("proficiency_catalyst: cases executed");
        Ok(())
    })
}

async fn catalyst_on_wire(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    harness: &Harness,
    catalyst: std::num::NonZeroU32,
    weapon: std::num::NonZeroU32,
    delay: Duration,
) -> TestResult {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let peer = async {
        let (mut source, mut target) = (None, None);
        loop {
            let frame = super::item_look_fixtures::read_frame(&mut client).await?;
            let envelope = wire::decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
            if envelope.message_type() == MessageType::SnapshotChunk {
                for domain in wire::decode_snapshot_chunk(envelope.payload()).map_err(debug)?.1 {
                    if domain.domain_id == wire::item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                        let inventory = wire::item_view::decode_character_inventory(domain.payload)
                            .map_err(debug)?;
                        for item in inventory.entries {
                            if item.item_definition_ref == catalyst {
                                source = Some(item.handle);
                            }
                            if item.item_definition_ref == weapon {
                                target = Some(item.handle);
                            }
                        }
                    }
                }
            }
        }
        let payload = world_object::encode_item_use_intent(&world_object::UseIntent {
            target: world_object::UseTarget::Item(source.ok_or("catalyst handle")?),
            use_with: None,
            use_with_map_item: Some(world_object::MapUseTarget::Item(target.ok_or("weapon handle")?)),
            use_with_position: None,
        }).map_err(debug)?;
        let mut command_id = admitted.continuity.next_command_id;
        send_use(&mut client, admitted, command_id, &payload).await?;
        let mut expect_busy = delay >= holder_timeout_delay();
        loop {
            let frame = super::item_look_fixtures::read_frame(&mut client).await?;
            let envelope = wire::decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() != MessageType::CommandResult {
                continue;
            }
            if expect_busy {
                assert_holder_refusal(&envelope, admitted, command_id)?;
                let stored: (i64, i64) = sqlx::query_as(
                    "SELECT (SELECT count(*) FROM game_character_proficiency_receipts WHERE cause='catalyst'),(SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid)",
                )
                .bind(id(UNITS).as_slice())
                .fetch_one(&harness.pool)
                .await?;
                assert_eq!(stored, (0, 2));
                command_id += 1;
                expect_busy = false;
                send_use(&mut client, admitted, command_id, &payload).await?;
                continue;
            }
            let result = wire::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.command_id, command_id);
            assert_eq!(result.status, wire::CommandStatus::Accepted);
            assert_eq!(
                world_object::decode_use_result(result.payload).map_err(debug)?,
                UseDisposition::Committed
            );
            break;
        }
        client.shutdown().await?;
        Ok(())
    };
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    contend_use(owner.root, "catalyst_commit", delay, async {
        let (_, result) = tokio::join!(serving, peer);
        result
    })
    .await?
}

/// One main backpack holding two catalysts, the sword and a coin.
async fn seed_bag(h: &Harness, catalyst: &str) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for table in ["game_item_container_entries", "game_item_container_slots"] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE character_id=encode($1,'hex')::uuid"
        )))
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await?;
    }
    let item = |seed, key: &str, count| {
        crate::durability::item_fee_burn::test_support::item(seed, key, "definition-r1", count)
    };
    sqlx::raw_sql(sqlx::AssertSqlSafe(item(BAG, "oteryn:item.tibia.i2854", 1)))
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)").bind(id(41).as_slice()).bind(id(BAG).as_slice()).bind(id(42).as_slice()).bind(id(245).as_slice()).execute(&mut *tx).await?;
    for (ordinal, (seed, key, count)) in [(UNITS, catalyst, 2), (WEAPON, SWORD, 1), (COIN, GOLD, 1)]
        .into_iter()
        .enumerate()
    {
        sqlx::raw_sql(sqlx::AssertSqlSafe(item(seed, key, count)))
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::bigint,encode($6,'hex')::uuid)").bind(id(41).as_slice()).bind(id(BAG).as_slice()).bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(i64::try_from(ordinal + 1)?).bind(id(246).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

#[test]
fn proficiency_catalyst_unknown_commit_does_not_decline_ownership() -> TestResult {
    std::thread::Builder::new()
        .name("proficiency_catalyst_unknown".into())
        .stack_size(64 << 20)
        .spawn(|| unknown_scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "proficiency catalyst unknown scenario panicked")?
        .map_err(Into::into)
}

fn unknown_scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "proficiency_catalyst_unknown", false, false)
                .await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("active")?;
        let room = pin.qualified_room().clone();
        let frame_digest = pin.frame_binding().digest();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let catalogue = ProductionProficiencyCatalogue::embedded().ok_or("catalogue")?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
        owner.proficiency_catalogue = Some(Arc::clone(&catalogue));
        deps.initialize(&owner).await;
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(frame_digest.as_slice()).execute(&harness.pool).await?;
        seed_bag(&harness, CATALYST).await?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?
            .item_fence();
        let command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(fence),
        };

        // Force the production commit to fail; reconciliation finds no receipt.
        sqlx::raw_sql("CREATE FUNCTION reject_catalyst() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected catalyst commit failure'; END $$; CREATE TRIGGER reject_catalyst BEFORE INSERT ON game_character_proficiency_receipts FOR EACH ROW EXECUTE FUNCTION reject_catalyst()")
            .execute(&harness.pool).await?;
        let outcome = FreshAdmissionAuthority::use_proficiency_catalyst(
            &owner, actor, command(1), ItemKey::Instance(id(UNITS)), ItemKey::Instance(id(WEAPON)),
        ).await;
        assert!(matches!(
            outcome,
            Err(crate::gameplay_transport::connection::UseOwnerError::OutcomeUnknown)
        ));
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
