//! Crystal onMove/addEvent footprints over the existing physical Item writer.
//! The movement producer seals the observer; timers retain that historical cause.
use super::*;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use oteryn_gameplay_entities::carrier::native_translation_events::CommittedRottenTranslation;
use std::sync::Arc;

#[derive(Debug)]
struct Source {
    cause: CreatureGroundCause,
    target: SpellGroundTarget,
    position: MovementLocalPosition,
    definitions: Vec<QualifiedItemDefinition>,
    guards: &'static [u16],
}
#[derive(Debug, Clone)]
pub struct RottenFootprintCallback {
    source: Arc<Source>,
    cause: CreatureGroundCause,
    phase: usize,
    due_us: u64,
    execution_us: Option<u64>,
    intrinsic_decay_clock: Option<SourceIntrinsicDecayClock>,
}
pub struct RottenFootprintReceipt {
    pub receipt: CreatureGroundReceipt,
    pub next: Option<RottenFootprintCallback>,
}
fn item(id: u16) -> String {
    format!("oteryn:item.tibia.i{id}")
}
impl RottenFootprintCallback {
    pub fn from_committed_translation(
        runtime: &ChannelRuntimeV1,
        room: &crate::content::QualifiedNativeEntryRoom,
        native: &crate::content::native_gameplay::NativeGameplayState,
        event: &CommittedRottenTranslation,
    ) -> Result<Self> {
        if !event.current_for(runtime)
            || !native.qualifies_rotten_blood_callbacks(event.server_digest())
        {
            return Err(SpellItemError::Rejected(
                "Rotten onMove current source fence",
            ));
        }
        let key = event.creature().key.as_str();
        let (role, ids, guards): (
            crate::content::bosses::activity_template::PinnedActivitySourceRoom,
            Vec<u16>,
            &'static [u16],
        ) = match (key, event.is_player()) {
            ("oteryn:creature.vemiath", true) => (
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::Vemiath,
                vec![43589, 43590, 43297],
                &[43589, 43590, 43625],
            ),
            ("oteryn:creature.chagorz", true) => (
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::Chagorz,
                vec![43589, 43590, 43625],
                &[43589, 43590, 43625],
            ),
            ("oteryn:creature.bakragore", true) => (
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::Bakragore,
                vec![43589, 43590, 43297],
                &[43589, 43590, 43297],
            ),
            ("oteryn:creature.bakragore", false) if event.mover() == event.observer() => (
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::Bakragore,
                vec![43294, 43295],
                &[],
            ),
            _ => return Err(SpellItemError::Rejected("Rotten onMove source branch")),
        };
        if room.pinned_activity_source_room() != Some(role) {
            return Err(SpellItemError::Rejected("Rotten onMove exact source map"));
        }
        // All guard sprites are dynamic callbacks in these pinned private
        // captures. Refuse a future source map adding one statically rather
        // than incorrectly comparing only the canonical dynamic Item stack.
        static STATIC_GUARDS_ABSENT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if !*STATIC_GUARDS_ABSENT.get_or_init(|| {
            use crate::content::bosses::activity_template::PinnedActivitySourceRoom as Room;
            [Room::Vemiath, Room::Chagorz, Room::Bakragore]
                .into_iter()
                .all(|room| {
                    let Ok(capture) = serde_json::from_slice::<serde_json::Value>(room.bytes())
                    else {
                        return false;
                    };
                    capture
                        .get("tiles")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|tiles| {
                            tiles.iter().all(|tile| {
                                tile.get("items")
                                    .and_then(serde_json::Value::as_array)
                                    .is_some_and(|items| {
                                        items.iter().all(|item| {
                                            item.get("server_item_id")
                                                .and_then(serde_json::Value::as_u64)
                                                .is_some_and(|id| {
                                                    ![43589, 43590, 43297, 43625, 43294]
                                                        .contains(&id)
                                                })
                                        })
                                    })
                            })
                        })
                })
        }) {
            return Err(SpellItemError::Rejected("Rotten static guard changed"));
        }
        let position = event.after().position();
        let target = SpellGroundTarget::for_native_tile_read(room, runtime, position)
            .map_err(SpellItemError::Rejected)?;
        let mut definitions = Vec::with_capacity(ids.len());
        for id in ids {
            let key = item(id);
            let policy = native
                .item_policy(&key, "definition-r1")
                .ok_or(SpellItemError::Rejected("Rotten onMove actual source Item"))?;
            let attributes = &policy.record().attributes;
            let definition = QualifiedItemDefinition::from_native_policy(policy)
                .map_err(SpellItemError::Rejected)?;
            if !definition.materializable
                || !definition.ground_destination
                || definition.stack_maximum != 1
                || definition.container_capacity.is_some()
                || !definition.decay_chain.is_empty()
                || attributes.blocks_movement != Some(false)
                || attributes.blocks_projectile != Some(false)
                || attributes.immovable_block_solid != Some(false)
            {
                return Err(SpellItemError::Rejected(
                    "Rotten onMove exact native Item policy",
                ));
            }
            definitions.push(definition);
        }
        let creature = TypedDefinitionRef {
            family: "Creature".into(),
            production_key: key.into(),
            revision_ref: event.creature().revision.clone(),
        };
        let cause = CreatureGroundCause {
            world: *event.observer().world_id().as_bytes(),
            scope: runtime.binding().runtime_scope(),
            scope_generation: runtime.binding().scope_generation().get(),
            source_items: vec![definitions[0].definition.clone()],
            placement: event.observer().placement_identity(),
            generation: event.observer().actor_local_generation(),
            creature: creature.clone(),
            ability: creature,
            event_rule: Some(CreatureEventRule::RottenFootprintCreate),
            occurrence: format!("rotten-move:{}:0", rotten_portal::hex(&event.occurrence())),
            content: event.server_digest(),
            map: event.map_digest(),
            frame: event.frame_digest(),
            body: event.source_body(),
            cast: event.occurrence(),
        };
        Ok(Self {
            source: Arc::new(Source {
                cause: cause.clone(),
                target,
                position,
                definitions,
                guards,
            }),
            cause,
            phase: 0,
            due_us: event.observed_us(),
            execution_us: None,
            intrinsic_decay_clock: None,
        })
    }
    /// Retain the first actual callback turn before SQL can yield. Unknown
    /// outcome retries use this same logical execution and nested deadline.
    pub fn begin_execution(&self, now: u64) -> Result<Self> {
        if now < self.due_us {
            return Err(SpellItemError::Rejected("Rotten callback not due"));
        }
        let mut next = self.clone();
        if next.execution_us.is_none() {
            let executed = if next.phase == 0 { next.due_us } else { now };
            next.execution_us = Some(executed);
            next.cause.cast = Sha256::new()
                .chain_update(b"oteryn:rotten-move-execution:v1")
                .chain_update(next.cause.cast)
                .chain_update(executed.to_be_bytes())
                .finalize()
                .into();
        }
        Ok(next)
    }
    pub fn needs_intrinsic_decay_clock(&self) -> bool {
        self.phase < self.source.definitions.len()
            && self.phase > 0
            && self.source.definitions[self.phase].decay.is_some()
            && self.intrinsic_decay_clock.is_none()
    }
    pub fn with_intrinsic_decay_clock(
        &self,
        sample: RottenFootprintClockSample,
        sampled_owner_us: u64,
    ) -> Result<Self> {
        let execution = self
            .execution_us
            .ok_or(SpellItemError::Rejected("source execution absent"))?;
        if !self.needs_intrinsic_decay_clock() {
            return Err(SpellItemError::Rejected("source decay clock already bound"));
        }
        let clock = sample.project(
            self.cause.scope,
            self.cause.scope_generation,
            self.cause.cast,
            execution,
            sampled_owner_us,
        )?;
        let mut next = self.clone();
        next.intrinsic_decay_clock = Some(clock);
        Ok(next)
    }
    pub fn runtime_scope(&self) -> crate::foundation::RuntimeScopeRefV1 {
        self.cause.scope
    }
    pub fn scope_generation(&self) -> u64 {
        self.cause.scope_generation
    }
    pub fn position(&self) -> MovementLocalPosition {
        self.source.position
    }
    pub fn due_us(&self) -> u64 {
        self.due_us
    }
    pub fn execution_started(&self) -> bool {
        self.execution_us.is_some()
    }
    pub fn is_create(&self) -> bool {
        self.phase == 0
    }
    pub fn occurrence(&self) -> [u8; 32] {
        self.source.cause.cast
    }
    fn next(&self, changed: bool) -> Result<Option<Self>> {
        if !changed {
            return Ok(None);
        }
        let len = self.source.definitions.len();
        // Bak's own spore has one five-second transform, no scripted removal.
        if len == 2 && self.phase == 1 || self.phase == len {
            return Ok(None);
        }
        let phase = self.phase + 1;
        let delay = if len == 2 {
            5_000_000
        } else if phase == 1 {
            1_000_000
        } else {
            3_000_000
        };
        let mut cause = self.source.cause.clone();
        cause.event_rule = Some(if phase == len {
            CreatureEventRule::RottenFootprintRemove
        } else {
            CreatureEventRule::RottenFootprintTransform
        });
        cause.source_items = vec![
            self.source.definitions[phase.min(len - 1)]
                .definition
                .clone(),
        ];
        cause.occurrence = format!(
            "rotten-move:{}:{phase}",
            rotten_portal::hex(&self.source.cause.cast)
        );
        cause.cast = Sha256::new()
            .chain_update(b"oteryn:rotten-move-phase:v1")
            .chain_update(self.source.cause.cast)
            .chain_update((phase as u64).to_be_bytes())
            .finalize()
            .into();
        Ok(Some(Self {
            source: Arc::clone(&self.source),
            cause,
            phase,
            execution_us: None,
            intrinsic_decay_clock: None,
            due_us: self
                .execution_us
                .unwrap_or(self.due_us)
                .checked_add(delay)
                .ok_or(SpellItemError::Rejected("Rotten onMove callback overflow"))?,
        }))
    }
}

pub async fn execute_rotten_footprint_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    callback: &RottenFootprintCallback,
    now: u64,
) -> Result<RottenFootprintReceipt> {
    if callback.execution_us.is_none()
        || now < callback.due_us
        || authority.runtime_scope() != callback.cause.scope
        || authority.generation() != callback.cause.scope_generation
    {
        return Err(SpellItemError::Rejected("Rotten onMove due/current scope"));
    }
    rotten_portal::accepted(tx, authority, &callback.cause).await?;
    let transaction = primal_pod::identity(&callback.cause, b"oteryn:rotten-move-transaction:v1");
    if let Some(receipt) =
        reconcile_creature_ground_in_transaction(tx, authority, &callback.cause, || Ok(())).await?
    {
        let count:i32=sqlx::query_scalar("SELECT operation_count FROM game_spell_item_receipts WHERE transaction_id=encode($1,'hex')::uuid")
            .bind(transaction.as_slice()).fetch_one(&mut **tx).await?;
        return Ok(RottenFootprintReceipt {
            next: callback.next(count == 1)?,
            receipt,
        });
    }
    let current =
        read_creature_scope_tile_in_transaction(tx, authority, &callback.source.target).await?;
    let definitions = &callback.source.definitions;
    let operations = if callback.phase == 0 {
        let blocked = current.items.first().is_some_and(|row| {
            callback
                .source
                .guards
                .iter()
                .any(|id| row.definition.production_key == item(*id))
        });
        if blocked {
            vec![]
        } else {
            vec![SpellItemOperation::MintGround {
                item_instance_id: primal_pod::identity(
                    &callback.cause,
                    b"oteryn:rotten-move-item:v1",
                ),
                definition: definitions[0].clone(),
                quantity: 1,
                placement: callback
                    .source
                    .target
                    .placement(callback.cause.occurrence.as_bytes().to_vec()),
                lifetime_millis: None,
                blocks_movement: false,
                blocks_projectile: false,
                description: None,
            }]
        }
    } else {
        let before = &definitions[callback.phase - 1].definition;
        if let Some(row) = current.items.iter().find(|row| row.definition == *before) {
            let reserved = prepare_creature_scope_removal_in_transaction(
                tx,
                authority,
                &callback.source.target,
                row.item_instance_id,
            )
            .await?;
            if callback.phase == definitions.len() {
                vec![SpellItemOperation::RemoveGround(reserved)]
            } else {
                vec![SpellItemOperation::TransformGround(
                    DurableGroundItemTransform {
                        current: reserved,
                        target: definitions[callback.phase].clone(),
                        intrinsic_decay_clock: callback.intrinsic_decay_clock.clone(),
                        blocks_movement: false,
                        blocks_projectile: false,
                    },
                )]
            }
        } else {
            vec![]
        }
    };
    let changed = !operations.is_empty();
    let prepared = PreparedCreatureGroundOperations {
        placement: callback.cause.placement,
        generation: callback.cause.generation,
        body: callback.cause.body,
        cast: callback.cause.cast,
        operations,
    };
    let receipt = apply_creature_ground_in_transaction(
        tx,
        authority,
        &callback.cause,
        transaction,
        primal_pod::identity(&callback.cause, b"oteryn:rotten-move-event:v1"),
        &prepared,
        || Ok(()),
    )
    .await?;
    Ok(RottenFootprintReceipt {
        receipt,
        next: callback.next(changed)?,
    })
}

/// Opaque DATA sampled under the same actual scoped Item authority. A sample
/// can only be attached to its original execution, scope and generation.
#[derive(Debug)]
pub struct RottenFootprintClockSample {
    scope: crate::foundation::RuntimeScopeRefV1,
    generation: u64,
    cast: [u8; 32],
    unix_ms: i64,
}

impl RottenFootprintClockSample {
    fn project(
        self,
        scope: crate::foundation::RuntimeScopeRefV1,
        generation: u64,
        cast: [u8; 32],
        execution: u64,
        sampled_owner_us: u64,
    ) -> Result<SourceIntrinsicDecayClock> {
        if self.scope != scope || self.generation != generation || self.cast != cast {
            return Err(SpellItemError::Rejected("source decay clock binding"));
        }
        let elapsed_ms = sampled_owner_us
            .checked_sub(execution)
            .ok_or(SpellItemError::Rejected("source decay clock regression"))?
            / 1000;
        let execution_unix_ms = self
            .unix_ms
            .checked_sub(
                i64::try_from(elapsed_ms)
                    .map_err(|_| SpellItemError::Rejected("source decay clock overflow"))?,
            )
            .filter(|value| *value > 0)
            .ok_or(SpellItemError::Rejected("source decay epoch underflow"))?;
        Ok(SourceIntrinsicDecayClock {
            original_execution_us: execution,
            execution_unix_ms,
            sampled_owner_us,
            sampled_unix_ms: self.unix_ms,
        })
    }
}

struct FootprintContext<'a> {
    permit: &'a crate::durability::spell_owner_commit::SpellLanePermit,
    callback: &'a RottenFootprintCallback,
    now_us: u64,
}
impl DurabilityRoot {
    pub async fn sample_rotten_footprint_clock(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        callback: &RottenFootprintCallback,
    ) -> Result<RottenFootprintClockSample> {
        let record = creature_item_recovery_record(recovery, self)?;
        let node = node.clone();
        let scope = callback.runtime_scope();
        let generation = callback.scope_generation();
        let mut context = FootprintContext {
            permit,
            callback,
            now_us: 0,
        };
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let result: Result<RottenFootprintClockSample> = async {
                        let authority = assert_spell_item_scope_with_recovery(
                            &mut tx,
                            context.permit,
                            &record,
                            &node,
                            scope,
                            generation,
                        )
                        .await?;
                        check_scope_transaction(&mut tx, &authority).await?;
                        if !context.callback.needs_intrinsic_decay_clock()
                            || !context.callback.execution_started()
                        {
                            return Err(SpellItemError::Rejected(
                                "source decay clock not required",
                            ));
                        }
                        let unix_ms: i64 = sqlx::query_scalar(
                            "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
                        )
                        .fetch_one(&mut *tx)
                        .await?;
                        Ok(RottenFootprintClockSample {
                            scope,
                            generation,
                            cast: context.callback.cause.cast,
                            unix_ms,
                        })
                    }
                    .await;
                    Ok(result)
                })
            })
            .await?
    }
    pub async fn commit_rotten_footprint_callback(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        callback: &RottenFootprintCallback,
        now_us: u64,
    ) -> Result<RottenFootprintReceipt> {
        let record = creature_item_recovery_record(recovery, self)?;
        let node = node.clone();
        let scope = callback.runtime_scope();
        let generation = callback.scope_generation();
        let mut context = FootprintContext {
            permit,
            callback,
            now_us,
        };
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let outcome: Result<RottenFootprintReceipt> = async {
                        let authority = assert_spell_item_scope_with_recovery(
                            &mut tx,
                            context.permit,
                            &record,
                            &node,
                            scope,
                            generation,
                        )
                        .await?;
                        execute_rotten_footprint_in_transaction(
                            &mut tx,
                            &authority,
                            context.callback,
                            context.now_us,
                        )
                        .await
                    }
                    .await;
                    match outcome {
                        Ok(value) => {
                            commit_creature_item_owner_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

#[cfg(test)]
mod intrinsic_clock_tests {
    use super::*;
    type TestResult<T = ()> = std::result::Result<T, String>;
    fn scope(instance: u8) -> TestResult<crate::foundation::RuntimeScopeRefV1> {
        let bytes = [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1];
        Ok(crate::foundation::RuntimeScopeRefV1::Instance {
            world_id: crate::foundation::WorldId::decode(&bytes)
                .map_err(|error| format!("{error:?}"))?,
            instance_id: [instance; 16],
        })
    }
    fn sample() -> TestResult<RottenFootprintClockSample> {
        Ok(RottenFootprintClockSample {
            scope: scope(1)?,
            generation: 9,
            cast: [3; 32],
            unix_ms: 1_000_000,
        })
    }
    #[test]
    fn source_intrinsic_clock_preserves_elapsed_and_retained_retry_deadline() -> TestResult {
        let original = sample()?
            .project(scope(1)?, 9, [3; 32], 2_000_000, 7_000_000)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(original.execution_unix_ms, 995_000);
        assert_eq!(original.execution_unix_ms + 248_000, 1_243_000);
        // A retry retains this proof rather than sampling a new epoch.
        let retained = original.clone();
        assert_eq!(retained, original);
        // A late first attempt keeps the already-expired original deadline.
        let late = sample()?
            .project(scope(1)?, 9, [3; 32], 0, 300_000_000)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(late.execution_unix_ms + 248_000, 948_000);
        Ok(())
    }
    #[test]
    fn source_intrinsic_clock_refuses_foreign_scope_generation_or_execution() -> TestResult {
        assert!(sample()?.project(scope(2)?, 9, [3; 32], 0, 0).is_err());
        assert!(sample()?.project(scope(1)?, 10, [3; 32], 0, 0).is_err());
        assert!(sample()?.project(scope(1)?, 9, [4; 32], 0, 0).is_err());
        assert!(sample()?.project(scope(1)?, 9, [3; 32], 2, 1).is_err());
        assert!(
            sample()?
                .project(scope(1)?, 9, [3; 32], 0, u64::MAX)
                .is_err()
        );
        Ok(())
    }
}
