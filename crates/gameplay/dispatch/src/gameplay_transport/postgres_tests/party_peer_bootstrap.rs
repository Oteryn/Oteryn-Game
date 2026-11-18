//! Shared PostgreSQL fixture for bootstrapping an additional party character.
use super::TestResult;
use super::fixture::{Harness, debug, id};

pub(super) async fn create_peer(
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    evidence: &crate::gameplay_transport::fresh_evidence::FreshEvidenceSource,
    member: u8,
) -> TestResult<[u8; 16]> {
    let account = crate::gameplay_transport::canonical_uuid(&id(120 + member));
    evidence
        .refresh_account(&h.root, &h.node, &account)
        .await
        .map_err(debug)?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let operation = id(80 + member);
    let source = serde_json::json!({
        "contract_version":3,
        "variant":"OPERATOR_CONTROL_PLANE_BOOTSTRAP",
        "issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY",
        "issuer_decision_id":crate::gameplay_transport::canonical_uuid(&id(90+member)),
        "source_revision":(2+u64::from(member)).to_string(),
        "operation_id":crate::gameplay_transport::canonical_uuid(&operation),
        "operation":"INITIAL_CHARACTER_BOOTSTRAP",
        "account_id":account,
        "target_world_id":crate::gameplay_transport::canonical_uuid(&id(42)),
        "requested_name":format!("Party A{}",char::from(b'a'+member)),
        "requested_sex":"Male",
        "interpretation_context":{"profile_revision":"profile-1","ruleset_revision":"ruleset-1","content_revision":"content-1","starter_template_revision":"starter-1"},
        "issued_at_source":now.to_string(),
        "expires_at_source":(now+120).to_string(),
        "audience":"OTERYN_GAME_CHARACTER_AUTHORITY",
    });
    // The strict producer fixture decoder is also used by Character authority
    // tests. Birth, creation receipt and audit are committed by the real owner.
    let intent = crate::character_bootstrap_intent::decode_producer_response(
        &serde_json::to_vec(&source)?,
        operation,
    )
    .map_err(debug)?;
    let created = h
        .root
        .bootstrap_character(authority, &h.node, &intent)
        .await
        .map_err(debug)?;
    assert_eq!(created.revision.get(), 1);
    Ok(*created.character_id.as_bytes())
}
