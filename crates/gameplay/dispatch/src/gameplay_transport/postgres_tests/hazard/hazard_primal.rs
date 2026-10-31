//! Native party-kill transaction, then real command 169 read/selection/replay.
//! The qualified encounter owner is composed separately; this fixture supplies
//! its death occurrence through the existing party XP writer, never a raw grant.
use super::*;
use crate::durability::character_hazard::apply_hazard_kill_in_transaction;
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use crate::durability::party_kill_xp::{self, PartyKillXpMember, PartyKillXpRequest};
use crate::durability::spell_item_transaction as item_tx;
use crate::foundation::RuntimeScopeRefV1;

#[test]
fn primal_hazard_postgres_kill_unlock_selection_replay_cap_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("PRIMAL_HAZARD POSTGRES_CASE_COMPLETED skipped_no_database");
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("primal-hazard", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "primal_hazard", false, false).await?;
        prepare_current_session(&h).await?;
        let deps = Dependencies::new()?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let (controller, _) = super::super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id, "content-1",
        )?;
        let generation = controller.active().ok_or("active progression generation")?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        deps.initialize(&owner).await;
        owner.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        // The existing admission owner binds and initializes the actual generation's
        // policy. A bare Dependencies owner has no native-XP progression binding.
        owner.admit_progression_session(&mut admitted).await;
        assert!(owner.progression_binding(admitted.game_session_id).is_some());
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(admitted.game_session_id).await?.0;
        admitted.item_fence = item_fence_of(current);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(), &[CAPABILITY_HAZARD_V1],
        ).ok_or("hazard capability not offered")?;
        assert_eq!(exchange(&owner, &mut admitted, 1, &[]).await?, (CommandStatus::Accepted, vec![1, 1]));
        let first = settle(&h, &owner, &admitted, 1, true).await?;
        assert_eq!(exchange(&owner, &mut admitted, 2, &[]).await?, (CommandStatus::Accepted, vec![2, 1]));
        let xp: i64 = sqlx::query_scalar("SELECT xp_awarded FROM game_party_kill_xp_members WHERE death_key=$1")
            .bind(first.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(xp, 0, "zero stamina never removes a real contributor's unlock");
        settle(&h, &owner, &admitted, 1, true).await?;
        assert_eq!(exchange(&owner, &mut admitted, 3, &[2]).await?, (CommandStatus::Accepted, vec![2, 2]));
        assert_eq!(exchange(&owner, &mut admitted, 3, &[2]).await?, (CommandStatus::DuplicateReplay, vec![2, 2]));
        settle(&h, &owner, &admitted, 1, true).await?;
        assert_eq!(exchange(&owner, &mut admitted, 4, &[]).await?, (CommandStatus::Accepted, vec![2, 2]));
        settle(&h, &owner, &admitted, 2, true).await?;
        assert_eq!(exchange(&owner, &mut admitted, 5, &[1]).await?, (CommandStatus::Accepted, vec![3, 1]));
        let before: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(fixture::id(41).as_slice()).fetch_one(&h.pool).await?;
        settle(&h, &owner, &admitted, 3, true).await?;
        let after: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(fixture::id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(before, after, "selected below own maximum records a no-op");
        assert_eq!(exchange(&owner, &mut admitted, 6, &[]).await?, (CommandStatus::Accepted, vec![3, 1]));
        for level in 3_u8..=12 {
            // The real session ingress requires the next contiguous CommandId.
            // Native death occurrences have their own independent serial namespace.
            let select_command = admitted.continuity.next_command_id;
            assert_eq!(exchange(&owner, &mut admitted, select_command, &[level]).await?,
                (CommandStatus::Accepted, vec![level, level]));
            settle(&h, &owner, &admitted, u64::from(level)+10, true).await?;
            let read_command = admitted.continuity.next_command_id;
            assert_eq!(exchange(&owner, &mut admitted, read_command, &[]).await?,
                (CommandStatus::Accepted, vec![(level+1).min(12), level]));
        }
        let invalid_command = admitted.continuity.next_command_id;
        assert_eq!(exchange(&owner, &mut admitted, invalid_command, &[13]).await?, (CommandStatus::Rejected, vec![]));
        assert_refusals(&h, &owner, &admitted, first).await?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_primal_hazard_receipts WHERE death_key=$1")
            .bind(first.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(count, 1, "one retained result per contributor and native death");
        let access: (bool,bool,bool) = sqlx::query_as("SELECT has_table_privilege('oteryn_game_runtime','game_character_primal_hazard_receipts','INSERT'),has_table_privilege('oteryn_game_runtime','game_character_primal_hazard_receipts','UPDATE'),has_table_privilege('oteryn_game_runtime','game_character_primal_hazard_receipts','DELETE')")
            .fetch_one(&h.pool).await?;
        assert_eq!(access, (true,false,false));
        eprintln!("PRIMAL_HAZARD POSTGRES_CASE_COMPLETED unlock_selection_replay_cap_fences");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn current_fence(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &AdmittedSession)
    -> TestResult<CurrentCharacterGameplayFence>
{
    crate::gameplay_transport::character_gameplay_fence::current(owner, admitted.game_session_id, false)
        .await.ok_or_else(|| "current Hazard fence".into())
}

async fn settle(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    serial: u64,
    consume: bool,
) -> TestResult<[u8;52]> {
    let fence = current_fence(owner, admitted).await?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let RuntimeScopeRefV1::Channel { world_id, channel_id } = fence.runtime_scope else {
        return Err("fixture Channel".into());
    };
    let mut death = [0;52];
    death[..16].copy_from_slice(world_id.as_bytes());
    death[16..32].copy_from_slice(channel_id.as_bytes());
    death[32..40].copy_from_slice(&fence.scope_ownership_generation.get().to_be_bytes());
    death[40..44].copy_from_slice(&7_u32.to_be_bytes());
    death[44..52].copy_from_slice(&serial.to_be_bytes());
    let binding = owner.progression_binding(admitted.game_session_id).ok_or("progression binding")?;
    let permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
    let scope = item_tx::assert_spell_item_scope_in_transaction(&mut tx, &permit, owner.root,
        owner.character, owner.holder, fence.runtime_scope, fence.scope_ownership_generation.get()).await.map_err(debug)?;
    party_kill_xp::commit_in_transaction(&mut tx, &scope, owner.holder, PartyKillXpRequest {
        death_key: death, principal: fence.character_id, creature: "oteryn:creature.the_primal_menace".into(),
        base_xp: 10, death_at_ms: serial, death_at_us: serial*1000,
        shared: false, party_id: None, party_revision: None,
        members: vec![PartyKillXpMember {
            fence,
            request: ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(fixture::id(u8::try_from(serial)?)).map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(10),
                context: binding.context.clone(), policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(), policy: binding.policy.clone(),
            },
            premium: false, forge_dust: None, live_stamina: Some(0),
            shared: false, party_id: None, party_revision: None,
        }],
    }).await.map_err(debug)?;
    if consume {
        apply_hazard_kill_in_transaction(&mut tx, owner.holder, &death,
            "oteryn:creature.the_primal_menace", &[(fence,true)]).await.map_err(debug)?;
    }
    tx.commit().await?;
    slot.observe_endgame_progression();
    Ok(death)
}

async fn assert_refusals(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    first: [u8;52],
) -> TestResult {
    let fence = current_fence(owner, admitted).await?;
    let mut stale = fence;
    stale.character_lease_generation+=1;
    let mut tx = h.pool.begin().await?;
    assert!(apply_hazard_kill_in_transaction(&mut tx, owner.holder, &first,
        "oteryn:creature.the_primal_menace", &[(stale,true)]).await.is_err());
    tx.rollback().await?;
    let mut wrong_scope = first;
    wrong_scope[0]^=1;
    let mut tx = h.pool.begin().await?;
    assert!(apply_hazard_kill_in_transaction(&mut tx, owner.holder, &wrong_scope,
        "oteryn:creature.the_primal_menace", &[(fence,true)]).await.is_err());
    assert!(apply_hazard_kill_in_transaction(&mut tx, owner.holder, &first,
        "oteryn:creature.the_primal_menace", &[(fence,false)]).await.is_err());
    assert!(apply_hazard_kill_in_transaction(&mut tx, owner.holder, &first,
        "oteryn:creature.the_primal_menace", &[(fence,true),(fence,true)]).await.is_err());
    tx.rollback().await?;
    // A genuine retained native death without an unlock cannot become a new grant later.
    let historic = settle(h, owner, admitted, 90, false).await?;
    let fence = current_fence(owner, admitted).await?;
    let mut tx = h.pool.begin().await?;
    assert!(apply_hazard_kill_in_transaction(&mut tx, owner.holder, &historic,
        "oteryn:creature.the_primal_menace", &[(fence,true)]).await.is_err());
    tx.rollback().await?;
    let bad = sqlx::query("UPDATE game_character_hazard SET selected=1 WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).execute(&h.pool).await;
    assert!(bad.is_err(), "unreceipted Hazard mutation must roll back");
    Ok(())
}
