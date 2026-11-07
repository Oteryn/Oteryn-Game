//! S17 P1: revisioned Character consent, default off. These values describe a
//! durable preference; they do not prove current visibility or authorize egress.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};

/// Historical preference only. Missing rows have revision zero and deny consent.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CastConsent {
    pub allowed: bool,
    pub revision: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastConsentOutcome {
    Read(CastConsent),
    Written(CastConsent),
    Conflict(CastConsent),
    Unchanged(CastConsent),
}

#[derive(Clone, Copy)]
enum Pass {
    Prepare,
    Commit(Option<i64>),
}
enum PassOutcome {
    Complete(CastConsentOutcome),
    Prepared(i64),
    Pending(i64),
}

impl DurabilityRoot {
    /// Fenced preference CAS. Before an actual withdrawal, publish a durable
    /// denial floor and wait outside every transaction/owner lock for all live
    /// media workers to invalidate their old frames (or lose their media lease).
    /// The final preference and audit event still commit atomically.
    pub async fn character_cast_consent(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        change: Option<(i64, bool)>,
    ) -> Result<CastConsentOutcome, CharacterProgressionError> {
        if change.is_some_and(|(r, _)| r < 0 || r == i64::MAX) {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let mut pass = if change.is_some_and(|(_, allow)| !allow) {
            Pass::Prepare
        } else {
            Pass::Commit(None)
        };
        let until = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            match self
                .cast_consent_pass(authority, node, fence, change, pass)
                .await?
            {
                PassOutcome::Complete(outcome) => return Ok(outcome),
                PassOutcome::Prepared(seq) => pass = Pass::Commit(Some(seq)),
                PassOutcome::Pending(seq) => {
                    if std::time::Instant::now() >= until {
                        return Err(DurabilityError::Unavailable.into());
                    }
                    pass = Pass::Commit(Some(seq));
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
            }
        }
    }
    async fn cast_consent_pass(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        change: Option<(i64, bool)>,
        pass: Pass,
    ) -> Result<PassOutcome, CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if let Err(error)=assert_gameplay_fence(&mut tx,&fence,&node).await? {return Ok(Err(error));}
            let stored:Option<(bool,i64)>=sqlx::query_as("SELECT allowed,revision FROM game_character_cast_consent WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let stored=stored.map_or_else(CastConsent::default,|(allowed,revision)|CastConsent{allowed,revision});
            let outcome=match change {
                None=>PassOutcome::Complete(CastConsentOutcome::Read(stored)),
                Some((expected,_)) if expected!=stored.revision=>PassOutcome::Complete(CastConsentOutcome::Conflict(stored)),
                Some((_,allowed)) if allowed==stored.allowed=>PassOutcome::Complete(CastConsentOutcome::Unchanged(stored)),
                Some((expected,false)) if matches!(pass,Pass::Prepare)=>{
                    PassOutcome::Prepared(super::cast_privacy::prepare(&mut tx,fence.character_id,expected+1).await?)
                },
                Some((expected,allowed))=>{
                    let barrier=if allowed {None} else {
                        let Pass::Commit(Some(seq))=pass else {return Err(DurabilityError::InvalidStoredState)};
                        super::cast_privacy::control(&mut tx).await?;
                        Some(seq)
                    };
                    if let Some(seq)=barrier && !super::cast_privacy::all_applied(&mut tx,seq).await? {
                        PassOutcome::Pending(seq)
                    } else {
                        let revision=expected.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
                        sqlx::query("INSERT INTO game_character_cast_consent(character_id,allowed,revision) VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT(character_id) DO UPDATE SET allowed=EXCLUDED.allowed,revision=EXCLUDED.revision")
                            .bind(fence.character_id.as_bytes().as_slice()).bind(allowed).bind(revision).execute(&mut *tx).await?;
                        super::cast_audit::record_consent(&mut tx,&node,fence,revision,allowed).await?;
                        PassOutcome::Complete(CastConsentOutcome::Written(CastConsent{allowed,revision}))
                    }
                }
            };
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(outcome))
        })).await?
    }
}

impl DurabilityRoot {
    /// Worker-only bounded preference census of the committed interest set.
    /// This supplies no visibility authority. Missing rows are explicit deny.
    pub async fn read_cast_consents(
        &self,
        node: &NodeIncarnationProof,
        characters: &[oteryn_gameplay_domain::domain::CharacterId],
    ) -> Result<Vec<(oteryn_gameplay_domain::domain::CharacterId, CastConsent)>, DurabilityError>
    {
        if characters.len() > 128 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let node = node.clone();
        let characters = characters.to_vec();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            if !super::runtime_scope_assignment::prove_current_incarnation(&mut tx,&node).await? {
                return Err(DurabilityError::Unavailable);
            }
            let ids:Vec<Vec<u8>>=characters.iter().map(|id|id.as_bytes().to_vec()).collect();
            let rows:Vec<(Vec<u8>,bool,i64)>=sqlx::query_as(
                "SELECT uuid_send(c.character_id),(c.allowed AND c.revision>=COALESCE(f.minimum_revision,0)),c.revision \
                 FROM game_character_cast_consent c LEFT JOIN game_cast_consent_floors f USING(character_id) \
                 WHERE c.character_id IN (SELECT encode(id,'hex')::uuid FROM unnest($1::bytea[]) AS id)")
                .bind(ids).fetch_all(&mut *tx).await?;
            let result=characters.into_iter().map(|character| {
                let value=rows.iter().find(|(id,_,_)|id.as_slice()==character.as_bytes())
                    .map_or(CastConsent::default(),|(_,allowed,revision)|CastConsent{allowed:*allowed,revision:*revision});
                (character,value)
            }).collect();
            commit_semantic_transaction(tx,deadline).await?;
            Ok(result)
        })).await
    }
}
