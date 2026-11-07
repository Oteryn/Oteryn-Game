//! Retained pending attempts across content cutover, and unreceipted SQL refusals.
use super::super::fixture::debug;
use super::*;
use oteryn_gameplay_items::forge::EXALTED_CORE_KEY;

#[test]
fn forge_postgres_pending_attempt_retains_revision_change_refusal_without_spending() -> TestResult {
    super::super::kill_reward_e2e::scenario_thread::run("forge-revision", || {
        support::scenario(Case::RevisionChanged)
    })
}

#[test]
fn forge_postgres_sql_rejects_unreceipted_tier_and_core_changes() -> TestResult {
    super::super::kill_reward_e2e::scenario_thread::run("forge-sql-guards", || {
        support::scenario(Case::SqlGuards)
    })
}

#[test]
fn forge_postgres_pending_dust_limit_never_requotes_after_another_paid_upgrade() -> TestResult {
    super::super::kill_reward_e2e::scenario_thread::run("forge-dust-stage", || {
        support::scenario(Case::DustStageChanged)
    })
}

pub(super) async fn dust_stage_changed(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &super::super::Dependencies,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let original = request(Operation::DustLimit, 191);
    sqlx::query("ALTER TABLE game_forge_operations ADD CONSTRAINT forge_test_stop_stage_effects CHECK(false) NOT VALID")
        .execute(&h.pool).await?;
    let stopped = exchange(owner, admitted, original).await;
    sqlx::query("ALTER TABLE game_forge_operations DROP CONSTRAINT forge_test_stop_stage_effects")
        .execute(&h.pool)
        .await?;
    assert!(stopped.is_err(), "the effects transaction was interrupted");
    let pending: (i16, i64, i64) = sqlx::query_as("SELECT expected_dust_upgrade_count,(SELECT count(*) FROM game_forge_dust_limit_reservations),(SELECT count(*) FROM game_forge_operations) FROM game_forge_reservations WHERE occurrence_id=encode($1,'hex')::uuid")
        .bind(original.occurrence.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        pending,
        (0, 0, 0),
        "the original quote survives without legacy effects"
    );
    assert_eq!(balances(h).await?, (100, 100, 0, 0));

    // The failed wire command remains uncertain on its old session. Recover by
    // genuine terminal logout and fresh login, retaining the same native quote.
    super::recovery::fresh_login(h, owner, deps, admitted).await?;
    cooldown_elapsed(h).await?;
    let (status, paid) = exchange(owner, admitted, request(Operation::DustLimit, 192)).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(paid["succeeded"], true);
    assert_eq!(balances(h).await?, (75, 120, 0, 0));
    let before_retry: Value = sqlx::query_scalar("SELECT jsonb_build_object('dust',(SELECT to_jsonb(d) FROM game_character_forge_dust d WHERE character_id=encode($1,'hex')::uuid),'entries',(SELECT count(*) FROM game_character_forge_dust_entries),'history',(SELECT count(*) FROM game_forge_history),'audit',(SELECT count(*) FROM game_item_audit_outbox))")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let (status, refused) = exchange(owner, admitted, original).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(refused["reason"], "STAGE_CHANGED");
    assert_eq!(refused["applied"], false);
    assert_eq!(
        exchange(owner, admitted, original).await?,
        (status, refused)
    );
    let after_retry: Value = sqlx::query_scalar("SELECT jsonb_build_object('dust',(SELECT to_jsonb(d) FROM game_character_forge_dust d WHERE character_id=encode($1,'hex')::uuid),'entries',(SELECT count(*) FROM game_character_forge_dust_entries),'history',(SELECT count(*) FROM game_forge_history),'audit',(SELECT count(*) FROM game_item_audit_outbox))")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        after_retry, before_retry,
        "old quote cannot debit the next price"
    );
    Ok(())
}

pub(super) async fn revision_changed(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    _deps: &super::super::Dependencies,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let before = balances(h).await?;
    let action = request(Operation::DustToSlivers, 190);
    // Add a stricter temporary constraint, leaving every production guard enabled.
    // The actual command persists its reservation, then its effects transaction
    // must fail. This exercises the real two-phase recovery boundary.
    sqlx::query("ALTER TABLE game_forge_operations ADD CONSTRAINT forge_test_stop_after_reservation CHECK(false) NOT VALID").execute(&h.pool).await?;
    let stopped = exchange(owner, admitted, action).await;
    sqlx::query(
        "ALTER TABLE game_forge_operations DROP CONSTRAINT forge_test_stop_after_reservation",
    )
    .execute(&h.pool)
    .await?;
    assert!(
        stopped.is_err(),
        "unavailable commit state disconnects without a terminal reply"
    );
    let pending: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_forge_reservations),(SELECT count(*) FROM game_forge_operations)").fetch_one(&h.pool).await?;
    assert_eq!(
        pending,
        (1, 0),
        "the real reservation survives the rejected effects transaction"
    );
    assert_eq!(balances(h).await?, before);
    let frozen: Value = sqlx::query_scalar("SELECT to_jsonb(r)-'work_units_used' FROM game_forge_reservations r WHERE occurrence_id=encode($1,'hex')::uuid")
        .bind(action.occurrence.as_slice()).fetch_one(&h.pool).await?;
    // Stop the old owner before booting the next admitted Content. Rotating only
    // the database row would correctly fail the inventory snapshot's independent
    // content check before command181 can retain its REVISION_CHANGED receipt.
    let old_lease = super::recovery::retire_session(h, owner, admitted).await?;
    let previous: (Vec<u8>, Vec<u8>, Vec<u8>) = sqlx::query_as("SELECT server_artifact_digest,client_artifact_digest,frame_binding_digest FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).fetch_one(&h.pool).await?;
    let (controller, next_deps) = activated_dependencies(h, 2, true).await?;
    let next = controller.active().ok_or("next generation")?;
    assert_ne!(
        previous.0.as_slice(),
        next.identity().server_artifact_digest().as_slice()
    );
    let mut next_owner = next_deps.owner(h, owner.character);
    next_owner.active_generation = Some(next);
    super::recovery::admit_successor(h, &next_owner, &next_deps, admitted, old_lease, 110).await?;
    let (status, refused) = exchange(&next_owner, admitted, action).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(refused["reason"], "REVISION_CHANGED");
    assert_eq!(refused["applied"], false);
    assert_eq!(balances(h).await?, before);
    let still_frozen: Value = sqlx::query_scalar("SELECT to_jsonb(r)-'work_units_used' FROM game_forge_reservations r WHERE occurrence_id=encode($1,'hex')::uuid")
        .bind(action.occurrence.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        still_frozen, frozen,
        "recovery preserves every reserved draw and identity"
    );
    // Even restoring the admitted source cannot turn that terminal refusal into
    // a new roll, resource charge or successful action.
    let next_lease = super::recovery::retire_session(h, &next_owner, admitted).await?;
    let (restored_controller, restored_deps) = activated_dependencies(h, 3, false).await?;
    let restored = restored_controller.active().ok_or("restored generation")?;
    assert_eq!(
        previous.0.as_slice(),
        restored.identity().server_artifact_digest().as_slice()
    );
    assert_eq!(
        previous.1.as_slice(),
        restored.identity().client_artifact_digest().as_slice()
    );
    assert_eq!(
        previous.2.as_slice(),
        restored_deps
            .runtime
            .lock()
            .await
            .content_pin()
            .frame_binding_digest()
            .as_slice()
    );
    let mut restored_owner = restored_deps.owner(h, owner.character);
    restored_owner.active_generation = Some(restored);
    super::recovery::admit_successor(
        h,
        &restored_owner,
        &restored_deps,
        admitted,
        next_lease,
        111,
    )
    .await?;
    assert_eq!(
        exchange(&restored_owner, admitted, action).await?,
        (CommandStatus::Accepted, refused)
    );
    let evidence: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_forge_history),(SELECT count(*) FROM game_item_audit_outbox a JOIN game_forge_reservations r USING(transaction_id)),(SELECT count(*) FROM game_character_forge_dust_entries WHERE cause_occurrence_id=encode($1,'hex')::uuid)")
        .bind(action.occurrence.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        evidence,
        (0, 0, 0),
        "revision refusal creates no history, event or dust change"
    );
    assert_eq!(balances(h).await?, before);
    Ok(())
}

/// The native boot/activation path from death_ml_loss_composed's production
/// generation helper, with the actual successor issuance sequence. The raw
/// presentation section is length/hash bound (native_gameplay.rs:881–888), so
/// JSON whitespace changes artifact identity without changing decoded facts or
/// the Character's accepted training interpretation, content-1.
async fn activated_dependencies(
    h: &Harness,
    sequence: i64,
    repack: bool,
) -> TestResult<(
    crate::content::ContentActivationController,
    super::super::Dependencies,
)> {
    use crate::content::{
        ContentActivationController, NativeEntryActivationIssuance, NodeBootQuiescence,
        activate_native_entry_room_with_gameplay, native_gameplay::NativeGameplayInput,
    };
    use sha2::Digest;
    assert!(matches!((sequence, repack), (2, true) | (3, false)));
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
    let manifest = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/spells.manifest.json"
    ));
    let mut supplied = NativeGameplayInput::from_manifest(manifest).map_err(debug)?;
    // Match the original fixture helper's native entry room and interpretation.
    supplied.source_world = None;
    supplied
        .build_training
        .as_mut()
        .ok_or("training input")?
        .content_revision = "content-1".into();
    if repack {
        let before: Value = serde_json::from_slice(&supplied.presentation_profiles.bytes)?;
        supplied.presentation_profiles.bytes.push(b'\n');
        supplied.presentation_profiles.sha256 = {
            use std::fmt::Write as _;
            let mut hex = String::with_capacity(64);
            for byte in sha2::Sha256::digest(&supplied.presentation_profiles.bytes) {
                write!(&mut hex, "{byte:02x}")?;
            }
            hex
        };
        assert_eq!(
            before,
            serde_json::from_slice::<Value>(&supplied.presentation_profiles.bytes)?
        );
    }
    let room =
        crate::content::qualify_selected_native_gameplay_room(world, &supplied).map_err(debug)?;
    let issuance = NativeEntryActivationIssuance {
        world_id: world,
        activation_sequence: u64::try_from(sequence)?,
        server_artifact_digest: room.compiled().server_digest(),
        client_artifact_digest: room.compiled().client_digest(),
        frame_binding_digest: room.frame_binding().digest(),
    };
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::numeric,$4::numeric,$5,$6,$7)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(sequence).bind(sequence - 1)
        .bind(issuance.server_artifact_digest.as_slice()).bind(issuance.client_artifact_digest.as_slice())
        .bind(issuance.frame_binding_digest.as_slice()).execute(&h.pool).await?;
    let mut controller = ContentActivationController::new();
    let pin = activate_native_entry_room_with_gameplay(
        &mut controller,
        &NodeBootQuiescence::before_channel_runtime(),
        world,
        &issuance,
        &supplied,
    )
    .map_err(debug)?;
    let room = pin.qualified_room().clone();
    let channel_pin = pin.into_channel_pin();
    let mut deps = super::super::Dependencies::with_content(
        channel_pin.clone(),
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    // Dependencies' default actor is never admitted to this new owner. Start the
    // boot runtime empty; only the durable successor gets a reserved actor.
    deps.runtime = tokio::sync::Mutex::new(
        crate::foundation::ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            channel_pin,
        )
        .map_err(debug)?,
    );
    Ok((controller, deps))
}

pub(super) async fn sql_guards(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    seed_items(
        h,
        &[
            (80, "oteryn:item.tibia.i3264", 1, 0),
            (81, EXALTED_CORE_KEY, 5, 0),
            (82, FORGE_SLIVER_KEY, 50, 0),
        ],
    )
    .await?;
    let before = balances(h).await?;
    denied(
        h,
        "INSERT INTO game_item_tiers(item_instance_id,tier) VALUES(encode($1,'hex')::uuid,1)",
        80,
    )
    .await?;
    denied(h, "UPDATE game_item_instances SET quantity=6,last_transaction_id=game_character_uuid_v7() WHERE item_instance_id=encode($1,'hex')::uuid", 81).await?;
    assert_eq!(balances(h).await?, before);
    let tiers: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_tiers WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(80).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(tiers, 0);
    let action = request(Operation::SliversToCores, 200);
    let (status, result) = exchange(owner, admitted, action).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        result["applied"], true,
        "the same guards admit a complete native owner proof"
    );
    assert_eq!(balances(h).await?, (100, 100, 0, 6));
    denied(
        h,
        "UPDATE game_item_instances SET quantity=7 WHERE item_instance_id=encode($1,'hex')::uuid",
        81,
    )
    .await?;
    assert_eq!(
        balances(h).await?,
        (100, 100, 0, 6),
        "a previous transaction's receipt cannot authorize another mint"
    );
    Ok(())
}

async fn denied(h: &Harness, statement: &'static str, item: u8) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let error = match sqlx::query(statement)
        .bind(id(item).as_slice())
        .execute(&mut *tx)
        .await
    {
        Err(error) => error,
        Ok(_) => match tx.commit().await {
            Err(error) => return assert_guard(error),
            Ok(()) => return Err("unreceipted runtime mutation committed".into()),
        },
    };
    tx.rollback().await?;
    assert_guard(error)
}

fn assert_guard(error: sqlx::Error) -> TestResult {
    let database = error
        .as_database_error()
        .ok_or("expected a SQL proof refusal")?;
    assert_eq!(
        database.code().as_deref(),
        Some("23514"),
        "the native integrity guard rejected the write: {database}"
    );
    Ok(())
}
