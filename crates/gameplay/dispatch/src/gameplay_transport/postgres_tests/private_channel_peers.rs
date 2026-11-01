//! Extra cap-test peers use the same verified fresh-admission owner as the two main nodes.
use crate::gameplay_transport::ComposedFreshAdmission;
use super::fixture::{Harness,debug};
use super::TestResult;
use super::chat_world_fixture as world;
use crate::durability::admission_authority_guards::AdmissionGuardStore;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::guild_chat_fixture as historical;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::runtime_scope_assignment::{AssignmentCommand,AssignmentOutcome,AssignmentRequest,ControlActor,OperationKey,RuntimeScopeAssignmentWriter};
use crate::foundation::admission_authority_publication::{AdmissionAuthorityGuardKeyV1,AdmissionAuthorityPublicationV1,AdmissionPublicationPreconditionV1};
use crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
use crate::foundation::{AuthenticatedTransportRefV1,GameSessionId,RuntimeScopeRefV1,WorldId};
use super::fresh_fixture::fresh::Source;

pub(super) async fn active_peer(h:&Harness,owner:&ComposedFreshAdmission<'_,'_,'_>,base:CurrentCharacterItemFence,n:u32,world_id:[u8;16],name:&str)->TestResult<CurrentCharacterItemFence> {
    let character=crate::domain::CharacterId::from_bytes(historical::sid(n,1)).map_err(debug)?;
    let account=historical::sid(n,2);
    historical::historical_character(&h.pool,n,character,account,world_id,name).await?;
    let scope=RuntimeScopeRefV1::channel(WorldId::decode(&world_id).map_err(debug)?,owner.channel_id);
    let scope_generation=if world_id!=*base.runtime_scope.world_id().as_bytes() {
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
            .bind(world_id.as_slice()).bind(owner.channel_id.as_bytes().as_slice()).execute(&h.pool).await?;
        let writer=RuntimeScopeAssignmentWriter::open(h.root.clone(),"sequencer-writer").await.map_err(debug)?;
        let outcome=writer.submit(&AssignmentRequest {operation_key:OperationKey::from_bytes([83;32]),actor:ControlActor::new("oteryn_test_admin").map_err(debug)?,command:AssignmentCommand::Assign {scope,target:owner.holder.fact()}}).await.map_err(debug)?;
        let AssignmentOutcome::Committed(receipt)=outcome else {return Err(format!("foreign World assignment: {outcome:?}").into());};
        receipt.assignment.ownership_generation
    } else {base.scope_ownership_generation.get()};
    let now=world::clock(h).await?;
    let mut current=Source::new(now)?.current;
    current.account_id=historical::uuid(account);
    current.character_id=crate::foundation::CharacterId::decode(character.as_bytes()).map_err(debug)?;
    current.world_id=scope.world_id();
    current.channel_id=owner.channel_id;
    current.scope_ownership_generation=scope_generation;
    let mut source=Source::with_current(now,current)?;
    let guards=AdmissionGuardStore::from_root(h.root.clone());
    let keys:Vec<_>=source.rows.iter().filter(|row|matches!(row.key,AdmissionAuthorityGuardKeyV1::Runtime(_)|AdmissionAuthorityGuardKeyV1::SigningTrust {..})).map(|row|row.key.clone()).collect();
    for key in keys {
        if let Some(prior)=guards.load(std::slice::from_ref(&key)).await.map_err(debug)?.into_iter().next().flatten() {
            let row=source.rows.iter_mut().find(|row|row.key==key).ok_or("peer publication row")?;
            row.precondition=AdmissionPublicationPreconditionV1::CompareAndSet {expected_publication_revision:prior.publication_revision};
            row.publication_revision=prior.publication_revision+1;
            row.source.source_revision=prior.source.source_revision+1;
            row.source.decision_identity=format!("private-peer-{}-{}",n,row.source.source_revision);
        }
    }
    let rows=source.rows.clone();
    source.rows.retain(|row|matches!(row.key,AdmissionAuthorityGuardKeyV1::Runtime(_)));
    let publication=AdmissionAuthorityPublicationV1::prepare(&source,now).map_err(debug)?;
    h.root.publish_runtime_readiness(owner.holder,&publication).await.map_err(debug)?;
    source.rows=rows.clone();
    source.rows.retain(|row|!matches!(row.key,AdmissionAuthorityGuardKeyV1::Runtime(_)));
    let publication=AdmissionAuthorityPublicationV1::prepare(&source,now).map_err(debug)?;
    guards.publish(&publication).await.map_err(debug)?;
    source.rows=rows;
    let session=GameSessionId::decode(&historical::sid(n,20)).map_err(debug)?;
    let transport=AuthenticatedTransportRefV1::decode(&historical::sid(n,21)).map_err(debug)?;
    let mut nonce=[0_u8;32];
    nonce[..16].copy_from_slice(session.as_bytes());
    nonce[16..].copy_from_slice(&historical::sid(n,21));
    let character_name=historical::uuid(*character.as_bytes());
    let request=source.request_with_nonce(session,transport,Some((&source.current.account_id,&character_name)),world::clock(h).await?,Some(nonce))?;
    let outcome=FreshAdmissionStore::from_root(h.root.clone()).commit(&request).await.map_err(debug)?;
    assert!(matches!(outcome,FreshAdmissionDurableOutcomeV1::Committed(_)),"peer admission: {outcome:?}");
    let fence=CurrentCharacterItemFence {character_id:character,game_session_id:session,character_lease_generation:2,runtime_scope:scope,scope_ownership_generation:crate::foundation::ScopeOwnershipGeneration::new(scope_generation).map_err(debug)?,..base};
    assert!(h.root.private_channel_admit(owner.character,owner.holder,fence,true).await.map_err(debug)?);
    Ok(fence)
}
