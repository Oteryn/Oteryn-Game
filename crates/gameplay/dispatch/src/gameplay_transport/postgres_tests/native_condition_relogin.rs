use super::house_transfer_fixtures::{RecoverySource, prepare_owned_session};
use super::*;
use crate::durability::item_use::ItemUseError;
use crate::gameplay_transport::connection::{
    ConnectionIdentifiers, FirstEntryOutcome, FreshAdmissionAuthority, LogoutReply,
};
use crate::gameplay_transport::paced_movement::underwater_drown::{
    DROWN_DEFINITION_KEY, DROWN_SOURCE, drown_definition,
};
use crate::gameplay_transport::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
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
    on_thread(admin, false, None)
}

#[test]
fn native_condition_postgres_terminal_save_excludes_environment_and_stale_lease() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_thread(admin, true, None)
}

#[test]
fn spell_cooldowns_postgres_relogin_keeps_active_deadlines_and_refuses_recast() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_thread(admin, false, Some(false))
}

#[test]
fn spell_cooldowns_postgres_relogin_drops_expired_deadlines() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_thread(admin, false, Some(true))
}

#[test]
fn native_condition_relogin_retries_its_busy_read_in_place_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("relog-conditions-holder", move || {
        let runtime = crate::bestiary_postgres_harness::runtime()?;
        runtime.block_on(async {
            let h = Harness::create_with_admission_seed(admin, "relog_conditions_holder", true, false).await?;
            let result = scenario(&h, false, None, true).await;
            let cleanup = h.cleanup().await;
            result?;
            cleanup
        })
    })
}

fn on_thread(admin: String, with_environment: bool, expire_cooldowns: Option<bool>) -> TestResult {
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
                    rt.block_on(scenario(&h, with_environment, expire_cooldowns, false))
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

async fn scenario(
    h: &Harness,
    with_environment: bool,
    expire_cooldowns: Option<bool>,
    contend_conditions: bool,
) -> TestResult {
    use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;
    use oteryn_protocol_oteryn::presentation::{self, ActorCooldowns};
    let cast = crate::gameplay_transport::actor_spell::tests::exura();
    let recovery = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    if expire_cooldowns.is_some() {
        // Use a fixture spell window long enough to cover durable release and fresh admission.
        let spell = &mut deps.spells.spells[cast.spell.get() as usize - 1];
        spell.learning_required = false;
        spell.vocations.insert(crate::spell::Vocation::None);
        spell.cooldown_micros = 60_000_000;
        for group in &mut spell.groups {
            group.cooldown_micros = spell.cooldown_micros;
        }
    }
    let mut owner = deps.owner(h, &authority);
    owner.evidence = &recovery.evidence;
    let mut admitted = deps.admitted;
    let session = admitted.game_session_id;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(session)
        .await?;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let old_fence = admitted.item_fence.ok_or("old lease fence")?;
    // Logout checkpoints Monk state, so the actor must use its durable vocation and values.
    let character = old_fence.character_id;
    let facts = match load_character_cast_facts(&h.root, &authority, character).await {
        CastFactsLoad::Ready { facts, .. } | CastFactsLoad::NoVocation { facts, .. } => facts,
        CastFactsLoad::Unavailable => return Err("fixture character facts".into()),
    };
    let monk = owner
        .load_monk_state(character)
        .await
        .ok_or("fixture monk state")?;
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        owner
            .states
            .spell_states
            .try_lock()
            .map_err(debug)?
            .initialize(
                &runtime,
                deps.actor,
                session,
                facts,
                monk,
                owner.owner_now(),
            )
            .ok_or("fixture spell state")?;
    }
    let before: Option<ActorCooldowns> = if expire_cooldowns.is_some() {
        assert_eq!(
            owner.cast_spell(deps.actor, session, 1, cast).await.disposition,
            SpellCastDisposition::Cast
        );
        let cooldowns = owner
            .observe_actor_cooldowns(deps.actor, session)
            .await
            .ok_or("paid cooldowns")?;
        for kind in [presentation::ActorCooldownKind::Spell, presentation::ActorCooldownKind::Group] {
            assert!(cooldowns.entries.iter().any(|entry| entry.kind == kind));
        }
        Some(cooldowns)
    } else {
        None
    };
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

    if expire_cooldowns == Some(true) {
        let persisted: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT snapshot->'spell_cooldowns_unix_us'
             FROM game_character_condition_snapshots WHERE character_id = encode($1, 'hex')::uuid",
        )
        .bind(character.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        let entries = persisted.as_ref().and_then(serde_json::Value::as_array)
            .ok_or("logout must save active spell and group deadlines")?;
        assert_eq!(entries.len(), before.as_ref().ok_or("paid entries")?.entries.len());
        let expired = serde_json::Value::Array(
            entries.iter().map(|row| serde_json::json!([row[0], 1])).collect(),
        );
        sqlx::query("UPDATE game_character_condition_snapshots
            SET snapshot = jsonb_set(snapshot, '{spell_cooldowns_unix_us}', $1)
            WHERE character_id = encode($2, 'hex')::uuid")
            .bind(expired)
            .bind(character.as_bytes().as_slice())
            .execute(&h.pool)
            .await?;
    }
    let mut fresh_deps = Dependencies::new()?;
    fresh_deps.spells = deps.spells.clone();
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
    let admit = crate::gameplay_transport::connection::admit_frame(
        &mut server,
        &frame,
        &fresh_owner,
        &FreshIds,
    );
    let mut fresh = if contend_conditions {
        use crate::gameplay_transport::composed_admission::RELOG_CONDITIONS_BARRIER;
        let barrier = std::sync::Arc::new((tokio::sync::Notify::new(), tokio::sync::Notify::new()));
        let hold = async {
            barrier.0.notified().await;
            let held = h.root.try_issue_semantic_pass()?;
            barrier.1.notify_one();
            tokio::time::sleep(Duration::from_millis(200)).await;
            held.run(|_, _| Box::pin(async { Ok(()) })).await
        };
        let (admitted, held) = tokio::time::timeout(Duration::from_secs(15), async {
            tokio::join!(RELOG_CONDITIONS_BARRIER.scope(barrier.clone(), admit), hold)
        }).await?;
        held?;
        // A full first-entry retry would hit the barrier a second time and time out.
        admitted.map_err(debug)?
    } else {
        admit.await.map_err(debug)?
    };
    assert_ne!(fresh.game_session_id, session);
    assert!(
        matches!(
            fresh.first_entry,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        ),
        "fresh first entry: {:?}",
        fresh.first_entry
    );
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

    if let (Some(expired), Some(before)) = (expire_cooldowns, before) {
        // The legacy fixture has no activated map-presentation source to advertise.
        // Supply the negotiated pair for the transport half of the relog probe.
        use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
        use oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES;
        let spatial = CAPABILITY_WORLD_SPATIAL_ENTITIES;
        let present = presentation::CAPABILITY_PRESENTATION_V1;
        fresh.continuity.selected_capabilities = SelectedCapabilities::select(
            &[
                OfferedCapability { id: spatial, requires: &[] },
                OfferedCapability {
                    id: present,
                    requires: &presentation::CAPABILITY_PRESENTATION_V1_REQUIRES,
                },
            ],
            &[spatial, present],
        ).ok_or("fixture presentation selection")?;
        let cooldowns = login_cooldowns(&fresh_owner, fresh).await?;
        if expired {
            assert!(cooldowns.entries.is_empty());
        } else {
            assert_eq!(cooldowns.entries.len(), before.entries.len());
            for entry in &cooldowns.entries {
                let previous = before.entries.iter()
                    .find(|old| old.kind == entry.kind && old.id == entry.id)
                    .ok_or("restored cooldown key")?;
                let remaining = entry.expires_at_ms.saturating_sub(cooldowns.server_now_ms);
                assert!(remaining > 0);
                assert!(remaining < previous.expires_at_ms - before.server_now_ms);
            }
            let recast = fresh_owner
                .cast_spell(fresh.runtime_actor.ok_or("fresh actor")?, fresh.game_session_id, 1, cast)
                .await;
            assert_eq!(recast.disposition, SpellCastDisposition::CoolingDown);
            assert!(recast.vitals.is_none());
        }
        eprintln!("SPELL_COOLDOWNS_RELOGIN POSTGRES_CASE_COMPLETED expired={expired}");
    }

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

async fn login_cooldowns(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    fresh: AdmittedSession,
) -> TestResult<oteryn_protocol_oteryn::presentation::ActorCooldowns> {
    use oteryn_protocol_oteryn::{self as wire, presentation};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.shutdown().await?;
    crate::gameplay_transport::connection::serve_admitted(
        &mut server, fresh, owner, crate::gameplay_transport::connection::IDLE_LIVENESS,
    ).await;
    drop(server);
    let mut body = Vec::new();
    while let Ok(length) = client.read_u32().await {
        let mut bytes = vec![0; usize::try_from(length)?];
        client.read_exact(&mut bytes).await?;
        let frame = crate::foundation::decode_wire_envelope(&bytes)?;
        if frame.message_type() == crate::foundation::MessageType::SnapshotChunk {
            body.extend_from_slice(wire::decode_snapshot_chunk_framing(frame.payload())?.1);
        }
    }
    let snapshot = wire::decode_snapshot_body(&body)?.into_iter()
        .find(|row| row.domain_id == presentation::STATE_DOMAIN_ACTOR_COOLDOWNS)
        .ok_or("login cooldown domain")?;
    assert_eq!(snapshot.snapshot_type, presentation::SNAPSHOT_TYPE_ACTOR_COOLDOWNS_V1);
    presentation::decode_actor_cooldowns(snapshot.payload).map_err(debug)
}
