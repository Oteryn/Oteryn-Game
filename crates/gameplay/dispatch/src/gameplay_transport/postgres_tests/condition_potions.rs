//! Q4/Q6: native content -> framed USE -> reserved actual condition -> fenced burn/counter.
//! Each case starts with a valid production command, so disconnecting the consumer is RED.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{ChannelContentPin, ConnectionGeneration};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::{FirstEntryOutcome, UseCommand};
use crate::gameplay_transport::item_use::condition_potions::inject_fault;
use crate::gameplay_transport::item_view::ItemKey;
use crate::spell::Vocation;
use oteryn_gameplay_progression::condition_potions::ConditionPotion;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, UseDisposition, UseIntent, UseTarget, decode_use_result,
    encode_item_use_intent,
};
use std::time::Duration;

#[test]
fn condition_potions_postgres_berserk_buff_expiry_receipt_and_refusals() -> TestResult {
    run_case(
        ConditionPotion::Berserk,
        Vocation::Knight,
        "knight",
        "q6_berserk",
    )
}
#[test]
fn condition_potions_postgres_mastermind_buff_expiry_receipt_and_refusals() -> TestResult {
    run_case(
        ConditionPotion::Mastermind,
        Vocation::Druid,
        "druid",
        "q6_mastermind",
    )
}
#[test]
fn condition_potions_postgres_bullseye_buff_expiry_receipt_and_refusals() -> TestResult {
    run_case(
        ConditionPotion::Bullseye,
        Vocation::Paladin,
        "paladin",
        "q6_bullseye",
    )
}
#[test]
fn condition_potions_postgres_transcendence_buff_expiry_receipt_and_refusals() -> TestResult {
    // No vocation limit in Crystal: a vocationless Character is a valid consumer.
    run_case(
        ConditionPotion::Transcendence,
        Vocation::None,
        "none",
        "q6_transcendence",
    )
}

fn run_case(
    potion: ConditionPotion,
    vocation: Vocation,
    vocation_key: &'static str,
    tag: &'static str,
) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            scenario(potion, vocation, vocation_key, tag).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "condition potion scenario panicked")??;
    Ok(())
}

fn source_id(potion: ConditionPotion) -> [u8; 16] {
    id(match potion {
        ConditionPotion::Berserk => 80,
        ConditionPotion::Mastermind => 90,
        ConditionPotion::Bullseye => 100,
        ConditionPotion::Transcendence => 110,
    })
}

fn scenario(
    potion: ConditionPotion,
    vocation: Vocation,
    vocation_key: &'static str,
    tag: &'static str,
) -> TestResult {
    use super::cyclopedia::support::exchange;
    run(async move |admin| {
        reset();
        inject_fault(0);
        let h = Harness::create_with_admission_seed(admin, tag, true, false).await?;
        eprintln!("POSTGRES_STARTED Q6 {tag}");
        prepare_current_session(&h).await?;
        {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
            let (controller, activation) =
                crate::content::native_gameplay::test_support::activated_with_loot_tables(
                    world, None,
                );
            let generation = controller.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native content")?;
            let index = native.item_index().ok_or("item index")?;
            let room = activation.qualified_room().clone();
            let pin = ChannelContentPin::from_activation(
                world,
                activation.activation_sequence(),
                activation.identity().server_artifact_digest(),
                activation.identity().client_artifact_digest(),
                activation.frame_binding().digest(),
                room.map_revision_digest(),
                (0, 0, 0),
            );
            let deps = Dependencies::with_content(
                pin.clone(),
                room,
                crate::spell::cast::v1_spell_book().map_err(debug)?,
            )?;
            sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
                .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,NULL,$4,$5,$6)")
                .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.activation_sequence().to_string())
                .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice())
                .bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
            let current =
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(deps.admitted.game_session_id)
                    .await?
                    .0;
            let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
            {
                let mut runtime = deps.runtime.lock().await;
                runtime
                    .bind_attacker_lease(
                        deps.actor,
                        deps.admitted.game_session_id,
                        current.current_character_lease(),
                    )
                    .map_err(debug)?;
                runtime
                    .initialize_first_entry_position(deps.actor)
                    .map_err(debug)?;
            }
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            let bag_key = "oteryn:item.tibia.i2854";
            for (source, key, amount) in std::iter::once((id(70), bag_key, 1_i64)).chain(
                [
                    ConditionPotion::Berserk,
                    ConditionPotion::Mastermind,
                    ConditionPotion::Bullseye,
                    ConditionPotion::Transcendence,
                ]
                .into_iter()
                .map(|p| (source_id(p), p.key(), 5_i64)),
            ) {
                let revision = index.revision(key).ok_or("admitted source item")?;
                assert!(
                    native.item_policy(key, revision).is_some(),
                    "condition potion content must be materializable"
                );
                sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,$5,1,encode($1,'hex')::uuid)")
                    .bind(source.as_slice()).bind(id(42).as_slice()).bind(key).bind(revision).bind(amount).execute(&mut *tx).await?;
            }
            sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($2,'hex')::uuid)")
                .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
            for (ordinal, p) in [
                ConditionPotion::Berserk,
                ConditionPotion::Mastermind,
                ConditionPotion::Bullseye,
                ConditionPotion::Transcendence,
            ]
            .into_iter()
            .enumerate()
            {
                sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($1,'hex')::uuid)")
                    .bind(source_id(p).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice()).bind((ordinal + 1).to_string()).execute(&mut *tx).await?;
            }
            sqlx::query("INSERT INTO game_character_achievement_progress(character_id,achievement_key,progress) VALUES(encode($1,'hex')::uuid,$2,98)")
                .bind(id(41).as_slice()).bind(potion.achievement().key).execute(&mut *tx).await?;
            tx.commit().await?;

            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&deps.room));
            owner.clock_origin = std::time::Instant::now()
                .checked_sub(Duration::from_secs(60))
                .ok_or("clock")?;
            {
                let runtime = deps.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                let facts = crate::spell::cast::CharacterCastFacts {
                    vocation,
                    magic_level: 12,
                    ..crate::gameplay_transport::actor_spell::tests::FACTS
                };
                states
                    .initialize(
                        &runtime,
                        deps.actor,
                        deps.admitted.game_session_id,
                        facts,
                        (0, 0),
                        owner.owner_now(),
                    )
                    .ok_or("spell state")?;
                let build = crate::durability::character_build::DurableBuildState::new(
                    vocation_key,
                    (12, 0),
                    [(50, 0); 7],
                )
                .map_err(debug)?;
                states
                    .get_mut(&runtime, deps.actor, deps.admitted.game_session_id)
                    .ok_or("state")?
                    .load_owned_build(&build)
                    .map_err(debug)?;
            }
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.item_fence = Some(fence);
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                &[
                    OfferedCapability {
                        id: 4,
                        requires: &[],
                    },
                    OfferedCapability {
                        id: 15,
                        requires: &[4],
                    },
                ],
                &[4, 15],
            )
            .ok_or("use capabilities")?;
            let payload = |p: ConditionPotion| -> TestResult<Vec<u8>> {
                let definition = index
                    .definition_ref("Item", p.key(), index.revision(p.key()).ok_or("revision")?)
                    .ok_or("item index")?;
                Ok(encode_item_use_intent(&UseIntent {
                    target: UseTarget::ItemByDefinition(definition),
                    use_with: None,
                    use_with_map_item: None,
                    use_with_position: None,
                })
                .map_err(debug)?)
            };
            let valid = payload(potion)?;
            let (status, reply) =
                exchange(&owner, &mut admitted, 1, COMMAND_TYPE_USE_INTENT, &valid).await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(
                decode_use_result(&reply).map_err(debug)?,
                UseDisposition::Committed,
                "the real command must apply the condition before counting this use"
            );
            assert_eq!(progress(&h, potion).await?, 99);
            assert_eq!(quantity(&h, potion).await?, 4);
            assert_eq!(awards(&h, potion).await?, 0);
            assert_effect(&owner, deps.actor, admitted.game_session_id, potion, &h).await?;
            let (status, replay) =
                exchange(&owner, &mut admitted, 1, COMMAND_TYPE_USE_INTENT, &valid).await?;
            assert_eq!(status, CommandStatus::DuplicateReplay);
            assert_eq!(reply, replay);
            assert_eq!(
                progress(&h, potion).await?,
                99,
                "replay cannot count a use twice"
            );
            assert_eq!(quantity(&h, potion).await?, 4);

            let forbidden = if potion == ConditionPotion::Berserk {
                ConditionPotion::Mastermind
            } else {
                ConditionPotion::Berserk
            };
            let (_, reply) = exchange(
                &owner,
                &mut admitted,
                2,
                COMMAND_TYPE_USE_INTENT,
                &payload(forbidden)?,
            )
            .await?;
            assert_eq!(
                decode_use_result(&reply).map_err(debug)?,
                UseDisposition::RequirementNotMet
            );
            assert_eq!(quantity(&h, forbidden).await?, 5);
            assert_eq!(progress(&h, forbidden).await?, 0);
            assert_eq!(progress(&h, potion).await?, 99);
            let command = |id, item_fence| UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: id,
                item_fence: Some(item_fence),
            };
            let mut stale = fence;
            stale.connection_generation = ConnectionGeneration::new(999).map_err(debug)?;
            assert_ne!(
                owner
                    .use_backpack_item(
                        deps.actor,
                        command(3, stale),
                        ItemKey::Instance(source_id(potion))
                    )
                    .await
                    .disposition,
                UseDisposition::Committed
            );
            assert_eq!(progress(&h, potion).await?, 99);
            assert_eq!(quantity(&h, potion).await?, 4);

            // A cancelled operation before COMMIT retains the reservation until an independently
            // fenced absence read proves rollback. Neither the unit nor progress was consumed.
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(2))
                .ok_or("clock")?;
            inject_fault(1);
            assert_eq!(
                owner
                    .use_backpack_item(
                        deps.actor,
                        command(9, fence),
                        ItemKey::Instance(source_id(potion))
                    )
                    .await
                    .disposition,
                UseDisposition::Rejected
            );
            assert!(deps.runtime.lock().await.actor_spell_reserved(deps.actor));
            assert_eq!(progress(&h, potion).await?, 99);
            let permit = owner
                .spell_lane_permit()
                .await
                .ok_or("resolve definite absence")?;
            drop(permit);
            assert!(!deps.runtime.lock().await.actor_spell_reserved(deps.actor));
            assert_eq!(quantity(&h, potion).await?, 4);

            // Lost acknowledgement after COMMIT: the receipt is durable, the actual actor is
            // reserved, and a later production lane pass installs the ORIGINAL condition.
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(2))
                .ok_or("clock")?;
            inject_fault(2);
            assert_eq!(
                owner
                    .use_backpack_item(
                        deps.actor,
                        command(10, fence),
                        ItemKey::Instance(source_id(potion))
                    )
                    .await
                    .disposition,
                UseDisposition::Rejected
            );
            assert!(deps.runtime.lock().await.actor_spell_reserved(deps.actor));
            assert_eq!(progress(&h, potion).await?, 100);
            assert_eq!(quantity(&h, potion).await?, 3);
            assert_eq!(awards(&h, potion).await?, 1);
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(45))
                .ok_or("clock")?;
            let permit = owner
                .spell_lane_permit()
                .await
                .ok_or("resolve committed receipt")?;
            drop(permit);
            assert!(!deps.runtime.lock().await.actor_spell_reserved(deps.actor));
            assert_effect(&owner, deps.actor, admitted.game_session_id, potion, &h).await?;
            assert_eq!(
                owner
                    .use_backpack_item(
                        deps.actor,
                        command(10, fence),
                        ItemKey::Instance(source_id(potion))
                    )
                    .await
                    .disposition,
                UseDisposition::Committed
            );
            assert_eq!(progress(&h, potion).await?, 100);
            assert_eq!(quantity(&h, potion).await?, 3);
            assert_eq!(awards(&h, potion).await?, 1);
            let receipts: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_item_condition_potion_receipts")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(receipts, 2);
            let wrong_counter: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_achievement_progress WHERE achievement_key='oteryn:achievement/potion_addict'").fetch_one(&h.pool).await?;
            assert_eq!(
                wrong_counter, 0,
                "condition potions do not enter the HP/MP Potion Addict branch"
            );
        }
        eprintln!("Q6 POSTGRES_CASE_COMPLETED {tag}");
        h.cleanup().await
    })
}

async fn progress(h: &Harness, potion: ConditionPotion) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT progress FROM game_character_achievement_progress WHERE character_id=encode($1,'hex')::uuid AND achievement_key=$2),0)")
        .bind(id(41).as_slice()).bind(potion.achievement().key).fetch_one(&h.pool).await?)
}
async fn quantity(h: &Harness, potion: ConditionPotion) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(source_id(potion).as_slice())
    .fetch_one(&h.pool)
    .await?)
}
async fn awards(h: &Harness, potion: ConditionPotion) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_account_achievements a JOIN game_character_roots r ON r.account_id=a.account_id WHERE r.character_id=encode($1,'hex')::uuid AND a.achievement_key=$2")
        .bind(id(41).as_slice()).bind(potion.achievement().key).fetch_one(&h.pool).await?)
}
async fn assert_effect(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    potion: ConditionPotion,
    h: &Harness,
) -> TestResult {
    let (start, end): (String, String) = sqlx::query_as("SELECT applied_owner_micros::text,expires_owner_micros::text FROM game_item_condition_potion_receipts WHERE condition_key=$1 ORDER BY applied_owner_micros DESC LIMIT 1")
        .bind(potion.key()).fetch_one(&h.pool).await?;
    let (start, end) = (start.parse::<u64>()?, end.parse::<u64>()?);
    assert_eq!(end - start, 600_000_000);
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let state = states.get(&runtime, actor, session).ok_or("live owner")?;
    let condition = state
        .conditions
        .instances()
        .iter()
        .find(|c| c.definition().key() == potion.key())
        .ok_or("applied condition")?;
    assert_eq!(
        condition.started_at(),
        start,
        "reconciliation cannot restart the duration"
    );
    let expected = match potion {
        ConditionPotion::Berserk => (0, 0, 5, 0, -10),
        ConditionPotion::Mastermind => (3, 0, 0, 0, 0),
        ConditionPotion::Bullseye => (0, 0, 0, 5, -10),
        ConditionPotion::Transcendence => (0, 4, 0, 0, 0),
    };
    for at in [start, end - 1] {
        let value = state
            .conditions
            .skill_adjustments(at)
            .ok_or("skill adjustment")?;
        assert_eq!(
            (
                value.magic_level,
                value.fist,
                value.melee,
                value.distance,
                value.shielding
            ),
            expected
        );
        assert_eq!(
            state.owned_effective_magic_level(at).map_err(debug)?,
            u32::try_from(12 + expected.0)?,
            "the actual spell consumer reads Mastermind's magic bonus"
        );
        for (skill, delta) in [
            (0, expected.1),
            (2, expected.2),
            (4, expected.3),
            (5, expected.4),
        ] {
            assert_eq!(
                state.live_skill(skill, at),
                Some(u32::try_from(50 + delta)?),
                "the actual combat consumer reads the buff"
            );
        }
    }
    assert_eq!(
        state.conditions.skill_adjustments(end),
        Some(Default::default())
    );
    assert_eq!(state.live_skill(2, end), Some(50));
    assert_eq!(state.owned_effective_magic_level(end).map_err(debug)?, 12);
    Ok(())
}
