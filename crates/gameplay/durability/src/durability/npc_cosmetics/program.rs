//! The source programme uses the existing fenced NPC transaction and receipts.
use super::super::item_mint_audit::Type2Transaction;
use super::*;
use oteryn_gameplay_quests::quest::npc_cosmetics::source_program as source;
use serde_json::{Value, json};
use source::Facts as _;
mod ajax_actor_effect;
pub use ajax_actor_effect::NpcSourceActorEffect;
mod context;
mod duration;
mod money_selector;
mod payment;
mod plan;
mod primal_hazard;
mod readonly_greet;
#[cfg(test)]
mod quest_child_baseline;
mod receipt;
mod snapshot;
#[cfg(any(test, feature = "test-support"))]
mod readonly_owner_probe;
pub(crate) use context::NpcProgramContext;

#[derive(Clone, Debug, Default)]
pub struct NpcSourceEffects {
    pub actor_effect: Option<NpcSourceActorEffect>,
    pub actor_observation: Option<NpcSourceActorObservation>,
    pub payment: Option<NpcSourcePaymentFacts>,
    pub greet: Option<NpcSourceGreetContext>,
    pub quest_catalogue: Option<std::sync::Arc<oteryn_gameplay_quests::quest::QuestStateCatalogue>>,
}
use snapshot::Snapshot;

/// A routing hint; genuine occurrence and source branch are rechecked in Root.
pub fn has_readonly_source_greet(npc: &str) -> bool { readonly_greet::has_registered(npc) }


pub(crate) fn source_fee_price(service: &str) -> Option<u64> {
    let document = source::embedded().ok()?;
    for npc in &document.npcs {
        for transition in &npc.transitions {
            let key = format!(
                "oteryn:service.npc-cosmetic.{}.{}",
                npc.npc.strip_prefix("oteryn:npc.")?,
                transition.id
            );
            if key == service {
                return transition
                    .effects
                    .iter()
                    .try_fold(0_u64, |total, effect| match effect {
                        source::Effect::ConsumeMoney { gold }
                        | source::Effect::ConsumeMoneyBank { gold } => total.checked_add(*gold),
                        _ => Some(total),
                    })
                    .filter(|total| *total > 0);
            }
        }
    }
    None
}
impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "the source composes independent Character, Node, item-lane, native-content and Premium owners"
    )]
    pub async fn commit_npc_source(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: NpcCosmeticRequest,
        permit: &SpellLanePermit,
        native: &NativeGameplayState,
        premium: Option<&dyn super::super::consume_transform_cosmetics::ItemCosmeticPremium>,
    ) -> Result<Option<NpcCosmeticOutcome>, CharacterProgressionError> {
        self.commit_npc_source_with_effects(
            authority,
            node,
            fence,
            request,
            permit,
            native,
            premium,
            &NpcSourceEffects::default(),
        )
        .await
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "the source composes independent Character, Node, item-lane, native-content and Premium owners"
    )]
    pub async fn commit_npc_source_with_effects(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: NpcCosmeticRequest,
        permit: &SpellLanePermit,
        native: &NativeGameplayState,
        premium: Option<&dyn super::super::consume_transform_cosmetics::ItemCosmeticPremium>,
        effects: &NpcSourceEffects,
    ) -> Result<Option<NpcCosmeticOutcome>, CharacterProgressionError> {
        let document = source::embedded().map_err(|_| DurabilityError::InvalidStoredState)?;
        let quest_bindings = super::super::quest_state::preload_npc_source_bindings()?;
        if request.command.game_session_id() != fence.game_session_id
            || request.message.is_empty()
            || request.message.len() > 256
            || !source::supported(&request.npc)
            || matches!(
                request.npc.as_str(),
                "oteryn:npc.appaloosa"
                    | "oteryn:npc.palomino"
                    | "oteryn:npc.walter_jaeger"
                    | "oteryn:npc.flickering_soul"
            )
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let content = native
            .cosmetics()
            .filter(|content| content.source_digest() == native.source_digest())
            .ok_or(DurabilityError::InvalidStoredState)?;
        if content.npc_acquisition_program_digest() != Some(source::digest()) {
            return Err(CharacterProgressionError::InvalidInput);
        }
        permit.check_scope(fence.runtime_scope)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut owners = (permit, native, premium, effects);
        self.try_issue_semantic_pass()?.run_with_context(&mut owners,move|holder,deadline,(permit,native,premium,effects)|Box::pin(async move {
            let (permit, native, premium, effects) = (*permit, *native, *premium, *effects);
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:npc-cosmetic:'||encode($1,'hex')||':'||$2,0))")
                        .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).execute(&mut **tx).await?;
            match super::replay(&mut tx, fence.character_id, &request).await {
                Ok(Some(outcome)) => {
                    tx.commit(deadline).await?;
                    return Ok(Ok(Some(outcome)));
                }
                Err(error) => return Ok(Err(error)),
                Ok(None) => {}
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
                Err(EquipmentError::Database(error)) => return Err(error.into()),
                Err(EquipmentError::Durability(error)) => return Err(error),
            }
            let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                Ok(root) => root,
                Err(error) => return Ok(Err(error)),
            };
            let account: sqlx::types::Uuid = sqlx::query_scalar(
                "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
            )
            .bind(fence.character_id.as_bytes().as_slice())
            .fetch_one(&mut **tx)
            .await?;
            super::super::character_appearance::lock_account_grants(&mut tx, account).await?;
            let mut state = snapshot::load(
                &mut tx,
                fence,
                account,
                &node,
                native,
                premium,
                (&request.message, &request.npc),
            )
            .await?;
            if let Some(observation)=&effects.actor_observation {
                let greet=effects.greet.as_ref().ok_or(DurabilityError::InvalidStoredState)?;
                let _genuine=greet.observation(&fence,&request)?;
                if request.npc!="oteryn:npc.vescu" || !observation.valid_for_greet(greet) {
                    return Err(DurabilityError::InvalidStoredState);
                }
                state.actor_drunk=Some(observation.drunk());
            }
            let npc = document
                .npc(&request.npc)
                .ok_or(DurabilityError::InvalidStoredState)?;
            let choice = document
                .choose_for_trigger(
                    &request.npc, &request.message, request.topic, &state, source_contains,
                    if effects.greet.is_some() { source::Trigger::Greet } else { source::Trigger::Say },
                )
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let Some(choice) = choice else {
                tx.commit(deadline).await?;
                return Ok(Ok(None));
            };
            let rendered_reply = match &choice.reply_template {
                Some(template) if template.is_hazard() => {
                    let primal = state.primal.ok_or(DurabilityError::InvalidStoredState)?;
                    template.render_hazard(&choice.reply, source::HazardReplyFacts {
                        selected: primal.selected, unlocked: primal.unlocked,
                        requested: state.selected.and_then(|value| u16::try_from(value).ok()),
                    }).map_err(|_| DurabilityError::InvalidStoredState)?
                }
                Some(template)=>template.render(&choice.reply,state.now,|storage|state.track(storage),duration::source_duration)
                    .map_err(|_|DurabilityError::InvalidStoredState)?,
                None=>choice.reply.clone(),
            };
            let mut outcome = NpcCosmeticOutcome {
                reply: rendered_reply.replace("|PLAYERNAME|",&state.name),
                reply_lines:choice.reply_lines.iter().map(|line|line.replace("|PLAYERNAME|",&state.name)).collect(),
                topic: choice.topic_after,
                ..empty(choice.topic_after)
            };
            if choice.reply_template.as_ref().is_some_and(|template| template.has_hazard_followup(&choice.reply)) {
                outcome.reply_lines = vec![outcome.reply.clone(), choice.reply.replace("|PLAYERNAME|", &state.name)];
            }
            // Only closed registered callbacks may consume a zero-gameplay greeting.
            // They may close focus but cannot advance gameplay progression.
            if effects.greet.is_some() && readonly_greet::qualified(&request.npc, &choice.source_key) {
                if choice.trigger != source::Trigger::Greet
                    || !choice.effects.iter().all(|effect| matches!(effect, source::Effect::CloseConversation))
                { return Err(DurabilityError::InvalidStoredState); }
                let readonly = plan::prepare(npc, choice, &state, native, effects)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                outcome.close = readonly.close;
                let mut proof = receipt::plan_proof(&readonly, &[])?;
                proof["outcome"] = serde_json::to_value(&outcome)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                let mut before = state.proof();
                before["greet"] = effects.greet.as_ref()
                    .ok_or(DurabilityError::InvalidStoredState)?.observation(&fence, &request)?;
                if let Some(observation) = &effects.actor_observation {
                    before["actor_observation"] = observation.observation();
                }
                let observation_id: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
                    .fetch_one(&mut **tx).await?;
                receipt::arm(&mut tx, &node, fence, &request, &observation_id,
                    root.revision, before, &proof).await?;
                sqlx::query("SELECT game_npc_source_readonly_greet_capture()")
                    .execute(&mut **tx).await?;
                #[cfg(any(test, feature = "test-support"))]
                if std::env::var_os("OTERYN_SRV7_READONLY_OWNER_PROBE").is_some() {
                    readonly_owner_probe::qualify_after_actual_capture(&mut tx).await?;
                }
                tx.commit(deadline).await?;
                return Ok(Ok(Some(outcome)));
            }
            if choice.effects.is_empty() {
                tx.commit(deadline).await?;
                return Ok(Ok(Some(outcome)));
            }
            let prepared = match plan::prepare(npc, choice, &state, native, effects) {
                Ok(plan) => plan,
                Err(plan::PlanError::Blocked(feature)) => {
                    let _blocked_adapter = feature;
                    tx.commit(deadline).await?;
                    return Ok(Ok(Some(empty(request.topic))));
                }
                Err(plan::PlanError::Invalid) => return Err(DurabilityError::InvalidStoredState),
            };
            let next = root
                .revision
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)?;
            let progress=sqlx::query("SELECT character_revision::text,profile_revision,ruleset_revision,content_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            if numeric_u64(&progress, "character_revision")? != root.revision
                || !state_matches_root(&progress, &root)
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            let content = native
                .cosmetics()
                .ok_or(DurabilityError::InvalidStoredState)?;
            let grants = receipt::grant_keys(&prepared, &state, content)?;
            let quest_children = if prepared.quest_changes.is_empty() {Default::default()} else {
                let catalogue=effects.quest_catalogue.as_ref().ok_or(DurabilityError::InvalidStoredState)?;
                if progress.try_get::<String,_>("content_revision")? != catalogue.content_revision() {
                    return Err(DurabilityError::InvalidStoredState);
                }
                match super::super::quest_state::prepare_npc_source_children(
                    &mut tx,super::super::quest_state::SourceQuestPlanInput {
                        character:fence.character_id,source_key:prepared.source_key,source_changes:&prepared.quest_changes,
                        catalogue,now:state.now,claims:&state.claim_projections,bindings:&quest_bindings,
                    },
                ).await? {
                    Ok(children)=>children,
                    Err(_refusal)=>{tx.commit(deadline).await?;return Ok(Ok(Some(empty(request.topic))));}
                }
            };

            for cost in &prepared.costs {
                if super::super::carried_cost::available(&mut tx, fence.item_fence(), &cost.definition)
                    .await?
                    < cost.count
                {
                    tx.commit(deadline).await?;
                    return Ok(Ok(Some(empty(request.topic))));
                }
            }
            let transaction: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
                .fetch_one(&mut **tx)
                .await?;
            outcome.committed = true;
            outcome.magic_effect = prepared.magic_effect;
            outcome.advance_notices = prepared.advance_notices.clone();
            outcome.close = prepared.close;
            outcome.inventory_changed = !prepared.costs.is_empty() || prepared.paid || prepared.mint_output;
            let mut proof = receipt::plan_proof(&prepared, &grants)?;
            proof["outcome"] =
                serde_json::to_value(&outcome).map_err(|_| DurabilityError::InvalidStoredState)?;
            if !prepared.quest_changes.is_empty() {
                proof["quest_noops"]=quest_children.noops_observation();
                proof["quest_changes"]=serde_json::to_value(&prepared.quest_changes)
                    .map_err(|_|DurabilityError::InvalidStoredState)?;
                proof["quest_children"]=Value::Array(quest_children.writes.iter()
                    .map(|child|child.observation()).collect::<Result<Vec<_>,_>>()?);
            }
            let mut before_proof = state.proof();
            if let Some(observation)=&effects.actor_observation {before_proof["actor_observation"]=observation.observation();}
            if let Some(greet) = &effects.greet {
                if choice.trigger != source::Trigger::Greet {
                    return Err(DurabilityError::InvalidStoredState);
                }
                before_proof["greet"] = greet.observation(&fence,&request)?;
            }
            if let Some(effect) = &effects.actor_effect {
                if prepared.source_key != "oteryn:npc-cosmetic/ajax/barbarian-fire" {
                    return Err(DurabilityError::InvalidStoredState);
                }
                before_proof["actor_effect"] = effect.observation();
            }
            let plan_hash = receipt::arm(
                &mut tx,
                &node,
                fence,
                &request,
                &transaction,
                root.revision,
                before_proof,
                &proof,
            )
            .await?;
            let context = NpcProgramContext {
                fence, command: request.command, npc_key: &request.npc,
                source_key: prepared.source_key, program_sha256: source::digest(),
                plan_sha256: plan_hash.as_slice().try_into().map_err(|_|DurabilityError::InvalidStoredState)?,
                fee_transaction_id: if prepared.paid { Some(effects.payment.as_ref().ok_or(DurabilityError::InvalidStoredState)?.transaction) } else { None },
                npc_transaction: *sqlx::types::Uuid::parse_str(&transaction)
                    .map_err(|_|DurabilityError::InvalidStoredState)?.as_bytes(),
                next_revision: next, native,
            };
            if prepared.royal_kit {
                let room: bool=sqlx::query_scalar("SELECT game_npc_royal_inbox_admit($1::uuid,$2)")
                    .bind(&transaction).bind(prepared.source_key).fetch_one(&mut **tx).await?;
                if !room { return Ok(Ok(Some(NpcCosmeticOutcome { reply: "Please make sure you have free slots in your store inbox.".into(), ..empty(2) }))); }
            }
            let cost = if prepared.costs.is_empty() {
                None
            } else {
                let id = super::super::carried_cost::burn(
                    &mut tx,
                    permit,
                    fence.item_fence(),
                    request.command,
                    super::super::carried_cost::ClosedCostCause::NpcCosmetic,
                    prepared.source_key,
                    &prepared.costs,
                )
                .await?
                .ok_or(DurabilityError::InvalidStoredState)?;
                sqlx::query("SELECT game_npc_source_bind_cost($1::uuid)")
                    .bind(&id)
                    .execute(&mut **tx)
                    .await?;
                Some(id)
            };

            if prepared.paid {
                match payment::burn_source_fee(&mut tx,&context,prepared.transition,
                    effects.payment.as_ref().ok_or(DurabilityError::InvalidStoredState)?).await? {
                    payment::SourceFeeDisposition::Burned=>{},
                    payment::SourceFeeDisposition::InsufficientGold=>{
                        let (reply,topic)=source::paid::insufficient(&request.npc,request.topic).ok_or(DurabilityError::InvalidStoredState)?;
                        return Ok(Ok(Some(NpcCosmeticOutcome { reply:reply.into(), ..empty(topic) })));
                    },
                }
            }
            let mint_output = if prepared.mint_output {
                use crate::durability::reward_claim_mint::{npc_source, npc_source_facts};
                let weights = npc_source_facts::SourceWeights { native, premium };
                let output_topic = request.topic;
                let request = match npc_source_facts::request_in_transaction(&mut tx, &context).await? {
                    Ok(request) => request,
                    Err(crate::durability::reward_claim_mint::RewardClaimMintError::Refused(_)) => return Ok(Ok(Some(NpcCosmeticOutcome { reply: "Please make room in your main backpack.".into(), ..empty(output_topic) }))),
                    Err(_) => return Err(DurabilityError::InvalidStoredState),
                };
                match npc_source::prepare_in_transaction(&mut tx,&node,&context,request,&weights).await? {
                    Ok(prepared) => Some(prepared),
                    Err(crate::durability::reward_claim_mint::RewardClaimMintError::Refused(_)) => return Ok(Ok(Some(NpcCosmeticOutcome { reply: "Please make room in your main backpack.".into(), ..empty(output_topic) }))),
                    Err(_) => return Err(DurabilityError::InvalidStoredState),
                }
            } else { None };
            let fee_text=context.fee_transaction_id.map(|id|sqlx::types::Uuid::from_bytes(id).to_string());
            receipt::write(
                &mut tx,
                fence,
                &request,
                account,
                &transaction,
                next,
                &prepared,
                &grants,
                cost.as_deref(),
                fee_text.as_deref(),
                &outcome,
                &plan_hash,
            )
            .await?;
            for query in [
                "UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
                "UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
            ] {
                if sqlx::query(query)
                    .bind(fence.character_id.as_bytes().as_slice())
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
            receipt::apply(
                &mut tx,
                fence.character_id,
                account,
                &transaction,
                next,
                &prepared,
                &grants,
            )
            .await?;
            for child in &quest_children.writes {
                super::super::quest_state::persist_npc_source_child(&mut tx,&context,child).await?;
            }
            if let Some(effect) = &effects.actor_effect {
                ajax_actor_effect::persist_child(&mut tx, &context, effect).await?;
            }
            if let Some(output) = &mint_output {
                crate::durability::reward_claim_mint::npc_source::persist_in_transaction(&mut tx,&context,output).await?;
            }
            if let Some(selected)=prepared.hazard_selection {
                primal_hazard::persist_child(&mut tx,&context,selected).await?;
            }
            if prepared.royal_kit {
                super::super::house_wrap::mint_royal_kit(&mut tx,&context).await?;
            }
            tx.commit(deadline).await?;
            outcome.fresh = true;
            Ok(Ok(Some(outcome)))

        })).await?
    }
}
fn empty(topic: u16) -> NpcCosmeticOutcome {
    NpcCosmeticOutcome {
        reply: String::new(),
        reply_lines: Vec::new(),
        advance_notices: Vec::new(),
        topic,
        inventory_changed: false,
        committed: false,
        close: false,
        magic_effect: None,
        fresh: false,
    }
}
pub fn requires_ajax_fire(request: &NpcCosmeticRequest) -> bool {
    request.npc == "oteryn:npc.ajax"
        && request.topic == 5
        && source_contains(&request.message, "yes")
}

fn source_contains(text: &str, word: &str) -> bool {
    let text = text.to_ascii_lowercase();
    let word = word.to_ascii_lowercase();
    let mut found = false;
    for (index, _) in text.match_indices(&word) {
        if index > 0 && text.as_bytes()[index - 1].is_ascii_alphanumeric() {
            return false;
        }
        found = true
    }
    found
}

pub(crate) fn source_fee_cash_minimum(service: &str) -> Option<u64> {
    let document = source::embedded().ok()?;
    for npc in &document.npcs {
        for transition in &npc.transitions {
            let key = format!(
                "oteryn:service.npc-cosmetic.{}.{}",
                npc.npc.strip_prefix("oteryn:npc.")?,
                transition.id
            );
            if key == service {
                if !transition.effects.iter().any(|effect| {
                    matches!(
                        effect,
                        source::Effect::ConsumeMoney { .. }
                            | source::Effect::ConsumeMoneyBank { .. }
                    )
                }) {
                    return None;
                }
                return transition
                    .effects
                    .iter()
                    .try_fold(0_u64, |total, effect| match effect {
                        source::Effect::ConsumeMoney { gold } => total.checked_add(*gold),
                        _ => Some(total),
                    });
            }
        }
    }
    None
}
