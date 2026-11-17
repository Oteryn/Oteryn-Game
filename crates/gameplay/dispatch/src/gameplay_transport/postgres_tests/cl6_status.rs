//! CL6: actual composed query, existing durable owners, and current-session refusals.
use super::*;
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use oteryn_protocol_oteryn::mechanics_status::{
    CAPABILITY_MECHANICS_STATUS_V1, COMMAND_TYPE_MECHANICS_STATUS_QUERY, MechanicsStatus,
};

#[test]
fn cl6_status_postgres_real_dispatch_existing_owners_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(|| {
            scenario(false)
                .and_then(|()| scenario(true))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "CL6 PostgreSQL thread panicked")??;
    Ok(())
}

fn scenario(pvp: bool) -> TestResult {
    let started = std::time::Instant::now();
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "mechanics_status", true, false).await?;
        eprintln!(
            "POSTGRES_STARTED CL6 pvp={pvp} cl6_status_postgres_real_dispatch_existing_owners_and_fences"
        );
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        deps.admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &[OfferedCapability {
                id: CAPABILITY_MECHANICS_STATUS_V1,
                requires: &[],
            }],
            &[CAPABILITY_MECHANICS_STATUS_V1],
        )
        .ok_or("capability")?;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let (controller, _) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        assert!(crate::gameplay_transport::connection::FreshAdmissionAuthority::mechanics_status_available(&owner));
        deps.initialize(&owner).await;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        let fence = admitted.item_fence.ok_or("fence")?;
        deps.runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let now = owner.owner_now().get();
        {
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.try_lock().map_err(debug)?;
            let state = states
                .get_mut(&runtime, deps.actor, admitted.game_session_id)
                .ok_or("actor")?;
            *state = state.after_food(180, now / 1_000).ok_or("food")?;
        }
        assert!(
            crate::gameplay_transport::battle_instinct_status::actor_is_current(
                &*deps.runtime.lock().await,
                admitted
            ),
            "fixture runtime must match the canonical current fence"
        );
        owner
            .root
            .read_mechanics_status(owner.character, owner.holder, fence)
            .await
            .map_err(debug)?;
        let command_id = admitted.continuity.next_command_id;
        let (status, bytes) = super::cyclopedia::support::exchange(
            &owner,
            &mut admitted,
            command_id,
            COMMAND_TYPE_MECHANICS_STATUS_QUERY,
            &[],
        )
        .await?;
        assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
        let projection = MechanicsStatus::decode(&bytes).ok_or("CL6 decode")?;
        assert_eq!((projection.dust, projection.dust_limit), (0, 100));
        assert!(projection.deaths.is_empty());
        assert!(projection.members.is_empty());
        let gameplay_fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("CL6 current gameplay fence")?;
        // This is the real dust ledger writer; no Forge operation/history exists.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
        crate::durability::character_forge_dust::gain_forge_dust_in_transaction(
            &mut tx,
            &gameplay_fence,
            crate::durability::character_forge_dust::ForgeDustGainCause::CreatureKill {
                occurrence: id(200),
            },
            73,
            crate::durability::character_forge_dust::ForgeDustWriteIds {
                entry_id: id(201),
                transaction_id: id(202),
            },
        )
        .await
        .map_err(debug)?;
        tx.commit().await?;
        let before_revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        let command_id = admitted.continuity.next_command_id;
        let (status, bytes) = super::cyclopedia::support::exchange(
            &owner,
            &mut admitted,
            command_id,
            COMMAND_TYPE_MECHANICS_STATUS_QUERY,
            &[],
        )
        .await?;
        assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
        assert_eq!(
            MechanicsStatus::decode(&bytes).ok_or("dust decode")?.dust,
            73
        );
        let after_revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            before_revision, after_revision,
            "read cannot mutate Character revision"
        );
        let mut request = crate::gameplay_transport::death_ml_loss_postgres::death(90)?;
        // Held blessing seed matches the existing death fixture. The real receipt writer
        // validates and consumes it with all normal production triggers enabled.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,'oteryn:blessing.spiritual_shielding','fixture'),(encode($1,'hex')::uuid,'oteryn:blessing.twist_of_fate','fixture')")
            .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        request.held_blessings = vec![
            "oteryn:blessing.spiritual_shielding".into(),
            "oteryn:blessing.twist_of_fate".into(),
        ];
        if pvp {
            super::fixture::pvp_source_death_fixture::seed_root(&h.pool, 41, 60, 42).await?;
            sqlx::query("INSERT INTO game_character_pvp_state(character_id,world_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
                .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&h.pool).await?;
            super::fixture::pvp_source_death_fixture::seed_ledger(
                &h.pool,
                41,
                60,
                true,
                Some(1),
                "0 seconds",
            )
            .await?;
            request.pvp_contributors = vec![
                crate::durability::pvp_death_consequences::PvpDeathContributor {
                    character_id: crate::domain::CharacterId::from_bytes(id(60)).map_err(debug)?,
                    kind: crate::durability::pvp_death_consequences::PvpContributorKind::Damage,
                    unjustified: true,
                },
            ];
        }
        let receipt = owner
            .settle_source_player_death(gameplay_fence, request, None, None)
            .await
            .map_err(debug)?;
        let command_id = admitted.continuity.next_command_id;
        let (status, bytes) = super::cyclopedia::support::exchange(
            &owner,
            &mut admitted,
            command_id,
            COMMAND_TYPE_MECHANICS_STATUS_QUERY,
            &[],
        )
        .await?;
        assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
        let status = MechanicsStatus::decode(&bytes).ok_or("death decode")?;
        let expected: Vec<_> = receipt
            .blessings_before
            .iter()
            .filter(|key| !receipt.blessings_after.contains(key))
            .cloned()
            .collect();
        assert_eq!(status.deaths.len(), 1);
        assert_eq!(status.deaths[0].consumed, expected);
        assert_eq!(
            status.deaths[0].consumed,
            [if pvp {
                "oteryn:blessing.twist_of_fate"
            } else {
                "oteryn:blessing.spiritual_shielding"
            }]
        );
        let mut stale = admitted;
        stale
            .item_fence
            .as_mut()
            .ok_or("fence")?
            .character_lease_generation += 1;
        assert!(owner.mechanics_status_page(stale).await.is_none());
        stale = admitted;
        stale.continuity.connection_generation += 1;
        assert!(owner.mechanics_status_page(stale).await.is_none());
        stale = admitted;
        stale.runtime_actor = None;
        assert!(owner.mechanics_status_page(stale).await.is_none());
        stale = admitted;
        stale.game_session_id = crate::foundation::GameSessionId::decode(&id(99)).map_err(debug)?;
        assert!(owner.mechanics_status_page(stale).await.is_none());
        stale = admitted;
        stale.continuity.selected_capabilities = SelectedCapabilities::NONE;
        assert!(owner.mechanics_status_page(stale).await.is_none());
        eprintln!(
            "CL6_STATUS POSTGRES_CASE_COMPLETED pvp={pvp} elapsed={:?}",
            started.elapsed()
        );
        Ok(())
    })
}

pub(super) async fn query_actor(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> TestResult<MechanicsStatus> {
    use crate::gameplay_transport::connection::{
        AdmittedSession, FirstEntryOutcome, SessionContinuity,
    };
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("CL6 current fence")?
        .item_fence();
    let mut continuity = SessionContinuity::FRESH;
    continuity.connection_generation = fence.connection_generation.get();
    continuity.selected_capabilities = SelectedCapabilities::select(
        &[OfferedCapability {
            id: CAPABILITY_MECHANICS_STATUS_V1,
            requires: &[],
        }],
        &[CAPABILITY_MECHANICS_STATUS_V1],
    )
    .ok_or("CL6 capability")?;
    let admitted = AdmittedSession {
        game_session_id: session,
        world_id: owner.world_id,
        channel_id: owner.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity,
        item_fence: Some(fence),
    };
    MechanicsStatus::decode(
        &owner
            .mechanics_status_page(admitted)
            .await
            .ok_or("CL6 projection")?,
    )
    .ok_or_else(|| "CL6 projection decode".into())
}
pub(super) async fn check_timer(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    key: &str,
    active: bool,
    max_ms: u64,
) -> TestResult {
    let status = query_actor(owner, actor, session).await?;
    let timer = status.timers.iter().find(|timer| timer.key == key);
    if active {
        let timer = timer.ok_or("CL6 active timer")?;
        assert!((1..=max_ms).contains(&timer.remaining_ms));
        assert_eq!(timer.online_only, key == "wealth_duplex");
    } else {
        assert!(timer.is_none(), "expired/instant timer: {key}");
    }
    eprintln!("CL6_TIMER POSTGRES_ASSERTION_EXECUTED key={key} active={active}");
    Ok(())
}
