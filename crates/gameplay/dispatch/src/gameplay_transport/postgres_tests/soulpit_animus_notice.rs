//! The terminal adapter publishes only the committed survivor credit.
use super::fixture::{Harness, debug, id, run};
use super::soulpit_shared_entry_fixtures::{admit, prepare_source};
use super::{Dependencies, TestResult};
use crate::domain::soulpit::{EncounterEffect, soulpit_entry_monster_name};
use crate::durability::soulpit::{SoulpitAction, SoulpitStepRequest};
use crate::foundation::GameSessionId;
use crate::gameplay_transport::capabilities::{PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::zones_nologout::{CAPABILITY_ZONES_NOLOGOUT_V1, ServerMessage};

#[test]
fn soulpit_postgres_committed_animus_notice_reaches_only_surviving_sessions() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, false)
}

#[test]
fn soulpit_postgres_completion_retry_does_not_duplicate_animus_notice() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, true)
}

fn scenario(admin: String, replay: bool) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("soulpit-animus-notice", move || {
        run(async move |_| {
            super::reset();
            let h = Harness::create_with_admission_seed(admin, "soulpit_animus_notice", true, false)
                .await?;
            let evidence = prepare_source(&h).await?;
            let deps = Dependencies::new()?;
            let admitted = admit(&h, &deps, &evidence).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            let survivor = admitted.destination;
            let (content, _) =
                crate::content::native_gameplay::test_support::activated_with_loot_tables(
                    survivor.runtime_scope.world_id(),
                    None,
                );
            owner.active_generation = content.active();
            let initial = h
                .root
                .read_soulpit_encounter(
                    &authority,
                    &h.node,
                    admitted.handoff_id,
                    survivor.runtime_scope,
                    survivor.scope_ownership_generation,
                )
                .await
                .map_err(debug)?
                .ok_or("admitted Soulpit")?;
            let (key, revision): (String, String) = sqlx::query_as(
                "SELECT definition_production_key,definition_revision_ref FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
            )
            .bind(id(71).as_slice())
            .fetch_one(&h.pool)
            .await?;
            let source_id = owner
                .active_generation
                .and_then(crate::content::ActiveGeneration::native_gameplay)
                .and_then(|native| native.item_policy(&key, &revision))
                .ok_or("burned Core definition")?
                .record()
                .production_binding
                .external_id
                .parse()?;
            let name = soulpit_entry_monster_name(source_id).ok_or("burned Core species name")?;
            let expected = ServerMessage::Advance(format!(
                "You have defeated the core of the {name} soul and unlocked its animus mastery!"
            ));

            let session = survivor.game_session_id;
            let generation = survivor.connection_generation.get();
            let bystander = GameSessionId::decode(&id(94)).map_err(debug)?;
            let mut connected = deps.admitted;
            connected.game_session_id = session;
            connected.item_fence = Some(survivor.item_fence());
            connected.continuity.connection_generation = generation;
            connected.continuity.selected_capabilities = SelectedCapabilities::select(
                PRODUCTION_OFFERED_CAPABILITIES,
                &[21, CAPABILITY_ZONES_NOLOGOUT_V1],
            )
            .ok_or("server messages selected")?;
            let messages = &owner.states.server_messages;
            assert!(owner.server_messages_joined(connected));
            assert!(messages.join(bystander, 1));

            let mut step = SoulpitStepRequest {
                handoff_id: admitted.handoff_id,
                occurrence: id(150),
                runtime_scope: survivor.runtime_scope,
                scope_ownership_generation: survivor.scope_ownership_generation,
                expected_revision: initial.revision,
                now_micros: initial.encounter.snapshot().started_at_micros,
                action: SoulpitAction::Advance,
                survivors: vec![survivor],
            };
            let completion = prepare_completion(&owner, &initial.species, &mut step).await?;
            assert_eq!(messages.take(session, generation), None);

            // An incomplete survivor set cannot commit the event or publish a notice.
            let mut refused = completion.clone();
            refused.survivors.clear();
            assert!(
                owner.commit_soulpit_progress(&initial.species, refused).await.is_err()
            );
            assert_eq!(messages.take(session, generation), None);
            assert_eq!(messages.take(bystander, 1), None);

            let committed = owner
                .commit_soulpit_progress(&initial.species, completion.clone())
                .await
                .map_err(debug)?;
            assert!(!committed.replayed);
            assert_eq!(
                committed.effect,
                EncounterEffect::Completed { surviving_slots: vec![0] }
            );
            let credits: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_soulpit_survival_rewards WHERE occurrence_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND species_key=$3",
            )
            .bind(completion.occurrence.as_slice())
            .bind(survivor.character_id.as_bytes().as_slice())
            .bind(&initial.species)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(credits, 1);
            assert_eq!(messages.take(bystander, 1), None);
            let recipient = if replay {
                connected
            } else {
                // The cadence can return the player before their connection drains
                // the queue. It must still be the same authenticated controller.
                let mut returned = connected;
                returned.game_session_id = GameSessionId::decode(&id(95)).map_err(debug)?;
                messages.transfer_connection(connected, returned)
                    .map_err(|()| "transfer live server-message recipient")?;
                assert_eq!(owner.take_server_notice(connected), None);
                returned
            };
            assert_eq!(owner.take_server_notice(recipient), Some(expected));

            if replay {
                let retried = owner
                    .commit_soulpit_progress(&initial.species, completion)
                    .await
                    .map_err(debug)?;
                assert!(retried.replayed);
                assert_eq!(retried.effect, committed.effect);
            }
            assert_eq!(owner.take_server_notice(recipient), None);

            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

async fn prepare_completion(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    species: &str,
    step: &mut SoulpitStepRequest,
) -> TestResult<SoulpitStepRequest> {
    let mut occurrence = 150_u8;
    let started = step.now_micros;
    for wave in 1_u8..=4 {
        let start = started + u64::from(wave - 1) * 5_000_000;
        let monster_ids = [1_u64, 2, 3, 4, 5, 6, 7].map(|slot| Some(u64::from(wave) * 100 + slot));
        let mut actions = vec![
            (SoulpitAction::Advance, start + 4_000_000),
            (SoulpitAction::Spawned { wave, monster_ids }, start + 4_000_000),
        ];
        actions.extend(monster_ids.into_iter().flatten().map(|monster_id| {
            (SoulpitAction::Defeated { wave, monster_id }, start + 4_500_000)
        }));
        actions.push((SoulpitAction::Advance, start + 5_000_000));
        let completion = wave == 4;
        let action_count = actions.len();
        for (index, (action, now_micros)) in actions.into_iter().enumerate() {
            step.action = action;
            step.now_micros = now_micros;
            step.occurrence = id(occurrence);
            if completion && index + 1 == action_count {
                return Ok(step.clone());
            }
            let outcome = owner.commit_soulpit_progress(species, step.clone()).await.map_err(debug)?;
            assert!(!outcome.replayed);
            step.expected_revision = outcome.revision;
            occurrence += 1;
        }
    }
    Err("fourth wave did not produce a completion request".into())
}
