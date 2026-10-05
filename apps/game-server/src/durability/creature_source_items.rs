//! Source-qualified Creature composition over the existing durable Item owner.
//! Compiled only by the real server lib, not standalone Durability fixtures.
#![allow(
    dead_code,
    reason = "Source-qualified SQL and native owner component is locally tested; Channel shipping activation remains the explicit #162 single-scheduler integration gate"
)]
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::spell_item_transaction::{
    PhysicalItemCause, PhysicalItemOwner, SpellItemError, SpellItemScopeAuthority,
    apply_physical_item_line, assert_spell_item_scope_with_recovery, begin_spell_owner_transaction,
    check_scope_transaction, commit_creature_item_owner_transaction, creature_item_recovery_record,
    decoded_definition, decoded_placement, ground, integer, item_definition_valid,
    locked_tile_rows_for_scope, prepare_creature_scope_removal_in_transaction,
    read_creature_scope_tile_in_transaction, reference, source_map_protection_tags, uuid, uuid_v7,
};
use crate::durability::spell_items_abi::*;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};
type Result<T> = std::result::Result<T, SpellItemError>;

/// Sealed cause is created by the trusted current Creature source dispatcher,
/// after immutable native ability membership and owner fence validation.
/// It grants no inventory, Player, cost or command authority.
#[derive(PartialEq, Eq)]
struct CreatureGroundCause {
    world: [u8; 16],
    channel: [u8; 16],
    scope_generation: u64,
    source_items: Vec<TypedDefinitionRef>,
    placement: [u8; 16],
    generation: u64,
    creature: TypedDefinitionRef,
    ability: TypedDefinitionRef,
    occurrence: String,
    content: [u8; 32],
    map: [u8; 32],
    frame: [u8; 32],
    body: [u8; 32],
    cast: [u8; 32],
}
struct PreparedCreatureGroundOperations {
    placement: [u8; 16],
    generation: u64,
    body: [u8; 32],
    cast: [u8; 32],
    operations: Vec<SpellItemOperation>,
}
pub(crate) struct CreatureGroundReceipt {
    pub(crate) transaction_id: [u8; 16],
    pub(crate) event_id: [u8; 16],
    pub(crate) already_committed: bool,
    pub(crate) binding: [u8; 32],
}

/// One same-table source-scope Item transaction. Caller must ROLLBACK on any
/// error; no caller may recover an error and COMMIT the partial SQL transaction.
/// The guard runs before a new write and also on receipt replay, so a stale
/// Creature occurrence never gains authority from a formerly committed receipt.
async fn apply_creature_ground_in_transaction<F>(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    cause: &CreatureGroundCause,
    transaction: [u8; 16],
    event: [u8; 16],
    prepared: &PreparedCreatureGroundOperations,
    current_guard: F,
) -> Result<CreatureGroundReceipt>
where
    F: FnOnce() -> Result<()>,
{
    let operations = &prepared.operations;
    if prepared.placement != cause.placement
        || prepared.generation != cause.generation
        || prepared.body != cause.body
        || prepared.cast != cause.cast
    {
        return Err(SpellItemError::Rejected("Creature prepared batch binding"));
    }
    check_scope_transaction(tx, authority).await?;
    current_guard()?;
    if authority.world_bytes() != cause.world
        || authority.channel_bytes() != cause.channel
        || authority.generation() != cause.scope_generation
    {
        return Err(SpellItemError::Rejected("Creature source foreign scope"));
    }
    if cause.placement == [0; 16]
        || cause.generation == 0
        || cause.creature.family != "Creature"
        || cause.ability.family != "Ability"
        || cause.creature.production_key.is_empty()
        || cause.creature.revision_ref.is_empty()
        || cause.ability.production_key.is_empty()
        || cause.ability.revision_ref.is_empty()
        || cause.occurrence.is_empty()
        || cause.occurrence.len() > 1024
        || [cause.content, cause.map, cause.frame, cause.body].contains(&[0; 32])
        || !uuid_v7(&transaction)
        || !uuid_v7(&event)
        || operations.len() > 500
    {
        return Err(SpellItemError::Rejected("Creature ground cause shape"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut encoded = Vec::new();
    for op in operations {
        let (id, value) = match op {
            SpellItemOperation::MintGround {
                item_instance_id,
                definition,
                quantity,
                placement,
                lifetime_millis,
                blocks_movement,
                blocks_projectile,
                description,
            } => {
                if !definition.materializable
                    || !definition.ground_destination
                    || definition.content_generation_digest != cause.content
                    || !item_definition_valid(&definition.definition)
                    || *quantity == 0
                    || *quantity != 1
                    || !cause.source_items.contains(&definition.definition)
                    || *quantity > definition.stack_maximum
                    || definition.stack_maximum > 100
                    || definition.container_capacity.is_some()
                    || description.is_some()
                    || matches!(lifetime_millis, Some(0))
                {
                    return Err(SpellItemError::Rejected(
                        "Creature ground mint qualification",
                    ));
                }
                (
                    *item_instance_id,
                    serde_json::json!({"kind":"mint_ground","id":item_instance_id,"definition":reference(&definition.definition),"quantity":quantity,"placement":ground(placement),"lifetime":lifetime_millis,"blocks_movement":blocks_movement,"blocks_projectile":blocks_projectile,"decay_chain":definition.decay_chain.iter().map(|s|serde_json::json!({"definition":reference(&s.definition),"duration_millis":s.duration_millis,"target":s.target.as_ref().map(reference),"blocks_movement":s.blocks_movement,"blocks_projectile":s.blocks_projectile,"immovable_block_solid":s.immovable_block_solid})).collect::<Vec<_>>()}),
                )
            }
            SpellItemOperation::RemoveGround(r) => {
                if !r.contents.is_empty()
                    || r.quantity == 0
                    || r.state_revision == 0
                    || r.top_down_ordinal == 0
                    || !item_definition_valid(&r.definition)
                {
                    return Err(SpellItemError::Rejected(
                        "Creature ground removal qualification",
                    ));
                }
                (
                    r.item_instance_id,
                    serde_json::json!({"kind":"remove_ground","id":r.item_instance_id,"definition":reference(&r.definition),"revision":r.state_revision,"ordinal":r.top_down_ordinal,"quantity":r.quantity,"placement":ground(&r.placement)}),
                )
            }
            _ => return Err(SpellItemError::Rejected("Creature ground-only authority")),
        };
        if !uuid_v7(&id) || !ids.insert(id) {
            return Err(SpellItemError::Rejected("Creature duplicate Item identity"));
        }
        encoded.push(value);
    }
    let hex = |b: &[u8]| b.iter().map(|v| format!("{v:02x}")).collect::<String>();
    let intent=serde_json::to_vec(&serde_json::json!({"schema":"OTERYN_CREATURE_GROUND_CAUSE/v1","player":null,"cost":null,"caster_origin":null,"world":hex(&authority.world_bytes()),"channel":hex(&authority.channel_bytes()),"ownership_generation":authority.generation().to_string(),"creature":{"key":cause.creature.production_key,"revision":cause.creature.revision_ref,"placement":hex(&cause.placement),"generation":cause.generation.to_string()},"ability":reference(&cause.ability),"occurrence":cause.occurrence,"content":hex(&cause.content),"map":hex(&cause.map),"frame":hex(&cause.frame),"source_body":hex(&cause.body),"source_cast":hex(&cause.cast),"operations":encoded})).map_err(|_|SpellItemError::Rejected("Creature intent encoding"))?;
    if intent.len() > 524288 {
        return Err(SpellItemError::Rejected("Creature audit budget"));
    }
    let binding: [u8; 32] = Sha256::digest(&intent).into();
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || ':' || $2 || ':' || $3 || ':' || $4,45))")
        .bind(cause.placement.as_slice()).bind(cause.generation.to_string()).bind(&cause.ability.production_key).bind(&cause.occurrence).execute(&mut **tx).await?;
    let prior=sqlx::query("SELECT transaction_id::text,event_id::text,binding FROM game_spell_item_receipts WHERE cause_kind=1 AND world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND ownership_generation=$3::text::numeric(20,0) AND creature_placement=$4 AND creature_actor_generation=$5::text::numeric(20,0) AND spell_production_key=$6 AND spell_revision=$7 AND creature_occurrence=$8")
        .bind(authority.world_bytes().as_slice()).bind(authority.channel_bytes().as_slice()).bind(authority.generation().to_string()).bind(cause.placement.as_slice()).bind(cause.generation.to_string()).bind(&cause.ability.production_key).bind(&cause.ability.revision_ref).bind(&cause.occurrence).fetch_optional(&mut **tx).await?;
    if let Some(row) = prior {
        if row.try_get::<Vec<u8>, _>("binding")?.as_slice() != binding {
            return Err(SpellItemError::Rejected(
                "conflicting Creature source replay",
            ));
        }
        return Ok(CreatureGroundReceipt {
            transaction_id: uuid(&row.try_get::<String, _>("transaction_id")?)?,
            event_id: uuid(&row.try_get::<String, _>("event_id")?)?,
            already_committed: true,
            binding,
        });
    }
    // Revalidate every exact removal before publishing any receipt or line.
    for op in operations {
        if let SpellItemOperation::RemoveGround(r) = op {
            let target = SpellGroundTarget {
                spatial_position: r.placement.spatial_position.clone(),
                map_revision: r.placement.map_revision.clone(),
                content_revision: r.placement.content_revision.clone(),
                placement_context: r.placement.native_room_placement_context.clone(),
            };
            let rows = locked_tile_rows_for_scope(tx, authority, &target).await?;
            let row = rows
                .iter()
                .find(|row| {
                    row.try_get::<String, _>("item_instance_id")
                        .ok()
                        .and_then(|s| uuid(&s).ok())
                        == Some(r.item_instance_id)
                })
                .ok_or(SpellItemError::Rejected(
                    "Creature reserved Item left Ground",
                ))?;
            if decoded_definition(row)? != r.definition
                || decoded_placement(row)? != r.placement
                || integer(row.try_get("state_revision")?)? != r.state_revision
                || integer(row.try_get("stack_ordinal")?)? != r.top_down_ordinal
                || row.try_get::<i64, _>("quantity")? != i64::from(r.quantity)
            {
                return Err(SpellItemError::Rejected(
                    "Creature exact Ground custody changed",
                ));
            }
            let children:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(r.item_instance_id.as_slice()).fetch_one(&mut **tx).await?;
            if children != 0 {
                return Err(SpellItemError::Rejected("Creature remove owns children"));
            }
        }
    }
    let occurred: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    // Deadline overflow is checked across the whole batch before the receipt.
    for op in operations {
        if let SpellItemOperation::MintGround {
            lifetime_millis: Some(ms),
            ..
        } = op
        {
            occurred
                .checked_add(i64::from(*ms))
                .ok_or(SpellItemError::Rejected("Creature field deadline overflow"))?;
        }
    }
    sqlx::query("INSERT INTO game_spell_item_receipts(transaction_id,event_id,world_id,channel_id,ownership_generation,spell_family,spell_production_key,spell_revision,catalog_digest,binding,intent,operation_count,occurred_at_unix_ms,cause_kind,creature_placement,creature_actor_generation,creature_definition_key,creature_definition_revision,creature_occurrence,creature_map_digest,creature_frame_digest,creature_source_body_digest,creature_cast_digest) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric(20,0),'Ability',$6,$7,$8,$9,$10,$11,$12,1,$13,$14::text::numeric(20,0),$15,$16,$17,$18,$19,$20,$21)")
        .bind(transaction.as_slice()).bind(event.as_slice()).bind(authority.world_bytes().as_slice()).bind(authority.channel_bytes().as_slice()).bind(authority.generation().to_string()).bind(&cause.ability.production_key).bind(&cause.ability.revision_ref).bind(cause.content.as_slice()).bind(binding.as_slice()).bind(&intent).bind(i32::try_from(operations.len()).map_err(|_|SpellItemError::Rejected("Creature operation count"))?).bind(occurred).bind(cause.placement.as_slice()).bind(cause.generation.to_string()).bind(&cause.creature.production_key).bind(&cause.creature.revision_ref).bind(&cause.occurrence).bind(cause.map.as_slice()).bind(cause.frame.as_slice()) .bind(cause.body.as_slice()).bind(cause.cast.as_slice()).execute(&mut **tx).await?;
    for (index, op) in operations.iter().enumerate() {
        match op {
            SpellItemOperation::MintGround {
                item_instance_id,
                definition,
                quantity,
                placement,
                lifetime_millis,
                blocks_movement,
                blocks_projectile,
                ..
            } => {
                let expires = lifetime_millis
                    .map(|ms| {
                        occurred
                            .checked_add(i64::from(ms))
                            .ok_or(SpellItemError::Rejected("Creature deadline"))
                    })
                    .transpose()?;
                apply_physical_item_line(
                    tx,
                    PhysicalItemOwner::Creature(authority),
                    PhysicalItemCause::Creature(transaction),
                    index + 1,
                    occurred,
                    op,
                    1,
                    *item_instance_id,
                    &definition.definition,
                    0,
                    *quantity,
                    0,
                    None,
                    placement,
                    cause.content,
                    *blocks_movement,
                    *blocks_projectile,
                    expires,
                    None,
                )
                .await?;
            }
            SpellItemOperation::RemoveGround(r) => {
                apply_physical_item_line(
                    tx,
                    PhysicalItemOwner::Creature(authority),
                    PhysicalItemCause::Creature(transaction),
                    index + 1,
                    occurred,
                    op,
                    2,
                    r.item_instance_id,
                    &r.definition,
                    r.quantity,
                    0,
                    r.state_revision,
                    Some(r.top_down_ordinal),
                    &r.placement,
                    cause.content,
                    false,
                    false,
                    None,
                    None,
                )
                .await?
            }
            _ => return Err(SpellItemError::Rejected("Creature operation invariant")),
        }
    }
    sqlx::query("INSERT INTO game_spell_item_audit_outbox(event_id,transaction_id,occurred_at_unix_ms,expires_at_unix_ms,envelope,envelope_sha256) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$3+7776000000,$4,$5)").bind(event.as_slice()).bind(transaction.as_slice()).bind(occurred).bind(&intent).bind(binding.as_slice()).execute(&mut **tx).await?;
    Ok(CreatureGroundReceipt {
        transaction_id: transaction,
        event_id: event,
        already_committed: false,
        binding,
    })
}

impl CreatureGroundCause {
    /// Only current native typed content and the exact actual Creature schedule
    /// can construct a cause. Detached authoring bodies/digests are not inputs.
    fn from_current_native_proposal(
        runtime: &crate::foundation::ChannelRuntimeV1,
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        compiled: &crate::content::CompiledFirstProductionContent,
        native: &crate::content::native_gameplay::NativeGameplayState,
        creature: &crate::content::ProjectV2DefinitionRef,
        proposal: &crate::ai_think::profile_schedule::ProfileAbilityProposal,
    ) -> Result<Self> {
        use crate::content::{ProjectV2AuthoringProfileData, ProjectV2Family};
        let actor = proposal.issuer;
        let scope =
            crate::foundation::RuntimeScopeRefV1::channel(actor.world_id(), actor.channel_id());
        if !fence.accepts_stamp(stamp)
            || !fence.is_current_for_scope(scope, actor.scope_generation())
            || !runtime
                .owner_fence()
                .map_err(|_| SpellItemError::Rejected("Creature owner unavailable"))?
                .accepts_stamp(stamp)
            || native.source_digest() != runtime.content_pin().server_artifact_digest()
            || compiled.server_digest() != runtime.content_pin().server_artifact_digest()
            || compiled.client_digest() != runtime.content_pin().client_artifact_digest()
            || creature.family != ProjectV2Family::Creature
            || !runtime.matches_live_creature_identity(actor, creature.key.as_bytes())
        {
            return Err(SpellItemError::Rejected("Creature native source owner/pin"));
        }
        let snapshot = runtime
            .companion_snapshot(actor)
            .map_err(|_| SpellItemError::Rejected("Creature source generation"))?;
        if snapshot.state.policy.definition_key != creature.key
            || snapshot.state.policy.definition_revision != creature.revision
        {
            return Err(SpellItemError::Rejected(
                "Creature source exact physical definition",
            ));
        }
        let body = native
            .source_item_ability(creature, &proposal.ability)
            .ok_or(SpellItemError::Rejected(
                "Creature immutable source Item membership",
            ))?;
        let row = native
            .creature_profiles()
            .records
            .iter()
            .find(|r| r.profile.target == *creature)
            .ok_or(SpellItemError::Rejected("Creature source profile absent"))?;
        let Some(behavior) = &row.behavior else {
            return Err(SpellItemError::Rejected("Creature source schedule absent"));
        };
        let ProjectV2AuthoringProfileData::Behavior(behavior) = &behavior.data else {
            return Err(SpellItemError::Rejected("Creature source schedule kind"));
        };
        let list = match proposal.list {
            crate::ai_think::profile_schedule::ScheduleList::Attack => &behavior.attacks,
            crate::ai_think::profile_schedule::ScheduleList::Defence => &behavior.defenses,
        };
        if list
            .get(proposal.entry_index)
            .is_none_or(|s| s.ability != proposal.ability)
        {
            return Err(SpellItemError::Rejected(
                "Creature exact AI source schedule entry",
            ));
        }
        let ProjectV2AuthoringProfileData::Ability(ability) = &body.data else {
            return Err(SpellItemError::Rejected("Creature source Ability kind"));
        };
        let details = ability
            .details
            .as_ref()
            .ok_or(SpellItemError::Rejected("Creature source Ability details"))?;
        let mut source_items = Vec::new();
        for effect in &details.effects {
            if let crate::content::ProjectV2AbilityEffect::Inline(effect) = effect {
                match &effect.operation {
                    crate::content::ProjectV2InlineEffectOperation::CreateItem { item } => {
                        source_items.push(TypedDefinitionRef {
                            family: "Item".into(),
                            production_key: item.key.clone(),
                            revision_ref: item.revision.clone(),
                        })
                    }
                    crate::content::ProjectV2InlineEffectOperation::RemoveItems {
                        items, ..
                    } => {
                        for item in items {
                            source_items.push(TypedDefinitionRef {
                                family: "Item".into(),
                                production_key: item.key.clone(),
                                revision_ref: item.revision.clone(),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        let current_source_position = runtime
            .read_actor_position(actor)
            .map_err(|_| SpellItemError::Rejected("Creature source position"))?;
        let current_target_position = runtime
            .read_actor_position(proposal.target)
            .map_err(|_| SpellItemError::Rejected("Creature target position"))?;
        let cast: [u8; 32] = Sha256::digest(
            format!("{proposal:?}:{current_source_position:?}:{current_target_position:?}")
                .as_bytes(),
        )
        .into();
        if proposal.range_tiles != details.range_tiles
            || proposal.magnitude != list[proposal.entry_index].magnitude
        {
            return Err(SpellItemError::Rejected("Creature AI source facts changed"));
        }
        let hex = |v: &[u8]| v.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let prefix = format!("ai-profile:{}:", hex(&actor.placement_identity()));
        let tail = proposal
            .occurrence
            .id()
            .as_str()
            .strip_prefix(&prefix)
            .ok_or(SpellItemError::Rejected("Creature AI occurrence caster"))?;
        let (sequence, suffix) = tail
            .split_once(':')
            .ok_or(SpellItemError::Rejected("Creature AI occurrence shape"))?;
        let parsed = sequence
            .parse::<u64>()
            .map_err(|_| SpellItemError::Rejected("Creature AI sequence"))?;
        let label = match proposal.list {
            crate::ai_think::profile_schedule::ScheduleList::Attack => "attack",
            crate::ai_think::profile_schedule::ScheduleList::Defence => "defence",
        };
        if sequence != parsed.to_string() || suffix != format!("{label}:{}", proposal.entry_index) {
            return Err(SpellItemError::Rejected(
                "Creature AI source entry occurrence",
            ));
        }
        let bytes = serde_json::to_vec(body)
            .map_err(|_| SpellItemError::Rejected("Creature source body encoding"))?;
        Ok(Self {
            world: *actor.world_id().as_bytes(),
            channel: *actor.channel_id().as_bytes(),
            scope_generation: actor.scope_generation().get(),
            source_items,
            placement: actor.placement_identity(),
            generation: actor.actor_local_generation(),
            creature: TypedDefinitionRef {
                family: "Creature".into(),
                production_key: creature.key.clone(),
                revision_ref: creature.revision.clone(),
            },
            ability: TypedDefinitionRef {
                family: "Ability".into(),
                production_key: proposal.ability.key.clone(),
                revision_ref: proposal.ability.revision.clone(),
            },
            occurrence: proposal.occurrence.id().as_str().to_owned(),
            content: native.source_digest(),
            map: runtime.content_pin().map_revision_digest(),
            frame: runtime.content_pin().frame_binding_digest(),
            body: Sha256::digest(bytes).into(),
            cast,
        })
    }
}

/// Retry the same durable source occurrence before re-reading/mutating its old
/// tile. Removing an Item cannot make the receipt disappear on A→B→retry A.
/// This grant is read-only: a prior receipt never bypasses the current guard.
async fn reconcile_creature_ground_in_transaction<F>(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    cause: &CreatureGroundCause,
    current_guard: F,
) -> Result<Option<CreatureGroundReceipt>>
where
    F: FnOnce() -> Result<()>,
{
    check_scope_transaction(tx, authority).await?;
    current_guard()?;
    if authority.world_bytes() != cause.world
        || authority.channel_bytes() != cause.channel
        || authority.generation() != cause.scope_generation
    {
        return Err(SpellItemError::Rejected("Creature replay foreign scope"));
    }
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || ':' || $2 || ':' || $3 || ':' || $4,45))")
        .bind(cause.placement.as_slice()).bind(cause.generation.to_string()).bind(&cause.ability.production_key).bind(&cause.occurrence).execute(&mut **tx).await?;
    let prior=sqlx::query("SELECT transaction_id::text,event_id::text,binding,catalog_digest,creature_definition_key,creature_definition_revision,creature_map_digest,creature_frame_digest,creature_source_body_digest,creature_cast_digest FROM game_spell_item_receipts WHERE cause_kind=1 AND world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND ownership_generation=$3::text::numeric(20,0) AND creature_placement=$4 AND creature_actor_generation=$5::text::numeric(20,0) AND spell_production_key=$6 AND spell_revision=$7 AND creature_occurrence=$8")
        .bind(authority.world_bytes().as_slice()).bind(authority.channel_bytes().as_slice()).bind(authority.generation().to_string()).bind(cause.placement.as_slice()).bind(cause.generation.to_string()).bind(&cause.ability.production_key).bind(&cause.ability.revision_ref).bind(&cause.occurrence).fetch_optional(&mut **tx).await?;
    let Some(row) = prior else {
        return Ok(None);
    };
    for (column, expected) in [
        ("catalog_digest", cause.content),
        ("creature_map_digest", cause.map),
        ("creature_frame_digest", cause.frame),
        ("creature_source_body_digest", cause.body),
        ("creature_cast_digest", cause.cast),
    ] {
        if row.try_get::<Vec<u8>, _>(column)?.as_slice() != expected {
            return Err(SpellItemError::Rejected(
                "conflicting Creature exact source replay",
            ));
        }
    }
    if row.try_get::<String, _>("creature_definition_key")? != cause.creature.production_key
        || row.try_get::<String, _>("creature_definition_revision")? != cause.creature.revision_ref
    {
        return Err(SpellItemError::Rejected(
            "conflicting Creature source definition replay",
        ));
    }
    let binding: Vec<u8> = row.try_get("binding")?;
    Ok(Some(CreatureGroundReceipt {
        transaction_id: uuid(&row.try_get::<String, _>("transaction_id")?)?,
        event_id: uuid(&row.try_get::<String, _>("event_id")?)?,
        already_committed: true,
        binding: binding
            .try_into()
            .map_err(|_| SpellItemError::Rejected("Creature receipt binding size"))?,
    }))
}

/// Actual source CreateItem footprint joins qualified Map facts, locked durable
/// Item reads and the same immutable native field-chain recipe used by Player casts.
#[allow(clippy::too_many_arguments)]
async fn prepare_creature_source_fields_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    runtime: &crate::foundation::ChannelRuntimeV1,
    room: &crate::content::QualifiedNativeEntryRoom,
    objects: &crate::world_runtime::LocalObjectRuntime,
    compiled: &crate::content::CompiledFirstProductionContent,
    native: &crate::content::native_gameplay::NativeGameplayState,
    cause: &CreatureGroundCause,
    proposal: &crate::ai_think::profile_schedule::ProfileAbilityProposal,
) -> Result<PreparedCreatureGroundOperations> {
    use crate::content::{
        ProjectV2AbilityEffect, ProjectV2AuthoringProfileData, ProjectV2InlineEffectOperation,
    };
    use crate::creature_attack_geometry as geometry;
    use crate::foundation::{MovementFacing, MovementLocalPosition as Pos};
    check_scope_transaction(tx, authority).await?;
    if authority.world_bytes() != cause.world
        || authority.channel_bytes() != cause.channel
        || authority.generation() != cause.scope_generation
        || cause.placement != proposal.issuer.placement_identity()
        || cause.generation != proposal.issuer.actor_local_generation()
        || cause.occurrence != proposal.occurrence.id().as_str()
        || native.source_digest() != runtime.content_pin().server_artifact_digest()
        || compiled.server_digest() != runtime.content_pin().server_artifact_digest()
        || cause.map != runtime.content_pin().map_revision_digest()
        || cause.frame != runtime.content_pin().frame_binding_digest()
    {
        return Err(SpellItemError::Rejected(
            "Creature source fields current binding",
        ));
    }
    let source = runtime
        .read_actor_position(proposal.issuer)
        .map_err(|_| SpellItemError::Rejected("Creature current position"))?;
    let target = runtime
        .read_actor_position(proposal.target)
        .map_err(|_| SpellItemError::Rejected("Creature target position"))?;
    let fp = source.position();
    let tp = target.position();
    if fp.floor != tp.floor {
        return Err(SpellItemError::Rejected("Creature field target floor"));
    }
    let creature = crate::content::ProjectV2DefinitionRef {
        family: crate::content::ProjectV2Family::Creature,
        key: cause.creature.production_key.clone(),
        revision: cause.creature.revision_ref.clone(),
    };
    let profile = native
        .source_item_ability(&creature, &proposal.ability)
        .ok_or(SpellItemError::Rejected("Creature field exact membership"))?;
    let bytes = serde_json::to_vec(profile)
        .map_err(|_| SpellItemError::Rejected("Creature field body encoding"))?;
    if <[u8; 32]>::from(Sha256::digest(bytes)) != cause.body {
        return Err(SpellItemError::Rejected("Creature field body changed"));
    }
    let ProjectV2AuthoringProfileData::Ability(ability) = &profile.data else {
        return Err(SpellItemError::Rejected("Creature field Ability kind"));
    };
    let details = ability
        .details
        .as_ref()
        .ok_or(SpellItemError::Rejected("Creature field details"))?;
    if details.effects.len() != 1 || details.windup.is_some() {
        return Err(SpellItemError::Rejected(
            "Creature field unsupported compound/delayed body",
        ));
    }
    let ProjectV2AbilityEffect::Inline(effect) = &details.effects[0] else {
        return Err(SpellItemError::Rejected("Creature field inline body"));
    };
    let ProjectV2InlineEffectOperation::CreateItem { item } = &effect.operation else {
        return Err(SpellItemError::Rejected("Creature field create body"));
    };
    if proposal.range_tiles != details.range_tiles
        || i64::from(fp.x)
            .abs_diff(i64::from(tp.x))
            .max(i64::from(fp.y).abs_diff(i64::from(tp.y)))
            > u64::from(details.range_tiles)
    {
        return Err(SpellItemError::Rejected("Creature field range"));
    }
    let center = if details.needs_target {
        tp
    } else if details.needs_direction {
        let (dx, dy) = match source
            .facing()
            .ok_or(SpellItemError::Rejected("Creature field facing"))?
        {
            MovementFacing::North => (0, -1),
            MovementFacing::East => (1, 0),
            MovementFacing::South => (0, 1),
            MovementFacing::West => (-1, 0),
        };
        Pos {
            x: fp
                .x
                .checked_add(dx)
                .ok_or(SpellItemError::Rejected("Creature field x overflow"))?,
            y: fp
                .y
                .checked_add(dy)
                .ok_or(SpellItemError::Rejected("Creature field y overflow"))?,
            floor: fp.floor,
        }
    } else {
        fp
    };
    let offsets = if let Some(area) = &details.area {
        let diagonal = matches!(area,crate::content::ProjectV2AbilityArea::Matrix{diagonal,..}if !diagonal.is_empty());
        geometry::offsets(
            area,
            geometry::direction(
                i64::from(center.x) - i64::from(fp.x),
                i64::from(center.y) - i64::from(fp.y),
                diagonal,
            ),
        )
        .map_err(|_| SpellItemError::Rejected("Creature field source geometry"))?
    } else {
        vec![(0, 0)]
    };
    if offsets.len() > 256 {
        return Err(SpellItemError::Rejected("Creature field footprint"));
    }
    // Reuse the actual ordinary cast sight kernel and its exempt endpoint rule.
    for (ray, exempt) in crate::spell::world_execution::source_sight_steps(fp, center)
        .map_err(|_| SpellItemError::Rejected("Creature field sight geometry"))?
    {
        if exempt {
            continue;
        }
        let tile = crate::spell::world_execution::qualified_creature_source_tile_in_transaction(
            tx, authority, room, runtime, objects, ray,
        )
        .await
        .map_err(|_| SpellItemError::Rejected("Creature field ray Map qualification"))?;
        let address = SpellGroundTarget::for_native_tile_read(room, runtime, ray)
            .map_err(SpellItemError::Rejected)?;
        let items = read_creature_scope_tile_in_transaction(tx, authority, &address).await?;
        if tile.block_projectile() || items.items.iter().any(|i| i.blocks_projectile) {
            return Err(SpellItemError::Rejected("Creature field ray blocked"));
        }
    }
    let policy = native
        .item_policy(&item.key, &item.revision)
        .ok_or(SpellItemError::Rejected(
            "Creature field Item source absent",
        ))?;
    if !crate::spell::world_items_execution::source_magic_field(policy)? {
        return Err(SpellItemError::Rejected(
            "Creature created Item must be source field",
        ));
    }
    let attrs = &policy.record().attributes;
    let movement = attrs
        .blocks_movement
        .ok_or(SpellItemError::Rejected("Creature field movement source"))?;
    let projectile = attrs
        .blocks_projectile
        .ok_or(SpellItemError::Rejected("Creature field projectile source"))?;
    let definition = crate::spell::world_items_execution::qualify_source_field_chain(
        native,
        QualifiedItemDefinition::from_native_policy(policy).map_err(SpellItemError::Rejected)?,
    )?;
    let mut operations = Vec::new();
    let mut positions = std::collections::BTreeSet::new();
    'footprint: for (ordinal, (dx, dy)) in offsets.into_iter().enumerate() {
        let pos = Pos {
            x: center
                .x
                .checked_add(dx)
                .ok_or(SpellItemError::Rejected("Creature field x"))?,
            y: center
                .y
                .checked_add(dy)
                .ok_or(SpellItemError::Rejected("Creature field y"))?,
            floor: center.floor,
        };
        if !positions.insert((pos.x, pos.y, pos.floor)) {
            return Err(SpellItemError::Rejected("Creature field duplicate tile"));
        }
        let tile = crate::spell::world_execution::qualified_creature_source_tile_in_transaction(
            tx, authority, room, runtime, objects, pos,
        )
        .await
        .map_err(|_| SpellItemError::Rejected("Creature field tile qualification"))?;
        if !tile.ground_present()
            || tile.block_solid()
            || tile.floor_change()
            || tile.protection_zone()
        {
            continue;
        }
        for (ray, exempt) in crate::spell::world_execution::source_sight_steps(center, pos)
            .map_err(|_| SpellItemError::Rejected("Creature field area sight geometry"))?
        {
            if exempt {
                continue;
            }
            let sight =
                crate::spell::world_execution::qualified_creature_source_tile_in_transaction(
                    tx, authority, room, runtime, objects, ray,
                )
                .await
                .map_err(|_| SpellItemError::Rejected("Creature area ray Map qualification"))?;
            let ray_address = SpellGroundTarget::for_native_tile_read(room, runtime, ray)
                .map_err(SpellItemError::Rejected)?;
            let blockers =
                read_creature_scope_tile_in_transaction(tx, authority, &ray_address).await?;
            if sight.block_projectile() || blockers.items.iter().any(|i| i.blocks_projectile) {
                continue 'footprint;
            }
        }
        let address = SpellGroundTarget::for_native_tile_read(room, runtime, pos)
            .map_err(SpellItemError::Rejected)?;
        let existing = read_creature_scope_tile_in_transaction(tx, authority, &address).await?;
        for current in existing.items {
            let current_policy = native
                .item_policy(
                    &current.definition.production_key,
                    &current.definition.revision_ref,
                )
                .ok_or(SpellItemError::Rejected("Creature existing Item policy"))?;
            if crate::spell::world_items_execution::source_magic_field(current_policy)? {
                if current_policy.record().attributes.field_replaceable != Some(true) {
                    return Err(SpellItemError::Rejected(
                        "Creature field replacement policy",
                    ));
                }
                let removal = prepare_creature_scope_removal_in_transaction(
                    tx,
                    authority,
                    &address,
                    current.item_instance_id,
                )
                .await?;
                operations.push(SpellItemOperation::RemoveGround(removal));
                break;
            }
        }
        let mut hash = Sha256::new();
        hash.update(b"oteryn:creature-source-field:v1");
        hash.update(cause.world);
        hash.update(cause.channel);
        hash.update(cause.scope_generation.to_be_bytes());
        hash.update(cause.placement);
        hash.update(cause.generation.to_be_bytes());
        hash.update(cause.occurrence.as_bytes());
        hash.update(
            u64::try_from(ordinal)
                .map_err(|_| SpellItemError::Rejected("Creature field ordinal"))?
                .to_be_bytes(),
        );
        hash.update(cause.body);
        let digest = hash.finalize();
        let mut identity = [0u8; 16];
        identity.copy_from_slice(&digest[..16]);
        identity[6] = (identity[6] & 15) | 0x70;
        identity[8] = (identity[8] & 63) | 0x80;
        operations.push(SpellItemOperation::MintGround {
            item_instance_id: identity,
            definition: definition.clone(),
            quantity: 1,
            placement: address.placement(Vec::new()),
            lifetime_millis: None,
            blocks_movement: movement,
            blocks_projectile: projectile,
            description: None,
        });
    }
    Ok(PreparedCreatureGroundOperations {
        placement: cause.placement,
        generation: cause.generation,
        body: cause.body,
        cast: cause.cast,
        operations,
    })
}

#[allow(clippy::too_many_arguments)]
async fn prepare_creature_source_removals_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    runtime: &crate::foundation::ChannelRuntimeV1,
    room: &crate::content::QualifiedNativeEntryRoom,
    objects: &crate::world_runtime::LocalObjectRuntime,
    _compiled: &crate::content::CompiledFirstProductionContent,
    native: &crate::content::native_gameplay::NativeGameplayState,
    cause: &CreatureGroundCause,
    proposal: &crate::ai_think::profile_schedule::ProfileAbilityProposal,
) -> Result<PreparedCreatureGroundOperations> {
    use crate::content::{
        ProjectV2AbilityEffect, ProjectV2AuthoringProfileData, ProjectV2InlineEffectOperation,
        ProjectV2RemoveItemsSelection,
    };
    check_scope_transaction(tx, authority).await?;
    let creature = crate::content::ProjectV2DefinitionRef {
        family: crate::content::ProjectV2Family::Creature,
        key: cause.creature.production_key.clone(),
        revision: cause.creature.revision_ref.clone(),
    };
    let profile = native
        .source_item_ability(&creature, &proposal.ability)
        .ok_or(SpellItemError::Rejected("Creature removal source member"))?;
    let ProjectV2AuthoringProfileData::Ability(ability) = &profile.data else {
        return Err(SpellItemError::Rejected("Creature removal Ability kind"));
    };
    let details = ability
        .details
        .as_ref()
        .ok_or(SpellItemError::Rejected("Creature removal details"))?;
    if details.effects.len() != 1 || details.windup.is_some() {
        return Err(SpellItemError::Rejected(
            "Creature removal compound/delayed body",
        ));
    }
    let ProjectV2AbilityEffect::Inline(effect) = &details.effects[0] else {
        return Err(SpellItemError::Rejected("Creature removal inline body"));
    };
    let ProjectV2InlineEffectOperation::RemoveItems { items, selection } = &effect.operation else {
        return Err(SpellItemError::Rejected("Creature removal operation"));
    };
    if *selection == ProjectV2RemoveItemsSelection::TopItemFirstTile {
        return prepare_creature_source_destroy_in_transaction(
            tx, authority, runtime, room, objects, cause, proposal, items,
        )
        .await;
    }
    if items.iter().map(|r| r.key.as_str()).collect::<Vec<_>>()
        != ["oteryn:item.tibia.i2130", "oteryn:item.tibia.i2129"]
    {
        return Err(SpellItemError::Rejected(
            "source Anomaly ordered Item identities",
        ));
    }
    let current = runtime
        .read_actor_position(proposal.issuer)
        .map_err(|_| SpellItemError::Rejected("Creature removal current caster"))?;
    let fp = current.position();
    let mut operations = Vec::new();
    // Actual source x-major/y-minor, one first-listed Item per current tile.
    for dx in -1i32..=1 {
        for dy in -1i32..=1 {
            let pos = crate::foundation::MovementLocalPosition {
                x: fp
                    .x
                    .checked_add(dx)
                    .ok_or(SpellItemError::Rejected("Creature removal x"))?,
                y: fp
                    .y
                    .checked_add(dy)
                    .ok_or(SpellItemError::Rejected("Creature removal y"))?,
                floor: fp.floor,
            };
            let _tile =
                crate::spell::world_execution::qualified_creature_source_tile_in_transaction(
                    tx, authority, room, runtime, objects, pos,
                )
                .await
                .map_err(|_| {
                    SpellItemError::Rejected("Creature removal source tile qualification")
                })?;
            let address = SpellGroundTarget::for_native_tile_read(room, runtime, pos)
                .map_err(SpellItemError::Rejected)?;
            let existing = read_creature_scope_tile_in_transaction(tx, authority, &address).await?;
            let selected = items.iter().find_map(|wanted| {
                existing.items.iter().find(|i| {
                    i.definition.family == "Item"
                        && i.definition.production_key == wanted.key
                        && i.definition.revision_ref == wanted.revision
                })
            });
            if let Some(selected) = selected {
                operations.push(SpellItemOperation::RemoveGround(
                    prepare_creature_scope_removal_in_transaction(
                        tx,
                        authority,
                        &address,
                        selected.item_instance_id,
                    )
                    .await?,
                ));
            }
        }
    }
    Ok(PreparedCreatureGroundOperations {
        placement: cause.placement,
        generation: cause.generation,
        body: cause.body,
        cast: cause.cast,
        operations,
    })
}

/// The actual committed-cause bridge. Native owner lock, LocalObject borrow and
/// independently fenced SQL transaction remain held throughout this call.
/// Receipt reconciliation precedes re-reading the original mutable Ground.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn execute_creature_source_items_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    runtime: &crate::foundation::ChannelRuntimeV1,
    fence: &crate::foundation::ScopeRuntimeFence,
    stamp: crate::foundation::RuntimeWorkStamp,
    room: &crate::content::QualifiedNativeEntryRoom,
    objects: &crate::world_runtime::LocalObjectRuntime,
    compiled: &crate::content::CompiledFirstProductionContent,
    native: &crate::content::native_gameplay::NativeGameplayState,
    creature: &crate::content::ProjectV2DefinitionRef,
    proposal: &crate::ai_think::profile_schedule::ProfileAbilityProposal,
) -> Result<CreatureGroundReceipt> {
    let cause = CreatureGroundCause::from_current_native_proposal(
        runtime, fence, stamp, compiled, native, creature, proposal,
    )?;
    let guard = || {
        let current = CreatureGroundCause::from_current_native_proposal(
            runtime, fence, stamp, compiled, native, creature, proposal,
        )?;
        if current != cause {
            return Err(SpellItemError::Rejected(
                "Creature source current cause changed",
            ));
        }
        Ok(())
    };
    if let Some(prior) =
        reconcile_creature_ground_in_transaction(tx, authority, &cause, guard).await?
    {
        return Ok(prior);
    }
    let source = native
        .source_item_ability(creature, &proposal.ability)
        .ok_or(SpellItemError::Rejected("Creature source exact member"))?;
    let crate::content::ProjectV2AuthoringProfileData::Ability(ability) = &source.data else {
        return Err(SpellItemError::Rejected("Creature source Ability kind"));
    };
    let details = ability
        .details
        .as_ref()
        .ok_or(SpellItemError::Rejected("Creature source details absent"))?;
    let Some(crate::content::ProjectV2AbilityEffect::Inline(effect)) = details.effects.first()
    else {
        return Err(SpellItemError::Rejected(
            "Creature source inline body absent",
        ));
    };
    let prepared = match &effect.operation {
        crate::content::ProjectV2InlineEffectOperation::CreateItem { .. } => {
            prepare_creature_source_fields_in_transaction(
                tx, authority, runtime, room, objects, compiled, native, &cause, proposal,
            )
            .await?
        }
        crate::content::ProjectV2InlineEffectOperation::RemoveItems { .. } => {
            prepare_creature_source_removals_in_transaction(
                tx, authority, runtime, room, objects, compiled, native, &cause, proposal,
            )
            .await?
        }
        _ => {
            return Err(SpellItemError::Rejected(
                "Creature source physical operation unsupported",
            ));
        }
    };
    let identity = |tag: &[u8]| {
        let mut h = Sha256::new();
        h.update(tag);
        h.update(cause.world);
        h.update(cause.channel);
        h.update(cause.scope_generation.to_be_bytes());
        h.update(cause.placement);
        h.update(cause.generation.to_be_bytes());
        h.update(cause.ability.production_key.as_bytes());
        h.update(cause.ability.revision_ref.as_bytes());
        h.update(cause.occurrence.as_bytes());
        h.update(cause.cast);
        let digest = h.finalize();
        let mut id = [0u8; 16];
        id.copy_from_slice(&digest[..16]);
        id[6] = (id[6] & 15) | 0x70;
        id[8] = (id[8] & 63) | 0x80;
        id
    };
    let transaction = identity(b"oteryn:creature-source-item-transaction:v1");
    let event = identity(b"oteryn:creature-source-item-event:v1");
    apply_creature_ground_in_transaction(
        tx,
        authority,
        &cause,
        transaction,
        event,
        &prepared,
        || {
            let current = CreatureGroundCause::from_current_native_proposal(
                runtime, fence, stamp, compiled, native, creature, proposal,
            )?;
            if current != cause {
                return Err(SpellItemError::Rejected(
                    "Creature source owner changed before physical write",
                ));
            }
            Ok(())
        },
    )
    .await
}

/// A real existing AI timer proposal captured under the current owner lock.
/// Native content membership is immutable; this receipt is not Item authority.
#[derive(Debug)]
pub(crate) struct ScheduledCreatureSourceItems {
    creature: crate::content::ProjectV2DefinitionRef,
    proposal: crate::ai_think::profile_schedule::ProfileAbilityProposal,
    issued: crate::foundation::RuntimeWorkStamp,
    source: crate::foundation::MovementPositionSnapshot,
    target: crate::foundation::MovementPositionSnapshot,
    content: [u8; 32],
    map: [u8; 32],
    frame: [u8; 32],
    body: [u8; 32],
}
impl ScheduledCreatureSourceItems {
    pub(crate) fn prepare(
        runtime: &crate::foundation::ChannelRuntimeV1,
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        native: &crate::content::native_gameplay::NativeGameplayState,
        creature: &crate::content::ProjectV2DefinitionRef,
        proposal: &crate::ai_think::profile_schedule::ProfileAbilityProposal,
    ) -> Result<Self> {
        let actor = proposal.issuer;
        let scope =
            crate::foundation::RuntimeScopeRefV1::channel(actor.world_id(), actor.channel_id());
        if native.source_digest() != runtime.content_pin().server_artifact_digest()
            || !fence.accepts_stamp(stamp)
            || !fence.is_current_for_scope(scope, actor.scope_generation())
            || !runtime
                .owner_fence()
                .map_err(|_| SpellItemError::Rejected("Creature scheduled owner unavailable"))?
                .accepts_stamp(stamp)
            || !runtime.matches_live_creature_identity(actor, creature.key.as_bytes())
        {
            return Err(SpellItemError::Rejected("Creature scheduled source owner"));
        }
        let body = native
            .source_item_ability(creature, &proposal.ability)
            .ok_or(SpellItemError::Rejected(
                "Creature scheduled immutable membership",
            ))?;
        let bytes = serde_json::to_vec(body)
            .map_err(|_| SpellItemError::Rejected("Creature scheduled body encoding"))?;
        Ok(Self {
            creature: creature.clone(),
            proposal: proposal.clone(),
            issued: stamp,
            source: runtime
                .read_actor_position(actor)
                .map_err(|_| SpellItemError::Rejected("Creature scheduled source position"))?,
            target: runtime
                .read_actor_position(proposal.target)
                .map_err(|_| SpellItemError::Rejected("Creature scheduled target position"))?,
            content: native.source_digest(),
            map: runtime.content_pin().map_revision_digest(),
            frame: runtime.content_pin().frame_binding_digest(),
            body: Sha256::digest(bytes).into(),
        })
    }
}
/// Actual current scope SQL consumer for the timer-produced sealed item cast.
/// Whole operation preparation and same-source receipt replay are delegated to
/// the one shared physical owner above; no Player or cost authority is invented.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn execute_scheduled_creature_source_items_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    runtime: &crate::foundation::ChannelRuntimeV1,
    fence: &crate::foundation::ScopeRuntimeFence,
    stamp: crate::foundation::RuntimeWorkStamp,
    room: &crate::content::QualifiedNativeEntryRoom,
    objects: &crate::world_runtime::LocalObjectRuntime,
    compiled: &crate::content::CompiledFirstProductionContent,
    native: &crate::content::native_gameplay::NativeGameplayState,
    scheduled: &ScheduledCreatureSourceItems,
) -> Result<CreatureGroundReceipt> {
    if !fence.accepts_stamp(scheduled.issued)
        || !fence.accepts_stamp(stamp)
        || scheduled.content != runtime.content_pin().server_artifact_digest()
        || scheduled.map != runtime.content_pin().map_revision_digest()
        || scheduled.frame != runtime.content_pin().frame_binding_digest()
        || runtime
            .read_actor_position(scheduled.proposal.issuer)
            .map_err(|_| SpellItemError::Rejected("Creature scheduled stale caster"))?
            != scheduled.source
        || runtime
            .read_actor_position(scheduled.proposal.target)
            .map_err(|_| SpellItemError::Rejected("Creature scheduled stale target"))?
            != scheduled.target
    {
        return Err(SpellItemError::Rejected(
            "Creature scheduled current source binding",
        ));
    }
    let body = native
        .source_item_ability(&scheduled.creature, &scheduled.proposal.ability)
        .ok_or(SpellItemError::Rejected(
            "Creature scheduled current exact member",
        ))?;
    let bytes = serde_json::to_vec(body)
        .map_err(|_| SpellItemError::Rejected("Creature scheduled body encoding"))?;
    if <[u8; 32]>::from(Sha256::digest(bytes)) != scheduled.body {
        return Err(SpellItemError::Rejected(
            "Creature scheduled source body changed",
        ));
    }
    execute_creature_source_items_in_transaction(
        tx,
        authority,
        runtime,
        fence,
        stamp,
        room,
        objects,
        compiled,
        native,
        &scheduled.creature,
        &scheduled.proposal,
    )
    .await
}

// PROJECT_CONSERVATIVE_TOP_REFUSES_OCCUPIED_OR_UNKNOWN_LAYERS.
// Cached Canary47 getTopVisibleThing checks visible actors, then non-lookThrough
// DownItems, then TopItems. The four admitted field types have neither top-layer
// nor ignore_look flags in the pinned DAT. Refuse every actor (including invisible)
// and every foreign/static layer: never infer visibility from filtered matches.
fn conservative_destroy_stack_is_known(
    static_layers_known_empty: bool,
    occupied: bool,
    listed: &[crate::spell::native_items::NativeItemRef],
    rows: &[crate::durability::spell_items_abi::DurableTileItem],
) -> Result<bool> {
    if rows.iter().any(|row| row.top_down_ordinal == 0)
        || rows
            .windows(2)
            .any(|pair| pair[0].top_down_ordinal <= pair[1].top_down_ordinal)
    {
        return Err(SpellItemError::Rejected(
            "Destroy current StackOwner ordering",
        ));
    }
    Ok(static_layers_known_empty
        && !occupied
        && rows.iter().all(|row| {
            row.definition.family == "Item"
                && listed.iter().any(|wanted| {
                    wanted.key == row.definition.production_key
                        && wanted.revision == row.definition.revision_ref
                })
        }))
}

async fn creature_scope_item_tags_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    item: &crate::durability::spell_items_abi::DurableTileItem,
) -> Result<(bool, bool)> {
    check_scope_transaction(tx, authority).await?;
    let row = sqlx::query("SELECT nm.source_attributes, EXISTS(SELECT 1 FROM game_spell_item_lines sl WHERE sl.transaction_id=i.minted_transaction_id AND sl.item_instance_id=i.item_instance_id AND sl.operation_kind=1) AS spell_origin FROM game_item_instances i LEFT JOIN game_native_map_item_receipts nm ON nm.transaction_id=i.minted_transaction_id AND nm.item_instance_id=i.item_instance_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.state_revision=$3::text::numeric(20,0) AND i.definition_family=$4 AND i.definition_production_key=$5 AND i.definition_revision_ref=$6 FOR UPDATE OF i")
        .bind(item.item_instance_id.as_slice()).bind(authority.world_bytes().as_slice())
        .bind(item.state_revision.to_string()).bind(&item.definition.family)
        .bind(&item.definition.production_key).bind(&item.definition.revision_ref)
        .fetch_optional(&mut **tx).await?.ok_or(SpellItemError::Rejected("Destroy current Item changed"))?;
    if let Some(attributes) = row.try_get::<Option<serde_json::Value>, _>("source_attributes")? {
        return source_map_protection_tags(&attributes);
    }
    if row.try_get::<bool, _>("spell_origin")? {
        return Ok((false, false));
    }
    Err(SpellItemError::Rejected(
        "Destroy Item protection provenance unknown",
    ))
}

#[allow(clippy::too_many_arguments)]
async fn prepare_creature_source_destroy_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    runtime: &crate::foundation::ChannelRuntimeV1,
    room: &crate::content::QualifiedNativeEntryRoom,
    objects: &crate::world_runtime::LocalObjectRuntime,
    cause: &CreatureGroundCause,
    proposal: &crate::ai_think::profile_schedule::ProfileAbilityProposal,
    items: &[crate::content::ProjectV2DefinitionRef],
) -> Result<PreparedCreatureGroundOperations> {
    use crate::spell::native_items::{
        ItemOperation, NativeItemRef, SourceRemovalTile, WorldItemFacts, plan_source_item_removals,
    };
    let listed: Vec<_> = items
        .iter()
        .map(|item| NativeItemRef {
            key: item.key.clone(),
            revision: item.revision.clone(),
        })
        .collect();
    // This helper is reachable only after the sealed source body/current cast guard.
    if listed.iter().map(|r| r.key.as_str()).collect::<Vec<_>>()
        != [
            "oteryn:item.tibia.i10181",
            "oteryn:item.tibia.i2128",
            "oteryn:item.tibia.i10182",
            "oteryn:item.tibia.i2130",
        ]
    {
        return Err(SpellItemError::Rejected(
            "Destroy ordered source identities",
        ));
    }
    let current = runtime
        .read_actor_position(proposal.issuer)
        .map_err(|_| SpellItemError::Rejected("Destroy current caster"))?;
    let census = runtime
        .positioned_actor_census()
        .map_err(|_| SpellItemError::Rejected("Destroy current actor census"))?;
    let origin = current.position();
    let mut witnesses = Vec::new();
    let mut addresses = Vec::new();
    let mut snapshots = Vec::new();
    witnesses
        .try_reserve(25)
        .map_err(|_| SpellItemError::Rejected("Destroy witness allocation"))?;
    addresses
        .try_reserve(25)
        .map_err(|_| SpellItemError::Rejected("Destroy address allocation"))?;
    snapshots
        .try_reserve(25)
        .map_err(|_| SpellItemError::Rejected("Destroy snapshot allocation"))?;
    for dx in -2i8..=2 {
        for dy in -2i8..=2 {
            let pos = crate::foundation::MovementLocalPosition {
                x: origin
                    .x
                    .checked_add(i32::from(dx))
                    .ok_or(SpellItemError::Rejected("Destroy x"))?,
                y: origin
                    .y
                    .checked_add(i32::from(dy))
                    .ok_or(SpellItemError::Rejected("Destroy y"))?,
                floor: origin.floor,
            };
            let tile =
                crate::spell::world_execution::qualified_creature_source_tile_in_transaction(
                    tx, authority, room, runtime, objects, pos,
                )
                .await
                .map_err(|_| SpellItemError::Rejected("Destroy current SourceMap tile"))?;
            let address = SpellGroundTarget::for_native_tile_read(room, runtime, pos)
                .map_err(SpellItemError::Rejected)?;
            let snapshot = read_creature_scope_tile_in_transaction(tx, authority, &address).await?;
            let occupied = census
                .iter()
                .any(|(_, snapshot, _)| snapshot.position() == pos)
                || runtime
                    .position_occupied_by_other(proposal.issuer, pos)
                    .map_err(|_| {
                        SpellItemError::Rejected("Destroy occupied or unknown reserved actor layer")
                    })?;
            let qualified = tile.qualified_tile();
            let known = conservative_destroy_stack_is_known(
                qualified.source_step().is_some() && qualified.top_source_ids().is_empty(),
                occupied,
                &listed,
                &snapshot.items,
            )?;
            let mut facts = Vec::new();
            if known {
                facts
                    .try_reserve(snapshot.items.len())
                    .map_err(|_| SpellItemError::Rejected("Destroy Item facts allocation"))?;
                for row in &snapshot.items {
                    let (script_tagged, action_tagged) =
                        creature_scope_item_tags_in_transaction(tx, authority, row).await?;
                    facts.push(WorldItemFacts {
                        instance_key: destroy_instance_key(&row.item_instance_id),
                        item: NativeItemRef {
                            key: row.definition.production_key.clone(),
                            revision: row.definition.revision_ref.clone(),
                        },
                        movable: false,
                        script_tagged,
                        action_tagged,
                    });
                }
            }
            let top_visible_item = facts.first().cloned();
            witnesses.push(SourceRemovalTile {
                offset: (dx, dy),
                tile_exists: true,
                tile_items: facts,
                top_visible_item,
            });
            addresses.push(address);
            snapshots.push(snapshot);
        }
    }
    runtime
        .validate_positioned_actor_census(&census)
        .map_err(|_| SpellItemError::Rejected("Destroy census changed"))?;
    let selected = plan_source_item_removals(
        crate::content::ProjectV2RemoveItemsSelection::TopItemFirstTile,
        &listed,
        &witnesses,
    )
    .map_err(|_| SpellItemError::Rejected("Destroy shared top-visible selection"))?;
    let mut operations = Vec::new();
    operations
        .try_reserve(selected.len())
        .map_err(|_| SpellItemError::Rejected("Destroy operation allocation"))?;
    for (index, operation) in selected {
        let ItemOperation::RemoveField { remove_instance } = operation else {
            return Err(SpellItemError::Rejected("Destroy unexpected operation"));
        };
        let row = snapshots
            .get(index)
            .and_then(|snapshot| snapshot.items.first())
            .filter(|row| destroy_instance_key(&row.item_instance_id) == remove_instance)
            .ok_or(SpellItemError::Rejected(
                "Destroy exact actual StackOwner top",
            ))?;
        let address = addresses
            .get(index)
            .ok_or(SpellItemError::Rejected("Destroy tile index"))?;
        operations.push(SpellItemOperation::RemoveGround(
            prepare_creature_scope_removal_in_transaction(
                tx,
                authority,
                address,
                row.item_instance_id,
            )
            .await?,
        ));
    }
    Ok(PreparedCreatureGroundOperations {
        placement: cause.placement,
        generation: cause.generation,
        body: cause.body,
        cast: cause.cast,
        operations,
    })
}

fn destroy_instance_key(bytes: &[u8; 16]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[allow(clippy::expect_used)]
#[cfg(test)]
mod conservative_destroy_tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::spell::native_items::NativeItemRef;
    fn listed() -> Vec<NativeItemRef> {
        [10181, 2128, 10182, 2130]
            .into_iter()
            .map(|id| NativeItemRef {
                key: format!("oteryn:item.tibia.i{id}"),
                revision: "definition-r1".into(),
            })
            .collect()
    }
    fn row(id: u32, ordinal: u64, instance: u8) -> DurableTileItem {
        DurableTileItem {
            item_instance_id: [instance; 16],
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: format!("oteryn:item.tibia.i{id}"),
                revision_ref: "definition-r1".into(),
            },
            state_revision: 1,
            top_down_ordinal: ordinal,
            blocks_movement: false,
            blocks_projectile: false,
            immovable_block_solid: false,
            expires_at_unix_ms: None,
            field_origin: None,
            creature_field_origin: None,
        }
    }
    #[test]
    fn destroy_refuses_actor_static_and_foreign_layers_without_filtering_under_top() {
        let refs = listed();
        let known = [row(2130, 9, 1), row(2128, 4, 2)];
        assert!(
            conservative_destroy_stack_is_known(true, false, &refs, &known)
                .expect("qualified test fixture")
        );
        assert!(
            !conservative_destroy_stack_is_known(true, true, &refs, &known)
                .expect("qualified test fixture")
        );
        assert!(
            !conservative_destroy_stack_is_known(false, false, &refs, &known)
                .expect("qualified test fixture")
        );
        assert!(
            !conservative_destroy_stack_is_known(
                true,
                false,
                &refs,
                &[row(999, 10, 3), known[0].clone()]
            )
            .expect("qualified test fixture")
        );
        // Unknown lower layer could have source alwaysOnTop semantics: refuse too.
        assert!(
            !conservative_destroy_stack_is_known(
                true,
                false,
                &refs,
                &[known[0].clone(), row(999, 4, 3)]
            )
            .expect("qualified test fixture")
        );
        let mut wrong_revision = known[0].clone();
        wrong_revision.definition.revision_ref = "unqualified".into();
        assert!(
            !conservative_destroy_stack_is_known(true, false, &refs, &[wrong_revision])
                .expect("qualified test fixture")
        );
    }
    #[test]
    fn destroy_rejects_legacy_duplicate_or_non_top_down_order() {
        let refs = listed();
        for rows in [
            vec![row(2130, 0, 1)],
            vec![row(2130, 4, 1), row(2128, 9, 2)],
            vec![row(2130, 9, 1), row(2128, 9, 2)],
        ] {
            assert!(conservative_destroy_stack_is_known(true, false, &refs, &rows).is_err());
        }
    }
}

/// Borrow the already locked native owners for the existing semantic SQL pass.
/// No copied fence, fabricated Player cast, or independently running scheduler.
struct CreatureSourceScopeContext<'a> {
    runtime: &'a crate::foundation::ChannelRuntimeV1,
    fence: &'a crate::foundation::ScopeRuntimeFence,
    stamp: crate::foundation::RuntimeWorkStamp,
    room: &'a crate::content::QualifiedNativeEntryRoom,
    objects: &'a crate::world_runtime::LocalObjectRuntime,
    compiled: &'a crate::content::CompiledFirstProductionContent,
    native: &'a crate::content::native_gameplay::NativeGameplayState,
    scheduled: &'a ScheduledCreatureSourceItems,
}
impl DurabilityRoot {
    /// Physically commit an existing AI proposal through the same bounded scope
    /// pass as source field expiry. Caller retains native runtime/object locks
    /// across this await; transaction authority is re-proved from actual recovery
    /// and the current database assignment inside the pass, never from metadata.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn commit_creature_source_items(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        runtime: &crate::foundation::ChannelRuntimeV1,
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        room: &crate::content::QualifiedNativeEntryRoom,
        objects: &crate::world_runtime::LocalObjectRuntime,
        compiled: &crate::content::CompiledFirstProductionContent,
        native: &crate::content::native_gameplay::NativeGameplayState,
        scheduled: &ScheduledCreatureSourceItems,
    ) -> Result<CreatureGroundReceipt> {
        let record = creature_item_recovery_record(recovery, self)?;
        let node = node.clone();
        let actor = scheduled.proposal.issuer;
        let scope =
            crate::foundation::RuntimeScopeRefV1::channel(actor.world_id(), actor.channel_id());
        let generation = actor.scope_generation();
        let mut context = CreatureSourceScopeContext {
            runtime,
            fence,
            stamp,
            room,
            objects,
            compiled,
            native,
            scheduled,
        };
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let outcome: Result<CreatureGroundReceipt> = async {
                        let authority = assert_spell_item_scope_with_recovery(
                            &mut tx,
                            &record,
                            &node,
                            scope,
                            generation.get(),
                        )
                        .await?;
                        execute_scheduled_creature_source_items_in_transaction(
                            &mut tx,
                            &authority,
                            context.runtime,
                            context.fence,
                            context.stamp,
                            context.room,
                            context.objects,
                            context.compiled,
                            context.native,
                            context.scheduled,
                        )
                        .await
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

impl crate::durability::spell_items_abi::DurableCreatureFieldOrigin {
    /// Historical provenance survives caster retirement; it never reconstructs that actor.
    pub(crate) fn matches_current_content(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
    ) -> bool {
        self.condition_source_key().is_some()
            && self.world == *runtime.binding().world_id().as_bytes()
            && self.channel == *runtime.binding().channel_id().as_bytes()
            && self.source_scope_generation == runtime.binding().scope_generation().get()
            && self.content_digest == runtime.content_pin().server_artifact_digest()
            && native.source_digest() == self.content_digest
            && self.map_digest == runtime.content_pin().map_revision_digest()
            && self.frame_digest == runtime.content_pin().frame_binding_digest()
            && {
                use sha2::{Digest, Sha256};
                let creature = crate::content::ProjectV2DefinitionRef {
                    family: crate::content::ProjectV2Family::Creature,
                    key: self.creature_key.clone(),
                    revision: self.creature_revision.clone(),
                };
                let ability = crate::content::ProjectV2DefinitionRef {
                    family: crate::content::ProjectV2Family::Ability,
                    key: self.ability_key.clone(),
                    revision: self.ability_revision.clone(),
                };
                native
                    .source_item_ability(&creature, &ability)
                    .and_then(|body| serde_json::to_vec(body).ok())
                    .is_some_and(|bytes| {
                        let actual: [u8; 32] = Sha256::digest(bytes).into();
                        actual == self.source_body_digest
                    })
            }
    }
}
