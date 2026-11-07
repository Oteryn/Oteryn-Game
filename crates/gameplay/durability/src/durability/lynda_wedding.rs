//! Two real Characters' reciprocal Lynda consent; no fabricated partner progression.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_equipment::{EquipmentError, assert_current_equipment_in_transaction};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use super::db::{begin_type2_transaction, lock_admission_relations};
use super::item_mint::TypedDefinitionRef;
use super::item_mint_audit::Type2Transaction;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_owner_commit::SpellLanePermit;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::appearance::{AppearanceContent, UnlockFact};
use crate::foundation::CommandRef;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_quests::quest::lynda_wedding as rules;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::Row;
mod refund;
pub const PROVENANCE: &str = "crystal-201189f-npc-cosmetics-v1";
pub const GRANT: &str = "oteryn:appearance-grant/npc/lynda/newly_wed/base";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    Propose(String),
    Cancel,
}
#[derive(Clone, Debug)]
pub struct Request {
    pub command: CommandRef,
    pub action: Action,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Outcome {
    pub reply: String,
    pub inventory_changed: bool,
    pub accepted: bool,
    #[serde(default)]
    pub fresh: bool,
}
fn refused(text: &str) -> Outcome {
    Outcome {
        reply: text.into(),
        inventory_changed: false,
        accepted: false,
        fresh: false,
    }
}
fn binding(request: &Request) -> Result<[u8; 32], DurabilityError> {
    Ok(Sha256::new()
        .chain_update(b"oteryn:lynda-wedding:v1\0")
        .chain_update(request.command.game_session_id().as_bytes())
        .chain_update(request.command.command_id().get().to_be_bytes())
        .chain_update(
            serde_json::to_vec(&request.action).map_err(|_| DurabilityError::InvalidStoredState)?,
        )
        .finalize()
        .into())
}
fn status(value: i16) -> Result<rules::Status, DurabilityError> {
    match value {
        0 => Ok(rules::Status::Free),
        1 => Ok(rules::Status::Proposed),
        2 => Ok(rules::Status::Married),
        3 => Ok(rules::Status::Accepted),
        _ => Err(DurabilityError::InvalidStoredState),
    }
}
async fn participant(
    tx: &mut Type2Transaction<'_>,
    id: sqlx::types::Uuid,
) -> Result<rules::Participant, DurabilityError> {
    let state:Option<(i16,Option<sqlx::types::Uuid>)>=sqlx::query_as("SELECT status,partner_character_id FROM game_lynda_wedding_states WHERE character_id=$1 FOR UPDATE").bind(id).fetch_optional(&mut ***tx).await?;
    let (state, partner) = state.unwrap_or((0, None));
    Ok(rules::Participant {
        character: *id.as_bytes(),
        status: status(state)?,
        partner: partner.map(|p| *p.as_bytes()),
    })
}
impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "independent current Character, Node, source command, native artifact and item-lane proofs"
    )]
    pub async fn commit_lynda_wedding(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: Request,
        permit: &SpellLanePermit,
        native: &NativeGameplayState,
    ) -> Result<Outcome, CharacterProgressionError> {
        if request.command.game_session_id() != fence.game_session_id {
            return Err(CharacterProgressionError::InvalidInput);
        }
        if let Action::Propose(name) = &request.action
            && (name.is_empty() || name.len() > 64 || name.chars().any(char::is_control))
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        permit.check_scope(fence.runtime_scope)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut owners = (permit, native);
        self.try_issue_semantic_pass()?.run_with_context(&mut owners,move|holder,deadline,(permit,native)|Box::pin(async move{
            let (permit, native) = (*permit, *native);
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let bound = binding(&request)?;
            let old:Option<(Vec<u8>,serde_json::Value)>=sqlx::query_as("SELECT request_binding,outcome FROM game_lynda_wedding_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                    .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
            if let Some((before, outcome)) = old {
                if before != bound {
                    return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                }
                let mut outcome: Outcome =
                    serde_json::from_value(outcome).map_err(|_| DurabilityError::InvalidStoredState)?;
                outcome.fresh = false;
                tx.commit(deadline).await?;
                return Ok(Ok(outcome));
            }
            let own = sqlx::types::Uuid::from_bytes(*fence.character_id.as_bytes());
            // Resolve the real name first, then acquire BOTH roots in global order before
            // any single-character equipment/root/account helper can take its own lock.
            let target:Option<sqlx::types::Uuid>=match &request.action{
                    Action::Propose(name)=>sqlx::query_scalar("SELECT character_id FROM game_character_roots WHERE world_id=encode($1,'hex')::uuid AND lifecycle=1 AND lower(name)=lower($2)").bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(name).fetch_optional(&mut **tx).await?,
                    Action::Cancel=>sqlx::query_scalar("SELECT partner_character_id FROM game_lynda_wedding_states WHERE character_id=$1 AND status=1").bind(own).fetch_optional(&mut **tx).await?,
                       };
            let Some(target) = target else {
                tx.commit(deadline).await?;
                return Ok(Ok(refused(if matches!(request.action, Action::Cancel) {
                    "You don't have any pending proposal to be removed."
                } else {
                    "A player with this name does not exist."
                })));
            };
            if target == own {
                tx.commit(deadline).await?;
                return Ok(Ok(refused(
                    "You REALLY want to marry yourself? c'mon, be serious.",
                )));
            }
            let pair=sqlx::query("SELECT character_id,account_id,name FROM game_character_roots WHERE character_id=ANY($1) AND world_id=encode($2,'hex')::uuid AND lifecycle=1 ORDER BY character_id FOR UPDATE")
                    .bind(vec![own,target]).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_all(&mut **tx).await?;
            if pair.len() != 2 {
                return Ok(Err(CharacterProgressionError::InvalidInput));
            }
            match assert_current_equipment_in_transaction(
                &mut tx,
                &recovery,
                &node,
                &fence.item_fence(),
                Some(request.command),
                native.source_digest(),
            )
            .await
            {
                Ok(_) => {}
                Err(EquipmentError::Rejected(_)) => {
                    return Ok(Err(CharacterProgressionError::AuthorityRejected));
                }
                Err(EquipmentError::Database(e)) => return Err(e.into()),
                Err(EquipmentError::Durability(e)) => return Err(e),
            }
            let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                Ok(r) => r,
                Err(e) => return Ok(Err(e)),
            };
            let target_name = pair
                .iter()
                .find(|row| row.try_get::<sqlx::types::Uuid, _>("character_id").ok() == Some(target))
                .ok_or(DurabilityError::InvalidStoredState)?
                .try_get::<String, _>("name")?;
            if let Action::Propose(name) = &request.action
                && !target_name.eq_ignore_ascii_case(name)
            {
                return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
            }
            let progress=sqlx::query("SELECT character_revision::text,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_progression_state WHERE character_id=$1 FOR UPDATE").bind(own).fetch_one(&mut **tx).await?;
            if super::character_progression::numeric_u64(&progress, "character_revision")? != root.revision
                || !super::character_progression::state_matches_root(&progress, &root)
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            let mut accounts = pair
                .iter()
                .map(|r| r.try_get::<sqlx::types::Uuid, _>("account_id"))
                .collect::<Result<Vec<_>, _>>()?;
            accounts.sort_unstable();
            accounts.dedup();
            for account in &accounts {
                super::character_appearance::lock_account_grants(&mut tx, *account).await?;
            }
            let actor = participant(&mut tx, own).await?;
            let partner = participant(&mut tx, target).await?;
            let mut costs = Vec::new();
            for (id, count) in rules::MATERIALS {
                let key = format!("oteryn:item.tibia.i{id}");
                let index = native
                    .item_index()
                    .ok_or(DurabilityError::InvalidStoredState)?;
                let definition = TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key.clone(),
                    revision_ref: index
                        .revision(&key)
                        .ok_or(DurabilityError::InvalidStoredState)?
                        .into(),
                };
                oteryn_gameplay_items::item_ref_admission::generation_item_facts(native, &definition)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                costs.push(super::carried_cost::NpcItemCost { definition, count });
            }
            let has_ring =
                super::carried_cost::available(&mut tx, fence.item_fence(), &costs[0].definition).await?
                    >= 1;
            let has_box = super::carried_cost::available(&mut tx, fence.item_fence(), &costs[1].definition)
                .await?
                >= 1;
            let transition = match &request.action {
                Action::Cancel => rules::cancel(actor),
                Action::Propose(_) => rules::propose(actor, Some(partner), has_ring, has_box),
            };
            let transition = match transition {
                Ok(t) => t,
                Err(e) => {
                    tx.commit(deadline).await?;
                    return Ok(Ok(refused(match e {
                        rules::Refusal::MissingMaterials => {
                            "As I said, you need a wedding ring and the wedding outfit box in order to marry."
                        }
                        rules::Refusal::AlreadyProposed => {
                            "You already made a wedding proposal. You can always remove the proposal by saying remove proposal."
                        }
                        rules::Refusal::PartnerCommitted => "Your candidate is already married or engaged.",
                        rules::Refusal::PartnerProposedElsewhere => {
                            "Your candidate already made a wedding proposal to someone else."
                        }
                        rules::Refusal::NothingToCancel => {
                            "You don't have any pending proposal to be removed."
                        }
                        _ => "You are already married.",
                    })));
                }
            };
            let operation = match transition {
                rules::Transition::Propose { .. } => "propose",
                rules::Transition::Accept { .. } => "accept",
                rules::Transition::Cancel { .. } => "cancel",
            };
            let content = native
                .cosmetics()
                .ok_or(DurabilityError::InvalidStoredState)?;
            if operation == "accept"
                && (!rules::OUTFIT_LOOKS.into_iter().all(|look| {
                    content.earned_source_look_addon(GRANT, PROVENANCE, u32::from(look), 0)
                        == UnlockFact::Compatible
                }))
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            let parent: sqlx::types::Uuid = sqlx::query_scalar("SELECT game_character_uuid_v7()")
                .fetch_one(&mut **tx)
                .await?;
            let next = root
                .revision
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)?;
            let fact = node.fact();
            let crate::foundation::RuntimeScopeRefV1::Channel {
                world_id,
                channel_id,
            } = fence.runtime_scope
            else {
                return Ok(Err(CharacterProgressionError::InvalidInput));
            };
            let context = json!({"parent":parent.to_string(),"actor":own.to_string(),"partner":target.to_string(),"game_session":sqlx::types::Uuid::from_bytes(*fence.game_session_id.as_bytes()).to_string(),"command":request.command.command_id().get().to_string(),"world":sqlx::types::Uuid::from_bytes(*world_id.as_bytes()).to_string(),"channel":sqlx::types::Uuid::from_bytes(*channel_id.as_bytes()).to_string(),"connection":fence.connection_generation.get().to_string(),"lease":fence.character_lease_generation.to_string(),"scope_generation":fence.scope_ownership_generation.get().to_string(),"original_revision":root.revision.to_string(),"operation":operation,"binding":bound.iter().map(|b|format!("{b:02x}")).collect::<String>()});
            sqlx::query("SELECT game_lynda_wedding_arm(encode($1,'hex')::uuid,$2::text::numeric,$3,$4)")
                .bind(fact.node_id().as_bytes().as_slice())
                .bind(fact.registration_revision().to_string())
                .bind(node.sql_possession_secret())
                .bind(context)
                .execute(&mut **tx)
                .await?;
            let cost = if operation == "cancel" {
                None
            } else {
                super::carried_cost::burn(
                    &mut tx,
                    permit,
                    fence.item_fence(),
                    request.command,
                    super::carried_cost::ClosedCostCause::LyndaWedding,
                    "oteryn:lynda.wedding.materials",
                    &costs,
                )
                .await?
                .ok_or(DurabilityError::InvalidStoredState)
                .map(Some)?
            };
            let outcome=Outcome{reply:match operation{"accept"=>"Since both of you are willing to marry, I accept to celebrate your marriage, go prepare yourself, and tell me when you are ready for the {celebration}".into(),"cancel"=>format!("Ok, your marriage proposal to {{{target_name}}} has been removed. Take your wedding ring back."),_=>format!("Ok, now let's wait and see if {{{target_name}}} accepts your proposal. I'll give you back your wedding ring as soon as {{{target_name}}} accepts your proposal or you {{remove}} it.")},inventory_changed:true,accepted:operation=="accept",fresh:true};
            sqlx::query("INSERT INTO game_lynda_wedding_receipts(transaction_id,game_session_id,command_id,character_id,partner_character_id,world_id,operation,request_binding,cost_transaction_id,original_revision,committed_revision,outcome,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) VALUES($1,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,encode($6,'hex')::uuid,$7,$8,$9::uuid,$10::text::numeric,$11::text::numeric,$12,$13,$13,$14,$14,$15,$16,$17,$18,$19,$20,$21,$22)")
                    .bind(parent).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(own).bind(target).bind(world_id.as_bytes().as_slice()).bind(operation).bind(bound.as_slice()).bind(cost.as_deref()).bind(root.revision.to_string()).bind(next.to_string()).bind(serde_json::to_value(&outcome).map_err(|_|DurabilityError::InvalidStoredState)?)
                    .bind(progress.try_get::<i64,_>("level")?).bind(progress.try_get::<i64,_>("total_experience")?)
                    .bind(progress.try_get::<String,_>("profile_revision")?).bind(progress.try_get::<String,_>("ruleset_revision")?)
                    .bind(progress.try_get::<String,_>("content_revision")?).bind(progress.try_get::<String,_>("simulation_revision")?)
                    .bind(progress.try_get::<String,_>("evidence_revision")?).bind(progress.try_get::<String,_>("declaration_revision")?)
                    .bind(progress.try_get::<String,_>("policy_revision")?).bind(progress.try_get::<String,_>("reward_revision")?)
                    .execute(&mut **tx).await?;
            let status = if operation == "propose" {
                1_i16
            } else if operation == "accept" {
                3
            } else {
                0
            };
            sqlx::query("INSERT INTO game_lynda_wedding_states(character_id,status,partner_character_id,last_transaction_id) VALUES($1,$2,$3,$4) ON CONFLICT(character_id)DO UPDATE SET status=EXCLUDED.status,partner_character_id=EXCLUDED.partner_character_id,last_transaction_id=EXCLUDED.last_transaction_id")
                    .bind(own).bind(status).bind((operation!="cancel").then_some(target)).bind(parent).execute(&mut **tx).await?;
            if operation == "accept" {
                sqlx::query("UPDATE game_lynda_wedding_states SET status=3,last_transaction_id=$1 WHERE character_id=$2 AND status=1 AND partner_character_id=$3").bind(parent).bind(target).bind(own).execute(&mut **tx).await?;
                for row in &pair {
                    let character: sqlx::types::Uuid = row.try_get("character_id")?;
                    let account: sqlx::types::Uuid = row.try_get("account_id")?;
                    let held:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_account_appearance_unlocks WHERE account_id=$1 AND unlock_key=$2)").bind(account).bind(GRANT).fetch_one(&mut **tx).await?;
                    if held {
                        let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_account_appearance_unlocks u JOIN game_account_appearance_unlock_receipts e USING(grant_occurrence_id) JOIN game_lynda_wedding_receipts w ON e.source_event_id=uuid_send(w.transaction_id)||uuid_send(e.granting_character_id) WHERE u.account_id=$1 AND u.unlock_key=$2 AND e.unlock_key=u.unlock_key AND e.account_id=u.account_id AND e.source_kind='npc_wedding' AND e.content_revision=$3 AND w.operation='accept' AND ((e.granting_character_id=w.character_id AND e.account_id=w.actor_account)OR(e.granting_character_id=w.partner_character_id AND e.account_id=w.partner_account)))")
                                    .bind(account).bind(GRANT).bind(PROVENANCE).fetch_one(&mut **tx).await?;
                        if !valid {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                    } else {
                        let occurrence: sqlx::types::Uuid =
                            sqlx::query_scalar("SELECT game_character_uuid_v7()")
                                .fetch_one(&mut **tx)
                                .await?;
                        let event = [
                            parent.as_bytes().as_slice(),
                            character.as_bytes().as_slice(),
                        ]
                        .concat();
                        sqlx::query("INSERT INTO game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) VALUES($1,$2,$3,$4,$5,'npc_wedding',$6)").bind(occurrence).bind(account).bind(GRANT).bind(PROVENANCE).bind(character).bind(event).execute(&mut **tx).await?;
                        sqlx::query("INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id)VALUES($1,$2,$3)").bind(account).bind(GRANT).bind(occurrence).execute(&mut **tx).await?;
                    }
                }
            }
            for query in [
                "UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=$1 AND character_revision=$3::text::numeric",
                "UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=$1 AND character_revision=$3::text::numeric",
            ] {
                if sqlx::query(query)
                    .bind(own)
                    .bind(next.to_string())
                    .bind(root.revision.to_string())
                    .execute(&mut **tx)
                    .await?
                    .rows_affected()
                    != 1
                {
                    return Err(DurabilityError::InvalidStoredState);
                }
            }
            if operation == "cancel" {
                refund::mint(
                    &mut tx,
                    fence.item_fence(),
                    request.command,
                    parent,
                    bound,
                    native,
                )
                .await?;
            }
            tx.commit(deadline).await?;
            Ok(Ok(outcome))
        })).await?
    }
}
impl DurabilityRoot {
    pub async fn read_lynda_pending_name(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: crate::domain::CharacterId,
    ) -> Result<Option<String>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let name=sqlx::query_scalar("SELECT p.name FROM game_lynda_wedding_states s JOIN game_character_roots p ON p.character_id=s.partner_character_id WHERE s.character_id=encode($1,'hex')::uuid AND s.status=1").bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            tx.commit(deadline).await?;
            Ok(name)
        })).await
    }
}
