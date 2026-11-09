//! Real framed6121 read over an actually admitted bundle session; compare full asset footprint.
use super::*;
use oteryn_protocol_oteryn::gem_presentation as presentation;
#[test]
fn gem_presentation_postgres_real_catalogue_revision_and_readonly_fences() -> TestResult {
    check(Case::Resources)
}
pub(super) async fn check_read(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    a: &wire::AtelierReply,
) -> TestResult {
    let assets = scope_footprint(h).await?;
    let r = presentation::Request {
        nonce: 1,
        revision: a.revision,
    };
    let request = r.encode().ok_or("query")?;
    let (status, bytes) = send_atelier_status(owner, admitted, 6121, &request).await?;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    let reply = presentation::Reply::decode(&bytes).ok_or("executed catalogue")?;
    assert_eq!(reply.request, r);
    assert_eq!(reply.modifiers.len(), 69);
    let capacity = reply
        .modifiers
        .iter()
        .find(|m| m.kind == 0 && m.id == 48)
        .ok_or("capacity")?;
    assert!(capacity.effects.iter().all(|e| e.unit == 1));
    let dodge = reply
        .modifiers
        .iter()
        .find(|m| m.kind == 1 && m.id == 0)
        .ok_or("dodge")?;
    assert_eq!(dodge.effects[0].values, [280, 300, 330, 420]);
    let wrong = presentation::Request {
        nonce: 2,
        revision: a.revision + 1,
    };
    assert!(
        owner
            .gem_presentation_query(*admitted, wrong)
            .await
            .is_none()
    );
    let mut stale = *admitted;
    stale.continuity.connection_generation += 1;
    assert!(owner.gem_presentation_query(stale, r).await.is_none());
    assert_eq!(
        scope_footprint(h).await?,
        assets,
        "metadata read does not change custody, root revisions, balances or paid receipts"
    );
    eprintln!(
        "GEM_PRESENTATION POSTGRES_READ_PROOF current_catalogue_revision_session_fenced_readonly"
    );
    Ok(())
}
