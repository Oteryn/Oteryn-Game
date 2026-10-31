//! Actual recovery/session/item writers and allocated Instance provenance. Until #2769
//! installs shared session admission, a prepared group must never commit a Soulpit debit.
//! These cases keep all session, custody and deferred handoff guards enabled.

use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::soulpit::{SoulpitAction, SoulpitEntryRequest, SoulpitParticipantFact, SoulpitStepRequest, SoulpitWriteError};
use crate::foundation::{CommandId, CommandRef, ConnectionGeneration, ScopeOwnershipGeneration};
use sqlx::Row;

#[path = "soulpit_lifecycle/provenance.rs"]
mod provenance;

#[path = "soulpit_lifecycle/completion_atomicity.rs"]
mod completion_atomicity;

#[test]
fn soulpit_entry_requires_atomic_shared_handoff_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("SOULPIT_LIFECYCLE POSTGRES_CASE_NOT_RUN: no configured PostgreSQL route");
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("soulpit-provenance", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "soulpit_lifecycle", true, false).await?;
        super::reset();
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let deps = Dependencies::new()?;
        let (controller, pin) = crate::content::native_gameplay::test_support::activated_with_loot_tables(deps.admitted.world_id, None);
        let native = controller.active().and_then(crate::content::ActiveGeneration::native_gameplay).ok_or("native content")?;
        let core = native.creature_profiles().records.iter().find_map(|record| {
            let crate::content::ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data else { return None; };
            let core = profile.details.as_ref()?.soul_core_item.as_ref()?;
            let policy = native.item_policy(&core.key, &core.revision)?.record();
            let source_id = policy.production_binding.external_id.parse::<u32>().ok()?;
            (core.revision=="definition-r1" && oteryn_gameplay_domain::domain::soulpit::is_soul_core(source_id)
                && policy.admission.materializable).then(|| core.key.clone())
        }).ok_or("native Soul Core with its creature binding")?;
        let purple_core = "oteryn:item.tibia.i47490";
        assert!(native.item_policy(purple_core, "definition-r1")
            .is_some_and(|policy| policy.record().admission.materializable));
        assert!(native.creature_profiles().records.iter().any(|record|
            record.profile.target.key == "oteryn:creature.purple_butterfly"),
            "the regression requires the existing native Canary Purple Butterfly admission");
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(deps.admitted.game_session_id).await?.0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("source item fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(fence.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        super::item_look::seed_main_backpack(&h, &[(&core, 2), (purple_core, 2)]).await?;
        let purple_before: serde_json::Value = sqlx::query_scalar(
            "SELECT jsonb_build_object('item',to_jsonb(i),'custody',to_jsonb(e)) \
             FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) \
             WHERE item_instance_id=encode($1,'hex')::uuid",
        ).bind(id(72).as_slice()).fetch_one(&h.pool).await?;
        provenance::save_entry(&h, fence.game_session_id).await?;
        let scope = provenance::allocate_instance(&h).await?;
        let permit = owner.spell_lane_permit().await.ok_or("current spell lane")?;
        let request = SoulpitEntryRequest {
            handoff_id: id(92), command: CommandRef::new(fence.game_session_id, CommandId::new(1).map_err(debug)?),
            source: id(71), intent: vec![1, 2, 3], started_at_micros: 0,
            participants: vec![SoulpitParticipantFact { character: fence.character_id, game_master: false }],
        };
        // No shared preparation: current source authority alone cannot admit a fight.
        let mut tx = provenance::transaction(&h).await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut **tx).await?;
        assert!(matches!(h.root.write_soulpit_entry_in_transaction(&mut tx, &permit, &authority, &h.node, fence, &request, native).await,
            Err(SoulpitWriteError::AuthorityRejected)));
        tx.into_inner().rollback().await?;
        unchanged(&h).await?;

        // A real prepared group still does not weaken the current connection fence.
        let mut tx = provenance::transaction(&h).await?;
        provenance::prepare(&mut tx, &request, fence.game_session_id, scope).await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut **tx).await?;
        let mut stale = fence;
        stale.connection_generation = ConnectionGeneration::new(999).map_err(debug)?;
        assert!(matches!(h.root.write_soulpit_entry_in_transaction(&mut tx, &permit, &authority, &h.node, stale, &request, native).await,
            Err(SoulpitWriteError::AuthorityRejected)));
        tx.into_inner().rollback().await?;
        unchanged(&h).await?;

        provenance::null_consumption_is_not_a_free_entry(&h, &request, fence.game_session_id, scope).await?;
        unchanged(&h).await?;

        // Source registration and native admission are independent requirements. This
        // admitted Canary variant has no exact Crystal MonsterType and must retain its
        // core, including its complete custody row, before the transaction is rolled back.
        let purple_request = SoulpitEntryRequest {
            command: CommandRef::new(fence.game_session_id, CommandId::new(2).map_err(debug)?),
            source: id(72), intent: vec![4, 7, 4, 9, 0], ..request.clone()
        };
        let mut tx = provenance::transaction(&h).await?;
        provenance::prepare(&mut tx, &purple_request, fence.game_session_id, scope).await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut **tx).await?;
        assert!(matches!(h.root.write_soulpit_entry_in_transaction(
            &mut tx, &permit, &authority, &h.node, fence, &purple_request, native,
        ).await, Err(SoulpitWriteError::Rule(
            oteryn_gameplay_domain::domain::soulpit::SoulpitError::InvalidMonster,
        ))));
        let purple_after: serde_json::Value = sqlx::query_scalar(
            "SELECT jsonb_build_object('item',to_jsonb(i),'custody',to_jsonb(e)) \
             FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) \
             WHERE item_instance_id=encode($1,'hex')::uuid",
        ).bind(id(72).as_slice()).fetch_one(&mut **tx).await?;
        assert_eq!(purple_after, purple_before, "source-absent core was changed before refusal");
        let effects: (i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_soulpit_encounters), \
             (SELECT count(*) FROM game_item_craft_records), \
             (SELECT count(*) FROM game_item_audit_outbox)",
        ).fetch_one(&mut **tx).await?;
        assert_eq!(effects, (0, 0, 0), "source-absent entry wrote a fight, debit or audit");
        tx.into_inner().rollback().await?;
        unchanged(&h).await?;

        for attempt_commit in [false, true] {
            let mut tx = provenance::transaction(&h).await?;
            provenance::prepare(&mut tx, &request, fence.game_session_id, scope).await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut **tx).await?;
            let written = h.root.write_soulpit_entry_in_transaction(&mut tx, &permit, &authority, &h.node, fence, &request, native).await.map_err(debug)?;
            assert!(!written.replayed);
            assert_eq!(written.handoff_id, request.handoff_id);
            let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).fetch_one(&mut **tx).await?;
            assert_eq!(quantity, 1, "entry debit is written inside the shared transaction");
            let replay = h.root.write_soulpit_entry_in_transaction(&mut tx, &permit, &authority, &h.node, stale, &request, native).await.map_err(debug)?;
            assert!(replay.replayed);
            assert_eq!(replay.source_transaction_id, written.source_transaction_id);
            let mut conflict = request.clone(); conflict.intent.push(4);
            assert!(matches!(h.root.write_soulpit_entry_in_transaction(&mut tx, &permit, &authority, &h.node, fence, &conflict, native).await,
                Err(SoulpitWriteError::ConflictingOccurrence)));
            sqlx::query("RESET ROLE").execute(&mut **tx).await?;
            if attempt_commit {
                // Exercise the shared group's real deferred barrier. No destination session
                // or participant authority is forged to make this pending dependency pass.
                sqlx::query("UPDATE game_activity_handoff_groups SET state=2,decided_at=1 WHERE handoff_id=encode($1,'hex')::uuid")
                    .bind(request.handoff_id.as_slice()).execute(&mut **tx).await?;
            }
            assert!(tx.commit(std::time::Instant::now()+std::time::Duration::from_secs(5)).await.is_err(),
                "an absent destination admission or uncommitted handoff must roll back the core");
            unchanged(&h).await?;
        }

        // Ready native Instance allocation alone cannot authorize fight progress/rewards.
        for (runtime_scope, generation) in [(scope, 1), (scope, 2), (fence.runtime_scope, 1)] {
            let step = SoulpitStepRequest {
                handoff_id: request.handoff_id, occurrence: id(97), runtime_scope,
                scope_ownership_generation: ScopeOwnershipGeneration::new(generation).map_err(debug)?,
                expected_revision: 0, now_micros: 4_500_000, action: SoulpitAction::Advance, survivors: vec![],
            };
            assert!(matches!(h.root.advance_soulpit(&authority, &h.node, step).await, Err(SoulpitWriteError::AuthorityRejected)));
        }
        provenance::runtime_cannot_capture_commit(&h).await?;
        unchanged(&h).await?;
        eprintln!("SOULPIT_LIFECYCLE POSTGRES_CASE_COMPLETED");
        drop(permit); drop(owner); drop(authority); drop(seal);
        h.cleanup().await
    })
}

async fn unchanged(h: &Harness) -> TestResult {
    let row = sqlx::query("SELECT (SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid) AS quantity,(SELECT count(*) FROM game_soulpit_encounters) AS fights,(SELECT count(*) FROM game_item_craft_records) AS crafts,(SELECT count(*) FROM game_item_audit_outbox) AS audits,(SELECT count(*) FROM game_character_animus_masteries) AS masteries,(SELECT count(*) FROM game_soulpit_entry_handoff_commits) AS commits")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(row.try_get::<i64,_>("quantity")?, 2);
    let purple_quantity: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    ).bind(id(72).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(purple_quantity, 2);
    for field in ["fights", "crafts", "audits", "masteries", "commits"] {
        assert_eq!(row.try_get::<i64,_>(field)?, 0, "failed handoff retained {field}");
    }
    Ok(())
}
