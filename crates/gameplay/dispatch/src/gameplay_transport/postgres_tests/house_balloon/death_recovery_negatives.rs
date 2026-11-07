//! Untrusted recovery candidates cannot change the genuine retained death exit.
use super::*;
pub(super) async fn qualify(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    source: AdmittedSession,
    retained: &crate::durability::house_transfer::HouseTransferRecovery,
) -> TestResult {
    use crate::durability::recovery_evidence_composition::RecoveryEvidenceSubject;
    use crate::foundation::fnd04_verifier::fresh_grant_signing_key_id;
    let original = footprint(h, source).await?;
    for (case, nonce) in [180_u8, 181, 182, 183, 184].into_iter().enumerate() {
        let token = RecoverySource::token(nonce)?;
        let key = fresh_grant_signing_key_id(&token).ok_or("real registered recovery key")?;
        owner
            .evidence
            .refresh_recovery(owner.root, owner.holder, &retained.current.account_id, &key)
            .await
            .map_err(debug)?;
        let subject =
            RecoveryEvidenceSubject::new(&retained.current.account_id, &key).map_err(debug)?;
        let verified = owner
            .root
            .verify_registered_recovery(owner.holder, &subject, &token, &retained.current)
            .await?
            .map_err(debug)?;
        let capabilities = owner
            .required_capabilities
            .authenticate_resume_support(
                source.continuity.selected_capabilities.as_slice(),
                source.continuity.selected_capabilities,
            )
            .map_err(debug)?;
        let mut wrong = retained.clone();
        match case {
            0 => wrong.ack.position[0] += 1,
            1 => wrong.pending_death = None,
            2 => wrong.placement.key.character_lease_generation = wrong.ack.lease_generation,
            3 => {
                wrong.placement.key.write_sequence = wrong
                    .placement
                    .key
                    .write_sequence
                    .checked_add(1)
                    .ok_or("source key bound")?
            }
            _ => wrong.placement.record.server_artifact_digest = [0; 32],
        }
        assert!(
            owner
                .root
                .recover_house_transfer(
                    owner.character,
                    owner.holder,
                    wrong,
                    verified,
                    crate::foundation::AuthenticatedTransportRefV1::decode(&[nonce; 16])
                        .map_err(debug)?,
                    capabilities.current_check()
                )
                .await
                .is_err(),
            "untrusted candidate mutation {case} must not authorize pending death recovery"
        );
        assert_eq!(
            footprint(h, source).await?,
            original,
            "candidate rejection changes no Character, obligation or handoff"
        );
    }
    Ok(())
}
async fn footprint(h: &Harness, source: AdmittedSession) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT md5(jsonb_build_object('root',(SELECT to_jsonb(r) FROM game_character_roots r WHERE character_id=encode($1,'hex')::uuid),'pending',(SELECT to_jsonb(p) FROM game_character_pending_respawns p WHERE character_id=encode($1,'hex')::uuid),'position',(SELECT to_jsonb(p) FROM game_character_last_positions p WHERE character_id=encode($1,'hex')::uuid),'exits',(SELECT jsonb_agg(to_jsonb(t) ORDER BY transfer_id) FROM game_house_transfer_receipts t WHERE character_id=encode($1,'hex')::uuid))::text)").bind(source.item_fence.ok_or("negative source fence")?.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?)
}
