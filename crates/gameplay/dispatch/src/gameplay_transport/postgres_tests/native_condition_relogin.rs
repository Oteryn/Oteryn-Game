use super::house_transfer_fixtures::{RecoverySource, prepare_owned_session};
use super::*;
use crate::durability::item_use::ItemUseError;
use crate::gameplay_transport::connection::{
    ConnectionIdentifiers, FirstEntryOutcome, LogoutReply,
};
use crate::gameplay_transport::paced_movement::underwater_drown::{
    DROWN_DEFINITION_KEY, DROWN_SOURCE, drown_definition,
};
use oteryn_gameplay_domain::domain::condition::{ConditionValues, ConflictKey};
use oteryn_gameplay_entities::{ActorConditionTransition, ConditionSource};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind, TickFacts, TickKind,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
use std::time::Duration;

// Sub-ID 0 shares the Recovery slot.
const REGENERATION: ConflictKey = ConflictKey::Recovery;
const DURATION_US: u64 = 60_000_000;
const FIRST_TICK_US: u64 = 2_000_000;

#[test]
fn native_finite_condition_postgres_relogin_preserves_remaining_ticks() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_thread(admin, false)
}

#[test]
fn native_condition_postgres_terminal_save_excludes_environment_and_stale_lease() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_thread(admin, true)
}

fn on_thread(admin: String, with_environment: bool) -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let run = || -> TestResult {
                let rt = crate::bestiary_postgres_harness::runtime()?;
                let h = rt.block_on(Harness::create_with_admission_seed(
                    admin,
                    "native_condition_relogin",
                    true,
                    false,
                ))?;
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    rt.block_on(scenario(&h, with_environment))
                }));
                let cleanup = rt.block_on(h.cleanup());
                match result {
                    Ok(result) => {
                        result?;
                        cleanup
                    }
                    Err(panic) => std::panic::resume_unwind(panic),
                }
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "native condition PostgreSQL thread panicked")??;
    Ok(())
}

struct FreshIds;
impl ConnectionIdentifiers for FreshIds {
    fn game_session_id(&self) -> Option<GameSessionId> {
        GameSessionId::decode(&id(91)).ok()
    }

    fn transport_ref(&self) -> Option<crate::foundation::AuthenticatedTransportRefV1> {
        crate::foundation::AuthenticatedTransportRefV1::decode(&[91; 16]).ok()
    }
}

async fn scenario(h: &Harness, with_environment: bool) -> TestResult {
    let recovery = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let mut owner = deps.owner(h, &authority);
    owner.evidence = &recovery.evidence;
    owner
        .runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    deps.initialize(&owner).await;
    let mut admitted = deps.admitted;
    let session = admitted.game_session_id;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(session)
        .await?;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let old_fence = admitted.item_fence.ok_or("old lease fence")?;
    let applied_at = install_conditions(&owner, &deps, with_environment).await?;
    owner.clock_origin -= Duration::from_millis(750);
    let _clock = owner.freeze_owner_clock_for_test();
    let elapsed_at_save = owner.owner_now().get() - applied_at;
    assert_eq!(
        owner.logout_session(admitted).await,
        LogoutReply::Result(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    );
    let saved = h
        .root
        .read_character_conditions(&authority, old_fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("terminal release snapshot")?;
    assert_eq!(saved.lease_generation, old_fence.character_lease_generation);
    let rows = saved.conditions["conditions"]
        .as_array()
        .ok_or("saved conditions")?;
    if with_environment {
        assert!(
            rows.iter()
                .all(|row| row["definition"]["key"] != DROWN_DEFINITION_KEY),
            "drowning is re-established by the map"
        );
    }
    assert_eq!(
        rows.len(),
        1,
        "terminal release saves the eligible native condition"
    );
    let ends_in = rows[0]["ends_at"].as_u64().ok_or("remaining duration")?;
    assert_eq!(ends_in, DURATION_US - elapsed_at_save);

    let fresh_deps = Dependencies::new()?;
    let mut fresh_owner = fresh_deps.owner(h, &authority);
    fresh_owner.evidence = &recovery.evidence;
    let token = RecoverySource::fresh_token(91)?;
    let frame = oteryn_protocol_oteryn::encode_client_bootstrap(
        &oteryn_protocol_oteryn::ClientBootstrapValue {
            schema_revision: 1,
            character_id: crate::foundation::CharacterId::decode(old_fence.character_id.as_bytes())
                .map_err(debug)?,
            admission_material: token.as_bytes(),
            client_build_id: "native-condition-relogin",
            supported_capabilities: &[],
        },
    )
    .map_err(debug)?;
    let (mut server, _client) = tokio::io::duplex(1 << 20);
    let fresh = crate::gameplay_transport::connection::admit_frame(
        &mut server,
        &frame,
        &fresh_owner,
        &FreshIds,
    )
    .await
    .map_err(debug)?;
    let fresh_fence = fresh.item_fence.ok_or("fresh lease fence")?;
    assert!(fresh_fence.character_lease_generation > saved.lease_generation);
    let restored = {
        let runtime = fresh_deps.runtime.lock().await;
        let states = fresh_owner.states.spell_states.lock().await;
        states
            .get(
                &runtime,
                fresh.runtime_actor.ok_or("fresh actor")?,
                fresh.game_session_id,
            )
            .ok_or("fresh spell state")?
            .owned_conditions()
            .clone()
    };
    verify_restored_phase(restored, ends_in)?;

    if with_environment {
        let mut replacement = saved.clone();
        replacement.conditions = serde_json::json!({"schema": 1, "conditions": []});
        assert!(matches!(
            h.root
                .save_character_conditions(
                    &authority,
                    &h.node,
                    old_fence,
                    *fresh.channel_id.as_bytes(),
                    replacement,
                )
                .await,
            Err(ItemUseError::AuthorityRejected)
        ));
        let current = h
            .root
            .read_character_conditions(&authority, old_fence.character_id)
            .await
            .map_err(debug)?
            .ok_or("snapshot after stale save")?;
        assert_eq!(current.lease_generation, saved.lease_generation);
        assert_eq!(current.conditions, saved.conditions);
    }
    Ok(())
}

async fn install_conditions(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    with_environment: bool,
) -> TestResult<u64> {
    let session = deps.admitted.game_session_id;
    let mut runtime = deps.runtime.lock().await;
    let now = owner.owner_now();
    let root = GameplayDecisionRoot::from_bytes([8; 32]);
    let facts = ApplicationFacts {
        now: now.get(),
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([8; 16]),
    };
    let definition = ConditionDefinition::new(
        "fixture:c08.native.regeneration",
        1,
        ConditionValues::SpellRegeneration {
            duration_ms: 60_000,
            sub_id: 0,
            health_gain: 3,
            health_interval_ms: 2_000,
            mana_gain: 2,
            mana_interval_ms: 3_000,
        },
    )
    .ok_or("native regeneration definition")?;
    let transition = ActorConditionTransition::Apply {
        definition: &definition,
        source: ConditionSource {
            actor: deps.actor,
            session: Some(session),
            kind: ConditionSourceKind::SelfUse,
        },
        immunities: &[],
        facts,
    };
    let plan = runtime
        .prepare_actor_condition(deps.actor, Some(session), transition, now)
        .map_err(debug)?;
    assert!(runtime.commit_actor_condition(&plan, now).map_err(debug)?);
    let native = runtime
        .actor_conditions(deps.actor, Some(session))
        .map_err(debug)?;
    assert_eq!(
        native
            .get(REGENERATION)
            .ok_or("native regeneration")?
            .provenance()
            .source,
        Some(deps.actor)
    );
    if with_environment {
        let mut states = owner.states.spell_states.try_lock().map_err(debug)?;
        let state = states
            .get_mut(&runtime, deps.actor, session)
            .ok_or("source spell state")?;
        let before = state.owned_conditions().clone();
        let mut next = before.clone();
        next.apply(
            &drown_definition().ok_or("drowning definition")?,
            Some(DROWN_SOURCE.to_owned()),
            ConditionSourceKind::Field,
            &[],
            &facts,
        )
        .map_err(debug)?;
        state
            .apply_batch_conditions(&before, &next)
            .map_err(debug)?;
    }
    Ok(now.get())
}

fn verify_restored_phase(
    mut conditions: oteryn_gameplay_foundation::foundation::condition::ConditionStore<String>,
    ends_in: u64,
) -> TestResult {
    let restored = conditions
        .get(REGENERATION)
        .ok_or("restored native regeneration")?;
    assert!(restored.provenance().is_unattributed());
    let elapsed_at_save = DURATION_US - ends_in;
    assert!((750_000..FIRST_TICK_US).contains(&elapsed_at_save));
    let first_tick = restored.started_at() + FIRST_TICK_US - elapsed_at_save;
    assert_eq!(
        restored.remaining_duration_ms(restored.started_at()),
        Some((ends_in / 1000).try_into()?)
    );
    assert!(
        conditions
            .take_due(first_tick - 1, TickFacts::default())
            .is_empty()
    );
    let ticks = conditions.take_due(first_tick, TickFacts::default());
    assert_eq!(ticks.len(), 1);
    assert_eq!(
        ticks[0].kind,
        TickKind::SpellRegeneration {
            health_gain: 3,
            mana_gain: 0,
            suppressed: false,
        }
    );
    let ticks = conditions.take_due(first_tick + 1_000_000, TickFacts::default());
    assert_eq!(ticks.len(), 1);
    assert_eq!(
        ticks[0].kind,
        TickKind::SpellRegeneration {
            health_gain: 0,
            mana_gain: 2,
            suppressed: false,
        }
    );
    Ok(())
}
