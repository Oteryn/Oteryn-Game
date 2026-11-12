//! NPC-MARRIAGE-1: immutable consent and stage receipts compose existing value owners.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root,
};
use super::db::{begin_type2_transaction, lock_admission_relations};
use super::item_loop_move::ItemLoopContent;
use super::item_mint::TypedDefinitionRef;
use super::item_mint_audit::check_uuid_v7;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::CommandRef;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, Row, types::Uuid};

pub const NPC_KEY: &str = "oteryn:npc.lynda";
pub const GRANT_KEY: &str = oteryn_gameplay_content::content::native_gameplay::MARRIAGE_GRANT_KEY;
pub const PROVENANCE: &str = oteryn_gameplay_content::content::native_gameplay::MARRIAGE_PROVENANCE;

#[derive(Clone, Debug)]
pub enum MarriageAction {
    Marry(String),
    Cancel,
    Celebrate,
    Divorce,
}
impl MarriageAction {
    fn code(&self) -> u8 {
        match self {
            Self::Marry(_) => 1,
            Self::Cancel => 3,
            Self::Celebrate => 4,
            Self::Divorce => 5,
        }
    }
}
#[derive(Clone, Debug)]
pub struct MarriageRequest {
    pub command: CommandRef,
    pub occurrence: [u8; 16],
    pub event: [u8; 16],
    pub npc_identity: [u8; 16],
    pub npc_generation: u64,
    pub wire_binding: [u8; 32],
    pub action: MarriageAction,
    /// Current admitted Item definitions: wedding ring, outfit box, engraved ring.
    pub definitions: [TypedDefinitionRef; 3],
}
#[derive(Clone, Debug)]
pub struct MarriageReceipt {
    pub character: CharacterId,
    pub committed_revision: CharacterRevision,
    pub operation: i16,
    pub npc_identity: [u8; 16],
    pub npc_generation: u64,
    pub wire_binding: [u8; 32],
    pub reply: String,
}
#[derive(Clone, Debug)]
pub enum MarriageOutcome {
    Committed(MarriageReceipt),
    Refused(String),
}

pub fn wire_binding(identity: [u8; 16], generation: u64, text: &str) -> [u8; 32] {
    Sha256::new()
        .chain_update(b"oteryn:npc-marriage-wire:v1\0")
        .chain_update(identity)
        .chain_update(generation.to_be_bytes())
        .chain_update((text.len() as u64).to_be_bytes())
        .chain_update(text.as_bytes())
        .finalize()
        .into()
}
fn binding(character: CharacterId, request: &MarriageRequest) -> Vec<u8> {
    let mut hash = Sha256::new();
    hash.update(b"oteryn:npc-marriage:v1\0");
    hash.update(character.as_bytes());
    hash.update(request.command.game_session_id().as_bytes());
    hash.update(request.command.command_id().get().to_be_bytes());
    hash.update(request.npc_identity);
    hash.update(request.npc_generation.to_be_bytes());
    hash.update(request.wire_binding);
    hash.update([request.action.code()]);
    if let MarriageAction::Marry(name) = &request.action {
        hash.update((name.len() as u64).to_be_bytes());
        hash.update(name.as_bytes());
    }
    hash.finalize().to_vec()
}
async fn load(
    connection: &mut PgConnection,
    character: CharacterId,
    command: CommandRef,
) -> Result<Option<(Vec<u8>, MarriageReceipt)>, DurabilityError> {
    let row=sqlx::query("SELECT request_binding,committed_character_revision::text,operation,npc_identity,npc_generation::text,wire_binding,reply FROM game_character_marriage_receipts WHERE character_id=encode($1,'hex')::uuid AND command_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric")
        .bind(character.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(connection).await?;
    row.map(|row| {
        Ok((
            row.try_get("request_binding")?,
            MarriageReceipt {
                character,
                committed_revision: CharacterRevision::new(numeric_u64(
                    &row,
                    "committed_character_revision",
                )?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
                operation: row.try_get("operation")?,
                npc_identity: row
                    .try_get::<Vec<u8>, _>("npc_identity")?
                    .try_into()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                npc_generation: numeric_u64(&row, "npc_generation")?,
                wire_binding: row
                    .try_get::<Vec<u8>, _>("wire_binding")?
                    .try_into()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                reply: row.try_get("reply")?,
            },
        ))
    })
    .transpose()
}
async fn relation(
    connection: &mut PgConnection,
    character: Uuid,
) -> Result<Value, DurabilityError> {
    Ok(sqlx::query_scalar("SELECT coalesce((SELECT to_jsonb(s) FROM game_character_marriages s WHERE character_id=$1),'null'::jsonb)").bind(character).fetch_one(&mut *connection).await?)
}
fn status(state: &Value) -> Result<i64, DurabilityError> {
    if state.is_null() {
        return Ok(0);
    }
    state
        .get("status")
        .and_then(Value::as_i64)
        .ok_or(DurabilityError::InvalidStoredState)
}
fn partner(state: &Value) -> Result<Option<Uuid>, DurabilityError> {
    state
        .get("partner_id")
        .and_then(Value::as_str)
        .map(|id| Uuid::parse_str(id).map_err(|_| DurabilityError::InvalidStoredState))
        .transpose()
}
fn after(
    before: &Value,
    character: Uuid,
    spouse: Option<Uuid>,
    status: i64,
    occurrence: Uuid,
) -> Result<Value, DurabilityError> {
    let version = if before.is_null() {
        1
    } else {
        before
            .get("version")
            .and_then(Value::as_i64)
            .and_then(|n| n.checked_add(1))
            .ok_or(DurabilityError::InvalidStoredState)?
    };
    Ok(
        json!({"character_id":character.to_string(),"partner_id":spouse.map(|id|id.to_string()),"status":status,"version":version,"last_receipt":occurrence.to_string()}),
    )
}
struct Transition {
    operation: i16,
    actor: Value,
    partner: Option<Value>,
    consent: Option<Uuid>,
    reply: &'static str,
}
fn transition(
    a: &Value,
    b: &Value,
    actor: Uuid,
    target: Uuid,
    occurrence: Uuid,
    action: &MarriageAction,
) -> Result<Option<Transition>, DurabilityError> {
    if actor == target {
        return Ok(None);
    }
    let (sa, sb) = (status(a)?, status(b)?);
    let (op, as_, bs, reply) = match action {
        MarriageAction::Marry(_) if sa == 0 && sb == 0 => {
            (1, 1, None, "Your marriage proposal has been recorded.")
        }
        MarriageAction::Marry(_) if sa == 0 && sb == 1 && partner(b)? == Some(actor) => (
            2,
            3,
            Some(3),
            "You are now engaged. Prepare for the celebration.",
        ),
        MarriageAction::Cancel if sa == 1 && partner(a)? == Some(target) => (
            3,
            0,
            None,
            "Your proposal has been removed. Your wedding ring and outfit box have been returned.",
        ),
        MarriageAction::Celebrate
            if sa == 3 && sb == 3 && partner(a)? == Some(target) && partner(b)? == Some(actor) =>
        {
            (
                4,
                2,
                Some(2),
                "You are now married. Take your engraved wedding ring.",
            )
        }
        MarriageAction::Divorce
            if sa == 2 && sb == 2 && partner(a)? == Some(target) && partner(b)? == Some(actor) =>
        {
            (5, 0, Some(0), "You are now divorced.")
        }
        _ => return Ok(None),
    };
    let consent = if op == 2 {
        Some(
            Uuid::parse_str(
                b.get("last_receipt")
                    .and_then(Value::as_str)
                    .ok_or(DurabilityError::InvalidStoredState)?,
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        )
    } else {
        None
    };
    Ok(Some(Transition {
        operation: op,
        actor: after(a, actor, (as_ != 0).then_some(target), as_, occurrence)?,
        partner: bs
            .map(|s| after(b, target, (s != 0).then_some(actor), s, occurrence))
            .transpose()?,
        consent,
        reply,
    }))
}
async fn write_relation(
    connection: &mut PgConnection,
    state: &Value,
) -> Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_character_marriages(character_id,partner_id,status,version,last_receipt) SELECT character_id,partner_id,status,version,last_receipt FROM jsonb_populate_record(NULL::game_character_marriages,$1) ON CONFLICT(character_id) DO UPDATE SET partner_id=EXCLUDED.partner_id,status=EXCLUDED.status,version=EXCLUDED.version,last_receipt=EXCLUDED.last_receipt")
        .bind(state).execute(connection).await?;
    Ok(())
}
impl DurabilityRoot {
    /// Historical completion only. The caller still owns accepted-command and actor publication authority.
    pub async fn reconcile_npc_marriage(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        command: CommandRef,
    ) -> Result<Option<MarriageReceipt>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let result = load(&mut tx, character, command)
                        .await?
                        .map(|(_, receipt)| receipt);
                    tx.commit(deadline).await?;
                    Ok(result)
                })
            })
            .await
    }
    pub(super) async fn commit_npc_marriage<C: ItemLoopContent>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: MarriageRequest,
        content: &C,
    ) -> Result<MarriageOutcome, CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut content = content;
        self.try_issue_semantic_pass()?.run_with_context(&mut content,move |holder,deadline,content|Box::pin(async move {
            let mut tx=begin_type2_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?; lock_admission_relations(&mut tx).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:npc-marriage:'||encode($1,'hex')||':'||$2,0))")
                .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).execute(&mut **tx).await?;
            let binding=binding(fence.character_id,&request);
            if let Some((stored,receipt))=load(&mut tx,fence.character_id,request.command).await? {
                if stored!=binding {return Ok(Err(CharacterProgressionError::ConflictingOccurrence));}
                tx.commit(deadline).await?; return Ok(Ok(MarriageOutcome::Committed(receipt)));
            }
            let other_command:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_npc_stage_receipts WHERE character_id=encode($1,'hex')::uuid AND command_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric) OR EXISTS(SELECT 1 FROM game_character_npc_purchase_receipts WHERE character_id=encode($1,'hex')::uuid AND command_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_one(&mut **tx).await?;
            if other_command {return Ok(Err(CharacterProgressionError::ConflictingOccurrence));}
            if request.command.game_session_id()!=fence.game_session_id || request.npc_generation==0
             || check_uuid_v7(&request.occurrence).is_err() || check_uuid_v7(&request.event).is_err() {
                return Ok(Err(CharacterProgressionError::InvalidInput));
            }
            let actor=Uuid::from_bytes(*fence.character_id.as_bytes());
            let first=relation(&mut tx,actor).await?;
            let target=match &request.action {
                MarriageAction::Marry(name)=>sqlx::query_scalar::<_,Uuid>("SELECT r.character_id FROM game_character_name_reservations n JOIN game_character_roots r USING(character_id) WHERE game_character_is_name($1) AND n.name_key=game_character_name_key($1) AND r.name_key=n.name_key AND r.world_id=encode($2,'hex')::uuid AND r.lifecycle=1")
                    .bind(name).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut **tx).await?,
                _=>partner(&first)?,
            };
            let Some(target)=target.filter(|id|*id!=actor) else {return Ok(Ok(MarriageOutcome::Refused("No eligible partner or pending marriage stage was found.".into())));};
            // Consent is pair-owned. Lock both roots in identity order BEFORE the actor fence.
            let participants=vec![actor,target];
            let roots:Vec<Uuid>=sqlx::query_scalar("SELECT character_id FROM game_character_roots WHERE character_id=ANY($1) ORDER BY character_id FOR UPDATE")
                .bind(&participants).fetch_all(&mut **tx).await?;
            if roots.len()!=2 {return Ok(Err(CharacterProgressionError::AuthorityRejected));}
            let root=match assert_gameplay_fence(&mut tx,&fence,&node).await? {Ok(root)=>root,Err(error)=>return Ok(Err(error))};
            if root.revision==u64::MAX {return Ok(Err(CharacterProgressionError::InvalidInput));}
            let world_matches:bool=sqlx::query_scalar("SELECT world_id=encode($2,'hex')::uuid AND lifecycle=1 FROM game_character_roots WHERE character_id=$1")
                .bind(target).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            if !world_matches {return Ok(Ok(MarriageOutcome::Refused("Your partner must belong to this world.".into())));}
            let state=sqlx::query("SELECT character_revision::text,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_progression_state WHERE character_id=$1 FOR UPDATE")
                .bind(actor).fetch_optional(&mut **tx).await?;
            let Some(state)=state else {return Ok(Err(CharacterProgressionError::MissingProgressionState));};
            if numeric_u64(&state,"character_revision")?!=root.revision || !state_matches_root(&state,&root) {return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));}
            if sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=$1)").bind(actor).fetch_one(&mut **tx).await? {return Ok(Err(CharacterProgressionError::RespawnPending));}
            let (a,b)=(relation(&mut tx,actor).await?,relation(&mut tx,target).await?);
            let occurrence=Uuid::from_bytes(request.occurrence);
            let Some(effect)=transition(&a,&b,actor,target,occurrence,&request.action)? else {return Ok(Ok(MarriageOutcome::Refused("This marriage stage is not available. Existing proposals and couples cannot be overwritten.".into())));};
            let occurred:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint").fetch_one(&mut **tx).await?;
            let items=match super::item_craft::marriage::prepare(&mut tx,fence.item_fence(),&request,effect.operation,occurred,*content).await? {
                Ok(items)=>items,Err(line)=>return Ok(Ok(MarriageOutcome::Refused(line.into()))),
            };
            sqlx::query("INSERT INTO game_character_marriage_receipts(purchase_occurrence_id,character_id,account_id,command_session_id,command_id,request_binding,npc_key,npc_identity,npc_generation,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at,partner_id,operation,wire_binding,actor_before,actor_after,partner_before,partner_after,consent_receipt,reply,world_id) SELECT $1,s.character_id,r.account_id,encode($2,'hex')::uuid,$3::text::numeric,$4,'oteryn:npc.lynda',$5,$6::text::numeric,s.character_revision,s.character_revision+1,s.level,s.level,s.total_experience,s.total_experience,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,r.world_id FROM game_character_progression_state s JOIN game_character_roots r USING(character_id) WHERE s.character_id=$17")
                .bind(occurrence).bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(binding)
                .bind(request.npc_identity.as_slice()).bind(request.npc_generation.to_string()).bind(occurred).bind(target).bind(effect.operation).bind(request.wire_binding.as_slice())
                .bind(&a).bind(&effect.actor).bind(&b).bind(&effect.partner).bind(effect.consent).bind(effect.reply).bind(actor).execute(&mut **tx).await?;
            if let Some(items)=items {super::item_craft::marriage::apply(&mut tx,fence.item_fence(),&request,items,occurred).await?;}
            if effect.operation==2 {super::character_appearance::grant_marriage_outfits(&mut tx,&participants,request.occurrence).await?;}
            write_relation(&mut tx,&effect.actor).await?;
            if let Some(partner)=&effect.partner {write_relation(&mut tx,partner).await?;}
            sqlx::query("UPDATE game_character_roots SET character_revision=character_revision+1 WHERE character_id=$1").bind(actor).execute(&mut **tx).await?;
            sqlx::query("UPDATE game_character_progression_state SET character_revision=character_revision+1 WHERE character_id=$1").bind(actor).execute(&mut **tx).await?;
            let receipt=load(&mut tx,fence.character_id,request.command).await?.ok_or(DurabilityError::InvalidStoredState)?.1;
            tx.commit(deadline).await?; Ok(Ok(MarriageOutcome::Committed(receipt)))
        })).await?
    }
}

pub(super) async fn verify_integrity(connection: &mut PgConnection) -> Result<(), DurabilityError> {
    let invalid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_marriages s LEFT JOIN game_character_marriage_receipts r ON r.purchase_occurrence_id=s.last_receipt LEFT JOIN game_character_marriages p ON p.character_id=s.partner_id WHERE r.purchase_occurrence_id IS NULL OR NOT ((r.character_id=s.character_id AND r.actor_after=to_jsonb(s)) OR (r.partner_id=s.character_id AND r.partner_after=to_jsonb(s))) OR (s.status IN(2,3) AND (p.status IS DISTINCT FROM s.status OR p.partner_id IS DISTINCT FROM s.character_id)))")
        .fetch_one(&mut *connection).await?;
    if invalid {
        return Err(DurabilityError::InvalidStoredState);
    }
    let missing:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_marriage_receipts r WHERE (r.operation<>5)<>EXISTS(SELECT 1 FROM game_item_marriage_records i WHERE i.transaction_id=r.purchase_occurrence_id)) OR EXISTS(SELECT 1 FROM game_character_marriage_receipts r JOIN game_character_roots c ON c.character_id IN(r.character_id,r.partner_id) WHERE r.operation=2 AND NOT EXISTS(SELECT 1 FROM game_account_appearance_unlocks u JOIN game_account_appearance_unlock_receipts g USING(grant_occurrence_id) WHERE u.account_id=c.account_id AND u.unlock_key='oteryn:appearance-grant/npc/lynda/newly-wed-base' AND g.source_kind='npc' AND g.content_revision='crystal-201189f-lynda-marriage-v1'))")
        .fetch_one(&mut *connection).await?;
    if missing {
        Err(DurabilityError::InvalidStoredState)
    } else {
        Ok(())
    }
}

impl DurabilityRoot {
    /// Public Item inscription data, after the caller has independently observed visibility.
    pub async fn read_marriage_inscription(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        world: [u8; 16],
        item: [u8; 16],
    ) -> Result<Option<String>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_type2_transaction(holder,deadline).await?; assert_recovery_fence(&mut tx,&recovery).await?;
            let row:Option<(Option<String>,Option<bool>)>=sqlx::query_as("SELECT t.text,sha256(convert_to(t.text,'UTF8'))=r.inscription_sha256 FROM game_item_marriage_records r JOIN game_item_instances i ON i.item_instance_id=r.anchor_item_id LEFT JOIN game_item_texts t ON t.item_instance_id=i.item_instance_id WHERE r.anchor_item_id=encode($1,'hex')::uuid AND r.world_id=encode($2,'hex')::uuid AND i.world_id=r.world_id AND i.lifecycle=1 AND r.inscription_sha256 IS NOT NULL")
                .bind(item.as_slice()).bind(world.as_slice()).fetch_optional(&mut **tx).await?;
            let text=match row {None=>None,Some((Some(text),Some(true)))=>Some(text),_=>return Err(DurabilityError::InvalidStoredState)};
            tx.commit(deadline).await?; Ok(text)
        })).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn marriage_consent_is_reciprocal_exclusive_and_cancellation_does_not_claim_the_target()
    -> Result<(), DurabilityError> {
        let a = Uuid::from_bytes([1; 16]);
        let b = Uuid::from_bytes([2; 16]);
        let c = Uuid::from_bytes([3; 16]);
        let event = Uuid::from_bytes([4; 16]);
        let empty = Value::Null;
        let proposal = transition(
            &empty,
            &empty,
            a,
            b,
            event,
            &MarriageAction::Marry("B".into()),
        )?
        .ok_or(DurabilityError::InvalidStoredState)?;
        assert_eq!(proposal.operation, 1);
        assert!(proposal.partner.is_none());
        let incoming = transition(
            &empty,
            &empty,
            c,
            b,
            event,
            &MarriageAction::Marry("B".into()),
        )?
        .ok_or(DurabilityError::InvalidStoredState)?;
        let accepted = transition(
            &empty,
            &proposal.actor,
            b,
            a,
            event,
            &MarriageAction::Marry("A".into()),
        )?
        .ok_or(DurabilityError::InvalidStoredState)?;
        assert_eq!(accepted.operation, 2);
        assert_eq!(status(&accepted.actor)?, 3);
        assert!(accepted.consent.is_some());
        assert!(
            transition(
                &accepted.actor,
                &empty,
                b,
                c,
                event,
                &MarriageAction::Marry("C".into())
            )?
            .is_none(),
            "engaged actor cannot overwrite its pair"
        );
        assert!(
            transition(
                &empty,
                &accepted.actor,
                c,
                b,
                event,
                &MarriageAction::Marry("B".into())
            )?
            .is_none(),
            "engaged target is exclusive"
        );
        let cancelled = transition(
            &incoming.actor,
            &accepted.actor,
            c,
            b,
            event,
            &MarriageAction::Cancel,
        )?
        .ok_or(DurabilityError::InvalidStoredState)?;
        assert_eq!(cancelled.operation, 3);
        assert!(cancelled.partner.is_none());
        assert_eq!(status(&cancelled.actor)?, 0);
        assert!(
            transition(
                &empty,
                &empty,
                a,
                a,
                event,
                &MarriageAction::Marry("A".into())
            )?
            .is_none()
        );
        Ok(())
    }
    #[test]
    fn ceremony_is_once_per_pair_and_divorce_preserves_item_and_appearance_ownership()
    -> Result<(), DurabilityError> {
        let a = Uuid::from_bytes([1; 16]);
        let b = Uuid::from_bytes([2; 16]);
        let event = Uuid::from_bytes([4; 16]);
        let engaged_a = after(&Value::Null, a, Some(b), 3, event)?;
        let engaged_b = after(&Value::Null, b, Some(a), 3, event)?;
        assert!(
            transition(&engaged_a, &engaged_b, a, b, event, &MarriageAction::Cancel)?.is_none()
        );
        let married = transition(
            &engaged_a,
            &engaged_b,
            a,
            b,
            event,
            &MarriageAction::Celebrate,
        )?
        .ok_or(DurabilityError::InvalidStoredState)?;
        assert_eq!(married.operation, 4);
        let spouse = married
            .partner
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?;
        assert!(
            transition(
                &married.actor,
                spouse,
                a,
                b,
                event,
                &MarriageAction::Celebrate
            )?
            .is_none()
        );
        let divorced = transition(
            &married.actor,
            spouse,
            a,
            b,
            event,
            &MarriageAction::Divorce,
        )?
        .ok_or(DurabilityError::InvalidStoredState)?;
        assert_eq!(divorced.operation, 5);
        assert_eq!(status(&divorced.actor)?, 0);
        assert_eq!(
            status(
                divorced
                    .partner
                    .as_ref()
                    .ok_or(DurabilityError::InvalidStoredState)?
            )?,
            0
        );
        // Operation5 intentionally has no Item participant and never revokes an earned grant.
        assert!(partner(&divorced.actor)?.is_none());
        Ok(())
    }
    #[test]
    fn marriage_wire_binding_fences_actor_generation_and_exact_original_text() {
        let identity = [1; 16];
        let original = wire_binding(identity, 1, "Spouse Name");
        assert_eq!(original, wire_binding(identity, 1, "Spouse Name"));
        assert_ne!(original, wire_binding([2; 16], 1, "Spouse Name"));
        assert_ne!(original, wire_binding(identity, 2, "Spouse Name"));
        assert_ne!(original, wire_binding(identity, 1, "Different Name"));
    }
}
