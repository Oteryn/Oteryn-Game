//! Crystal soul_war_mechanics.lua:830-855: actual STEP consumes native blood.
use super::*;
use crate::durability::spell_item_transaction::StandingTileRead;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementPositionSnapshot, RuntimeScopeRefV1,
};

pub fn cloak_blood_percentage(key: &str) -> Option<u8> {
    match key {
        "oteryn:item.tibia.i33854" => Some(20),
        "oteryn:item.tibia.i34006" => Some(15),
        "oteryn:item.tibia.i34007" => Some(10),
        _ => None,
    }
}

#[derive(Debug)]
pub struct CloakBloodPlayerStep {
    scope: RuntimeScopeRefV1,
    generation: u64,
    content: [u8; 32],
    map: [u8; 32],
    frame: [u8; 32],
    target: SpellGroundTarget,
    source: [u8; 32],
    items: Vec<[u8; 16]>,
}
impl CloakBloodPlayerStep {
    #[allow(
        clippy::too_many_arguments,
        reason = "the committed movement and sealed destination read independently bind this callback"
    )]
    pub fn from_current_player_step(
        runtime: &ChannelRuntimeV1,
        room: &crate::content::QualifiedNativeEntryRoom,
        native: &crate::content::native_gameplay::NativeGameplayState,
        actor: ExactActorRef,
        session: GameSessionId,
        committed: MovementPositionSnapshot,
        now: u64,
        read: &StandingTileRead,
    ) -> Result<Self> {
        runtime
            .owner_fence()
            .map_err(|_| SpellItemError::Rejected("blood STEP owner"))?;
        runtime
            .bound_attacker_lease(actor, session)
            .map_err(|_| SpellItemError::Rejected("blood STEP Player lease"))?;
        let target = SpellGroundTarget::for_native_tile_read(room, runtime, committed.position())
            .map_err(SpellItemError::Rejected)?;
        if !native.qualifies_soul_war_callbacks(runtime.content_pin().server_artifact_digest())
            || runtime.read_actor_position(actor).ok() != Some(committed)
            || committed.last_translation_revision() == 0
            || committed.revision() != committed.last_translation_revision()
            || read.fence().game_session_id != session
            || read.fence().runtime_scope != runtime.binding().runtime_scope()
            || read.fence().scope_ownership_generation != runtime.binding().scope_generation()
            || read.content_digest() != native.source_digest()
            || read.target() != &target
        {
            return Err(SpellItemError::Rejected(
                "blood exact committed Player STEP/read",
            ));
        }
        let items: Vec<_> = read
            .items()
            .iter()
            .filter(|item| cloak_blood_percentage(&item.definition.production_key).is_some())
            .map(|item| item.item_instance_id)
            .collect();
        if items.is_empty() || items.len() > 500 {
            return Err(SpellItemError::Rejected("blood STEP bounded real Items"));
        }
        let mut hash = Sha256::new();
        hash.update(b"oteryn:cloak-blood-player-step:v1");
        hash.update(actor.placement_identity());
        hash.update(actor.actor_local_generation().to_be_bytes());
        hash.update(session.as_bytes());
        hash.update(now.to_be_bytes());
        hash.update(committed.revision().to_be_bytes());
        hash.update(&target.spatial_position);
        hash.update(native.source_digest());
        hash.update(runtime.map_revision_digest());
        hash.update(runtime.content_pin().frame_binding_digest());
        for item in &items {
            hash.update(item);
        }
        Ok(Self {
            scope: runtime.binding().runtime_scope(),
            generation: runtime.binding().scope_generation().get(),
            content: native.source_digest(),
            map: runtime.map_revision_digest(),
            frame: runtime.content_pin().frame_binding_digest(),
            target,
            source: hash.finalize().into(),
            items,
        })
    }
}

#[derive(Debug)]
pub struct CloakBloodStepReceipt {
    percentages: Vec<u8>,
}
impl CloakBloodStepReceipt {
    pub fn percentages(&self) -> &[u8] {
        &self.percentages
    }
}
fn prefix(step: &CloakBloodPlayerStep) -> String {
    format!(
        "cloak-blood-step:{}:",
        super::rotten_portal::hex(&step.source)
    )
}
fn instance_bytes(authority: &SpellItemScopeAuthority) -> Option<Vec<u8>> {
    match authority.runtime_scope() {
        RuntimeScopeRefV1::Instance { instance_id, .. } => Some(instance_id.to_vec()),
        _ => None,
    }
}
async fn prior(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    step: &CloakBloodPlayerStep,
) -> Result<Option<CloakBloodStepReceipt>> {
    let rows=sqlx::query("SELECT decode(replace(l.item_instance_id::text,'-',''),'hex') AS item,l.definition_production_key FROM game_spell_item_receipts r JOIN game_spell_item_lines l USING(transaction_id) WHERE r.cause_kind=2 AND r.spell_production_key='oteryn:encounter.soul_war_taint_zones' AND r.world_id=encode($1,'hex')::uuid AND r.channel_id IS NOT DISTINCT FROM CASE WHEN $2::bytea IS NULL THEN NULL ELSE encode($2,'hex')::uuid END AND r.instance_id IS NOT DISTINCT FROM CASE WHEN $3::bytea IS NULL THEN NULL ELSE encode($3,'hex')::uuid END AND r.ownership_generation=$4::text::numeric AND r.creature_occurrence LIKE $5||'%' AND r.catalog_digest=$6 AND r.creature_map_digest=$7 AND r.creature_frame_digest=$8 AND l.operation_kind=2 AND l.spatial_position=$9 LIMIT 500")
        .bind(authority.world_bytes().as_slice()).bind(authority.channel_bytes().map(|v|v.to_vec())).bind(instance_bytes(authority))
        .bind(authority.generation().to_string()).bind(prefix(step)).bind(step.content.as_slice()).bind(step.map.as_slice()).bind(step.frame.as_slice()).bind(&step.target.spatial_position)
        .fetch_all(&mut **tx).await?;
    if rows.is_empty() {
        return Ok(None);
    }
    if rows.len() > step.items.len() {
        return Err(SpellItemError::Rejected("blood STEP partial replay"));
    }
    let mut found = std::collections::BTreeMap::new();
    for row in rows {
        let item: [u8; 16] = row
            .try_get::<Vec<u8>, _>("item")?
            .try_into()
            .map_err(|_| SpellItemError::Rejected("blood Item identity"))?;
        let key: String = row.try_get("definition_production_key")?;
        let percent =
            cloak_blood_percentage(&key).ok_or(SpellItemError::Rejected("blood replay stage"))?;
        if found.insert(item, percent).is_some() {
            return Err(SpellItemError::Rejected("blood STEP duplicate replay"));
        }
    }
    let percentages = step
        .items
        .iter()
        .filter_map(|item| found.remove(item))
        .collect();
    if !found.is_empty() {
        return Err(SpellItemError::Rejected("blood STEP replay identity"));
    }
    Ok(Some(CloakBloodStepReceipt { percentages }))
}

async fn execute(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    step: &CloakBloodPlayerStep,
) -> Result<Option<CloakBloodStepReceipt>> {
    check_scope_transaction(tx, authority).await?;
    if authority.runtime_scope() != step.scope || authority.generation() != step.generation {
        return Err(SpellItemError::Rejected("blood STEP current scope"));
    }
    if let Some(receipt) = prior(tx, authority, step).await? {
        return Ok(Some(receipt));
    }
    let tile = read_creature_scope_tile_in_transaction(tx, authority, &step.target).await?;
    for id in &step.items {
        let item = tile
            .items
            .iter()
            .find(|item| &item.item_instance_id == id)
            .ok_or(SpellItemError::Rejected("blood STEP actual pool absent"))?;
        if cloak_blood_percentage(&item.definition.production_key).is_none()
            || item.definition.revision_ref != "definition-r1"
        {
            return Err(SpellItemError::Rejected("blood STEP current source stage"));
        }
        let Some(row)=sqlx::query("SELECT r.creature_placement,r.creature_actor_generation::text,r.creature_source_body_digest FROM game_item_instances i JOIN game_spell_item_receipts r ON r.transaction_id=i.minted_transaction_id JOIN game_spell_item_lines l ON l.transaction_id=r.transaction_id AND l.item_instance_id=i.item_instance_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND r.cause_kind=2 AND r.spell_production_key='oteryn:encounter.soul_war_taint_zones' AND r.creature_definition_key='oteryn:creature.cloak_of_terror' AND r.creature_occurrence ~ '^cloak-blood:[0-9a-f]{64}$' AND r.world_id=encode($2,'hex')::uuid AND r.channel_id IS NOT DISTINCT FROM CASE WHEN $3::bytea IS NULL THEN NULL ELSE encode($3,'hex')::uuid END AND r.instance_id IS NOT DISTINCT FROM CASE WHEN $4::bytea IS NULL THEN NULL ELSE encode($4,'hex')::uuid END AND r.ownership_generation=$5::text::numeric AND r.catalog_digest=$6 AND r.creature_map_digest=$7 AND r.creature_frame_digest=$8 AND l.operation_kind=1 AND l.definition_production_key='oteryn:item.tibia.i33854'")
            .bind(id.as_slice()).bind(authority.world_bytes().as_slice()).bind(authority.channel_bytes().map(|v|v.to_vec())).bind(instance_bytes(authority))
            .bind(authority.generation().to_string()).bind(step.content.as_slice()).bind(step.map.as_slice()).bind(step.frame.as_slice()).fetch_optional(&mut **tx).await?
            else { continue; };
        let cause = CreatureGroundCause {
            world: authority.world_bytes(),
            scope: step.scope,
            scope_generation: step.generation,
            source_items: vec![item.definition.clone()],
            placement: row
                .try_get::<Vec<u8>, _>("creature_placement")?
                .try_into()
                .map_err(|_| SpellItemError::Rejected("blood historical creator"))?,
            generation: integer(row.try_get("creature_actor_generation")?)?,
            creature: TypedDefinitionRef {
                family: "Creature".into(),
                production_key: "oteryn:creature.cloak_of_terror".into(),
                revision_ref: "definition-r1".into(),
            },
            ability: TypedDefinitionRef {
                family: "Encounter".into(),
                production_key: "oteryn:encounter.soul_war_taint_zones".into(),
                revision_ref: "definition-r1".into(),
            },
            event_rule: None,
            occurrence: format!("{}{}", prefix(step), super::rotten_portal::hex(id)),
            content: step.content,
            map: step.map,
            frame: step.frame,
            body: row
                .try_get::<Vec<u8>, _>("creature_source_body_digest")?
                .try_into()
                .map_err(|_| SpellItemError::Rejected("blood original body"))?,
            cast: Sha256::new()
                .chain_update(b"oteryn:cloak-blood-per-item-step:v1")
                .chain_update(step.source)
                .chain_update(id)
                .finalize()
                .into(),
        };
        let removal =
            prepare_creature_scope_removal_in_transaction(tx, authority, &step.target, *id).await?;
        let prepared = PreparedCreatureGroundOperations {
            placement: cause.placement,
            generation: cause.generation,
            body: cause.body,
            cast: cause.cast,
            operations: vec![SpellItemOperation::RemoveGround(removal)],
        };
        apply_creature_ground_in_transaction(
            tx,
            authority,
            &cause,
            super::primal_pod::identity(&cause, b"oteryn:cloak-blood-step-transaction:v1"),
            super::primal_pod::identity(&cause, b"oteryn:cloak-blood-step-event:v1"),
            &prepared,
            || Ok(()),
        )
        .await?;
    }
    prior(tx, authority, step).await
}
impl DurabilityRoot {
    pub async fn commit_cloak_blood_step(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        step: &CloakBloodPlayerStep,
    ) -> Result<Option<CloakBloodStepReceipt>> {
        let record = creature_item_recovery_record(recovery, self)?;
        let node = node.clone();
        let mut context = (permit, step);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, ctx| {
                Box::pin(async move {
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let outcome: Result<Option<CloakBloodStepReceipt>> = async {
                        let authority = assert_spell_item_scope_with_recovery(
                            &mut tx,
                            ctx.0,
                            &record,
                            &node,
                            ctx.1.scope,
                            ctx.1.generation,
                        )
                        .await?;
                        execute(&mut tx, &authority, ctx.1).await
                    }
                    .await;
                    match outcome {
                        Ok(receipt) => {
                            commit_creature_item_owner_transaction(tx, deadline).await?;
                            Ok(Ok(receipt))
                        }
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}
