//! SRVGAP-C4: novelty USE and cosmetic acquisition compose with the existing item-use writer.
//! Rules/citations live in the pinned source catalogues, not in client-supplied item metadata.
use super::super::character_equipment::assert_current_equipment_in_transaction;
use super::super::item_loop_move::ItemLoopContent;
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::foundation::CommandRef;
use oteryn_gameplay_items::usable_actions::{self as rules, Action, Refusal};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::collections::BTreeSet;

/// Live, account-bound Premium owner; never a client request bit or cached entitlement.
pub trait ActionPremium: Sync {
    fn current(&self, account: [u8; 16]) -> bool;
}
#[derive(Debug, Clone)]
pub struct ActionRequest {
    pub command: CommandRef,
    pub source: [u8; 16],
    pub intent: Vec<u8>,
    pub player_target: bool,
}
#[derive(Debug, Clone)]
pub struct ActionReceipt {
    pub disposition: UseDisposition,
    pub action: Option<Action>,
    pub replay: bool,
}
fn refused(disposition: UseDisposition) -> ActionReceipt {
    ActionReceipt {
        disposition,
        action: None,
        replay: false,
    }
}
impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn use_novelty_appearance<C: ItemLoopContent>(
        &self,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ActionRequest,
        native: &NativeGameplayState,
        content: &C,
        premium: &dyn ActionPremium,
        probe: bool,
    ) -> std::result::Result<Option<ActionReceipt>, DurabilityError> {
        if request.command.game_session_id() != fence.game_session_id
            || request.intent.is_empty()
            || request.intent.len() > 529
            || content.loop_content_digest() != native.source_digest()
        {
            return Ok(Some(refused(UseDisposition::Rejected)));
        }
        permit.check_scope(fence.runtime_scope)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (native, content, premium);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:c4:' || encode($1,'hex') || ':' || $2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).execute(&mut **tx).await?;
            let row = sqlx::query("SELECT intent,uuid_send(character_id) AS character,uuid_send(world_id) AS world,uuid_send(channel_id) AS channel,action FROM game_usable_action_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
            if let Some(row) = row {
                let receipt = if row.try_get::<Vec<u8>, _>("intent")? == request.intent && row.try_get::<Vec<u8>, _>("character")? == fence.character_id.as_bytes() && row.try_get::<Vec<u8>, _>("world")? == fence.runtime_scope.world_id().as_bytes() && row.try_get::<Vec<u8>, _>("channel")? == scope_of_channel(&fence)? {
                    ActionReceipt { disposition: UseDisposition::Committed, action: Some(serde_json::from_value(row.try_get("action")?).map_err(|_| DurabilityError::InvalidStoredState)?), replay: true }
                } else { refused(UseDisposition::Rejected) };
                tx.commit(deadline).await?;
                return Ok(Some(receipt));
            }
            if probe { tx.commit(deadline).await?; return Ok(None); }
            if assert_current_equipment_in_transaction(&mut tx, &recovery, &node, &fence, Some(request.command), context.0.source_digest()).await.is_err() {
                return Ok(Some(refused(UseDisposition::Rejected)));
            }
            let root = assert_item_only_fence(&mut tx, &fence, &node).await.map_err(|_| DurabilityError::Unavailable)?.map_err(|_| DurabilityError::Unavailable)?;
            let result = attempt(&mut tx, &fence, &request, context.0, context.1, context.2, root.content_revision, root.ruleset_revision).await?;
            if result.as_ref().is_some_and(|r| r.disposition == UseDisposition::Committed) { tx.commit(deadline).await?; }
            Ok(result)
        })).await
    }
}
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn attempt<C: ItemLoopContent>(
    tx: &mut mint_audit::Type2Transaction<'_>,
    fence: &CurrentCharacterItemFence,
    request: &ActionRequest,
    native: &NativeGameplayState,
    content: &C,
    premium: &dyn ActionPremium,
    revision: String,
    ruleset: String,
) -> std::result::Result<Option<ActionReceipt>, DurabilityError> {
    let inventory = read_use_inventory(tx, fence, revision.clone(), ruleset.clone())
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let entry = inventory
        .backpack
        .iter()
        .flat_map(|b| &b.entries)
        .chain(inventory.nested.iter().map(|(_, e)| e))
        .find(|e| e.item.item_instance_id == request.source);
    let item = entry.map(|e| &e.item).or_else(|| {
        inventory
            .equipment
            .iter()
            .find(|i| i.item_instance_id == request.source)
    });
    let Some(item) = item else {
        return Ok(None);
    };
    if !rules::supported(&item.definition.production_key) {
        return Ok(None);
    }
    if !native
        .item_policy(
            &item.definition.production_key,
            &item.definition.revision_ref,
        )
        .is_some_and(|p| {
            p.record().admission.materializable
                && p.record().production_definition.family == item.definition.family
        })
    {
        return Ok(Some(refused(UseDisposition::NotPossible)));
    }
    let account: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&mut ***tx)
    .await?;
    super::super::character_appearance::lock_account_grants(tx, account).await?;
    let keys: Vec<String> = rules::cosmetics()
        .and_then(|c| c["grants"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|g| g["key"].as_str().map(str::to_owned))
        .collect();
    let owned: Vec<String> = sqlx::query_scalar("SELECT u.unlock_key FROM game_account_appearance_unlocks u JOIN game_account_appearance_unlock_receipts r USING(grant_occurrence_id,account_id,unlock_key) WHERE u.account_id=$1 AND u.unlock_key=ANY($2::text[]) AND r.content_revision=$3 AND r.source_kind='item_action'")
        .bind(account).bind(&keys).bind(rules::PROVENANCE).fetch_all(&mut ***tx).await?;
    let mut owned: BTreeSet<String> = owned.into_iter().collect();
    // Existing quest receipts already prove these exact bases, even before a World authors
    // its selectable catalogue: character_appearance/quest_grants.rs (Revenant/DecayingDefender).
    let aliases = [
        ("oteryn:appearance-grant/quest/soul-war/revenant-base", "crystal-201189f-soul-war-revenant-v1", "oteryn:item-grant.outfit.1322-1323.base"),
        ("oteryn:appearance-grant/quest/rotten-blood/decaying-defender-base", "owner-pr2784-decaying-defender-base-v1", "oteryn:item-grant.outfit.1662-1663.base"),
    ];
    let alias_keys: Vec<_> = aliases.iter().map(|(key,_,_)| *key).collect();
    let existing: Vec<(String, String)> = sqlx::query_as("SELECT f.unlock_key,r.content_revision FROM game_account_appearance_unlocks f JOIN game_account_appearance_unlock_receipts r USING(grant_occurrence_id,account_id,unlock_key) WHERE f.account_id=$1 AND f.unlock_key=ANY($2::text[]) AND r.source_kind='quest'").bind(account).bind(&alias_keys).fetch_all(&mut ***tx).await?;
    for (key, revision) in existing {
        if let Some((_, _, source)) = aliases.iter().find(|(k, r, _)| *k == key && *r == revision) { owned.insert((*source).to_owned()); }
    }
    if let Some(view) = native.cosmetics() {
        use crate::domain::appearance::{
            AccountUnlocks, AppearanceContent, EarnedCosmetics, Sex, UnlockFact,
        };
        let account_id = crate::domain::AccountId::from_bytes(*account.as_bytes())
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let facts = super::super::character_appearance::read_account_appearance_unlocks(
            tx,
            account_id,
            &view.earned_grant_keys(),
        )
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?;
        let facts: Vec<_> = facts
            .into_iter()
            .filter(|f| {
                matches!(
                    f.source_kind.as_str(),
                    "quest" | "npc" | "mount_taming" | "task_board" | "item_action"
                )
            })
            .map(|f| (f.unlock_key, f.content_revision))
            .collect();
        let earned = EarnedCosmetics {
            content: view,
            facts: &facts,
        };
        for grant in rules::cosmetics()
            .and_then(|c| c["grants"].as_array())
            .into_iter()
            .flatten()
        {
            let Some(key) = grant["key"].as_str() else {
                continue;
            };
            let compatible = if grant["kind"] == "mount" {
                view.mount_keys().any(|m| {
                    view.mount_metadata(m)
                        .is_some_and(|(look, _, _)| grant["look_type"] == look)
                        && earned.mount(&m.to_owned()) == UnlockFact::Compatible
                })
            } else {
                view.outfit_keys().any(|o| {
                    let same = [Sex::Female, Sex::Male].iter().all(|sex| {
                        view.look_type(o, *sex).is_some_and(|look| {
                            grant["look_types"]
                                .as_array()
                                .is_some_and(|looks| looks.contains(&serde_json::Value::from(look)))
                        })
                    });
                    same && if grant["kind"] == "outfit" {
                        view.outfit_is_starter(&o.to_owned(), Sex::Female)
                            && view.outfit_is_starter(&o.to_owned(), Sex::Male)
                            || earned.outfit(&o.to_owned()) == UnlockFact::Compatible
                    } else {
                        grant["addon"].as_u64().is_some_and(|addon| {
                            u8::try_from(addon).is_ok_and(|addon| {
                                earned.addon(&o.to_owned(), addon) == UnlockFact::Compatible
                            })
                        })
                    }
                })
            };
            if compatible {
                owned.insert(key.to_owned());
            }
        }
    }
    let on_head: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid AND slot=1 AND item_instance_id=encode($2,'hex')::uuid)").bind(fence.character_id.as_bytes().as_slice()).bind(request.source.as_slice()).fetch_one(&mut ***tx).await?;
    let action = match rules::decide(
        &item.definition.production_key,
        *fence.game_session_id.as_bytes(),
        request.command.command_id().get(),
        request.source,
        &owned,
        premium.current(*account.as_bytes()),
        on_head,
        request.player_target,
    ) {
        Ok(action) => action,
        Err(reason) => {
            return Ok(Some(refused(match reason {
                Refusal::Requirement => UseDisposition::RequirementNotMet,
                Refusal::Duplicate => UseDisposition::NothingToUse,
                Refusal::Invalid => UseDisposition::NotPossible,
            })));
        }
    };
    let (transaction, event, fresh, occurred): (Vec<u8>, Vec<u8>, Vec<u8>, i64) = sqlx::query_as("SELECT uuid_send(game_character_uuid_v7()),uuid_send(game_character_uuid_v7()),uuid_send(game_character_uuid_v7()),(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut ***tx).await?;
    let transaction: [u8; 16] = transaction
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let event: [u8; 16] = event
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if action.consume {
        let Some(entry) = entry else {
            return Ok(Some(refused(UseDisposition::RequirementNotMet)));
        };
        let flask = if let Some(output) = action.output {
            let Some(policy) = native
                .item_policy_for_source_id(output)
                .filter(|p| p.record().admission.materializable)
            else {
                return Ok(Some(refused(UseDisposition::NotPossible)));
            };
            let output = &policy.record().production_definition;
            let output = TypedDefinitionRef {
                family: output.family.clone(),
                production_key: output.production_key.clone(),
                revision_ref: output.revision_ref.clone(),
            };
            let Some(facts) = content.loop_facts(&output) else {
                return Ok(Some(refused(UseDisposition::NotPossible)));
            };
            let Some(source_weight) = content.loop_facts(&item.definition).and_then(|f| f.weight)
            else {
                return Ok(Some(refused(UseDisposition::NotPossible)));
            };
            let Some(output_weight) = facts.weight else {
                return Ok(Some(refused(UseDisposition::NotPossible)));
            };
            let carried = super::super::item_loop_move::carried_weight(
                tx,
                fence.character_id,
                *fence.runtime_scope.world_id().as_bytes(),
                content,
            )
            .await
            .map_err(|_| DurabilityError::InvalidStoredState)?;
            let capacity = super::super::item_transfer::read_derived_transfer_capacity_centi_oz(
                tx,
                fence.character_id,
                content,
            )
            .await?;
            if carried
                .checked_sub(u64::from(source_weight))
                .and_then(|weight| weight.checked_add(u64::from(output_weight)))
                .is_none_or(|weight| weight > capacity)
            {
                return Ok(Some(refused(UseDisposition::Full)));
            }
            let bag = inventory
                .backpack
                .as_ref()
                .ok_or(DurabilityError::InvalidStoredState)?;
            let capacity = content
                .loop_facts(&bag.backpack.definition)
                .and_then(|f| f.container_capacity)
                .unwrap_or(0);
            match super::drome_cube::plan_output(&inventory, entry, facts, capacity, &[], || {
                fresh.try_into().ok()
            }) {
                Ok(plan) => plan,
                Err(_) => return Ok(Some(refused(UseDisposition::Full))),
            }
        } else {
            ItemUseFlaskPlan::None
        };
        let use_request = ItemUseRequest {
            kind: ItemUseKind::UsableAction,
            command_id: request.command.command_id().get(),
            content_revision: revision,
            ruleset_revision: ruleset,
            sim_revision: "srvgap-c4-v1".into(),
            transaction_id: transaction,
            event_id: event,
            occurred_at_unix_ms: occurred,
            server_build_id: super::super::character_authority::SERVER_BUILD_ID.into(),
            source_item_instance_id: request.source,
            source_definition: item.definition.clone(),
            source_quantity_before: item.quantity,
            flask,
        };
        use_request
            .validate()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let tuple = tx.tuple();
        super::write(tx, fence, &use_request, tuple)
            .await
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if action.item_id == 6572 {
            let ItemUseFlaskPlan::New {
                item_instance_id,
                definition,
                ..
            } = &use_request.flask
            else {
                return Err(DurabilityError::InvalidStoredState);
            };
            sqlx::query("INSERT INTO game_spell_item_temporal_schedules(item_instance_id,source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,target_family,target_production_key,target_revision,content_digest) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,4000,$3+4000,'Item',$4,$5,'Item',$6,$7,$8)")
                .bind(item_instance_id.as_slice()).bind(transaction.as_slice()).bind(occurred).bind(&definition.production_key).bind(&definition.revision_ref).bind(&item.definition.production_key).bind(&item.definition.revision_ref).bind(native.source_digest().as_slice()).execute(&mut ***tx).await?;
        }
    }
    for grant in &action.grants {
        sqlx::query("WITH receipt AS (INSERT INTO game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) VALUES(game_character_uuid_v7(),$1,$2,$3,encode($4,'hex')::uuid,'item_action',$5) RETURNING *) INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id) SELECT account_id,unlock_key,grant_occurrence_id FROM receipt")
            .bind(account).bind(grant).bind(rules::PROVENANCE).bind(fence.character_id.as_bytes().as_slice()).bind(transaction.as_slice()).execute(&mut ***tx).await?;
    }
    super::super::account_achievement::grant_missing(
        tx,
        fence.character_id,
        &action
            .achievements
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        "item_action",
        &transaction,
    )
    .await?;
    for (key, total) in &action.progress {
        add_progress(
            tx,
            fence.character_id,
            key,
            *total,
            "item_action",
            &transaction,
        )
        .await?;
    }
    sqlx::query("INSERT INTO game_usable_action_receipts(transaction_id,game_session_id,command_id,character_id,world_id,channel_id,source_item_instance_id,intent,action,consumed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8,$9,CASE WHEN $10 THEN encode($1,'hex')::uuid END)")
        .bind(transaction.as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(scope_of_channel(fence)?).bind(request.source.as_slice()).bind(&request.intent).bind(serde_json::to_value(&action).map_err(|_| DurabilityError::InvalidStoredState)?).bind(action.consume).execute(&mut ***tx).await?;
    Ok(Some(ActionReceipt {
        disposition: UseDisposition::Committed,
        action: Some(action),
        replay: false,
    }))
}
fn scope_of_channel(
    fence: &CurrentCharacterItemFence,
) -> std::result::Result<Vec<u8>, DurabilityError> {
    match fence.runtime_scope {
        RuntimeScopeRefV1::Channel { channel_id, .. } => Ok(channel_id.as_bytes().to_vec()),
        _ => Err(DurabilityError::InvalidStoredState),
    }
}
