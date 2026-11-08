//! Real fixed-name negative proof survives neither canonical control loss nor a stale admission.
use super::*;
use crate::gameplay_transport::connection::ControlLossResult;

#[test]
fn srx3_source_names_zero_match_requires_current_negative_admission_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("srx3-source-names-negative", || {
        run(async |admin| {
            super::reset();
            let h = Harness::create_with_admission_seed(
                admin,
                "srx3_source_names_negative",
                false,
                false,
            )
            .await?;
            prepare_current_session(&h).await?;
            h.seed_appearance_observer().await?;
            super::appearance_change_postgres::observer_visibility_tests::admit(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            deps.initialize(&owner).await;
            let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?;
            assert_eq!(
                current.session_state(),
                crate::foundation::GameSessionState::Active
            );
            let other_actor =
                super::appearance_change_postgres::observer_visibility_tests::initialize(
                    &deps, &owner,
                )
                .await?;
            let other_session = GameSessionId::decode(&id(110)).map_err(debug)?;
            let (other_current, _) = FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(other_session)
                .await?;
            let other_account: Vec<u8> = sqlx::query_scalar(
                "SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1",
            ).bind(other_current.commit().character_id().as_bytes().as_slice())
                .fetch_one(&h.pool).await?;
            let other_controller = ControllerBinding {
                transport: other_current
                    .current_transport()
                    .ok_or("real observer transport")?,
                account_id: other_account
                    .try_into()
                    .map_err(|_| "actual observer account UUID")?,
            };
            let plan = {
                let mut runtime = owner.runtime.lock().await;
                runtime
                    .initialize_first_entry_position(deps.actor)
                    .map_err(debug)?;
                runtime
                    .bind_attacker_lease(
                        deps.actor,
                        deps.admitted.game_session_id,
                        current.current_character_lease(),
                    )
                    .map_err(debug)?;
                assert_eq!(
                    runtime
                        .bound_attacker_lease(other_actor, other_session)
                        .map_err(debug)?,
                    other_current.current_character_lease()
                );
                owner
                    .prepare_source_names(&runtime, &["shadow tentacle", "possessed tree"])
                    .map_err(|_| "capture actual negative-name admission")?
            };
            let permit = owner
                .spell_lane_permit()
                .await
                .ok_or("actual Node scope lane")?;
            let zero = owner
                .read_source_names(&permit, &plan)
                .await
                .map_err(|_| "actual fixed-name SQL over current admission")?;
            {
                let runtime = owner.runtime.lock().await;
                let names = owner
                    .seal_source_names(&runtime, zero)
                    .map_err(|_| "reprove genuine zero-name cohort")?;
                assert!(
                    names.is_empty(),
                    "actual admitted Player matches neither source guard name"
                );
            }
            drop(permit);
            // Revoke only the genuinely admitted OTHER Player. The requester
            // remains ACTIVE, so its current Node/scope reader authority stays
            // valid and the negative source-name query is actually reached.
            assert_eq!(
                owner
                    .commit_control_loss(other_session, other_actor, other_controller)
                    .await,
                ControlLossResult::Recorded,
            );
            let (lost, _) = FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(other_session)
                .await?;
            assert_eq!(
                lost.session_state(),
                crate::foundation::GameSessionState::Reconnectable
            );
            assert!(lost.current_control_loss_epoch().is_some());
            assert!(lost.current_transport().is_none());
            let (requester, _) = FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?;
            assert_eq!(
                requester.session_state(),
                crate::foundation::GameSessionState::Active
            );
            {
                let runtime = owner.runtime.lock().await;
                assert!(
                    runtime.read_actor_position(other_actor).is_ok(),
                    "real loss retains the original OTHER physical actor"
                );
            }
            let permit = owner
                .spell_lane_permit()
                .await
                .ok_or("retained Node scope lane")?;
            eprintln!("SRX3_SOURCE_NAMES_ACTUAL_NEGATIVE_ADMISSION_REVOKED");
            let stale = owner.read_source_names(&permit, &plan).await;
            assert!(
                stale.is_err(),
                "zero fixed-name matches cannot turn a superseded canonical admission into proved absence"
            );
            drop(permit);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
