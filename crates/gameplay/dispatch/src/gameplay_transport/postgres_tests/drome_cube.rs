//! W09: real USE dispatch, item-use fences, durable proof and custody/audit readback.
use crate::bestiary_postgres_harness as postgres_fixture;
use crate::content::item_ref::ItemDefinitionIndex;
use crate::foundation::{
    ChannelId, ChannelRuntimeV1, ExactActorRef, GameSessionId, NodeId, WorldId,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, SessionContinuity, UseCommand,
    UseOutcome,
};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_gameplay_items::drome_cube::{DROME_CUBE_KEY, reward, reward_index};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};
use postgres_fixture::{Harness, TestResult, configured_admin, debug, id, item_fence};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::sync::Mutex;

#[path = "drome_cube/cases.rs"]
mod cases;
#[path = "drome_cube/fixture.rs"]
mod fixture;
#[path = "drome_cube/kooldown_aid.rs"]
mod kooldown_aid;
#[path = "drome_cube/wire.rs"]
mod wire;

#[derive(Clone, Copy)]
enum Case {
    FullRetry,
    HandleReplay,
    ExistingStack,
    ConcurrentLastUnit,
    KooldownAid,
}

fn scenario(case: Case) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            postgres_fixture::runtime()
                .map_err(|error| error.to_string())?
                .block_on(fixture::scenario(admin, case))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "drome cube e2e panicked")?
        .map_err(Into::into)
}

#[test]
fn drome_cube_full_retry_same_reward_lost_ack_replay_and_restart() -> TestResult {
    scenario(Case::FullRetry)
}

#[test]
fn drome_cube_consumed_handle_replays_after_reconnect_and_emits_the_reward_text() -> TestResult {
    scenario(Case::HandleReplay)
}

#[test]
fn drome_cube_tops_up_the_existing_output_stack_in_one_transaction() -> TestResult {
    scenario(Case::ExistingStack)
}

#[test]
fn drome_cube_two_commands_compete_for_the_last_unit_without_a_second_reward() -> TestResult {
    scenario(Case::ConcurrentLastUnit)
}

#[test]
fn kooldown_aid_lost_ack_replays_once_and_keeps_its_durable_cooldown() -> TestResult {
    scenario(Case::KooldownAid)
}

struct WireOwner<'a, 'b, 'f, 's>(&'a ComposedFreshAdmission<'b, 'f, 's>);
impl FreshAdmissionAuthority for WireOwner<'_, '_, '_, '_> {
    fn take_tutorial_notice(
        &self,
        session: GameSessionId,
    ) -> Option<oteryn_protocol_oteryn::tutorial_notices::TutorialNotice> {
        self.0.take_tutorial_notice(session)
    }
    async fn observe(
        &self,
        actor: ExactActorRef,
    ) -> Option<crate::gameplay_transport::world_spatial::WorldSpatialObservation> {
        self.0.observe(actor).await
    }
    async fn admit(
        &self,
        _: crate::gameplay_transport::connection::FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, crate::gameplay_transport::connection::AdmissionRefusal> {
        Err(crate::gameplay_transport::connection::AdmissionRefusal::Unavailable)
    }
    async fn observe_character_inventory(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<crate::gameplay_transport::item_view::InventoryItems> {
        let bag = self
            .0
            .root
            .read_character_backpack(self.0.character, item_fence().ok()?.character_id)
            .await
            .ok()?;
        let runtime = self.0.runtime.lock().await;
        let index = crate::gameplay_transport::item_ref_admission::pinned_index(
            &runtime,
            self.0.active_generation,
        )?;
        crate::gameplay_transport::item_view::inventory_from_backpack(bag.as_ref(), |d| {
            crate::gameplay_transport::item_ref_admission::definition_ref(index, d)
        })
    }
    async fn observe_container(
        &self,
        actor: ExactActorRef,
        fence: Option<crate::durability::item_transfer::CurrentCharacterItemFence>,
        target: ItemKey,
    ) -> Option<crate::gameplay_transport::container_view::ContainerObservation> {
        self.0.observe_container(actor, fence, target).await
    }
    async fn use_drome_cube(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        self.0.use_drome_cube(actor, command, intent, source).await
    }
}

async fn run(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
    index: &ItemDefinitionIndex,
) -> TestResult {
    let admitted = cases::admitted(owner, actor, session)?;
    let cube = index
        .definition_ref(
            "Item",
            DROME_CUBE_KEY,
            index.revision(DROME_CUBE_KEY).ok_or("cube revision")?,
        )
        .ok_or("cube index")?;
    let intent = UseIntent {
        target: UseTarget::ItemByDefinition(cube),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    };
    let payload = encode_item_use_intent(&intent).map_err(debug)?;
    let selected = reward_index(id(110), 1)
        .and_then(reward)
        .ok_or("source-unit reward")?;
    // Every pool result must be present in the same active content used by the real writer.
    for kind in oteryn_gameplay_items::concoctions::ALL {
        assert!(
            index.revision(kind.production_key()).is_some(),
            "missing Cube output {:?}",
            kind
        );
    }
    let mut stale = item_fence()?;
    stale.connection_generation = crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
    let refused = owner
        .use_drome_cube(
            actor,
            UseCommand {
                game_session_id: session,
                command_id: 900,
                item_fence: Some(stale),
            },
            intent.clone(),
            Some(ItemKey::Instance(id(110))),
        )
        .await
        .map_err(|_| "stale fence outcome unknown")?
        .ok_or("stale fence result")?;
    assert_eq!(refused.disposition, UseDisposition::Rejected);
    assert_eq!(harness.count("game_drome_cube_uses").await?, 0);
    assert_eq!(source_quantity(harness).await?, 1);
    fixture::fill_main_backpack(harness, index, true).await?;
    let full = wire::exchange(owner, admitted, std::slice::from_ref(&payload)).await?;
    assert_eq!(full.results, vec![UseDisposition::Full]);
    assert!(full.notices.is_empty());
    let retained = full.retained;
    assert_eq!(harness.count("game_drome_cube_uses").await?, 0);
    assert_eq!(harness.count("game_item_use_records").await?, 0);
    assert_eq!(
        source_quantity(harness).await?,
        1,
        "full inventory burns nothing"
    );
    fixture::fill_main_backpack(harness, index, false).await?;
    assert_eq!(
        reward_index(id(110), 1).and_then(reward),
        Some(selected),
        "freeing space cannot reroll"
    );
    crate::gameplay_transport::item_use::drome_cube::test_faults::lose_next_ack();
    let missing_ack = wire::exchange(owner, retained, std::slice::from_ref(&payload)).await?;
    assert!(missing_ack.results.is_empty());
    assert!(missing_ack.notices.is_empty());
    let retained = missing_ack.retained;
    assert_eq!(
        retained.continuity.next_command_id, 2,
        "unknown result is not acknowledged"
    );
    assert_eq!(harness.count("game_drome_cube_uses").await?, 1);
    assert_eq!(source_quantity(harness).await?, 0);
    let replayed = wire::exchange(owner, retained, std::slice::from_ref(&payload)).await?;
    assert_eq!(replayed.results, vec![UseDisposition::Committed]);
    assert_eq!(
        replayed.notices,
        vec![cases::reward_notice(owner, selected)?]
    );
    assert_eq!(harness.count("game_drome_cube_uses").await?, 1);
    assert_eq!(harness.count("game_item_use_records").await?, 1);
    let output: (String, i64) = sqlx::query_as(
        "SELECT flask_definition_production_key,flask_quantity_after FROM game_item_use_records WHERE cause_kind=578")
        .fetch_one(&harness.pool).await?;
    assert_eq!(output, (selected.production_key().into(), 1));
    let (proof_index, same_xact): (i16, bool) = sqlx::query_as(
        "SELECT p.reward_index,p.created_xact_id=u.created_xact_id FROM game_drome_cube_uses p \
         JOIN game_item_use_records u USING(transaction_id)",
    )
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(proof_index, selected.index() as i16);
    assert!(
        same_xact,
        "source debit, output and proof have one transaction"
    );
    let wire: Vec<u8> = sqlx::query_scalar(
        "SELECT envelope FROM game_item_audit_outbox WHERE transaction_id=(SELECT transaction_id FROM game_drome_cube_uses)")
        .fetch_one(&harness.pool).await?;
    let (_, audit) =
        crate::durability::item_use_audit::decode_item_use_envelope(&wire).map_err(debug)?;
    assert_eq!(audit.cause.ok_or("cause")?.kind, 578);
    assert_eq!(audit.source_before.ok_or("source before")?.quantity, 1);
    assert_eq!(audit.source_after.ok_or("source after")?.quantity, 0);
    assert_eq!(
        audit
            .flask
            .ok_or("output")?
            .after
            .ok_or("output after")?
            .quantity,
        1
    );
    let mut altered = intent.clone();
    altered.target = UseTarget::ItemByDefinition(
        index
            .definition_ref(
                "Item",
                "oteryn:item.tibia.i2854",
                index
                    .revision("oteryn:item.tibia.i2854")
                    .ok_or("bag revision")?,
            )
            .ok_or("bag index")?,
    );
    let replay = owner
        .use_drome_cube(
            actor,
            UseCommand {
                game_session_id: session,
                command_id: 2,
                item_fence: Some(item_fence()?),
            },
            altered,
            None,
        )
        .await
        .map_err(|_| "altered intent unknown")?
        .ok_or("altered intent result")?;
    assert_eq!(replay.disposition, UseDisposition::Rejected);
    assert_eq!(harness.count("game_drome_cube_uses").await?, 1);
    // SQL proof rows are immutable and cannot exist without the same-transaction use.
    assert!(
        sqlx::query("UPDATE game_drome_cube_uses SET reward_index=(reward_index+1)%20")
            .execute(&harness.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM game_drome_cube_uses")
            .execute(&harness.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("TRUNCATE game_drome_cube_uses")
            .execute(&harness.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query(
            "INSERT INTO game_drome_cube_uses(transaction_id,wire_binding,reward_index) \
                        VALUES(encode($1,'hex')::uuid,$2,0)"
        )
        .bind(id(201).as_slice())
        .bind([0_u8; 32].as_slice())
        .execute(&harness.pool)
        .await
        .is_err()
    );
    assert_eq!(harness.count("game_drome_cube_uses").await?, 1);
    reload(harness, session, 2, &payload, selected.index() as u8).await?;
    // Reconciliation releases this command's reservations instead of wedging later Cube uses.
    fixture::another_cube(harness, index).await?;
    let next = wire::exchange(owner, replayed.retained, std::slice::from_ref(&payload)).await?;
    assert_eq!(next.results, vec![UseDisposition::Committed]);
    assert_eq!(harness.count("game_drome_cube_uses").await?, 2);
    eprintln!("DROME_CUBE POSTGRES_CASE_COMPLETED");
    Ok(())
}

async fn source_quantity(harness: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(110).as_slice())
    .fetch_one(&harness.pool)
    .await?)
}

async fn runtime_role(root: &crate::durability::DurabilityRoot) -> TestResult {
    let mut connection = root.try_acquire_ready()?;
    sqlx::query("SET ROLE oteryn_game_runtime")
        .execute(&mut *connection)
        .await?;
    let role: String = sqlx::query_scalar("SELECT current_user")
        .fetch_one(&mut *connection)
        .await?;
    assert_eq!(role, "oteryn_game_runtime");
    let returned = connection
        .return_to_pool_observed_until(std::time::Instant::now() + Duration::from_secs(5))
        .await;
    assert!(matches!(
        returned,
        sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle
    ));
    Ok(())
}

async fn reload(
    harness: &Harness,
    session: GameSessionId,
    command_id: u64,
    payload: &[u8],
    expected: u8,
) -> TestResult {
    let root = crate::durability::DurabilityRoot::connect_test_runtime(&harness.database.url)?;
    assert!(root.maintain_ready_once().await?);
    runtime_role(&root).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = root.open_character_authority(&seal).await.map_err(debug)?;
    let command = crate::foundation::CommandRef::new(
        session,
        crate::foundation::CommandId::new(command_id).map_err(debug)?,
    );
    let receipt = root
        .read_drome_cube_receipt(
            &authority,
            item_fence()?,
            command,
            Sha256::digest(payload).into(),
        )
        .await
        .map_err(debug)?
        .ok_or("restarted receipt")?;
    assert_eq!(receipt.reward_index, expected);
    assert_eq!(harness.count("game_item_use_records").await?, 1);
    Ok(())
}
