//! Two independently assigned Channel runtimes in one World, using the existing admission owner.
use super::fixture::{Harness, debug, id};
use super::fresh_fixture::fresh::Source;
use super::{Dependencies, TestResult};
use crate::durability::admission_authority_guards::AdmissionGuardStore;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::guild_chat_fixture::{historical_character, uuid};
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, NodeIncarnationProof,
    OperationKey, RuntimeScopeAssignmentWriter,
};
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityPublicationV1,
    AdmissionPublicationPreconditionV1,
};
use crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
use crate::foundation::{ChannelId, ChannelRuntimeV1, GameSessionId, RuntimeScopeRefV1};
use crate::gameplay_transport::connection::ControllerBinding;
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use tokio::sync::Mutex;

pub(super) const RECIPIENT: u8 = 101;
pub(super) const RECIPIENT_ACCOUNT: u8 = 100;
pub(super) const RECIPIENT_SESSION: u8 = 110;
pub(super) const RECIPIENT_CHANNEL: u8 = 44;
pub(super) const RECIPIENT_NAME: &str = "World Receiver";

pub(super) async fn clock(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?,
    )
}

pub(super) async fn admit_second_node(
    h: &Harness,
) -> TestResult<(NodeIncarnationProof, Dependencies)> {
    admit_second_node_inner(h, false).await
}

pub(super) async fn admit_existing_second_node(
    h: &Harness,
) -> TestResult<(NodeIncarnationProof, Dependencies)> {
    admit_second_node_inner(h, true).await
}

async fn admit_second_node_inner(
    h: &Harness,
    existing_character: bool,
) -> TestResult<(NodeIncarnationProof, Dependencies)> {
    let node = super::fixture::registered_node(&h.root, 2).await?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let channel = ChannelId::decode(&id(RECIPIENT_CHANNEL)).map_err(debug)?;
    let scope = RuntimeScopeRefV1::channel(world, channel);
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(id(42).as_slice()).bind(id(RECIPIENT_CHANNEL).as_slice()).execute(&h.pool).await?;
    let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "sequencer-writer")
        .await
        .map_err(debug)?;
    let outcome = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([82; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Assign {
                scope,
                target: node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    let AssignmentOutcome::Committed(receipt) = outcome else {
        return Err(format!("second Channel assignment: {outcome:?}").into());
    };
    if existing_character {
        let (account, world, name): (Vec<u8>, Vec<u8>, String) = sqlx::query_as(
            "SELECT uuid_send(account_id),uuid_send(world_id),name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        ).bind(id(RECIPIENT).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(account, id(RECIPIENT_ACCOUNT));
        assert_eq!(world, id(42));
        assert_eq!(name, RECIPIENT_NAME);
    } else {
        historical_character(
        &h.pool,
        501,
        crate::domain::CharacterId::from_bytes(id(RECIPIENT)).map_err(debug)?,
        id(RECIPIENT_ACCOUNT),
        id(42),
        RECIPIENT_NAME,
    )
        .await?;
    }
    let now = clock(h).await?;
    let mut current = Source::new(now)?.current;
    current.account_id = uuid(id(RECIPIENT_ACCOUNT));
    current.character_id = crate::foundation::CharacterId::decode(&id(RECIPIENT)).map_err(debug)?;
    current.channel_id = channel;
    current.scope_ownership_generation = receipt.assignment.ownership_generation;
    let mut source = Source::with_current(now, current)?;
    let guards = AdmissionGuardStore::from_root(h.root.clone());
    let signing_key = source
        .rows
        .iter()
        .find(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::SigningTrust { .. }))
        .ok_or("signing row")?
        .key
        .clone();
    let prior = guards
        .load(std::slice::from_ref(&signing_key))
        .await
        .map_err(debug)?
        .into_iter()
        .next()
        .flatten()
        .ok_or("current signing publication")?;
    let signing = source
        .rows
        .iter_mut()
        .find(|row| row.key == signing_key)
        .ok_or("signing row")?;
    signing.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
        expected_publication_revision: prior.publication_revision,
    };
    signing.publication_revision = prior.publication_revision + 1;
    signing.source.source_revision = prior.source.source_revision + 1;
    signing.source.decision_identity =
        format!("platform-observation-{}", signing.source.source_revision);
    let all_rows = source.rows.clone();
    source
        .rows
        .retain(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    let publication = AdmissionAuthorityPublicationV1::prepare(&source, now).map_err(debug)?;
    h.root
        .publish_runtime_readiness(&node, &publication)
        .await
        .map_err(debug)?;
    source.rows = all_rows.clone();
    source
        .rows
        .retain(|row| !matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    let publication = AdmissionAuthorityPublicationV1::prepare(&source, now).map_err(debug)?;
    guards.publish(&publication).await.map_err(debug)?;
    source.rows = all_rows;
    let request = source.appearance_observer_request(&uuid(id(RECIPIENT)), clock(h).await?)?;
    let outcome = FreshAdmissionStore::from_root(h.root.clone())
        .commit(&request)
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, FreshAdmissionDurableOutcomeV1::Committed(_)),
        "{outcome:?}"
    );

    let mut dependencies = Dependencies::new()?;
    let session = GameSessionId::decode(&id(RECIPIENT_SESSION)).map_err(debug)?;
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        node.fact().node_id(),
        node.fact().registration_revision(),
        receipt.assignment.ownership_generation,
        receipt.assignment.source_revision,
        &receipt.assignment.decision_identity,
        2,
        ChannelContentPin::test(world),
    )
    .map_err(debug)?;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_first_entry_position(actor)
        .map_err(|e| format!("recipient initial position: {e:?}"))?;
    runtime
        .bind_attacker_lease(
            actor,
            session,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(&id(RECIPIENT)).map_err(debug)?,
                2,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    dependencies.runtime = Mutex::new(runtime);
    dependencies.actor = actor;
    dependencies.admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
    dependencies.admitted.game_session_id = session;
    dependencies.admitted.channel_id = channel;
    dependencies.admitted.runtime_actor = Some(actor);
    dependencies.admitted.controller = Some(ControllerBinding {
        transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[8; 16])
            .map_err(debug)?,
        account_id: id(RECIPIENT_ACCOUNT),
    });
    let mut fence = super::fixture::fence(1)?.item_fence();
    fence.character_id = crate::domain::CharacterId::from_bytes(id(RECIPIENT)).map_err(debug)?;
    fence.game_session_id = session;
    fence.character_lease_generation = 2;
    fence.runtime_scope = scope;
    dependencies.admitted.item_fence = Some(fence);
    Ok((node, dependencies))
}
