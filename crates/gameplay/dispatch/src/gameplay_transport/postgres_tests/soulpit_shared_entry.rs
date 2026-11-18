//! Real shared entry: a Soul Core debit and fresh Instance session commit together.
//! Crystal 201189f data-global/scripts/quests/soulpit/soulpit_fight.lua:37-106.
use super::soulpit_shared_entry_fixtures::{admit, admit_inner, prepare_source};

use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult};
use crate::durability::activity_handoff::ActivityHandoffOwner;
use crate::durability::character_last_position::LastVitals;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    AuthenticatedTransportRefV1, GameSessionId, GameSessionState,
};
use crate::gameplay_transport::item_fence_of;

#[path = "soulpit_shared_entry/native.rs"]
pub(super) mod native;
#[path = "soulpit_shared_entry/staff.rs"]
mod staff;

#[test]
fn soulpit_shared_entry_debits_core_and_commits_real_instance_session_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("soulpit-shared-entry", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "soulpit_shared_entry", true, false)
                .await?;
            super::reset();
            let evidence = prepare_source(&h).await?;
            let deps = Dependencies::new()?;
            admit(&h, &deps, &evidence).await?;
            h.cleanup().await
        })
    })
}

#[test]
fn soulpit_terminal_return_requires_checkpoint_and_interrupt_never_grants_mastery_postgres()
-> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("soulpit-terminal-return", || {
        run(async |admin| {
            use crate::durability::activity_handoff::{
                ActivityRefusal, ActivityReturnOutcome, ActivityReturnReason, ActivityReturnRequest,
            };
            use crate::durability::soulpit::{SoulpitAction, SoulpitStepRequest};
            let h =
                Harness::create_with_admission_seed(admin, "soulpit_terminal_return", true, false)
                    .await?;
            super::reset();
            let evidence = prepare_source(&h).await?;
            let deps = Dependencies::new()?;
            let admitted = admit(&h, &deps, &evidence).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let owner = ActivityHandoffOwner::new(h.root.clone());
            let source = admitted.destination.item_fence();
            let placement = owner
                .read_return_placement(&authority, &h.node, source)
                .await?
                .ok_or("Soulpit return placement")?;
            let request = ActivityReturnRequest {
                return_id: id(95),
                handoff_id: admitted.handoff_id,
                command_id: None,
                source,
                destination_game_session_id: GameSessionId::decode(&id(94)).map_err(debug)?,
                destination_transport: Some(
                    AuthenticatedTransportRefV1::decode(&[94; 16]).map_err(debug)?,
                ),
                destination_generation: placement.destination_generation,
                reason: ActivityReturnReason::SoulpitTerminal,
                current_vitals: LastVitals::new(140, 40),
                current_rest: None,
            };
            let mut from_client = request.clone();
            from_client.command_id = Some(1);
            assert_eq!(
                owner
                    .return_participant(&authority, &h.node, from_client)
                    .await?,
                ActivityReturnOutcome::Refused(ActivityRefusal::InvalidInput),
                "terminal Soulpit return is a server lifecycle occurrence"
            );
            assert_eq!(
                owner
                    .return_participant(&authority, &h.node, request.clone())
                    .await?,
                ActivityReturnOutcome::Refused(ActivityRefusal::NotDue)
            );
            assert_eq!(
                FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(source.game_session_id)
                    .await?
                    .0
                    .session_state(),
                GameSessionState::Active
            );
            h.root
                .advance_soulpit(
                    &authority,
                    &h.node,
                    SoulpitStepRequest {
                        handoff_id: admitted.handoff_id,
                        occurrence: id(96),
                        runtime_scope: source.runtime_scope,
                        scope_ownership_generation: source.scope_ownership_generation,
                        expected_revision: 0,
                        now_micros: 1,
                        action: SoulpitAction::Interrupt,
                        survivors: vec![],
                    },
                )
                .await
                .map_err(debug)?;
            let returned = owner
                .return_participant(&authority, &h.node, request.clone())
                .await?;
            let ActivityReturnOutcome::Returned(receipt) = returned else {
                return Err(format!("terminal Soulpit return refused: {returned:?}").into());
            };
            assert_eq!(receipt.destination_scope, placement.destination_scope);
            assert!(matches!(
                owner
                    .return_participant(&authority, &h.node, request.clone())
                    .await?,
                ActivityReturnOutcome::Replayed(_)
            ));
            let (masteries,rewards):(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_animus_masteries),(SELECT count(*) FROM game_soulpit_survival_rewards)").fetch_one(&h.pool).await?;
            assert_eq!((masteries, rewards), (0, 0));
            let current = FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(request.destination_game_session_id)
                .await?
                .0;
            assert_eq!(current.session_state(), GameSessionState::Active);
            assert_eq!(
                item_fence_of(current)
                    .ok_or("returned Channel fence")?
                    .runtime_scope,
                placement.destination_scope
            );
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
