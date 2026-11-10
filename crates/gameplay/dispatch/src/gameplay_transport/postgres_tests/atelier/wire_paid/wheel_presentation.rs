//! Actual6119 dispatch/owner transaction, qualified canonical graph and no writes.
use super::*;
use oteryn_protocol_oteryn::wheel_presentation as graph;
#[test]
fn wheel_presentation_postgres_real_budget_topology_draft_and_session_fences() -> TestResult {
    check(Case::Resources)
}
pub(super) async fn check_read(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    a: &wire::AtelierReply,
) -> TestResult {
    let before = scope_footprint(h).await?;
    let wheel = wheel_wire::decode_wheel_reply(&send_atelier(owner, admitted, 161, &[8, 0]).await?)
        .map_err(debug)?;
    let mut vessels = [None; 4];
    for gem in &a.gems {
        if gem.vessel {
            vessels[usize::from(gem.domain)] = Some(gem.id);
        }
    }
    let request = graph::Request {
        nonce: 1,
        wheel_revision: wheel.revision,
        atelier_revision: a.revision,
        points: wheel.points,
        vessels,
    };
    let (status, payload) = send_atelier_status(
        owner,
        admitted,
        6119,
        &request.encode().ok_or("graph request")?,
    )
    .await?;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    let reply = graph::Reply::decode(&payload).ok_or("graph reply")?;
    assert_eq!(reply.request, request);
    assert_eq!(reply.nodes.len(), 36);
    assert_eq!(reply.revelations.len(), 4);
    assert!(reply.available >= reply.unlock_points);
    let rules = owner.wheel_ruleset.as_ref().ok_or("rules")?;
    for (node, rule) in reply.nodes.iter().zip(rules.slots()) {
        assert_eq!(node.capacity, rule.capacity);
        let mut expected = rule.unlock_from.iter().map(|v| v - 1).collect::<Vec<_>>();
        expected.sort_unstable();
        assert_eq!(node.unlock_from, expected);
    }
    let mut invalid = request.clone();
    invalid.nonce = 2;
    invalid.points = rules.capacities();
    let (status, bytes) = send_atelier_status(
        owner,
        admitted,
        6119,
        &invalid.encode().ok_or("invalid draft")?,
    )
    .await?;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    assert!(
        !graph::Reply::decode(&bytes)
            .ok_or("invalid preview")?
            .draft_valid
    );
    let mut stale = request.clone();
    stale.wheel_revision += 1;
    assert!(
        owner
            .wheel_presentation_query(*admitted, stale)
            .await
            .is_none()
    );
    let mut stale = *admitted;
    stale.continuity.connection_generation += 1;
    assert!(
        owner
            .wheel_presentation_query(stale, request)
            .await
            .is_none()
    );
    assert_eq!(
        scope_footprint(h).await?,
        before,
        "advisory reads and refused drafts create no root/state/custody/economy/receipt changes"
    );
    eprintln!(
        "WHEEL_PRESENTATION POSTGRES_READ_PROOF canonical_graph_current_budget_draft_and_session_fences_readonly"
    );
    Ok(())
}
