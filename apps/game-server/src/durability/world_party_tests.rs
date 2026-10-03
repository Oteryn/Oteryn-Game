//! Real PostgreSQL qualification of PARTY-1. SQL setup below is explicitly a constraint
//! fixture, never an invitation producer or a QualifiedPartyVisibleTarget constructor.
//! Source writer tests obtain SpellItemAuthority through the real recovery/session/node checks.
#![allow(clippy::expect_used, dead_code)]

use crate::bestiary_postgres_harness as harness;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::spell_item_transaction::{
    SpellItemAuthority, SpellItemError, assert_spell_item_authority_in_transaction,
};
use crate::durability::world_party::{
    apply_world_party_command_in_transaction, read_world_party_in_transaction,
};
use crate::foundation::{CommandId, CommandRef, GameSessionId, PartyAction};
use harness::{CHANNEL, CHARACTER, Harness, SESSION, TestResult, WORLD, debug, id};
use sqlx::{Postgres, Transaction};

fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = harness::configured_admin() else {
        return Ok(());
    };
    harness::runtime()?.block_on(async move {
        let h = Harness::create(admin, tag, true).await?;
        let result = body(&h).await;
        h.cleanup().await?;
        result
    })
}
fn fence() -> TestResult<CurrentCharacterItemFence> {
    let current = harness::fence(1)?;
    Ok(CurrentCharacterItemFence {
        character_id: current.character_id,
        game_session_id: current.game_session_id,
        connection_generation: current.connection_generation,
        character_lease_generation: current.character_lease_generation,
        runtime_scope: current.runtime_scope,
        scope_ownership_generation: current.scope_ownership_generation,
    })
}
fn command(number: u64) -> TestResult<CommandRef> {
    Ok(CommandRef::new(
        GameSessionId::decode(&id(SESSION)).map_err(debug)?,
        CommandId::new(number).map_err(debug)?,
    ))
}
async fn authority(
    tx: &mut Transaction<'_, Postgres>,
    h: &Harness,
    number: u64,
) -> TestResult<SpellItemAuthority> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    assert_spell_item_authority_in_transaction(
        tx,
        &h.root,
        &recovery,
        &h.node,
        &fence()?,
        command(number)?,
        [1; 32],
    )
    .await
    .map_err(|e| debug(e).into())
}
async fn apply(
    h: &Harness,
    number: u64,
    action: PartyAction,
) -> TestResult<crate::durability::world_party::PreparedWorldPartyReceipt> {
    let mut tx = h.pool.begin().await?;
    let proof = authority(&mut tx, h, number).await?;
    let receipt = apply_world_party_command_in_transaction(&mut tx, &proof, action, None)
        .await
        .map_err(debug)?;
    tx.commit().await?;
    Ok(receipt)
}
async fn footprint(h: &Harness) -> TestResult<(i64, i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_parties),(SELECT count(*) FROM game_party_members),(SELECT count(*) FROM game_party_invitations),(SELECT count(*) FROM game_world_party_receipts),(SELECT count(*) FROM game_world_party_audit_outbox)")
        .fetch_one(&h.pool).await?)
}
/// Administrative database-invariant fixture. It does not create an admission session,
/// runtime actor, visibility token, social consent projection or gameplay invitation.
async fn fixture_character(tx: &mut Transaction<'_, Postgres>, seed: u8, world: u8) -> TestResult {
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($1,'hex')::uuid,account_id,encode($2,'hex')::uuid,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,$3 FROM game_character_roots WHERE character_id=encode($4,'hex')::uuid")
        .bind(id(seed).as_slice()).bind(id(world).as_slice()).bind(format!("Party Fixture {}{}", char::from(b'A' + seed / 26), char::from(b'A' + seed % 26)))
        .bind(id(CHARACTER).as_slice()).execute(&mut **tx).await?;
    Ok(())
}
async fn fixture_party(tx: &mut Transaction<'_, Postgres>, party: u8, leader: u8) -> TestResult {
    sqlx::query("INSERT INTO game_parties(party_id,world_id,leader_character_id,revision,next_seq) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,2)")
        .bind(id(party).as_slice()).bind(id(WORLD).as_slice()).bind(id(leader).as_slice()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,encode($3,'hex')::uuid)")
        .bind(id(leader).as_slice()).bind(id(party).as_slice()).bind(id(CHANNEL).as_slice()).execute(&mut **tx).await?;
    Ok(())
}
fn assert_constraint(error: sqlx::Error) {
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514"),
        "{error:?}"
    );
}

#[test]
fn missing_visible_owner_records_refusal_replay_and_rejects_changed_intent() -> TestResult {
    run("party_unknown_visible", async |h| {
        assert_eq!(footprint(h).await?, (0, 0, 0, 0, 0));
        let action = PartyAction::Invite { target: id(60) };
        let first = apply(h, 1, action).await?;
        assert_eq!(first.disposition, "NOT_VISIBLE");
        assert!(!first.replayed);
        assert_eq!(first.party_id, None);
        assert_eq!(footprint(h).await?, (0, 0, 0, 1, 1));
        let replay = apply(h, 1, action).await?;
        assert!(replay.replayed);
        assert_eq!(replay.receipt_id, first.receipt_id);
        assert_eq!(replay.disposition, first.disposition);
        assert_eq!(footprint(h).await?, (0, 0, 0, 1, 1));
        let mut tx = h.pool.begin().await?;
        let proof = authority(&mut tx, h, 1).await?;
        let conflict = apply_world_party_command_in_transaction(
            &mut tx,
            &proof,
            PartyAction::Invite { target: id(61) },
            None,
        )
        .await;
        assert!(
            matches!(
                conflict,
                Err(SpellItemError::Rejected("party command conflict"))
            ),
            "{conflict:?}"
        );
        tx.rollback().await?;
        assert_eq!(footprint(h).await?, (0, 0, 0, 1, 1));
        Ok(())
    })
}

#[test]
fn absent_invitation_leadership_and_transfer_never_grant_membership() -> TestResult {
    run("party_absent_permissions", async |h| {
        for (number, action, disposition) in [
            (1, PartyAction::Accept { party: id(60) }, "NOT_INVITED"),
            (2, PartyAction::Revoke { target: id(61) }, "NOT_LEADER"),
            (
                3,
                PartyAction::TransferLeader { target: id(61) },
                "NOT_MEMBER",
            ),
        ] {
            let receipt = apply(h, number, action).await?;
            assert_eq!(receipt.disposition, disposition);
            assert_eq!(receipt.party_id, None);
        }
        assert_eq!(footprint(h).await?, (0, 0, 0, 3, 3));
        Ok(())
    })
}

#[test]
fn source_authority_is_transaction_bound_and_stale_current_fences_refuse() -> TestResult {
    run("party_current_fences", async |h| {
        let mut first = h.pool.begin().await?;
        let proof = authority(&mut first, h, 1).await?;
        first.rollback().await?;
        let mut another = h.pool.begin().await?;
        assert!(matches!(
            read_world_party_in_transaction(&mut another, &proof).await,
            Err(SpellItemError::Rejected(
                "authority belongs to another physical transaction"
            ))
        ));
        another.rollback().await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        for kind in 0..3 {
            let mut stale = fence()?;
            match kind {
                0 => stale.character_lease_generation += 1,
                1 => {
                    stale.connection_generation =
                        crate::foundation::ConnectionGeneration::new(2).map_err(debug)?
                }
                _ => {
                    stale.scope_ownership_generation =
                        crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?
                }
            }
            let mut tx = h.pool.begin().await?;
            assert!(
                assert_spell_item_authority_in_transaction(
                    &mut tx,
                    &h.root,
                    &recovery,
                    &h.node,
                    &stale,
                    command(2)?,
                    [1; 32]
                )
                .await
                .is_err()
            );
            tx.rollback().await?;
        }
        assert_eq!(footprint(h).await?, (0, 0, 0, 0, 0));
        Ok(())
    })
}

#[test]
fn expired_invitation_refuses_accept_and_explicit_decline_removes_only_own_row() -> TestResult {
    run("party_expired_invitation", async |h| {
        let now = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
        let leader = committed_fresh_player(h, 60, 62, 70, now).await?;
        let other = committed_fresh_player(h, 61, 63, 71, now).await?;
        let leader_character = *leader.current_character_lease().character_id().as_bytes();
        let other_character = *other.current_character_lease().character_id().as_bytes();
        let mut tx = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_parties(party_id,world_id,leader_character_id,revision,next_seq) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,2)")
            .bind(id(70).as_slice()).bind(id(WORLD).as_slice()).bind(leader_character.as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,encode($3,'hex')::uuid)")
            .bind(leader_character.as_slice()).bind(id(70).as_slice()).bind(id(CHANNEL).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_party_invitations(party_id,invitee,seq,created_at,expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,clock_timestamp()-interval '600 seconds',clock_timestamp()-interval '300 seconds'),(encode($1,'hex')::uuid,encode($3,'hex')::uuid,3,clock_timestamp(),clock_timestamp()+interval '300 seconds')")
            .bind(id(70).as_slice()).bind(id(CHARACTER).as_slice()).bind(other_character.as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let expired = apply(h, 1, PartyAction::Accept { party: id(70) }).await?;
        assert_eq!(expired.disposition, "NOT_INVITED");
        assert_eq!(expired.party_id, None);
        assert_eq!(footprint(h).await?, (1, 1, 2, 1, 1));
        let declined = apply(h, 2, PartyAction::Decline { party: id(70) }).await?;
        assert_eq!(declined.disposition, "APPLIED");
        let remaining: Vec<Vec<u8>> =
            sqlx::query_scalar("SELECT uuid_send(invitee) FROM game_party_invitations")
                .fetch_all(&h.pool)
                .await?;
        assert_eq!(remaining, vec![other_character.to_vec()]);
        assert_eq!(footprint(h).await?, (1, 1, 1, 2, 2));
        Ok(())
    })
}

#[test]
fn durable_world_read_observes_remote_channel_members_and_new_revision() -> TestResult {
    run("party_strong_world_read", async |h| {
        let now = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
        let other = committed_fresh_player(h, 60, 62, 70, now).await?;
        let other_character = *other.current_character_lease().character_id().as_bytes();
        let mut tx = h.pool.begin().await?;
        fixture_party(&mut tx, 70, CHARACTER).await?;
        sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,1,encode($3,'hex')::uuid)")
            .bind(other_character.as_slice()).bind(id(70).as_slice()).bind(id(44).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let mut read = h.pool.begin().await?;
        let proof = authority(&mut read, h, 1).await?;
        let snapshot = read_world_party_in_transaction(&mut read, &proof)
            .await
            .map_err(debug)?;
        assert_eq!(snapshot.party_id, Some(id(70)));
        assert_eq!(snapshot.revision, Some(1));
        assert_eq!(snapshot.members.len(), 2);
        assert_eq!(snapshot.members[1].channel, Some(id(44)));
        read.commit().await?;
        sqlx::query(
            "UPDATE game_parties SET revision=revision+1 WHERE party_id=encode($1,'hex')::uuid",
        )
        .bind(id(70).as_slice())
        .execute(&h.pool)
        .await?;
        let mut fresh = h.pool.begin().await?;
        let proof = authority(&mut fresh, h, 2).await?;
        let next = read_world_party_in_transaction(&mut fresh, &proof)
            .await
            .map_err(debug)?;
        assert_eq!(next.revision, Some(2));
        assert_eq!(next.members, snapshot.members);
        fresh.commit().await?;
        Ok(())
    })
}

#[test]
fn database_enforces_fifty_members_and_atomic_rejection_of_fifty_first() -> TestResult {
    run("party_member_cap", async |h| {
        let mut tx = h.pool.begin().await?;
        fixture_party(&mut tx, 200, CHARACTER).await?;
        for (offset, seed) in (80..129).enumerate() {
            fixture_character(&mut tx, seed, WORLD).await?;
            sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,2,NULL)")
                .bind(id(seed).as_slice()).bind(id(200).as_slice()).bind((offset+2) as i64).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        assert_eq!(h.count("game_party_members").await?, 50);
        let mut overflow = h.pool.begin().await?;
        fixture_character(&mut overflow, 129, WORLD).await?;
        sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,51,2,NULL)")
            .bind(id(129).as_slice()).bind(id(200).as_slice()).execute(&mut *overflow).await?;
        assert_constraint(overflow.commit().await.expect_err("member51"));
        assert_eq!(h.count("game_party_members").await?, 50);
        Ok(())
    })
}

#[test]
fn database_enforces_twenty_current_invitations_per_invitee() -> TestResult {
    run("party_invitee_cap", async |h| {
        let mut tx = h.pool.begin().await?;
        fixture_character(&mut tx, 60, WORLD).await?;
        for offset in 0..20 {
            let leader = 80 + offset;
            let party = 160 + offset;
            fixture_character(&mut tx, leader, WORLD).await?;
            fixture_party(&mut tx, party, leader).await?;
            sqlx::query("INSERT INTO game_party_invitations(party_id,invitee,seq,expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,clock_timestamp()+interval '300 seconds')")
                .bind(id(party).as_slice()).bind(id(60).as_slice()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        assert_eq!(h.count("game_party_invitations").await?, 20);
        let mut overflow = h.pool.begin().await?;
        fixture_character(&mut overflow, 100, WORLD).await?;
        fixture_party(&mut overflow, 180, 100).await?;
        sqlx::query("INSERT INTO game_party_invitations(party_id,invitee,seq,expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,clock_timestamp()+interval '300 seconds')")
            .bind(id(180).as_slice()).bind(id(60).as_slice()).execute(&mut *overflow).await?;
        assert_constraint(overflow.commit().await.expect_err("invite21"));
        assert_eq!(h.count("game_party_invitations").await?, 20);
        assert_eq!(h.count("game_parties").await?, 20);
        Ok(())
    })
}

#[test]
fn database_enforces_fifty_current_invitations_per_party() -> TestResult {
    run("party_invitation_cap", async |h| {
        let mut tx = h.pool.begin().await?;
        fixture_party(&mut tx, 200, CHARACTER).await?;
        for (offset, seed) in (80..130).enumerate() {
            fixture_character(&mut tx, seed, WORLD).await?;
            sqlx::query("INSERT INTO game_party_invitations(party_id,invitee,seq,expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,clock_timestamp()+interval '300 seconds')")
                .bind(id(200).as_slice()).bind(id(seed).as_slice()).bind((offset + 2) as i64)
                .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        assert_eq!(h.count("game_party_invitations").await?, 50);
        let mut overflow = h.pool.begin().await?;
        fixture_character(&mut overflow, 130, WORLD).await?;
        sqlx::query("INSERT INTO game_party_invitations(party_id,invitee,seq,expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,52,clock_timestamp()+interval '300 seconds')")
            .bind(id(200).as_slice()).bind(id(130).as_slice()).execute(&mut *overflow).await?;
        assert_constraint(overflow.commit().await.expect_err("invitation51"));
        assert_eq!(h.count("game_party_invitations").await?, 50);
        Ok(())
    })
}

#[test]
fn database_rejects_members_and_invitations_from_another_world() -> TestResult {
    run("party_world_invariant", async |h| {
        let mut setup = h.pool.begin().await?;
        fixture_party(&mut setup, 200, CHARACTER).await?;
        fixture_character(&mut setup, 60, 44).await?;
        setup.commit().await?;
        let mut member = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,2,NULL)")
            .bind(id(60).as_slice()).bind(id(200).as_slice()).execute(&mut *member).await?;
        assert_constraint(member.commit().await.expect_err("cross-World member"));
        let mut invitation = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_party_invitations(party_id,invitee,seq,expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,clock_timestamp()+interval '300 seconds')")
            .bind(id(200).as_slice()).bind(id(60).as_slice()).execute(&mut *invitation).await?;
        assert_constraint(
            invitation
                .commit()
                .await
                .expect_err("cross-World invitation"),
        );
        assert_eq!(footprint(h).await?, (1, 1, 0, 0, 0));
        Ok(())
    })
}

// Independent signed fixture owner derived from the existing FreshAdmission qualification.
// Production receipts are created only by FreshAdmissionStore::commit below.
mod fresh_source {
    use super::{CHANNEL, TestResult, WORLD, debug, id};
    fn checked<T, E: std::fmt::Debug>(r: Result<T, E>) -> TestResult<T> {
        r.map_err(|e| debug(e).into())
    }
    pub(super) fn uuid_text(id: &[u8; 16]) -> String {
        let h: String = id.iter().map(|v| format!("{v:02x}")).collect();
        format!(
            "{}-{}-{}-{}-{}",
            &h[..8],
            &h[8..12],
            &h[12..16],
            &h[16..20],
            &h[20..]
        )
    }
    use crate::foundation::admission_authority_publication::*;
    use crate::foundation::fnd04_verifier::*;
    use crate::foundation::fresh_admission_durability::*;
    use crate::foundation::*;
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey};

    pub struct Source {
        pub now: i64,
        pub current: FreshCurrentEvidence,
        pub rows: Vec<AdmissionAuthorityPublicationChangeV1>,
        key: SigningKey,
        session_seed: u8,
    }
    impl fresh_source_sealed::Sealed for Source {}

    fn provenance(purpose: FreshEvidencePurposeV1, now: i64) -> FreshEvidenceProvenanceV1 {
        FreshEvidenceProvenanceV1 {
            source_authority: "independent-platform-fixture".into(),
            purpose,
            scope: Fnd04EvidenceScope::FreshAdmission,
            source_revision: 1,
            accepted_source_revision: 1,
            decision_identity: "platform-observation-1".into(),
            accepted_decision_identity: "platform-observation-1".into(),
            source_observed_at: now,
            clock_uncertainty_seconds: 0,
            publication_revision: 1,
        }
    }
    impl FreshDurabilityEvidenceSourceV1 for Source {
        fn signing_trust(
            &self,
            key_id: &str,
            _now: i64,
        ) -> Result<FreshSigningTrustObservationV1, Fnd04EvidenceError> {
            if key_id != "fresh-1" {
                return Err(Fnd04EvidenceError::ExplicitlyDenied);
            }
            Ok(FreshSigningTrustObservationV1 {
                key_id: key_id.into(),
                public_key: self.key.verifying_key().to_bytes(),
                trusted: true,
                provenance: provenance(FreshEvidencePurposeV1::SigningTrust, self.now),
            })
        }
        fn account_security(
            &self,
            account_id: &str,
            _now: i64,
        ) -> Result<FreshAccountSecurityObservationV1, Fnd04EvidenceError> {
            if account_id != self.current.account_id {
                return Err(Fnd04EvidenceError::ExplicitlyDenied);
            }
            Ok(FreshAccountSecurityObservationV1 {
                account_id: account_id.into(),
                minimum_generation: 1,
                allowed: true,
                provenance: provenance(FreshEvidencePurposeV1::PlatformSecurity, self.now),
            })
        }
    }
    impl FreshDurabilityCurrentSourceV1 for Source {
        fn current(
            &self,
            account_id: &str,
            character_id: CharacterId,
            _now: i64,
        ) -> Result<FreshPublishedCurrentObservationV1, Fnd04ConsumerError> {
            if account_id != self.current.account_id || character_id != self.current.character_id {
                return Err(Fnd04ConsumerError::FreshAccountCharacterConflict);
            }
            Ok(FreshPublishedCurrentObservationV1 {
                facts: self.current.clone(),
                account_publication_revision: 1,
                character_publication_revision: 1,
                runtime_publication_revision: 1,
                expected_lease_generation: 1,
                proposed_lease_generation: 2,
                account_presence_available: true,
                character_eligible: true,
                runtime_ready: true,
            })
        }
    }
    impl AdmissionAuthorityPublicationCurrentSourceV1 for Source {
        fn current_publications(
            &self,
            keys: &[AdmissionAuthorityGuardKeyV1],
        ) -> Result<
            Vec<Option<AdmissionAuthorityPublicationChangeV1>>,
            AdmissionAuthorityPublicationErrorV1,
        > {
            Ok(keys
                .iter()
                .map(|key| self.rows.iter().find(|row| &row.key == key).cloned())
                .collect())
        }
    }
    impl AdmissionAuthorityOwningPublisherV1 for Source {
        fn resolve_publication(
            &self,
            _now: i64,
        ) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>, AdmissionAuthorityPublicationErrorV1>
        {
            Ok(self.rows.clone())
        }
    }
    impl AdmissionClaimOwningSourceV1 for Source {
        fn prepare_fresh_claim(
            &self,
            binding: &FreshAdmissionAuditBindingV1,
            now: i64,
        ) -> Result<AdmissionClaimTransitionEvidenceV1, AdmissionAuthorityPublicationErrorV1>
        {
            let predecessors = self.rows[..2].to_vec();
            let mut successors = predecessors.clone();
            for row in &mut successors {
                row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
                    expected_publication_revision: row.publication_revision,
                };
                row.publication_revision = row
                    .publication_revision
                    .checked_add(1)
                    .ok_or(AdmissionAuthorityPublicationErrorV1::Invalid)?;
                row.source.source_revision = row
                    .source
                    .source_revision
                    .checked_add(1)
                    .ok_or(AdmissionAuthorityPublicationErrorV1::Invalid)?;
                row.source.decision_identity = "game-acquire-2".into();
                row.source.source_observed_at = now;
                match &mut row.state {
                    AdmissionAuthorityGuardStateV1::Account { security, presence } => {
                        security.provenance.publication_revision = row.publication_revision;
                        *presence = Some((self.current.character_id, binding.candidate_session));
                    }
                    AdmissionAuthorityGuardStateV1::Character {
                        lease_generation,
                        holder,
                        ..
                    } => {
                        *lease_generation = lease_generation
                            .checked_add(1)
                            .ok_or(AdmissionAuthorityPublicationErrorV1::Invalid)?;
                        *holder = Some(binding.candidate_session);
                    }
                    _ => return Err(AdmissionAuthorityPublicationErrorV1::Invalid),
                }
            }
            Ok(AdmissionClaimTransitionEvidenceV1 {
                predecessors,
                successors,
                prepared_at: now,
            })
        }
    }
    impl Source {
        pub fn new(
            now: i64,
            character: [u8; 16],
            account_seed: u8,
            session_seed: u8,
        ) -> TestResult<Self> {
            let mut source = Self {
                now,
                key: SigningKey::from_bytes(&[31; 32]),
                session_seed,
                rows: vec![],
                current: FreshCurrentEvidence {
                    account_id: uuid_text(&id(account_seed)),
                    character_id: checked(CharacterId::decode(&character))?,
                    world_id: checked(WorldId::decode(&id(WORLD)))?,
                    channel_id: checked(ChannelId::decode(&id(CHANNEL)))?,
                    character_lease_generation: 1,
                    scope_ownership_generation: 1,
                    route_revision: "route-1".into(),
                    runtime_observation_revision: "runtime-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    content_revision: "content-1".into(),
                    map_revision: "map-1".into(),
                    world_policy_revision: "world-policy-1".into(),
                    offer_revision: "offer-1".into(),
                },
            };
            let f = &source.current;
            let security = checked(source.account_security(&f.account_id, now))?;
            let entries = vec![
                (
                    AdmissionAuthorityGuardKeyV1::Account {
                        account_id: f.account_id.clone(),
                    },
                    AdmissionPublicationPurposeV1::AccountSecurityAndPresence,
                    AdmissionAuthorityGuardStateV1::Account {
                        security,
                        presence: None,
                    },
                ),
                (
                    AdmissionAuthorityGuardKeyV1::Character(f.character_id),
                    AdmissionPublicationPurposeV1::CharacterOwnershipAndLease,
                    AdmissionAuthorityGuardStateV1::Character {
                        account_id: f.account_id.clone(),
                        world_id: f.world_id,
                        eligible: true,
                        lease_generation: 1,
                        holder: None,
                    },
                ),
                (
                    AdmissionAuthorityGuardKeyV1::Runtime(RuntimeScopeRefV1::channel(
                        f.world_id,
                        f.channel_id,
                    )),
                    AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
                    AdmissionAuthorityGuardStateV1::Runtime {
                        ownership_generation: 1,
                        ready: true,
                        route_revision: f.route_revision.clone(),
                        runtime_observation_revision: f.runtime_observation_revision.clone(),
                        protocol_major: 1,
                        transport_profile: 1,
                        ruleset_revision: f.ruleset_revision.clone(),
                        content_revision: f.content_revision.clone(),
                        map_revision: f.map_revision.clone(),
                        world_policy_revision: f.world_policy_revision.clone(),
                        offer_revision: f.offer_revision.clone(),
                    },
                ),
                (
                    AdmissionAuthorityGuardKeyV1::SigningTrust {
                        key_id: "fresh-1".into(),
                        profile: PRE_ADMISSION_PROFILE.into(),
                    },
                    AdmissionPublicationPurposeV1::FixedFreshSigningTrust,
                    AdmissionAuthorityGuardStateV1::SigningTrust {
                        public_key: source.key.verifying_key().to_bytes(),
                        trusted: true,
                    },
                ),
            ];
            source.rows = entries
                .into_iter()
                .map(
                    |(key, purpose, state)| AdmissionAuthorityPublicationChangeV1 {
                        key,
                        state,
                        publication_revision: 1,
                        precondition: AdmissionPublicationPreconditionV1::Bootstrap {
                            restored_publication_high_water: Some(0),
                        },
                        source: AdmissionPublicationSourceV1 {
                            authority: if purpose
                                == AdmissionPublicationPurposeV1::FixedFreshSigningTrust
                            {
                                "independent-platform-fixture"
                            } else {
                                "independent-game-fixture"
                            }
                            .into(),
                            purpose,
                            source_revision: 1,
                            decision_identity: if purpose
                                == AdmissionPublicationPurposeV1::FixedFreshSigningTrust
                            {
                                "platform-observation-1"
                            } else {
                                "game-observation-1"
                            }
                            .into(),
                            source_observed_at: now,
                            clock_uncertainty_seconds: 0,
                        },
                    },
                )
                .collect();
            Ok(source)
        }
        pub fn request(&self) -> TestResult<FreshAdmissionCommitRequestV1> {
            let header = r#"{"alg":"Ed25519","kid":"fresh-1","typ":"oteryn-admission+jwt"}"#;
            let mut payload: serde_json::Value = serde_json::from_str(&fresh_payload())?;
            payload["iat"] = self.now.into();
            payload["nbf"] = self.now.into();
            payload["exp"] = (self.now + 10).into();
            payload["jti"] = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode([self.session_seed; 32])
                .into();
            payload["attempt_ref"] = uuid_text(&id(self.session_seed)).into();
            payload["account_id"] = self.current.account_id.clone().into();
            payload["character_id"] = uuid_text(self.current.character_id.as_bytes()).into();
            payload["world_id"] = uuid_text(self.current.world_id.as_bytes()).into();
            payload["channel_id"] = uuid_text(self.current.channel_id.as_bytes()).into();
            payload["ruleset_revision"] = self.current.ruleset_revision.clone().into();
            payload["world_policy_revision"] = self.current.world_policy_revision.clone().into();
            let token = signed_token(&self.key, header, payload.to_string());
            let facts = checked(verify_fresh_grant_durability_v1(
                &token,
                self.now,
                &FreshDurabilityTrustContext::from_owning_source(self),
                &FreshDurabilityCurrentAuthorityV1::from_owning_source(self),
            ))?;
            let authorization = checked(FreshAdmissionCommitAuthorizationV1::new(
                &facts,
                checked(GameSessionId::decode(&id(self.session_seed)))?,
                checked(AuthenticatedTransportRefV1::decode(&id(self.session_seed)))?,
                self,
                self.now,
            ))?;
            let transition = checked(FreshAdmissionClaimTransitionV1::prepare(
                self,
                &authorization,
                self.now,
            ))?;
            let mut flow = checked(FreshAdmissionDurabilityFlowV1::begin(
                authorization,
                transition,
            ))?;
            let mut capture = Capture(None);
            checked(flow.submit(&mut capture))?;
            capture
                .0
                .ok_or_else(|| "fresh fixture did not submit".into())
        }
    }
    struct Capture(Option<FreshAdmissionCommitRequestV1>);
    impl FreshAdmissionDurabilityPortV1 for Capture {
        fn submit(
            &mut self,
            request: &FreshAdmissionCommitRequestV1,
        ) -> FreshAdmissionSubmissionV1 {
            if self.0.is_some() {
                return FreshAdmissionSubmissionV1::Unavailable;
            }
            self.0 = Some(request.clone());
            FreshAdmissionSubmissionV1::Accepted
        }
        fn reconcile(&mut self, _: &FreshAdmissionOperationV1) -> FreshAdmissionSubmissionV1 {
            FreshAdmissionSubmissionV1::Unavailable
        }
    }
    fn fresh_payload() -> String {
        let nonce = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([7; 32]);
        format!(
            r#"{{"iss":"urn:oteryn:platform:game-admission","aud":"urn:oteryn:game:admission","iat":100,"nbf":100,"exp":110,"jti":"{nonce}","profile":"oteryn-pre-admission-v1","purpose":"fresh_entry","attempt_ref":"00000000-0000-7000-8000-000000000001","account_id":"00000000-0000-4000-8000-000000000001","character_id":"00000000-0000-7000-8000-000000000002","world_id":"00000000-0000-7000-8000-000000000003","channel_id":"00000000-0000-7000-8000-000000000004","account_security_generation":"1","route_revision":"route-1","runtime_observation_revision":"runtime-1","scope_ownership_generation":"1","protocol_major":1,"transport_profile":1,"ruleset_revision":"rules-1","content_revision":"content-1","map_revision":"map-1","world_policy_revision":"policy-1","offer_revision":"offer-1"}}"#
        )
    }

    fn signed_token(signing_key: &SigningKey, header: &str, payload: String) -> String {
        let encoded_header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header);
        let encoded_payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);
        let signing_input = format!("{encoded_header}.{encoded_payload}");
        let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(signing_key.sign(signing_input.as_bytes()).to_bytes());
        format!("{signing_input}.{signature}")
    }
}

async fn bootstrap_party_character(
    h: &Harness,
    character: u8,
    account: u8,
    now: i64,
) -> TestResult<crate::domain::CharacterId> {
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
            namespace: "party-fixture-source".into(),
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
    let account_text = fresh_source::uuid_text(&id(account));
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
        "contract_version": 2, "variant": "OPERATOR_CONTROL_PLANE_BOOTSTRAP",
        "issuer_authority": "OTERYN_PLATFORM_CHARACTER_AUTHORITY",
        "issuer_decision_id": fresh_source::uuid_text(&id(character + 20)),
        "source_revision": revision.to_string(),
        "operation_id": fresh_source::uuid_text(&id(character)),
        "operation": "INITIAL_CHARACTER_BOOTSTRAP", "account_id": account_text,
        "target_world_id": fresh_source::uuid_text(&id(WORLD)),
        "requested_name": format!("Party Player {}{}", char::from(b'A' + character / 26), char::from(b'a' + character % 26)),
        "interpretation_context": {"profile_revision":"profile-1", "ruleset_revision":"ruleset-1",
            "content_revision":"content-1", "starter_template_revision":"starter-1"},
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

async fn committed_fresh_player(
    h: &Harness,
    character: u8,
    account: u8,
    session: u8,
    now: i64,
) -> TestResult<
    crate::foundation::GameSessionAuthoritySnapshot<crate::foundation::AuthenticatedTransportRefV1>,
> {
    use crate::durability::admission_authority_guards::AdmissionGuardStore;
    use crate::durability::fresh_admission::FreshAdmissionStore;
    use crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationV1;
    use crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
    let character_id = bootstrap_party_character(h, character, account, now).await?;
    let mut source = fresh_source::Source::new(now, *character_id.as_bytes(), account, session)?;
    let guards = AdmissionGuardStore::from_root(h.root.clone());
    // Keep independently committed runtime/signing publications exactly as actually read.
    // The fixture does not overwrite the actual assignment owner or invent its high-water.
    let rows = guards
        .load(&[source.rows[2].key.clone(), source.rows[3].key.clone()])
        .await
        .map_err(debug)?;
    for (index, row) in rows.into_iter().enumerate() {
        if let Some(row) = row {
            source.rows[index + 2] = row;
        }
    }
    let publication = AdmissionAuthorityPublicationV1::prepare(&source, now).map_err(debug)?;
    guards.publish(&publication).await.map_err(debug)?;
    let request = source.request()?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let outcome = store.commit(&request).await.map_err(debug)?;
    let FreshAdmissionDurableOutcomeV1::Committed(receipt) = outcome else {
        return Err(format!("genuine Fresh admission not committed:{outcome:?}").into());
    };
    assert_eq!(receipt.operation(), request.operation());
    assert!(receipt.decided_at() >= now);
    let current = store
        .current_session(GameSessionId::decode(&id(session)).map_err(debug)?)
        .await
        .map_err(debug)?;
    assert_eq!(
        current.session_state(),
        crate::foundation::GameSessionState::Active
    );
    assert_eq!(current.current_connection_generation().get(), 1);
    assert_eq!(current.current_character_lease().generation(), 2);
    Ok(current)
}

/// D88 initialization and A13 vocation choice use the actual Character writers. The
/// bounded two-level policy is the existing D88 test policy, used only to bind initial
/// level 1 / zero XP; it grants no XP and makes no claim about the full source XP curve.
async fn initialize_official_party_build(
    h: &Harness,
    current: &crate::foundation::GameSessionAuthoritySnapshot<
        crate::foundation::AuthenticatedTransportRefV1,
    >,
    vocation: &str,
    occurrence_seed: u8,
) -> TestResult<(
    crate::spell::cast::CharacterCastFacts,
    crate::durability::character_build::DurableBuildState,
    crate::durability::monk_state::DurableMonkState,
)> {
    use crate::domain::progression::{
        FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
    };
    use crate::domain::{CharacterId, CharacterRevision};
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildOccurrence, convert_vocation,
    };
    use crate::durability::character_progression::{
        CurrentCharacterGameplayFence, ProgressionInitializationRequest,
    };
    use crate::gameplay_transport::{CastFactsLoad, load_character_cast_facts};
    use oteryn_simulation_determinism::{ExactI64, RoundingMode};
    let character =
        CharacterId::from_bytes(*current.current_character_lease().character_id().as_bytes())
            .map_err(debug)?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    assert!(
        matches!(load_character_cast_facts(&h.root, &authority, character).await,
        CastFactsLoad::NoVocation { character_revision } if character_revision.get() == 1)
    );
    let fence = CurrentCharacterGameplayFence {
        character_id: character,
        game_session_id: current.current_game_session_id(),
        connection_generation: current.current_connection_generation(),
        character_lease_generation: current.current_character_lease().generation(),
        runtime_scope: current.current_runtime_scope(),
        scope_ownership_generation: current.current_scope_generation(),
        expected_character_revision: CharacterRevision::new(1).map_err(debug)?,
    };
    let context = ProgressionRevisionContext {
        profile: "profile-1".into(),
        ruleset: "ruleset-1".into(),
        content: "content-1".into(),
        simulation: "simulation-1".into(),
        evidence: "evidence-1".into(),
        declaration: "declaration-1".into(),
    };
    h.root
        .initialize_character_progression(
            &authority,
            &h.node,
            fence,
            ProgressionInitializationRequest {
                context: context.clone(),
                policy_revision: "policy-1".into(),
                reward_revision: "reward-1".into(),
                policy: FiniteProgressionPolicy {
                    context,
                    policy_revision: "policy-1".into(),
                    reward_revision: "reward-1".into(),
                    death_policy_revision: "death-1".into(),
                    declared_difference_revision: "declaration-1".into(),
                    thresholds: [
                        LevelThreshold {
                            level: 1,
                            minimum_experience: ExactI64::new(0),
                        },
                        LevelThreshold {
                            level: 2,
                            minimum_experience: ExactI64::new(100),
                        },
                    ],
                    terminal_exclusive_experience: ExactI64::new(200),
                    death_loss_numerator: 1,
                    death_loss_denominator: 10,
                    death_loss_rounding: RoundingMode::Floor,
                },
            },
        )
        .await
        .map_err(debug)?;
    let before = h
        .root
        .read_character_build_state(&authority, character)
        .await
        .map_err(debug)?;
    assert_eq!(
        before,
        crate::durability::character_build::DurableBuildState::default()
    );
    let formula = crate::spell::mana_training::CompiledTrainingFormula::from_profile(
        include_bytes!("../../../../tools/content-schema/native-gameplay/build-training.json"),
        "content-1",
    )
    .map_err(debug)?;
    let after = convert_vocation(&formula, &before, vocation).map_err(debug)?;
    h.root
        .commit_character_build(
            &authority,
            &h.node,
            fence,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(occurrence_seed)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before,
                after: after.clone(),
                pruned_stance: None,
            },
            &formula,
        )
        .await
        .map_err(debug)?;
    let CastFactsLoad::Ready {
        facts,
        character_revision,
    } = load_character_cast_facts(&h.root, &authority, character).await
    else {
        return Err("real classed Character facts unavailable".into());
    };
    assert_eq!(character_revision.get(), 2);
    assert_eq!(facts.level, 1);
    assert_eq!(facts.magic_level, u32::from(after.magic().0));
    let progression = h
        .root
        .read_character_progression(&authority, character)
        .await
        .map_err(debug)?
        .ok_or("missing initialized progression")?;
    assert_eq!(progression.level, 1);
    assert_eq!(progression.total_experience.get(), 0);
    let build = h
        .root
        .read_character_build_state(&authority, character)
        .await
        .map_err(debug)?;
    assert_eq!(build, after);
    let monk = h
        .root
        .read_character_monk_state(&authority, character)
        .await
        .map_err(debug)?;
    Ok((facts, build, monk))
}

#[test]
fn genuine_two_fresh_players_require_target_consent_and_block_policy_before_invite_accept()
-> TestResult {
    run("party_genuine_fresh_players", async |h| {
        use crate::durability::admission_journal::party_target_binding::prove_visible_party_target_in_transaction;
        use crate::foundation::{ChannelContentPin, ChannelId, ChannelRuntimeV1, WorldId};
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        let caster = committed_fresh_player(h, 60, 62, 70, now).await?;
        let target = committed_fresh_player(h, 61, 63, 71, now).await?;
        let (caster_facts, caster_build, caster_monk) =
            initialize_official_party_build(h, &caster, "monk", 90).await?;
        let (target_facts, target_build, target_monk) =
            initialize_official_party_build(h, &target, "knight", 91).await?;
        assert_eq!(caster_facts.vocation, crate::spell::Vocation::Monk);
        assert_eq!(target_facts.vocation, crate::spell::Vocation::Knight);
        let caster_character = *caster.current_character_lease().character_id().as_bytes();
        let target_character = *target.current_character_lease().character_id().as_bytes();
        let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
            h.node.fact().node_id(),
            h.node.fact().registration_revision(),
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            ChannelContentPin::test(world),
        )
        .map_err(debug)?;
        let caster_session = caster.current_game_session_id();
        let target_session = target.current_game_session_id();
        let reserved = runtime
            .reserve_fresh_session(caster_session)
            .map_err(debug)?;
        let caster_actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
        runtime
            .initialize_first_entry_position(caster_actor)
            .map_err(debug)?;
        let reserved = runtime
            .reserve_fresh_session(target_session)
            .map_err(debug)?;
        let target_actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
        runtime
            .initialize_first_entry_position(target_actor)
            .map_err(debug)?;
        let mut states = crate::gameplay_transport::ChannelSpellStates::default();
        let time = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0);
        for (actor, session, facts, build, monk) in [
            (
                caster_actor,
                caster_session,
                caster_facts,
                &caster_build,
                &caster_monk,
            ),
            (
                target_actor,
                target_session,
                target_facts,
                &target_build,
                &target_monk,
            ),
        ] {
            states
                .initialize(
                    &runtime,
                    actor,
                    session,
                    facts,
                    (monk.harmony(), monk.serene_forced_remaining_micros()),
                    time,
                )
                .ok_or("source Character facts failed actual player initialization")?;
            assert!(states.load_owned_build(&runtime, actor, session, build));
        }
        let caster_fence = CurrentCharacterItemFence {
            character_id: crate::domain::CharacterId::from_bytes(caster_character)
                .map_err(debug)?,
            game_session_id: caster_session,
            connection_generation: caster.current_connection_generation(),
            character_lease_generation: caster.current_character_lease().generation(),
            runtime_scope: caster.current_runtime_scope(),
            scope_ownership_generation: caster.current_scope_generation(),
        };
        let target_fence = CurrentCharacterItemFence {
            character_id: crate::domain::CharacterId::from_bytes(target_character)
                .map_err(debug)?,
            game_session_id: target_session,
            connection_generation: target.current_connection_generation(),
            character_lease_generation: target.current_character_lease().generation(),
            runtime_scope: target.current_runtime_scope(),
            scope_ownership_generation: target.current_scope_generation(),
        };
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        for (number, policy) in [(1, None), (2, Some(2)), (3, Some(1)), (4, Some(1))] {
            if let Some(policy) = policy {
                sqlx::query("INSERT INTO game_character_social_settings(character_id,party_invites,channel_visibility,source_policy) VALUES(encode($1,'hex')::uuid,$2,1,'PARTYPVP0-PARTIES-AND-PVP-V1') ON CONFLICT(character_id) DO UPDATE SET party_invites=EXCLUDED.party_invites")
                    .bind(target_character.as_slice()).bind(policy as i16).execute(&h.pool).await?;
            }
            if number == 3 {
                sqlx::query("INSERT INTO game_character_social_blocks(character_id,blocked_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
                    .bind(target_character.as_slice()).bind(caster_character.as_slice()).execute(&h.pool).await?;
            } else if number == 4 {
                sqlx::query("DELETE FROM game_character_social_blocks WHERE character_id=encode($1,'hex')::uuid")
                    .bind(target_character.as_slice()).execute(&h.pool).await?;
            }
            let mut tx = h.pool.begin().await?;
            let proof = assert_spell_item_authority_in_transaction(
                &mut tx,
                &h.root,
                &recovery,
                &h.node,
                &caster_fence,
                CommandRef::new(caster_session, CommandId::new(number).map_err(debug)?),
                [1; 32],
            )
            .await
            .map_err(debug)?;
            let visible = prove_visible_party_target_in_transaction(
                &mut tx,
                &h.root,
                &proof,
                &runtime,
                caster_actor,
                target_actor,
                target_session,
            )
            .await
            .map_err(debug)?
            .ok_or("genuine current Fresh target was not qualified")?;
            assert_eq!(visible.character(), target_character);
            let result = apply_world_party_command_in_transaction(
                &mut tx,
                &proof,
                PartyAction::Invite {
                    target: target_character,
                },
                Some(&visible),
            )
            .await
            .map_err(debug)?;
            assert_eq!(
                result.disposition,
                if number < 4 { "NOT_VISIBLE" } else { "APPLIED" }
            );
            tx.commit().await?;
            assert_eq!(
                h.count("game_parties").await?,
                if number < 4 { 0 } else { 1 }
            );
        }
        let party: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(party_id) FROM game_parties")
            .fetch_one(&h.pool)
            .await?;
        let party: [u8; 16] = party.try_into().map_err(|_| "party ID")?;
        let mut accept = h.pool.begin().await?;
        let proof = assert_spell_item_authority_in_transaction(
            &mut accept,
            &h.root,
            &recovery,
            &h.node,
            &target_fence,
            CommandRef::new(target_session, CommandId::new(1).map_err(debug)?),
            [1; 32],
        )
        .await
        .map_err(debug)?;
        let accepted = apply_world_party_command_in_transaction(
            &mut accept,
            &proof,
            PartyAction::Accept { party },
            None,
        )
        .await
        .map_err(debug)?;
        assert_eq!(accepted.disposition, "APPLIED");
        let snapshot = read_world_party_in_transaction(&mut accept, &proof)
            .await
            .map_err(debug)?;
        assert_eq!(snapshot.party_id, Some(party));
        assert_eq!(
            snapshot
                .members
                .iter()
                .map(|m| m.character)
                .collect::<Vec<_>>(),
            vec![caster_character, target_character]
        );
        accept.commit().await?;
        let mut view_tx = h.pool.begin().await?;
        let view_authority = assert_spell_item_authority_in_transaction(
            &mut view_tx,
            &h.root,
            &recovery,
            &h.node,
            &caster_fence,
            CommandRef::new(caster_session, CommandId::new(5).map_err(debug)?),
            [1; 32],
        )
        .await
        .map_err(debug)?;
        let view = crate::gameplay_transport::read_source_party_world_in_transaction(
            &mut view_tx,
            &h.root,
            &view_authority,
            &runtime,
            &states,
            caster_actor,
            caster_session,
        )
        .await
        .map_err(debug)?;
        assert_eq!(view.source().party_id, Some(party));
        assert_eq!(view.source().members.len(), 2);
        assert_eq!(view.member_facts().len(), 2);
        assert_eq!(view.caster_facts().vocation, crate::spell::Vocation::Monk);
        assert!(
            view.member_facts()
                .iter()
                .any(|f| f.vocation == crate::spell::Vocation::Knight)
        );
        assert_eq!(
            crate::spell::party::PartyWorld::party(&view)
                .ok_or("missing actual party view")?
                .len(),
            2
        );
        assert_eq!(
            crate::spell::harmony::SereneWorld::party(&view)
                .ok_or("missing actual Serene view")?
                .len(),
            2
        );
        let independent = read_world_party_in_transaction(&mut view_tx, &view_authority)
            .await
            .map_err(debug)?;
        assert!(view.matches_current_world(&independent));
        assert_eq!(view.player_predecessors().len(), 2);
        view.validate_physical(&runtime, &states).map_err(debug)?;
        view_tx.commit().await?;
        // The publisher's source qualifier consumes the same genuine canonical session and
        // an existing VIS-2 index populated from actual current physical actor reads. An empty
        // actual outbox must neither fabricate an event nor consume a presentation revision.
        let mut interest = crate::movement::interest::InterestIndex::new();
        for (actor, position, _) in runtime.positioned_actor_census().map_err(debug)? {
            let position = position.position();
            interest.upsert(crate::movement::interest::InterestEntity {
                identity: actor.placement_identity(),
                position: crate::movement::interest::VisibilityPosition::new(
                    position.x,
                    position.y,
                    position.floor,
                )
                .map_err(debug)?,
                revision: 1,
            });
        }
        let mut empty_outbox = crate::gameplay_transport::SpellPresentationOwner::new(&runtime);
        let turn = crate::gameplay_transport::CandidatePresentationTurn::take(
            &mut empty_outbox,
            &runtime,
            1,
        )
        .map_err(debug)?;
        let mut publisher =
            crate::gameplay_transport::CandidateSessionPublisher::from_retained_revision(41);
        let mut publish_tx = h.pool.begin().await?;
        let scope =
            crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
                &mut publish_tx,
                &h.root,
                &recovery,
                &h.node,
                caster.current_runtime_scope(),
                1,
            )
            .await
            .map_err(debug)?;
        let source = crate::gameplay_transport::ObserverSource {
            root: &h.root,
            scope: &scope,
            runtime: &runtime,
            index: &interest,
            settings: crate::movement::interest::VisibilitySettings::REFERENCE,
            actor: caster_actor,
            session: caster_session,
        };
        assert!(
            !publisher
                .publish_current(&mut publish_tx, &source, &turn)
                .await
                .map_err(debug)?
        );
        assert!(
            publisher
                .pop_for_current_transport(&mut publish_tx, &source)
                .await
                .map_err(debug)?
                .is_none()
        );
        publish_tx.commit().await?;
        // Presence is an independently current scoped operation, not a fabricated cast.
        // A consent change exposes no channel and advances only the member presence revision.
        sqlx::query("UPDATE game_character_social_settings SET channel_visibility=2 WHERE character_id=encode($1,'hex')::uuid")
            .bind(target_character.as_slice()).execute(&h.pool).await?;
        let mut presence_tx = h.pool.begin().await?;
        let scope =
            crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
                &mut presence_tx,
                &h.root,
                &recovery,
                &h.node,
                target.current_runtime_scope(),
                target.current_scope_generation().get(),
            )
            .await
            .map_err(debug)?;
        let hidden = crate::durability::world_party::refresh_world_party_presence_for_scope_actor_in_transaction(
            &mut presence_tx, &h.root, &scope, &runtime, target_actor, target_session,
        ).await.map_err(debug)?;
        assert_eq!(hidden.revision, independent.revision);
        let old_member = independent
            .members
            .iter()
            .find(|m| m.character == target_character)
            .ok_or("old member")?;
        let hidden_member = hidden
            .members
            .iter()
            .find(|m| m.character == target_character)
            .ok_or("hidden member")?;
        assert_eq!(
            hidden_member.presence,
            crate::foundation::PartyPresence::Hidden
        );
        assert_eq!(hidden_member.channel, None);
        assert_eq!(
            hidden_member.presence_revision,
            old_member.presence_revision + 1
        );
        let current_caster_view =
            crate::durability::world_party::read_world_party_for_scope_actor_in_transaction(
                &mut presence_tx,
                &h.root,
                &scope,
                &runtime,
                caster_actor,
                caster_session,
            )
            .await
            .map_err(debug)?;
        assert!(!view.matches_current_world(&current_caster_view));
        let idempotent = crate::durability::world_party::refresh_world_party_presence_for_scope_actor_in_transaction(
            &mut presence_tx, &h.root, &scope, &runtime, target_actor, target_session,
        ).await.map_err(debug)?;
        assert_eq!(idempotent.members, hidden.members);
        presence_tx.commit().await?;
        sqlx::query("UPDATE game_character_social_settings SET channel_visibility=1 WHERE character_id=encode($1,'hex')::uuid")
            .bind(target_character.as_slice()).execute(&h.pool).await?;
        let mut reveal_tx = h.pool.begin().await?;
        let scope =
            crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
                &mut reveal_tx,
                &h.root,
                &recovery,
                &h.node,
                target.current_runtime_scope(),
                target.current_scope_generation().get(),
            )
            .await
            .map_err(debug)?;
        let visible = crate::durability::world_party::refresh_world_party_presence_for_scope_actor_in_transaction(
            &mut reveal_tx, &h.root, &scope, &runtime, target_actor, target_session,
        ).await.map_err(debug)?;
        assert_eq!(visible.revision, independent.revision);
        let revealed = visible
            .members
            .iter()
            .find(|m| m.character == target_character)
            .ok_or("revealed member")?;
        assert_eq!(revealed.channel, Some(id(CHANNEL)));
        assert_eq!(revealed.presence_revision, old_member.presence_revision + 2);
        reveal_tx.commit().await?;
        assert_eq!(footprint(h).await?, (1, 2, 0, 5, 5));
        let canonical: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_durability_fresh_admission_receipts")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(canonical, 2);
        Ok(())
    })
}

#[test]
fn real_scoped_expiry_preserves_live_invites_and_disbands_only_empty_leader_parties() -> TestResult
{
    run("party_actual_scoped_expiry", async |h| {
        use crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction;
        use crate::durability::world_party::cleanup_expired_world_party_invitations_in_transaction;
        let now = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
        let first = committed_fresh_player(h, 60, 62, 70, now).await?;
        let second = committed_fresh_player(h, 61, 63, 71, now).await?;
        let first_character = *first.current_character_lease().character_id().as_bytes();
        let second_character = *second.current_character_lease().character_id().as_bytes();
        // Only time and party rows are administrative invariant fixtures. Character roots,
        // bootstrap/Fresh receipts and the current scope authority are genuinely owner-written.
        let mut setup = h.pool.begin().await?;
        fixture_party(&mut setup, 200, CHARACTER).await?;
        sqlx::query("INSERT INTO game_parties(party_id,world_id,leader_character_id,revision,next_seq) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,2)")
            .bind(id(201).as_slice()).bind(id(WORLD).as_slice()).bind(first_character.as_slice()).execute(&mut *setup).await?;
        sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,encode($3,'hex')::uuid)")
            .bind(first_character.as_slice()).bind(id(201).as_slice()).bind(id(CHANNEL).as_slice()).execute(&mut *setup).await?;
        sqlx::query("INSERT INTO game_party_invitations(party_id,invitee,seq,created_at,expires_at) VALUES(encode($1,'hex')::uuid,encode($3,'hex')::uuid,2,clock_timestamp()-interval '1200 seconds',clock_timestamp()-interval '900 seconds'),(encode($1,'hex')::uuid,encode($4,'hex')::uuid,3,clock_timestamp(),clock_timestamp()+interval '300 seconds'),(encode($2,'hex')::uuid,encode($4,'hex')::uuid,2,clock_timestamp()-interval '900 seconds',clock_timestamp()-interval '600 seconds')")
            .bind(id(200).as_slice()).bind(id(201).as_slice()).bind(first_character.as_slice()).bind(second_character.as_slice()).execute(&mut *setup).await?;
        setup.commit().await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let scope = first.current_runtime_scope();
        let mut stale = h.pool.begin().await?;
        assert!(
            assert_spell_item_scope_in_transaction(
                &mut stale, &h.root, &recovery, &h.node, scope, 2
            )
            .await
            .is_err()
        );
        stale.rollback().await?;
        assert_eq!(footprint(h).await?, (2, 2, 3, 0, 0));
        let mut drain = h.pool.begin().await?;
        let authority = assert_spell_item_scope_in_transaction(
            &mut drain, &h.root, &recovery, &h.node, scope, 1,
        )
        .await
        .map_err(debug)?;
        assert_eq!(
            cleanup_expired_world_party_invitations_in_transaction(&mut drain, &authority)
                .await
                .map_err(debug)?,
            1
        );
        drain.commit().await?;
        assert_eq!(footprint(h).await?, (2, 2, 2, 0, 0));
        let revision: i64 = sqlx::query_scalar(
            "SELECT revision::bigint FROM game_parties WHERE party_id=encode($1,'hex')::uuid",
        )
        .bind(id(200).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(revision, 2);
        let mut wrong_tx = h.pool.begin().await?;
        assert!(
            cleanup_expired_world_party_invitations_in_transaction(&mut wrong_tx, &authority)
                .await
                .is_err()
        );
        wrong_tx.rollback().await?;
        let mut drain = h.pool.begin().await?;
        let authority = assert_spell_item_scope_in_transaction(
            &mut drain, &h.root, &recovery, &h.node, scope, 1,
        )
        .await
        .map_err(debug)?;
        assert_eq!(
            cleanup_expired_world_party_invitations_in_transaction(&mut drain, &authority)
                .await
                .map_err(debug)?,
            1
        );
        drain.commit().await?;
        assert_eq!(footprint(h).await?, (1, 1, 1, 0, 0));
        let remaining: Vec<Vec<u8>> =
            sqlx::query_scalar("SELECT uuid_send(party_id) FROM game_parties")
                .fetch_all(&h.pool)
                .await?;
        assert_eq!(remaining, vec![id(200).to_vec()]);
        // A repeated current turn leaves the still-live invitation and party untouched.
        assert_eq!(
            h.root
                .drain_world_party_expiry(&recovery, &h.node, scope, 1)
                .await
                .map_err(debug)?,
            0
        );
        sqlx::query("UPDATE game_party_invitations SET created_at=clock_timestamp()-interval '600 seconds',expires_at=clock_timestamp()-interval '300 seconds' WHERE party_id=encode($1,'hex')::uuid")
            .bind(id(200).as_slice()).execute(&h.pool).await?;
        assert_eq!(
            h.root
                .drain_world_party_expiry(&recovery, &h.node, scope, 1)
                .await
                .map_err(debug)?,
            1
        );
        assert_eq!(footprint(h).await?, (0, 0, 0, 0, 0));
        assert_eq!(
            h.root
                .drain_world_party_expiry(&recovery, &h.node, scope, 1)
                .await
                .map_err(debug)?,
            0
        );
        Ok(())
    })
}

#[test]
fn genuine_session_absence_hides_presence_without_removing_protected_or_active_members()
-> TestResult {
    run("party_offline_less_disclosure", async |h| {
        let now = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
        let active = committed_fresh_player(h, 60, 62, 70, now).await?;
        let active_character = *active.current_character_lease().character_id().as_bytes();
        // This actual bootstrapped Character was never admitted: SQL absence is real.
        // No fabricated terminal admission receipt or combat-clear proof is created.
        let absent = bootstrap_party_character(h, 61, 63, now).await?;
        let mut setup = h.pool.begin().await?;
        fixture_party(&mut setup, 200, CHARACTER).await?;
        for (seq, character) in [(2_i64, active_character), (3, *absent.as_bytes())] {
            sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,1,encode($4,'hex')::uuid)")
                .bind(character.as_slice()).bind(id(200).as_slice()).bind(seq).bind(id(CHANNEL).as_slice()).execute(&mut *setup).await?;
        }
        setup.commit().await?;
        let states: Vec<i16> = sqlx::query_scalar(
            "SELECT session_state FROM game_durability_reconnect_sessions ORDER BY session_state",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(
            states,
            vec![1, 2],
            "legacy protected and genuine Fresh ACTIVE sessions exist"
        );
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut tx = h.pool.begin().await?;
        let scope =
            crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
                &mut tx,
                &h.root,
                &recovery,
                &h.node,
                active.current_runtime_scope(),
                1,
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            crate::durability::world_party::cleanup_world_party_offline_presence_in_transaction(
                &mut tx, &scope,
            )
            .await
            .map_err(debug)?,
            1
        );
        tx.commit().await?;
        type MemberRow = (Vec<u8>, i16, Option<Vec<u8>>, i64, bool);
        let rows: Vec<MemberRow> = sqlx::query_as("SELECT uuid_send(character_id),presence_state,uuid_send(presence_channel_id),presence_revision::bigint,absence_observed_at IS NOT NULL FROM game_party_members ORDER BY seq")
            .fetch_all(&h.pool).await?;
        assert_eq!(
            rows,
            vec![
                (
                    id(CHARACTER).to_vec(),
                    1,
                    Some(id(CHANNEL).to_vec()),
                    1,
                    false
                ),
                (
                    active_character.to_vec(),
                    1,
                    Some(id(CHANNEL).to_vec()),
                    1,
                    false
                ),
                (absent.as_bytes().to_vec(), 3, None, 2, true),
            ]
        );
        assert_eq!(footprint(h).await?, (1, 3, 0, 0, 0));
        let revision: i64 = sqlx::query_scalar("SELECT revision::bigint FROM game_parties")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(revision, 2);
        assert_eq!(
            h.root
                .drain_world_party_offline_presence(
                    &recovery,
                    &h.node,
                    active.current_runtime_scope(),
                    1
                )
                .await
                .map_err(debug)?,
            0
        );
        assert_eq!(footprint(h).await?, (1, 3, 0, 0, 0));
        let unchanged: i64 = sqlx::query_scalar("SELECT revision::bigint FROM game_parties")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(unchanged, revision);
        Ok(())
    })
}
