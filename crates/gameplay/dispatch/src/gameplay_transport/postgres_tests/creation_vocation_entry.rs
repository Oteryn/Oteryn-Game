//! Creation-vocation admission must work before any progression initialization.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, fresh_fixture};
use crate::durability::admission_authority_guards::AdmissionGuardStore;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityPublicationV1,
};
use crate::foundation::{AuthenticatedTransportRefV1, GameSessionId};
use crate::gameplay_transport::connection::FirstEntryOutcome;
fn uuid(id: &[u8; 16]) -> String {
    crate::gameplay_transport::canonical_uuid(id)
}

#[test]
fn bootstrap_v4_every_vocation_first_entry_without_progression_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("creation-vocation-entry", move || {
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async move {
            for vocation in ["Knight", "Paladin", "Sorcerer", "Druid", "Monk"] {
                let h = Harness::create_newhaven(admin.clone(), "creation_entry").await?;
                let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
                let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
                super::tutorial_vitals_fixture::publish_native_activation(&h, &pin).await?;
                let active = controller.active().ok_or("active generation")?;
                super::tutorial_vitals_fixture::native_asset_revisions(&h, active).await?;
                let room = pin.qualified_room().clone();
                let deps = Dependencies::with_content(pin.into_channel_parts().0, room, crate::spell::cast::v1_spell_book().map_err(debug)?)?;
                let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&h.pool).await?;
                let character = bootstrap(&h, vocation, now).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                assert!(h.root.read_character_progression(&authority, character).await?.is_none());
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
                let session = GameSessionId::decode(&id(50)).map_err(debug)?;
                let transport = AuthenticatedTransportRefV1::decode(&[9;16]).map_err(debug)?;
                let request = source.activity_room_request(session, transport, &uuid(character.as_bytes()), [60;32], now)?;
                let committed = FreshAdmissionStore::from_root(h.root.clone()).commit(&request).await?;
                assert!(matches!(committed, crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)), "{committed:?}");
                let mut owner = deps.owner(&h, &authority);
                owner.active_generation = Some(active);
                let (outcome, _, _) = Box::pin(owner.initialize_first_entry(&request, session, transport, deps.actor)).await;
                assert_eq!(outcome, FirstEntryOutcome::Positioned, "{vocation}");
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
