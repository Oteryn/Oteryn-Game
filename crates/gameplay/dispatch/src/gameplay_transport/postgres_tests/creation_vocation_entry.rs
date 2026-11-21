//! A v4 birth starts without progression; first entry supplies it before Familiar Login.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, fresh_fixture};
use crate::durability::admission_authority_guards::AdmissionGuardStore;
use crate::durability::character_familiar::{
    FamiliarCooldownSnapshot, FamiliarLifecycleV2, FamiliarLifecycleWriter,
    FamiliarStateOccurrence, FamiliarStateOutcome, FamiliarStateRequest,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::ExactActorRef;
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityPublicationV1,
};
use crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1;
use crate::foundation::{AuthenticatedTransportRefV1, GameSessionId};
use crate::gameplay_transport::AdmissionRetryBudget;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::composed_admission::FAMILIAR_LOGIN_CHECKOUT;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::spell_access_facts::{
    CurrentSpellAccessOwner, UnavailableAccessOwner,
};
use crate::spell::Execution;
use crate::spell::companion_lifecycle::familiar_return_placement_available;
use crate::spell::owned_cast_facts::OwnedFactsError;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

fn uuid(id: &[u8; 16]) -> String {
    crate::gameplay_transport::canonical_uuid(id)
}

#[test]
fn bootstrap_v4_every_vocation_first_entry_without_progression_postgres() -> TestResult {
    creation_vocation_entry(None)
}

#[test]
fn familiar_login_retries_first_write_checkout_without_replaying_first_entry_postgres() -> TestResult
{
    creation_vocation_entry(Some(FamiliarRegression::WriteCheckout))
}

fn creation_vocation_entry(regression: Option<FamiliarRegression>) -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("creation-vocation-entry", move || {
        let checkout = matches!(regression, Some(FamiliarRegression::WriteCheckout));
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async move {
            let normal_cases = [
                ("Knight", true),
                ("Paladin", true),
                ("Sorcerer", true),
                ("Druid", true),
                ("Monk", true),
                ("Sorcerer", false),
            ];
            let cases: &[(&str, bool)] = if checkout { &[("Druid", false)] } else if regression.is_some() { &normal_cases[5..] } else { &normal_cases[..] };
            for &(vocation, with_premium) in cases {
                let h = Harness::create_newhaven(admin.clone(), "creation_entry").await?;
                let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
                let (controller, pin) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
                super::tutorial_vitals_fixture::publish_native_activation(&h, &pin).await?;
                let active = controller.active().ok_or("active generation")?;
                super::tutorial_vitals_fixture::native_asset_revisions(&h, active).await?;
                let room = pin.qualified_room().clone();
                let native_familiar = checkout || vocation == "Sorcerer";
                let spells = if native_familiar {
                    let native = active.native_gameplay().ok_or("native gameplay")?;
                    assert!(native.familiar_config().is_some());
                    assert!(active.progression().is_some());
                    native.spell_book().clone()
                } else {
                    crate::spell::cast::v1_spell_book().map_err(debug)?
                };
                let deps = if matches!(regression, Some(FamiliarRegression::BlockedReturn)) {
                    Dependencies::with_content_and_capacity(pin.into_channel_parts().0, room, spells, 16)?
                } else {
                    Dependencies::with_content(pin.into_channel_parts().0, room, spells)?
                };
                let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&h.pool).await?;
                let character = bootstrap(&h, vocation, now).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                assert!(h.root.read_character_progression(&authority, character).await?.is_none());
                if regression.is_some() && !checkout {
                    let policy = active.progression().ok_or("progression policy")?
                        .policy_for("profile-1", "ruleset-1", "content-1").map_err(debug)?;
                    let xp = policy.thresholds.iter().find(|row| row.level == 250)
                        .ok_or("level 250 threshold")?.minimum_experience.get();
                    // Pre-existing progression for the returning Character. Admission still
                    // obtains its own session and progression fence from the real owners.
                    sqlx::query("INSERT INTO game_character_progression_state(character_id,character_revision,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) SELECT character_id,character_revision,250,$2,profile_revision,ruleset_revision,content_revision,$3,$4,$5,$6,$7 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                        .bind(character.as_bytes().as_slice()).bind(xp)
                        .bind(&policy.context.simulation).bind(&policy.context.evidence)
                        .bind(&policy.context.declaration).bind(&policy.policy_revision)
                        .bind(&policy.reward_revision).execute(&h.pool).await?;
                }
                let mut source = fresh_fixture::fresh::Source::new(now)?;
                source.current.character_id = crate::foundation::CharacterId::decode(character.as_bytes()).map_err(debug)?;
                for row in &mut source.rows {
                    if matches!(row.key, AdmissionAuthorityGuardKeyV1::Character(_)) { row.key = AdmissionAuthorityGuardKeyV1::Character(source.current.character_id); }
                }
                let mut runtime = fresh_fixture::fresh::Source::new(now)?;
                runtime.rows.retain(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
                let publication = AdmissionAuthorityPublicationV1::prepare(&runtime, now).map_err(debug)?;
                h.root.publish_runtime_readiness(&h.node, &publication).await.map_err(debug)?;
                source.rows.retain(|row| !matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
                let publication = AdmissionAuthorityPublicationV1::prepare(&source, now).map_err(debug)?;
                AdmissionGuardStore::from_root(h.root.clone()).publish(&publication).await.map_err(debug)?;
                source.rows.extend(runtime.rows.iter().cloned());
                let session = GameSessionId::decode(&id(50)).map_err(debug)?;
                let transport = AuthenticatedTransportRefV1::decode(&[9;16]).map_err(debug)?;
                let request = source.activity_room_request(session, transport, &uuid(character.as_bytes()), [60;32], now)?;
                let committed = FreshAdmissionStore::from_root(h.root.clone()).commit(&request).await?;
                assert!(matches!(committed, crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)), "{committed:?}");
                use crate::gameplay_transport::spell_premium_coordinator::{
                    SpellPremiumCoordinator, purchase_fixture,
                };
                let premium = if checkout {
                    revoked_premium(&h.root).await?
                } else if with_premium {
                    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
                    h.root.latch_premium_activation(&activation.activation_id, activation.switch_over_us, activation.switch_over_us + 60_000_000).await.map_err(debug)?;
                    SpellPremiumCoordinator::new(
                        consumer, Some(clock), Some(activation), Some(h.root.clone()),
                    )
                } else {
                    // Same cold first-entry boundary as native g-core: no activation, no
                    // snapshot client, and no account pull registered before placement.
                    SpellPremiumCoordinator::new(Default::default(), None, None, Some(h.root.clone()))
                };
                let mut owner = deps.owner(&h, &authority);
                owner.premium_coordinator = Some(&premium);
                if with_premium {
                    owner.admit_premium(id(40), session);
                }
                owner.active_generation = Some(active);
                if native_familiar {
                    let vocation_key = vocation.to_lowercase();
                    assert!(owner.active_familiar_index(&vocation_key).is_some());
                    let native = active.native_gameplay().ok_or("native gameplay")?;
                    let mut runtime = deps.runtime.lock().await;
                    native.install_companion_policies(&mut runtime).map_err(debug)?;
                    let familiar = runtime
                        .companion_policy(&format!("oteryn:creature.{vocation_key}_familiar"))
                        .map_err(debug)?;
                    assert!(familiar.is_familiar, "native Familiar policy must be installed");
                }
                let budget = AdmissionRetryBudget::for_root(&session, &h.root);
                let entry = Box::pin(
                    owner.enter_committed_actor(&budget, &request, session, transport, deps.actor),
                );
                if checkout {
                    let result = FAMILIAR_LOGIN_CHECKOUT.scope(Default::default(), async {
                        let barrier = FAMILIAR_LOGIN_CHECKOUT.with(|probe| probe.seed_barrier.clone());
                        let mut entry = entry;
                        tokio::select! {
                            (outcome, _, _) = &mut entry => {
                                return Err(format!("entry ended before the Familiar fixture checkpoint: {outcome:?}").into());
                            }
                            () = barrier.0.notified() => {}
                        }
                        let seed = FamiliarStateOccurrence::from_bytes(id(83)).map_err(debug)?;
                        let look = seed_checkout_look(&h, &owner, session, seed).await?;
                        barrier.1.notify_one();
                        let (outcome, item_fence, _) = entry.await;
                        let (backpack, kit, placement, writes, misses) =
                            FAMILIAR_LOGIN_CHECKOUT.with(|probe| (
                                probe.backpack.get(), probe.kit.get(), probe.placement.get(),
                                probe.writes.get(), probe.checkout_misses.get(),
                            ));
                        if (backpack, kit, placement, misses) != (1, 1, 1, 1) {
                            return Err(format!("unexpected checkout probe counts: backpack={backpack} kit={kit} placement={placement} writes={writes} misses={misses}").into());
                        }
                        let (seed_receipts, login_receipts): (i64, i64) = sqlx::query_as(
                            "SELECT count(*) FILTER (WHERE familiar_occurrence_id=encode($2,'hex')::uuid), count(*) FILTER (WHERE familiar_occurrence_id<>encode($2,'hex')::uuid) FROM game_character_familiar_receipts WHERE character_id=encode($1,'hex')::uuid",
                        )
                        .bind(character.as_bytes().as_slice())
                        .bind(seed.as_bytes().as_slice())
                        .fetch_one(&h.pool).await?;
                        assert_eq!((seed_receipts, login_receipts), (1, i64::from(outcome == FirstEntryOutcome::Positioned)));
                        eprintln!("PG_EXECUTED familiar_login_checkout before_begin_misses={misses} writes={writes} placement={placement} starter_backpack={backpack} starter_kit={kit} seed_receipts={seed_receipts} login_receipts={login_receipts}");
                        if outcome != FirstEntryOutcome::Positioned {
                            return Err(format!("expected Positioned after the first Familiar write checkout became available, got {outcome:?}").into());
                        }
                        if writes != 2 {
                            return Err(format!("expected one refused checkout and one write, observed {writes} write attempts").into());
                        }
                        check_checkout_success(&h, &owner, &deps, item_fence, &premium, look).await
                    }).await;
                    drop(owner);
                    drop(authority);
                    drop(seal);
                    let cleanup = h.cleanup().await;
                    result?;
                    return cleanup;
                }
                let (outcome, _, _) = entry.await;
                assert_eq!(
                    outcome,
                    FirstEntryOutcome::Positioned,
                    "{vocation} with_premium={with_premium}",
                );
                if let Some(regression) = regression {
                    let progression = h.root.read_character_progression(&authority, character).await?
                        .ok_or("historical progression")?;
                    assert_eq!(progression.level, 250);
                    let result = check_familiar_regression(&h, &owner, &request, deps.actor, session, regression).await;
                    drop(owner); drop(authority); drop(seal); h.cleanup().await?;
                    return result;
                }
                if vocation == "Sorcerer" {
                    assert!(owner.progression_binding(session).is_some());
                    let progression = h
                        .root
                        .read_character_progression(&authority, character)
                        .await?
                        .ok_or("Familiar first entry must have initialized progression")?;
                    assert_eq!((progression.level, progression.total_experience.get()), (1, 0));
                }
                if !with_premium {
                    let familiar = h.root.read_character_familiar_state(&authority, character).await?;
                    assert!(familiar.state().granted_looks.is_empty());
                    let native = active.native_gameplay().ok_or("native gameplay")?;
                    let owned = owner
                        .read_familiar_lifecycle_inputs(deps.actor, session, native, &*premium.view(id(40)))
                        .await
                        .map_err(|()| "current Familiar facts")?;
                    let index = owner.active_familiar_index("sorcerer").ok_or("Familiar spell")?;
                    let (spell, true) = native.spell_book().source_indexed(index).ok_or("Familiar index")?
                        else { return Err("Familiar spell must be active".into()); };
                    assert_eq!(
                        owned.check_current_access(spell, owner.owner_now().get()),
                        Err(crate::spell::owned_cast_facts::OwnedFactsError::UnavailablePremium),
                    );
                }
                let loaded = crate::gameplay_transport::load_character_cast_facts(&h.root, &authority, character).await;
                let crate::gameplay_transport::CastFactsLoad::Ready { facts, .. } = loaded else { return Err(format!("{vocation}: {loaded:?}").into()); };
                assert_eq!(facts.level, 1);
                assert_eq!(facts.vocation, crate::spell::Vocation::from_key(&vocation.to_lowercase()).ok_or("vocation")?);
                assert_eq!((facts.max_health, facts.max_mana, facts.max_soul), (150, 55, 100));
                drop(owner); drop(authority); drop(seal); h.cleanup().await?;
            }
            Ok(())
        })
    })
}

async fn seed_checkout_look(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    occurrence: FamiliarStateOccurrence,
) -> TestResult<u32> {
    let index = owner
        .active_familiar_index("druid")
        .ok_or("Druid Familiar index")?;
    let (spell, true) = owner
        .spells
        .source_indexed(index)
        .ok_or("Druid Familiar spell")?
    else {
        return Err("Druid Familiar spell must be active".into());
    };
    let Execution::NativeProfile(profile) = &spell.execution else {
        return Err("Druid Familiar native profile".into());
    };
    let look = u32::try_from(
        profile.spell()["execution"]["native_behavior"]["parameters"]["default_look_type"]
            .as_u64()
            .ok_or("Druid Familiar look")?,
    )?;
    let progression = owner
        .progression_binding(session)
        .ok_or("positioned progression binding")?;
    let fence = super::death_ml_loss_composed_fixtures::current_fence(owner, session).await?;
    let before = h
        .root
        .read_character_familiar_state(owner.character, fence.character_id)
        .await?;
    assert!(before.committed_character_revision().is_none());
    let mut after = before.state().for_next_write(
        before.checkpoint_committed_at_ms(),
        &FamiliarLifecycleWriter::from_fence(&fence)?,
    )?;
    after.selected_look = look;
    after.granted_looks = vec![look];
    let outcome = h
        .root
        .commit_character_familiar_state(
            owner.character,
            &h.node,
            fence,
            FamiliarStateRequest {
                occurrence,
                before: before.state().clone(),
                after,
                content_revision: progression.context.content.clone(),
                policy_revision: progression.policy_revision.clone(),
                policy_digest: [83; 32],
            },
        )
        .await?;
    assert!(
        matches!(outcome, FamiliarStateOutcome::Committed(_)),
        "seed must commit once"
    );
    Ok(look)
}

async fn check_checkout_success(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    item_fence: Option<crate::durability::item_transfer::CurrentCharacterItemFence>,
    premium: &crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator,
    look: u32,
) -> TestResult {
    use crate::gameplay_transport::connection::{LivenessPolicy, serve_admitted};
    use std::future::{Future, poll_fn};
    use std::task::Poll;
    use tokio::io::AsyncWriteExt;

    let item_fence = item_fence.ok_or("positioned entry must publish its item fence")?;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native gameplay")?;
    let session = deps.admitted.game_session_id;
    let access = premium.view(id(40));
    let owned = owner
        .read_familiar_lifecycle_inputs(deps.actor, session, native, &access)
        .await
        .map_err(|()| "current Familiar facts after checkout retry")?;
    let now = owner.owner_now().get();
    let denial = access
        .read_familiar_restore_denial(owned.binding(), now)
        .ok_or("current fenced REVOKED after checkout retry")?;
    assert!(denial.current(owned.binding(), now));
    assert_eq!(denial.authority_revision, 8);
    assert!(owned.current_premium(now).is_none());
    let familiar = h
        .root
        .read_character_familiar_state(owner.character, item_fence.character_id)
        .await?;
    assert_eq!(familiar.state().selected_look, look);
    assert!(
        familiar.state().granted_looks.is_empty(),
        "fenced REVOKED must remove the seeded grant"
    );
    let (backpacks, kit): (i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE kit_destination=0 AND outcome=1), count(*) FILTER (WHERE kit_destination>0 AND outcome=1) FROM game_character_starter_grants WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(item_fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let kit_size = oteryn_gameplay_items::starter_grant::newhaven_starter_items("druid")
        .ok_or("Druid starter kit")?
        .len();
    assert_eq!((backpacks, kit), (1, i64::try_from(kit_size)?));

    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = Some(item_fence);
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let reading = async {
        loop {
            let frame = crate::gameplay_transport::tcp_tls::read_frame(&mut client).await?;
            if crate::foundation::decode_wire_envelope(&frame)
                .map_err(debug)?
                .message_type()
                == crate::foundation::MessageType::SnapshotCommit
            {
                client.shutdown().await?;
                return TestResult::Ok(());
            }
        }
    };
    let policy = LivenessPolicy {
        interval: std::time::Duration::from_secs(2),
        missed_limit: 5,
    };
    let live_ready = std::cell::Cell::new(0);
    let mut serving = Box::pin(serve_admitted(&mut server, admitted, owner, policy));
    let observed = poll_fn(|cx| {
        let mut outcome = Poll::Pending;
        let lines = oteryn_error_codes::capture(|| outcome = serving.as_mut().poll(cx));
        let ready = lines
            .iter()
            .filter(|line| {
                line.contains("event=join_progress")
                    && line.contains("phase=live_ready")
                    && line.contains("first_entry=Positioned")
            })
            .count();
        live_ready.set(live_ready.get() + ready);
        outcome
    });
    let (_, read) = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        tokio::join!(observed, reading)
    })
    .await?;
    read?;
    assert_eq!(
        live_ready.get(),
        1,
        "the serving path must reach live_ready once"
    );
    Ok(())
}

async fn bootstrap(
    h: &Harness,
    vocation: &str,
    now: i64,
) -> TestResult<crate::domain::CharacterId> {
    let character = 60;
    let account = 40;
    // The independent producer fixture supplies a bounded authenticated decision. Only the
    // real bootstrap writer creates the root, issuer floor, operation receipt and audit data.
    use crate::durability::native_admission_source::{
        DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation, NativeSourceSubject,
        SourceObservation,
    };
    if character == 60 {
        let descriptor = DescriptorRegistration {
            revision: 1,
            facts: vec![1],
            installed_at: now,
        };
        let provenance = FreshStoreProvenance {
            namespace: "house-fixture-source".into(),
            authorization: "independent-platform-fixture".into(),
            source_authority: "platform".into(),
            initialized_at: now,
        };
        assert!(
            h.root
                .record_native_source_descriptor_issuance(
                    "platform",
                    descriptor.clone(),
                    Some(provenance.clone())
                )
                .await
                .map_err(debug)?
        );
        h.root
            .initialize_native_admission_source(&h.node, provenance, descriptor)
            .await
            .map_err(debug)?;
    }
    let account_text = uuid(&id(account));
    let revision = u64::from(character);
    let security = serde_json::json!({
        "version": 1, "operation": "ReadAccountSecurityV1", "result": "observed",
        "source_authority": "platform", "source_revision": revision.to_string(),
        "decision_identity": revision.to_string(), "source_observed_at": now.to_string(),
        "clock_uncertainty_seconds": "0", "account_id": account_text,
        "purpose": "platform_security", "scope": "fresh_admission", "allowed": true,
        "minimum_valid_generation": "1",
    });
    h.root
        .accept_native_source_observation(
            &h.node,
            SourceObservation {
                source_authority: "platform".into(),
                operation: NativeSourceOperation::ReadAccountSecurityV1,
                subject: NativeSourceSubject::account_security(account_text.clone())
                    .map_err(debug)?,
                source_revision: revision,
                decision_identity: revision.to_string(),
                observed_at: now,
                semantic_facts: serde_json::to_vec(&security)?,
            },
        )
        .await
        .map_err(debug)?;
    let wire = serde_json::json!({
        "contract_version": 4, "variant": "OPERATOR_CONTROL_PLANE_BOOTSTRAP",
        "issuer_authority": "OTERYN_PLATFORM_CHARACTER_AUTHORITY",
        "issuer_decision_id": uuid(&id(character + 20)),
        "source_revision": revision.to_string(),
        "operation_id": uuid(&id(character)),
        "operation": "INITIAL_CHARACTER_BOOTSTRAP", "account_id": account_text,
        "target_world_id": uuid(&id(42)),
        "requested_sex":"Male","requested_vocation":vocation,"requested_name": format!("House Reclaimer {}{}", char::from(b'A' + character / 26), char::from(b'a' + character % 26)),
        "interpretation_context": {"profile_revision":"profile-1", "ruleset_revision":"ruleset-1",
            "content_revision":"content-1", "starter_template_revision":"newhaven-starter-r1"},
        "issued_at_source": (now - 1).to_string(), "expires_at_source": (now + 120).to_string(),
        "audience":"OTERYN_GAME_CHARACTER_AUTHORITY",
    });
    let intent = crate::character_bootstrap_intent::decode_producer_response(
        &serde_json::to_vec(&wire)?,
        id(character),
    )
    .map_err(debug)?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let bootstrapped = h
        .root
        .bootstrap_character(&recovery, &h.node, &intent)
        .await
        .map_err(debug)?;
    assert_eq!(bootstrapped.revision.get(), 1);
    Ok(bootstrapped.character_id)
}

#[derive(Clone, Copy)]
enum FamiliarRegression {
    ExistingLook { revoked: bool },
    BlockedReturn,
    WriteCheckout,
}

#[test]
fn familiar_login_preserves_existing_look_until_premium_is_known_postgres() -> TestResult {
    creation_vocation_entry(Some(FamiliarRegression::ExistingLook { revoked: false }))
}

#[test]
fn familiar_login_removes_vocation_look_after_revoked_premium_postgres() -> TestResult {
    creation_vocation_entry(Some(FamiliarRegression::ExistingLook { revoked: true }))
}

#[test]
fn familiar_waiting_return_finishes_without_premium_after_tile_frees_postgres() -> TestResult {
    creation_vocation_entry(Some(FamiliarRegression::BlockedReturn))
}

#[allow(
    dead_code,
    reason = "shared mTLS fixture serves other Premium test cases"
)]
#[path = "../../../../social/src/premium/test_producer.rs"]
mod premium_test_producer;

async fn revoked_premium(
    root: &crate::durability::DurabilityRoot,
) -> TestResult<crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator> {
    use crate::gameplay_transport::spell_premium_coordinator::{
        PremiumActivation, RegisteredPremiumClock, SpellPremiumCoordinator, clock_registration,
    };
    use crate::premium::{
        IngestOutcome, PremiumClass, TrustedNow,
        client::{PremiumClientConfig, PremiumSnapshotClient, fresh_nonce},
    };
    use premium_test_producer::{Reply, TestPki, TestProducer, snapshot};

    // Use the producer's current, full-lease fixture window in the registered clock.
    struct FixtureClock(TrustedNow);
    impl clock_registration::Registered for FixtureClock {}
    impl RegisteredPremiumClock for FixtureClock {
        fn read_current_window(&self) -> Option<TrustedNow> {
            Some(self.0)
        }
    }

    const ISSUED: i64 = 1_790_769_600_000_000;
    let account = id(40);
    let now = TrustedNow::new(ISSUED + 60_000_000, 1_000_000).ok_or("trusted clock window")?;
    let reads = SpellPremiumCoordinator::new(
        Default::default(),
        Some(std::sync::Arc::new(FixtureClock(now))),
        Some(PremiumActivation {
            activation_id: "familiar-revocation".into(),
            switch_over_us: ISSUED,
        }),
        Some(root.clone()),
    );
    let pki = TestPki::new();
    let producer =
        TestProducer::start(&pki, |request| Reply::json(snapshot(request, 7, &[]))).await;
    let client = PremiumSnapshotClient::new(&PremiumClientConfig {
        origin: producer.origin.clone(),
        identity_pem: pki.client_identity_pem.into_bytes(),
        platform_ca_pem: pki.ca_pem.into_bytes(),
    })?;
    let consumer = reads.consumer();
    let nonce = fresh_nonce().ok_or("ACTIVE pull nonce")?;
    let body = client.pull(account, &nonce).await.map_err(debug)?;
    assert_eq!(
        consumer.ingest(root, account, &nonce, &body).await,
        IngestOutcome::Accepted
    );
    assert!(
        reads.premium_gate(account).await.premium_current(),
        "ACTIVE proof and latch"
    );
    producer.script(|request| {
        Reply::json(snapshot(
            request,
            8,
            &[
                ("entitlement_state", "REVOKED".into()),
                ("lifecycle_revision", 2.into()),
            ],
        ))
    });
    let nonce = fresh_nonce().ok_or("REVOKED pull nonce")?;
    let body = client.pull(account, &nonce).await.map_err(debug)?;
    assert_eq!(
        consumer.ingest(root, account, &nonce, &body).await,
        IngestOutcome::Accepted
    );
    assert_eq!(consumer.class(account, Some(now)), PremiumClass::Revoked);
    assert!(!reads.premium_gate(account).await.premium_current());
    assert_eq!(producer.requests().len(), 2);
    Ok(reads)
}

async fn check_familiar_regression(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    request: &FreshAdmissionCommitRequestV1,
    actor: ExactActorRef,
    session: GameSessionId,
    regression: FamiliarRegression,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native gameplay")?;
    let config = native.familiar_config().ok_or("Familiar config")?;
    let index = owner
        .active_familiar_index("sorcerer")
        .ok_or("Familiar index")?;
    let (spell, true) = owner.spells.source_indexed(index).ok_or("Familiar spell")? else {
        return Err("Familiar spell must be active".into());
    };
    let Execution::NativeProfile(profile) = &spell.execution else {
        return Err("Familiar native profile".into());
    };
    let parameters = &profile.spell()["execution"]["native_behavior"]["parameters"];
    let look = u32::try_from(
        parameters["default_look_type"]
            .as_u64()
            .ok_or("Familiar look")?,
    )?;
    let progression = owner
        .progression_binding(session)
        .ok_or("progression binding")?;
    let fence = super::death_ml_loss_composed_fixtures::current_fence(owner, session).await?;
    let before = h
        .root
        .read_character_familiar_state(owner.character, fence.character_id)
        .await?;
    let mut historical = before.state().clone();
    let mut blockers = Vec::new();
    let returning = matches!(regression, FamiliarRegression::BlockedReturn);
    historical.lifecycle = Some(FamiliarLifecycleV2 {
        version: 2,
        familiar_remaining_ms: if returning { 90_123 } else { 0 },
        open: false,
        writer: FamiliarLifecycleWriter::from_fence(&fence).map_err(debug)?,
        clean_end_game_session_id: None,
    });
    if returning {
        historical.cooldowns = vec![FamiliarCooldownSnapshot {
            reference_spell_id: u32::try_from(
                profile.spell()["reference_spell_id"]
                    .as_u64()
                    .ok_or("Familiar cooldown reference")?,
            )?,
            remaining_micros: 60_000_123,
        }];
        let mut runtime = owner.runtime.lock().await;
        let cells = owner.movement_cells();
        assert!(familiar_return_placement_available(
            &runtime, cells, profile, actor
        ));
        let center = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        for dx in -1..=1 {
            for dy in -1..=1 {
                let Some(x) = center.x.checked_add(dx) else {
                    continue;
                };
                let Some(y) = center.y.checked_add(dy) else {
                    continue;
                };
                let at = crate::foundation::MovementLocalPosition { x, y, ..center };
                if runtime.companion_creation_available(at).map_err(debug)? {
                    blockers.push(runtime.admit_pinned_test_creature(at).map_err(debug)?);
                }
            }
        }
        assert!(!blockers.is_empty());
        assert!(!familiar_return_placement_available(
            &runtime, cells, profile, actor
        ));
    } else {
        historical.selected_look = look;
        historical.granted_looks = vec![look];
    }
    // Seed a clean historical projection through the real fenced writer. The Login and
    // Return operations and their subsequent receipts are produced by dispatch below.
    let FamiliarStateOutcome::Committed(seed) = h
        .root
        .commit_character_familiar_state(
            owner.character,
            &h.node,
            fence,
            FamiliarStateRequest {
                occurrence: FamiliarStateOccurrence::from_bytes(id(83)).map_err(debug)?,
                before: before.state().clone(),
                after: historical.clone(),
                content_revision: progression.context.content.clone(),
                policy_revision: progression.policy_revision.clone(),
                policy_digest: [83; 32],
            },
        )
        .await?
    else {
        return Err("historical Familiar fixture did not commit".into());
    };
    let unknown = UnavailableAccessOwner;
    let revoked = matches!(
        regression,
        FamiliarRegression::ExistingLook { revoked: true }
    );
    let budget = AdmissionRetryBudget::for_root(&session, &h.root);
    let login = if revoked {
        let access = revoked_premium(&h.root).await?.view(id(40));
        let owned = owner
            .read_familiar_lifecycle_inputs(actor, session, native, &access)
            .await
            .map_err(|()| "current Familiar facts after REVOKED")?;
        assert_eq!(
            owned.check_current_access(spell, owner.owner_now().get()),
            Err(OwnedFactsError::UnavailablePremium),
        );
        owner
            .initialize_familiar_login(&budget, actor, session, request, native, &access, config)
            .await
    } else {
        owner
            .initialize_familiar_login(&budget, actor, session, request, native, &unknown, config)
            .await
    }
    .map_err(|()| "Familiar Login refused")?
    .ok_or("Familiar Login receipt")?;
    assert!(login.creature().is_none());
    let after_login = h
        .root
        .read_character_familiar_state(owner.character, fence.character_id)
        .await?;
    if !returning {
        assert_eq!(after_login.state().selected_look, look);
        if revoked {
            assert!(
                after_login.state().granted_looks.is_empty(),
                "current fenced REVOKED must remove the granted vocation look",
            );
            assert!(
                after_login
                    .committed_character_revision()
                    .ok_or("denied receipt")?
                    > seed.committed_character_revision()
            );
        } else {
            assert_eq!(
                after_login.state(),
                &historical,
                "missing proof must keep the granted look"
            );
            assert_eq!(
                after_login.committed_character_revision(),
                Some(seed.committed_character_revision())
            );
        }
        return Ok(());
    }
    assert!(
        crate::spell::companion_lifecycle::login_left_familiar_waiting(
            login.creature(),
            login.owner_operations()
        )
    );
    assert!(
        !owner
            .retry_waiting_familiar_return(actor, session, &unknown)
            .await
            .map_err(|()| "blocked Return")?
    );
    let waiting = h
        .root
        .read_character_familiar_state(owner.character, fence.character_id)
        .await?;
    assert_eq!(waiting.state(), after_login.state());
    assert_eq!(
        waiting.committed_character_revision(),
        after_login.committed_character_revision()
    );
    {
        let mut runtime = owner.runtime.lock().await;
        for blocker in blockers {
            runtime.remove_test_actor(blocker).map_err(debug)?;
            if familiar_return_placement_available(&runtime, owner.movement_cells(), profile, actor)
            {
                break;
            }
        }
        assert!(familiar_return_placement_available(
            &runtime,
            owner.movement_cells(),
            profile,
            actor
        ));
    }
    let access = owner
        .read_familiar_lifecycle_inputs(actor, session, native, &unknown)
        .await
        .map_err(|()| "current Familiar facts before Return")?;
    assert_eq!(
        access.check_current_access(spell, owner.owner_now().get()),
        Err(OwnedFactsError::UnavailablePremium)
    );
    assert!(
        owner
            .retry_waiting_familiar_return(actor, session, &unknown)
            .await
            .map_err(|()| "due Return refused with no Premium proof")?
    );
    let returned = h
        .root
        .read_character_familiar_state(owner.character, fence.character_id)
        .await?;
    let state = returned.state();
    let lifecycle = state.lifecycle.as_ref().ok_or("returned lifecycle")?;
    assert!(lifecycle.open);
    assert_eq!(lifecycle.familiar_remaining_ms, 90_123);
    assert_eq!(
        lifecycle.writer,
        historical.lifecycle.ok_or("seeded lifecycle")?.writer
    );
    assert_eq!(state.lifecycle_epoch, waiting.state().lifecycle_epoch + 1);
    assert_eq!(state.cooldowns, waiting.state().cooldowns);
    assert!(state.granted_looks.is_empty());
    assert_eq!(state.selected_look, 0);
    assert!(
        !owner
            .retry_waiting_familiar_return(actor, session, &unknown)
            .await
            .map_err(|()| "replayed Return")?
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .owned_companions(actor, session)
            .map_err(debug)?
            .len(),
        1
    );
    let still_returned = h
        .root
        .read_character_familiar_state(owner.character, fence.character_id)
        .await?;
    assert_eq!(still_returned.state(), state);
    assert_eq!(
        still_returned.committed_character_revision(),
        returned.committed_character_revision()
    );
    Ok(())
}
