//! Copied Gaz/Corrupted onDeath over the existing scoped Item writer.
use super::*;
use crate::content::{QualifiedNativeDeathEvent, QualifiedNativeEntryRoom};
use crate::durability::spell_item_transaction::assert_creature_event_scope_in_transaction;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::carrier::native_source_death::SourceCreatureDeathSeal;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use std::sync::Arc;
const GAZ: &str = "oteryn:item.tibia.i20121";
const CORRUPT: [&str; 3] = [
    "oteryn:item.tibia.i23726",
    "oteryn:item.tibia.i23727",
    "oteryn:item.tibia.i23728",
];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Gaz,
    Corrupted,
}
#[derive(Debug, Clone)]
pub struct W01DeathGroundCallback {
    begin: SourceCreatureDeathSeal,
    event: QualifiedNativeDeathEvent,
    cause: CreatureGroundCause,
    target: SpellGroundTarget,
    definitions: Arc<[QualifiedItemDefinition]>,
    kind: Kind,
    expiry: bool,
    position: MovementLocalPosition,
    due_us: u64,
}
pub struct W01DeathGroundReceipt {
    pub receipt: CreatureGroundReceipt,
    expiry: Option<W01DeathGroundCallback>,
}
impl W01DeathGroundReceipt {
    pub fn into_expiry(self) -> Option<W01DeathGroundCallback> {
        self.expiry
    }
}
impl W01DeathGroundCallback {
    pub fn from_source_death(
        runtime: &ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &crate::content::native_gameplay::NativeGameplayState,
        begin: &SourceCreatureDeathSeal,
        event: &QualifiedNativeDeathEvent,
        now_us: u64,
    ) -> Result<Option<Self>> {
        let (kind, keys) = match (begin.definition_key(), event.name()) {
            ("oteryn:creature.minion_of_gaz_haragoth", "MinionGazDeath") => (Kind::Gaz, &[GAZ][..]),
            ("oteryn:creature.corrupted_soul", "CorruptedSoul") => (Kind::Corrupted, &CORRUPT[..]),
            _ => return Ok(None),
        };
        let index = runtime
            .source_creature_death_event_index(begin)
            .map_err(|_| SpellItemError::Rejected("current copied death cursor"))?
            .ok_or(SpellItemError::Rejected("completed copied death"))?;
        if !begin.current_for(runtime)
            || begin.events().get(index) != Some(event)
            || native.source_digest() != begin.content_digest()
        {
            return Err(SpellItemError::Rejected("exact copied death source"));
        }
        let position = runtime
            .read_actor_position(begin.actor())
            .map_err(|_| SpellItemError::Rejected("current copied death position"))?
            .position();
        let target = SpellGroundTarget::for_native_tile_read(room, runtime, position)
            .map_err(SpellItemError::Rejected)?;
        let mut definitions = Vec::new();
        for key in keys {
            let policy = native
                .item_policy(key, "definition-r1")
                .ok_or(SpellItemError::Rejected("actual W01 event Item"))?;
            let definition = QualifiedItemDefinition::from_native_policy(policy)
                .map_err(SpellItemError::Rejected)?;
            if !definition.materializable
                || !definition.ground_destination
                || definition.stack_maximum != 1
                || definition.container_capacity.is_some()
                || definition.decay.is_some()
                || !definition.decay_chain.is_empty()
                || policy.record().attributes.blocks_movement != Some(false)
                || policy.record().attributes.blocks_projectile != Some(false)
            {
                return Err(SpellItemError::Rejected(
                    "exact nondecaying W01 Ground Item",
                ));
            }
            definitions.push(definition);
        }
        let actor = begin.actor();
        let cast: [u8; 32] = Sha256::new()
            .chain_update(b"oteryn:w01-copied-death-ground:v1")
            .chain_update(actor.placement_identity())
            .chain_update(actor.actor_local_generation().to_be_bytes())
            .chain_update(event.registration_index().to_be_bytes())
            .chain_update(event.source_sha256())
            .chain_update(begin.content_digest())
            .finalize()
            .into();
        let creature = TypedDefinitionRef {
            family: "Creature".into(),
            production_key: begin.definition_key().into(),
            revision_ref: begin.definition_revision().into(),
        };
        let cause = CreatureGroundCause {
            world: *actor.world_id().as_bytes(),
            scope: runtime.binding().runtime_scope(),
            scope_generation: runtime.binding().scope_generation().get(),
            source_items: vec![definitions[0].definition.clone()],
            placement: actor.placement_identity(),
            generation: actor.actor_local_generation(),
            ability: creature.clone(),
            creature,
            event_rule: Some(if kind == Kind::Gaz {
                CreatureEventRule::GazMinionCreate
            } else {
                CreatureEventRule::CorruptedSoulCreate
            }),
            occurrence: format!(
                "w01-death:{}:{}",
                event.registration_index(),
                super::rotten_portal::hex(&cast)
            ),
            content: begin.content_digest(),
            map: begin.map_digest(),
            frame: begin.frame_digest(),
            body: event.source_sha256(),
            cast,
        };
        Ok(Some(Self {
            begin: begin.clone(),
            event: event.clone(),
            cause,
            target,
            definitions: definitions.into(),
            kind,
            expiry: false,
            position,
            due_us: now_us,
        }))
    }
    pub fn matches(
        &self,
        begin: &SourceCreatureDeathSeal,
        event: &QualifiedNativeDeathEvent,
    ) -> bool {
        &self.begin == begin && &self.event == event && !self.expiry
    }
    pub fn identity(&self) -> [u8; 32] {
        self.cause.cast
    }
    pub fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.begin.current_for(runtime)
    }
    pub fn due_us(&self) -> u64 {
        self.due_us
    }
    pub fn position(&self) -> MovementLocalPosition {
        self.position
    }
    fn successor(&self, rule: &str) -> Result<Option<Self>> {
        let delay = match (self.kind, self.expiry, rule) {
            (Kind::Gaz, false, "w01.gaz-minion.create") => 60_000_000,
            (Kind::Corrupted, false, "w01.corrupted-soul.create") => return Ok(None),
            (Kind::Corrupted, false, "w01.corrupted-soul.transform") => 30_000_000,
            (Kind::Gaz, true, "w01.gaz-minion.expire")
            | (Kind::Corrupted, true, "w01.corrupted-soul.expire") => return Ok(None),
            _ => return Err(SpellItemError::Rejected("W01 receipt source branch")),
        };
        let mut next = self.clone();
        next.expiry = true;
        next.due_us = self
            .due_us
            .checked_add(delay)
            .ok_or(SpellItemError::Rejected("W01 original expiry deadline"))?;
        next.cause.event_rule = Some(if self.kind == Kind::Gaz {
            CreatureEventRule::GazMinionExpire
        } else {
            CreatureEventRule::CorruptedSoulExpire
        });
        next.cause.occurrence.push_str(":expire");
        next.cause.cast = Sha256::new()
            .chain_update(b"oteryn:w01-death-ground-expire:v1")
            .chain_update(self.cause.cast)
            .finalize()
            .into();
        Ok(Some(next))
    }
}
async fn receipt_result(
    tx: &mut Transaction<'_, Postgres>,
    callback: &W01DeathGroundCallback,
    receipt: CreatureGroundReceipt,
) -> Result<W01DeathGroundReceipt> {
    let rule:String=sqlx::query_scalar("SELECT convert_from(intent,'UTF8')::jsonb#>>'{source_event,rule}' FROM game_spell_item_receipts WHERE transaction_id=encode($1,'hex')::uuid").bind(receipt.transaction_id.as_slice()).fetch_one(&mut **tx).await?;
    Ok(W01DeathGroundReceipt {
        expiry: callback.successor(&rule)?,
        receipt,
    })
}
pub async fn execute_w01_death_ground_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    callback: &W01DeathGroundCallback,
    now_us: u64,
) -> Result<W01DeathGroundReceipt> {
    if now_us < callback.due_us
        || authority.runtime_scope() != callback.cause.scope
        || authority.generation() != callback.cause.scope_generation
    {
        return Err(SpellItemError::Rejected(
            "W01 event current NodeScope/deadline",
        ));
    }
    // Reconcile BEFORE sampling CURRENT Item. An ambiguous previous COMMIT
    // must not turn its own newly-created vortex into a second transform.
    if let Some(receipt) =
        reconcile_creature_ground_in_transaction(tx, authority, &callback.cause, || Ok(())).await?
    {
        return receipt_result(tx, callback, receipt).await;
    }
    let current = read_creature_scope_tile_in_transaction(tx, authority, &callback.target).await?;
    let mut cause = callback.cause.clone();
    let selected = callback.definitions.iter().find_map(|definition| {
        current
            .items
            .iter()
            .find(|item| item.definition == definition.definition)
            .map(|item| (definition, item))
    });
    let operations = if callback.expiry {
        cause.event_rule = Some(if callback.kind == Kind::Gaz {
            CreatureEventRule::GazMinionExpire
        } else {
            CreatureEventRule::CorruptedSoulExpire
        });
        if let Some((_, item)) = selected {
            cause.source_items = vec![item.definition.clone()];
            vec![SpellItemOperation::RemoveGround(
                prepare_creature_scope_removal_in_transaction(
                    tx,
                    authority,
                    &callback.target,
                    item.item_instance_id,
                )
                .await?,
            )]
        } else {
            vec![]
        }
    } else if callback.kind == Kind::Gaz || selected.is_none() {
        let definition = callback.definitions[0].clone();
        cause.source_items = vec![definition.definition.clone()];
        vec![SpellItemOperation::MintGround {
            item_instance_id: super::primal_pod::identity(
                &cause,
                b"oteryn:w01-death-ground-item:v1",
            ),
            definition,
            quantity: 1,
            placement: callback
                .target
                .placement(cause.occurrence.as_bytes().to_vec()),
            lifetime_millis: None,
            blocks_movement: false,
            blocks_projectile: false,
            description: None,
        }]
    } else {
        cause.event_rule = Some(CreatureEventRule::CorruptedSoulTransform);
        let (definition, item) =
            selected.ok_or(SpellItemError::Rejected("current Corrupted source vortex"))?;
        let index = callback
            .definitions
            .iter()
            .position(|candidate| candidate.definition == definition.definition)
            .ok_or(SpellItemError::Rejected("exact current vortex policy"))?;
        if index == 2 {
            cause.source_items = vec![definition.definition.clone()];
            vec![]
        } else {
            let target = callback.definitions[index + 1].clone();
            cause.source_items = vec![target.definition.clone()];
            vec![SpellItemOperation::TransformGround(
                DurableGroundItemTransform {
                    current: prepare_creature_scope_removal_in_transaction(
                        tx,
                        authority,
                        &callback.target,
                        item.item_instance_id,
                    )
                    .await?,
                    target,
                    intrinsic_decay_clock: None,
                    blocks_movement: false,
                    blocks_projectile: false,
                },
            )]
        }
    };
    let prepared = PreparedCreatureGroundOperations {
        placement: cause.placement,
        generation: cause.generation,
        body: cause.body,
        cast: cause.cast,
        operations,
    };
    let receipt = apply_creature_ground_in_transaction(
        tx,
        authority,
        &cause,
        super::primal_pod::identity(&cause, b"oteryn:w01-death-ground-transaction:v1"),
        super::primal_pod::identity(&cause, b"oteryn:w01-death-ground-event:v1"),
        &prepared,
        || Ok(()),
    )
    .await?;
    receipt_result(tx, callback, receipt).await
}
struct Context<'a> {
    permit: &'a SpellLanePermit,
    callback: &'a W01DeathGroundCallback,
    now_us: u64,
}
impl DurabilityRoot {
    pub async fn commit_w01_death_ground(
        &self,
        permit: &SpellLanePermit,
        node: &NodeIncarnationProof,
        callback: &W01DeathGroundCallback,
        now_us: u64,
    ) -> Result<W01DeathGroundReceipt> {
        let node = node.clone();
        let scope = callback.cause.scope;
        let generation = callback.cause.scope_generation;
        let mut context = Context {
            permit,
            callback,
            now_us,
        };
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let outcome: Result<W01DeathGroundReceipt> = async {
                        let authority = assert_creature_event_scope_in_transaction(
                            &mut tx,
                            context.permit,
                            &node,
                            scope,
                            generation,
                        )
                        .await?;
                        execute_w01_death_ground_in_transaction(
                            &mut tx,
                            &authority,
                            context.callback,
                            context.now_us,
                        )
                        .await
                    }
                    .await;
                    match outcome {
                        Ok(result) => {
                            commit_creature_item_owner_transaction(tx, deadline).await?;
                            Ok(Ok(result))
                        }
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}
