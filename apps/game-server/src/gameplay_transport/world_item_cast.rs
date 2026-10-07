//! Source-backed world Item casts in the existing Channel and durable Item owners.
//! The original normalized grant survives an uncertain COMMIT. Neither a planner
//! result nor a historical receipt can supply current player or Content authority.
use super::native_combat_cast::{
    NativeCastDispatch, ParkedMarker, ParkedMarkerKind, UnresolvedSpellCommit,
};
use super::{
    ChannelSpellStates, PlayerBatchPreflight, SpellCastIntent, SpellCastOutcome, check_owner_batch,
    install_owner_batch, stage_player_batch,
};
use crate::ability::{AbilityOccurrence, RevisionSet};
use crate::content::{QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};
use crate::durability::character_build::{BuildCommitOutcome, BuildOccurrence};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::spell_items_abi::*;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::durability::{DurabilityError, spell_item_transaction as items};
use crate::foundation::{
    ChannelRuntimeV1, CommandRef, ExactActorRef, GameSessionId, StagedSpellBatch,
};
use crate::spell::cast::{PlayerSpellState, prepare_native_owner_cast_with_caster};
use crate::spell::combat_batch::OwnerCombatBatch;
use crate::spell::harmony::HarmonyMultiplier;
use crate::spell::mana_training::PreparedPlayerTraining;
use crate::spell::native::{Facts, Plan};
use crate::spell::owned_cast_facts::OwnedCastFacts;
use crate::spell::{Execution, OperationalCastFacts, SpellDefinition};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_simulation_determinism::SemanticTimeMicros;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};

#[derive(Debug)]
pub(crate) struct PreparedWorldItemCast {
    actor: ExactActorRef,
    session: GameSessionId,
    intent: SpellCastIntent,
    fence: CurrentCharacterGameplayFence,
    before: PlayerSpellState,
    paid: PlayerSpellState,
    position: crate::foundation::MovementPositionSnapshot,
    owned: OwnedCastFacts,
    batch: OwnerCombatBatch,
    request: SpellItemTransactionRequest,
    training: PreparedPlayerTraining,
    physical: StagedSpellBatch,
    player: PlayerBatchPreflight,
    presentation: super::super::spell_presentations::PreparedPresentation,
}

fn nonce(
    tag: &[u8],
    actor: ExactActorRef,
    session: GameSessionId,
    command: u64,
    extra: &[u8],
) -> [u8; 16] {
    let hash = Sha256::new()
        .chain_update(tag)
        .chain_update(actor.placement_identity())
        .chain_update(session.as_bytes())
        .chain_update(command.to_be_bytes())
        .chain_update(extra)
        .finalize();
    let mut result = [0; 16];
    result.copy_from_slice(&hash[..16]);
    result[6] = (result[6] & 15) | 0x70;
    result[8] = (result[8] & 63) | 0x80;
    result
}
/// The verdict refusal of a new grant; the guarded writer returns it only without a receipt.
const WORLD_ITEM_NEW_GRANT_REFUSED: &str = "current new grant access";

fn denied<T>() -> Result<T, SpellCastDisposition> {
    Err(SpellCastDisposition::Rejected)
}
fn source_intent(intent: &SpellCastIntent) -> serde_json::Value {
    json!({"index":intent.spell.get(),"target":match intent.target{
        SpellTarget::None=>json!(["none"]),SpellTarget::AttackTarget=>json!(["attack"]),
        SpellTarget::Position(p)=>json!(["position",p.x,p.y,p.floor])},"aim":intent.aim_at_target})
}

fn current_facts_match(
    attempt: &PreparedWorldItemCast,
    current: &OwnedCastFacts,
    reconnect: Option<
        &crate::durability::admission_journal::spell_reconnect::SpellReconnectTransition,
    >,
) -> bool {
    let mut expected = attempt.owned.binding().clone();
    let mut equipment = attempt.owned.equipment().clone();
    if current.binding().connection_generation != expected.connection_generation {
        let Ok(connection) =
            crate::foundation::ConnectionGeneration::new(current.binding().connection_generation)
        else {
            return false;
        };
        if !reconnect.is_some_and(|proof| {
            proof.matches(
                attempt.fence.connection_generation,
                connection,
                attempt.request.command,
                attempt.fence.character_id,
                attempt.fence.character_lease_generation,
                attempt.fence.runtime_scope,
                attempt.fence.scope_ownership_generation.get(),
            )
        }) {
            return false;
        }
        expected.connection_generation = current.binding().connection_generation;
    }
    if current.equipment().revision != equipment.revision {
        if equipment.revision.checked_add(1) != Some(current.equipment().revision) {
            return false;
        }
        let mut consumed = false;
        for operation in &attempt.request.operations {
            let SpellItemOperation::ConsumeInventory(item) = operation else {
                continue;
            };
            let InventoryCustody::Equipment {
                slot,
                equipment_revision,
            } = item.custody
            else {
                continue;
            };
            if equipment_revision != equipment.revision {
                return false;
            }
            let Some(index) = equipment.items.iter().position(|actual| {
                actual.slot == slot
                    && actual.item_instance_id == item.item_instance_id
                    && actual.state_revision == item.state_revision
                    && actual.quantity == item.quantity_before
                    && actual.definition == item.definition.definition
            }) else {
                return false;
            };
            if item.quantity_after == 0 {
                equipment.items.remove(index);
            } else {
                let Some(next) = item.state_revision.checked_add(1) else {
                    return false;
                };
                equipment.items[index].quantity = item.quantity_after;
                equipment.items[index].state_revision = next;
            }
            consumed = true;
        }
        if !consumed {
            return false;
        }
        equipment.revision = current.equipment().revision;
        expected.equipment_revision = equipment.revision;
    }
    let expected_build = if current.binding().character_revision == expected.character_revision {
        attempt.owned.durable_build()
    } else if expected.character_revision.checked_add(1)
        == Some(current.binding().character_revision)
    {
        let Some(training) = attempt.training.request() else {
            return false;
        };
        expected.character_revision = current.binding().character_revision;
        equipment.character_revision = expected.character_revision;
        &training.after
    } else {
        return false;
    };
    current.binding() == &expected
        && current.equipment() == &equipment
        && current.durable_build() == expected_build
}

/// Exact Item definitions are resolved from activated explicit bindings. A
/// candidate numeric identifier never becomes a durable identity in this reader.
fn resolve_item(
    content: &NativeGameplayState,
    key: &str,
    revision: &str,
) -> Result<QualifiedItemDefinition, SpellCastDisposition> {
    let policy = content
        .item_policy(key, revision)
        .ok_or(SpellCastDisposition::NotAvailable)?;
    use crate::content::ReferenceItemField as F;
    let mut item = QualifiedItemDefinition::from_native_policy(policy)
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
    item.decay = match &policy.record().semantics.temporal {
        F::NotApplicable => None,
        F::Known(temporal)
            if matches!(
                temporal.duration,
                F::Known(crate::content::ReferenceMilliseconds(0))
            ) && matches!(temporal.decay_target, F::NotApplicable) =>
        {
            None
        }
        F::Known(temporal) if matches!(temporal.stop_duration, F::Known(false)) => {
            let F::Known(crate::content::ReferenceMilliseconds(duration)) = temporal.duration
            else {
                return Err(SpellCastDisposition::NotAvailable);
            };
            let duration = u32::try_from(duration)
                .ok()
                .filter(|d| *d > 0)
                .ok_or(SpellCastDisposition::NotAvailable)?;
            let target = match &temporal.decay_target {
                F::NotApplicable => None,
                F::Known(target) => {
                    let target_policy = content
                        .item_policy(&target.key, &target.revision)
                        .ok_or(SpellCastDisposition::NotAvailable)?;
                    let target = QualifiedItemDefinition::from_native_policy(target_policy)
                        .map_err(|_| SpellCastDisposition::NotAvailable)?;
                    if target.stack_maximum != 1
                        || target.container_capacity.is_some()
                        || !target.inventory_destination
                        || !target.ground_destination
                    {
                        return Err(SpellCastDisposition::NotAvailable);
                    }
                    Some(target.definition)
                }
                _ => return Err(SpellCastDisposition::NotAvailable),
            };
            Some(QualifiedItemDecay {
                duration_millis: duration,
                target,
            })
        }
        _ => return Err(SpellCastDisposition::NotAvailable),
    };
    if !item.inventory_destination
        || !item.ground_destination
        || !item.movable
        || item.stack_maximum == 0
        || item.stack_maximum > 100
    {
        return Err(SpellCastDisposition::NotAvailable);
    }
    Ok(item)
}

#[allow(clippy::too_many_arguments)]
async fn prepare_item_grant(
    tx: &mut Transaction<'_, Postgres>,
    authority: &items::SpellItemAuthority,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &crate::world_runtime::LocalObjectRuntime,
    content: &NativeGameplayState,
    owned: OwnedCastFacts,
    spell: &SpellDefinition,
    intent: SpellCastIntent,
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    now: SemanticTimeMicros,
    draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<PreparedWorldItemCast, SpellCastDisposition> {
    let food_profile = match &spell.execution {
        Execution::NativeProfile(profile)
            if profile.spell()["execution"]["native_behavior"]["key"] == "random_item_grant" =>
        {
            Some(profile)
        }
        _ => None,
    };
    let conjure = match &spell.execution {
        Execution::Conjure { .. } => spell
            .authored
            .as_ref()
            .and_then(|s| s.header.execution.conjure.as_ref()),
        _ => None,
    };
    if food_profile.is_none() && conjure.is_none() {
        return denied();
    }
    if intent.target != SpellTarget::None
        || intent.aim_at_target
        || spell.aggressive
        || !matches!(spell.carrier, crate::spell::Carrier::Instant { .. })
    {
        return denied();
    }
    let b = owned.binding();
    let actor = b.actor;
    let session = command.game_session_id();
    if b.session != session
        || b.content_digest != content.source_digest()
        || authority.command() != command
        || authority.character_id_bytes() != b.character
        || authority.compatible_content_digest() != content.source_digest()
    {
        return denied();
    }
    let before = states
        .get(runtime, actor, session)
        .ok_or(SpellCastDisposition::Rejected)?
        .clone();
    let position = runtime
        .read_actor_position(actor)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let p = position.position();
    let ground = SpellGroundTarget::for_native_tile_read(room, runtime, p)
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
    let in_protection_zone = {
        let tile = crate::spell::world_execution::qualified_combat_tile_in_transaction(
            tx, authority, room, runtime, objects, p,
        )
        .await
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
        if !tile.ground_present() {
            return denied();
        }
        tile.protection_zone()
    };
    let caster = owned
        .caster(&before, spell, content, HarmonyMultiplier::ONE, now.get())
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
    let operational = OperationalCastFacts {
        caster_position: crate::spell::chain::TilePosition {
            x: p.x,
            y: p.y,
            floor: p.floor,
        },
        target_position: None,
        target: None,
        line_of_sight_clear: None,
        direction_available: position.facing().is_some(),
        wheel_unlocked: None,
        in_protection_zone,
        target_tile_solid: None,
        target_tile_creature: None,
    };
    let empty = serde_json::Value::Null;
    let parameters = food_profile
        .map(|p| &p.spell()["execution"]["native_behavior"]["parameters"])
        .unwrap_or(&empty);
    let refs: Vec<crate::spell::native_items::NativeItemRef> = if food_profile.is_some() {
        parameters["pool"]
            .as_array()
            .ok_or(SpellCastDisposition::Rejected)?
            .iter()
            .map(|item| {
                Ok(crate::spell::native_items::NativeItemRef {
                    key: item["key"]
                        .as_str()
                        .ok_or(SpellCastDisposition::Rejected)?
                        .into(),
                    revision: item["revision"]
                        .as_str()
                        .ok_or(SpellCastDisposition::Rejected)?
                        .into(),
                })
            })
            .collect::<Result<_, SpellCastDisposition>>()?
    } else {
        let result = &conjure.ok_or(SpellCastDisposition::Rejected)?.result;
        vec![crate::spell::native_items::NativeItemRef {
            key: result.key.clone(),
            revision: result.revision.clone(),
        }]
    };
    // Qualify every possible grant before any draw or staged payment.
    let mut definitions = std::collections::BTreeMap::new();
    for item in &refs {
        definitions.insert(
            (item.key.clone(), item.revision.clone()),
            resolve_item(content, &item.key, &item.revision)?,
        );
    }
    let reagent = conjure
        .and_then(|c| c.reagent.as_ref())
        .map(|item| resolve_item(content, &item.key, &item.revision))
        .transpose()?;
    // Source Item2854 is the pinned actual backpack. Equipment slot3 is
    // Legs in the native owner and cannot stand in for main-bag custody.
    // The SQL owner independently compares the actual bag definition.
    let backpack_policy = content
        .item_policy_for_source_id(2854)
        .ok_or(SpellCastDisposition::NotAvailable)?;
    let backpack = QualifiedItemDefinition::from_native_policy(backpack_policy)
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
    if !backpack.inventory_destination || backpack.container_capacity.is_none() {
        return denied();
    }
    let presentation_owner = states
        .presentations
        .as_mut()
        .ok_or(SpellCastDisposition::NotAvailable)?;
    presentation_owner
        .reserve_before_draw(runtime, 2)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let training_occurrence = BuildOccurrence::from_bytes(nonce(
        b"oteryn:world-item-training:v1",
        actor,
        session,
        command.command_id().get(),
        &[],
    ))
    .map_err(|_| SpellCastDisposition::Rejected)?;
    let formula = content
        .training_formula()
        .ok_or(SpellCastDisposition::NotAvailable)?;
    let (paid, outputs, grants) = if food_profile.is_some() {
        let preview = prepare_native_owner_cast_with_caster(
            &before,
            spell,
            &operational,
            Facts::Food,
            now,
            &mut |minimum, _| minimum,
            &caster,
        )?;
        before
            .prepare_paid_training(
                &preview.next,
                &preview.anchor,
                formula,
                training_occurrence,
                now.get(),
            )
            .map_err(|_| SpellCastDisposition::Rejected)?;
        let paid = prepare_native_owner_cast_with_caster(
            &before,
            spell,
            &operational,
            Facts::Food,
            now,
            draw,
            &caster,
        )?;
        let Plan::Food {
            items: outputs,
            overflow: crate::spell::native_items::ItemGrantOverflow::DropOnCasterTile,
        } = paid.plan
        else {
            return denied();
        };
        let mut grants: Vec<(crate::spell::native_items::NativeItemRef, u32)> = Vec::new();
        for output in &outputs {
            if let Some((_, quantity)) = grants.iter_mut().find(|(item, _)| item == output) {
                *quantity += 1;
            } else {
                grants.push((output.clone(), 1));
            }
        }
        (
            crate::spell::cast::PaidSourceSelfCast {
                next: paid.next,
                anchor: paid.anchor,
                ability_variant: None,
            },
            outputs,
            grants,
        )
    } else {
        let paid = crate::spell::cast::prepare_conjure_owner_cast_with_caster(
            &before,
            spell,
            &operational,
            now,
            &caster,
        )?;
        let count = conjure.ok_or(SpellCastDisposition::Rejected)?.count;
        let result = refs.first().ok_or(SpellCastDisposition::Rejected)?.clone();
        before
            .prepare_paid_training(
                &paid.next,
                &paid.anchor,
                formula,
                training_occurrence,
                now.get(),
            )
            .map_err(|_| SpellCastDisposition::Rejected)?;
        (paid, vec![result.clone()], vec![(result, count)])
    };
    let placement = ground.placement(
        format!(
            "spell:{}:{}",
            command
                .game_session_id()
                .as_bytes()
                .iter()
                .map(|v| format!("{v:02x}"))
                .collect::<String>(),
            command.command_id().get()
        )
        .into_bytes(),
    );
    let mut operations = Vec::new();
    if let Some(reagent) = reagent {
        let source = items::prepare_inventory_consumption_in_transaction(
            tx, authority, &reagent, 1, None, &placement,
        )
        .await
        .map_err(|_| SpellCastDisposition::Rejected)?;
        operations.push(SpellItemOperation::ConsumeInventory(source));
    }
    for (output, quantity) in grants {
        let definition = definitions
            .get(&(output.key.clone(), output.revision.clone()))
            .ok_or(SpellCastDisposition::NotAvailable)?;
        let (merged, mut remaining) = items::prepare_inventory_merges_in_transaction(
            tx, authority, definition, quantity, &backpack, &placement,
        )
        .await
        .map_err(|_| SpellCastDisposition::Rejected)?;
        operations.extend(merged.into_iter().map(SpellItemOperation::MergeInventory));
        while remaining > 0 {
            let amount = remaining.min(definition.stack_maximum);
            let item_instance_id = nonce(
                b"oteryn:world-item-output:v1",
                actor,
                session,
                command.command_id().get(),
                &(operations.len() as u64).to_be_bytes(),
            );
            operations.push(SpellItemOperation::MintInventory {
                item_instance_id,
                definition: definition.clone(),
                quantity: amount,
                backpack: backpack.clone(),
                overflow_placement: placement.clone(),
                allow_ground_overflow: true,
            });
            remaining -= amount;
        }
    }
    let occurrence_bytes = nonce(
        b"oteryn:world-item-occurrence:v1",
        actor,
        session,
        command.command_id().get(),
        &[],
    );
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let revision = RevisionSet::new(
        "ruleset:spell-native-r20",
        &format!("content:{}", hex(&content.source_digest())),
        "world:spell-pve-candidate-r21",
        "formula:spell-p2-r20",
        "simulation:v1",
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    let occurrence =
        AbilityOccurrence::new(&format!("world-item:{}", hex(&occurrence_bytes)), revision)
            .map_err(|_| SpellCastDisposition::Rejected)?;
    let mut batch=OwnerCombatBatch{caster:actor,attacker:crate::foundation::CharacterId::decode(&b.character)
        .map_err(|_|SpellCastDisposition::Rejected)?,current_lease_generation:b.lease_generation,
        command,occurrence:occurrence.into(),anchor:Some(paid.anchor.clone()),now_ms:now.get()/1000,effects:Vec::new(),deferred:None,
        binding:serde_json::to_vec(&json!({"intent":source_intent(&intent),"spell":format!("{spell:?}"),
            "outputs":outputs.iter().map(|i|json!({"key":i.key,"revision":i.revision})).collect::<Vec<_>>(),
            "item_operations":format!("{operations:?}"),"before":format!("{before:?}"),
            "paid":format!("{:?}",paid.next),"owned":format!("{owned:?}")})).map_err(|_|SpellCastDisposition::Rejected)?};
    let effect = if food_profile.is_some() {
        parameters["effect_asset_binding"]
            .as_str()
            .map(str::to_owned)
    } else {
        conjure.and_then(|c| c.effect_asset_binding.clone())
    };
    let mut cues = Vec::new();
    if let Some(binding) = effect {
        cues.push(super::super::spell_presentations::LocatedCueRequest {
            binding,
            target: super::super::spell_presentations::CueTarget::Actor(actor),
        });
    }
    if let Some(binding) = spell
        .authored
        .as_ref()
        .and_then(|s| s.header.presentation.as_ref())
        .and_then(|p| p.cast_cue.clone())
    {
        cues.push(super::super::spell_presentations::LocatedCueRequest {
            binding,
            target: super::super::spell_presentations::CueTarget::Actor(actor),
        });
    }
    let presentation = states
        .presentations
        .as_mut()
        .ok_or(SpellCastDisposition::Rejected)?
        .prepare_source_definition(runtime, content, intent.spell, spell, &mut batch, cues)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let training = before
        .prepare_paid_training(
            &paid.next,
            &paid.anchor,
            formula,
            training_occurrence,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let identity = &spell
        .authored
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?
        .header
        .identity;
    let request = SpellItemTransactionRequest {
        command,
        spell: crate::durability::item_mint::TypedDefinitionRef {
            family: "Spell".into(),
            production_key: identity.key.clone(),
            revision_ref: identity.revision.clone(),
        },
        catalog_digest: content.source_digest(),
        transaction_id: nonce(
            b"oteryn:world-item-source:v1",
            actor,
            session,
            command.command_id().get(),
            &[],
        ),
        event_id: nonce(
            b"oteryn:world-item-event:v1",
            actor,
            session,
            command.command_id().get(),
            &batch.binding,
        ),
        cost: crate::spell::companion_lifecycle::familiar_cost_binding(
            &before,
            &paid.next,
            &paid.anchor,
        )
        .map_err(|_| SpellCastDisposition::Rejected)?,
        caster_origin: Some(SourceCasterOrigin {
            actor,
            character_lease_generation: b.lease_generation,
        }),
        operations,
        companion: None,
        direct_companion: None,
    };
    // Allocate and validate the complete physical successor before the source
    // transaction commits. Training rebinding uses an already staged allocation.
    let physical = runtime
        .stage_spell_batch(&batch)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let player = stage_player_batch(runtime, states, &batch, Some(paid.next.clone()))
        .map_err(|_| SpellCastDisposition::Rejected)?;
    Ok(PreparedWorldItemCast {
        actor,
        session,
        intent,
        fence,
        before,
        paid: paid.next,
        position,
        owned,
        batch,
        request,
        training,
        physical,
        player,
        presentation,
    })
}

impl ChannelSpellStates {
    pub(crate) fn has_pending_world_items(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        self.pending_world_items
            .iter()
            .any(|p| p.is_for(actor, session))
    }
}

/// Moves a parked world-item attempt into its caster's marker for the resolution (§1.6).
pub(crate) fn restore_parked_world_items(
    states: &mut ChannelSpellStates,
    attempt: PreparedWorldItemCast,
) -> ParkedMarker {
    let (actor, session, intent, command) = (
        attempt.actor,
        attempt.session,
        attempt.intent,
        attempt.batch.command,
    );
    super::PendingSpellMarker::restore(
        &mut states.pending_world_items,
        actor,
        session,
        command,
        intent,
        attempt,
    );
    ParkedMarker {
        kind: ParkedMarkerKind::WorldItem,
        actor,
        session,
    }
}

impl super::super::ComposedFreshAdmission<'_, '_, '_> {
    /// Background recovery consumes only the privately retained original
    /// occurrence. It cannot allocate a new grant when no pending source exists.
    /// The lane comes first, so an attempt parked in `unresolved` is resolved before the marker
    /// is read.
    pub(in crate::gameplay_transport) async fn reconcile_pending_world_items<
        A: super::super::spell_access_facts::CurrentSpellAccessOwner + Sync,
    >(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        access: &A,
    ) -> NativeCastDispatch {
        let Some(mut permit) = self.spell_lane_permit().await else {
            return NativeCastDispatch::Pending;
        };
        let original = {
            let states = self.spell_states.lock().await;
            states
                .pending_world_items
                .iter()
                .find(|p| p.is_for(actor, session))
                .map(|p| (p.intent, p.command.command_id().get()))
        };
        let Some((intent, command)) = original else {
            return NativeCastDispatch::NotApplicable;
        };
        self.cast_world_items_inner(
            actor,
            session,
            command,
            &intent,
            access,
            true,
            Some(&mut permit),
        )
        .await
    }
    /// Only this Item lane returns NotApplicable. A missing wire Item-instance
    /// handle remains unavailable; it is never supplied by target position.
    pub(in crate::gameplay_transport) async fn cast_world_items<
        A: super::super::spell_access_facts::CurrentSpellAccessOwner + Sync,
    >(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        intent: &SpellCastIntent,
        access: &A,
    ) -> NativeCastDispatch {
        self.cast_world_items_inner(actor, session, command_id, intent, access, false, None)
            .await
    }
    /// The resolver of a world-item attempt parked in `unresolved` (ARCH-SPELL-LOCK-2 §1.6). It
    /// resumes the writer's retained retry path with the original attempt: the AlreadyCommitted
    /// branch when committed, the `Applied` branch in a new commit window when not. An attempt
    /// the pass does not install or release goes back into `unresolved`. The caller moved the
    /// attempt into the caster's marker.
    pub(in crate::gameplay_transport) async fn resolve_parked_world_items(
        &self,
        permit: &mut SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> &'static str {
        let original = super::native_combat_cast::parked_original(
            &self.spell_states.lock().await.pending_world_items,
            actor,
            session,
        );
        let Some((command, intent)) = original else {
            return "consumed";
        };
        let access = self.refresh_spell_access(actor, session).await;
        let dispatch = self
            .cast_world_items_inner(
                actor,
                session,
                command,
                &intent,
                &access,
                true,
                Some(permit),
            )
            .await;
        let leftover = {
            let mut states = self.spell_states.lock().await;
            super::PendingSpellMarker::take_attempt(&mut states.pending_world_items, actor, session)
        };
        if let Some(attempt) = leftover {
            permit
                .open_commit_window(attempt, UnresolvedSpellCommit::park_world_item)
                .park();
        }
        super::native_combat_cast::dispatch_outcome_token(&dispatch)
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "the lane is the caller's when it resolves a parked attempt"
    )]
    async fn cast_world_items_inner<
        A: super::super::spell_access_facts::CurrentSpellAccessOwner + Sync,
    >(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        intent: &SpellCastIntent,
        access: &A,
        reconcile_only: bool,
        lane: Option<&mut SpellLanePermit>,
    ) -> NativeCastDispatch {
        use super::super::spell_access_facts::load_owned_cast_facts_in_transaction;
        use crate::durability::fresh_admission::FreshAdmissionStore;
        use crate::foundation::{CommandId, GameSessionState};
        let Some((spell, active_spell)) = self.spells.source_indexed(intent.spell) else {
            return NativeCastDispatch::NotApplicable;
        };
        let key = match &spell.execution {
            Execution::Conjure { .. } => "conjure",
            Execution::NativeProfile(profile) => {
                profile.spell()["execution"]["native_behavior"]["key"]
                    .as_str()
                    .unwrap_or("")
            }
            _ => return NativeCastDispatch::NotApplicable,
        };
        if !matches!(key, "random_item_grant" | "conjure" | "tile_item_operation") {
            return NativeCastDispatch::NotApplicable;
        }
        let rejected = || NativeCastDispatch::Outcome(SpellCastOutcome::rejected());
        if !active_spell || command_id == 0 {
            return rejected();
        }
        // Rune V1 supplies no exact ItemInstance/generation/slot handle. Refuse
        // rather than inventing a charge donor, corpse or Chameleon target.
        if !matches!(key, "random_item_grant" | "conjure") {
            return NativeCastDispatch::Outcome(SpellCastOutcome {
                disposition: SpellCastDisposition::NotAvailable,
                vitals: None,
            });
        }
        let Some(active) = self.active_generation else {
            return rejected();
        };
        let Some(content) = active.native_gameplay() else {
            return rejected();
        };
        if self.qualified_room.is_none() {
            return rejected();
        }
        let Ok((current, _)) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(session)
            .await
        else {
            return NativeCastDispatch::Pending;
        };
        if current.session_state() != GameSessionState::Active {
            return rejected();
        }
        let Ok(character) =
            crate::domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes())
        else {
            return rejected();
        };
        let Ok(record) = self
            .root
            .read_current_character(self.character, character)
            .await
        else {
            return NativeCastDispatch::Pending;
        };
        let fence = CurrentCharacterGameplayFence {
            character_id: character,
            game_session_id: session,
            connection_generation: current.current_connection_generation(),
            character_lease_generation: current.current_character_lease().generation(),
            runtime_scope: current.current_runtime_scope(),
            scope_ownership_generation: current.current_scope_generation(),
            expected_character_revision: record.revision,
        };
        let Ok(command) = CommandId::new(command_id).map(|id| CommandRef::new(session, id)) else {
            return rejected();
        };
        // Lock order (§1.2): the lane before any Channel guard.
        let mut acquired;
        let permit = match lane {
            Some(permit) => permit,
            None => {
                let Some(permit) = self.spell_lane_permit().await else {
                    return NativeCastDispatch::Pending;
                };
                acquired = permit;
                &mut acquired
            }
        };
        let mut runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
        if runtime.player_control_facts(actor, session).is_err()
            || runtime.owner_fence().is_err()
            || runtime.content_pin().server_artifact_digest() != content.source_digest()
        {
            return rejected();
        }
        let Ok(attacker) = crate::foundation::CharacterId::decode(character.as_bytes()) else {
            return rejected();
        };
        match super::native_combat_cast::replay(
            &mut runtime,
            &mut states,
            actor,
            attacker,
            fence.character_lease_generation,
            command,
            intent,
        ) {
            Ok(Some(_)) => {
                return NativeCastDispatch::Outcome(SpellCastOutcome {
                    disposition: SpellCastDisposition::Cast,
                    vitals: super::observe_vitals(&runtime, &states, actor, session),
                });
            }
            Ok(None) => {}
            Err(_) => return rejected(),
        }
        // The marker (§1.4) stays for the whole pass; the pass holds the attempt itself.
        let retained = match states
            .pending_world_items
            .iter_mut()
            .find(|p| p.is_for(actor, session))
        {
            Some(marker) => {
                if marker.intent != *intent || marker.command != command {
                    return NativeCastDispatch::Pending;
                }
                match marker.attempt.take() {
                    Some(attempt) => Some(attempt),
                    // The lane is held and holds nothing, so no pass or parked record owns it.
                    None => return NativeCastDispatch::Pending,
                }
            }
            None => {
                if reconcile_only {
                    return NativeCastDispatch::NotApplicable;
                }
                if states.pending_world_items.try_reserve(1).is_err() {
                    return rejected();
                }
                states.pending_world_items.push(super::PendingSpellMarker {
                    actor,
                    session,
                    command,
                    intent: *intent,
                    attempt: None,
                });
                None
            }
        };
        let intent = *intent;
        let Ok(pass) = self.root.try_issue_semantic_pass() else {
            super::PendingSpellMarker::settle(
                &mut states.pending_world_items,
                actor,
                session,
                command,
                intent,
                retained,
                permit.has_unresolved(),
            );
            return NativeCastDispatch::Pending;
        };
        // The descriptor is outside the bounded callback before any committing
        // await. Deadline cancellation cannot discard the original occurrence:
        // before the commit window it stays in the context, after it the window
        // parks it in the lane.
        let objects = self.door.lock().await;
        let mut context = (
            self,
            access,
            permit,
            Some((runtime, states, objects)),
            retained,
        );
        let result=pass.run_with_context(&mut context,move|holder,deadline,ctx|Box::pin(async move{
            let (owner,access,permit,guards,pending)=ctx;
            let active=owner.active_generation.ok_or(DurabilityError::Unavailable)?;
            let content=active.native_gameplay().ok_or(DurabilityError::Unavailable)?;
            let room=owner.qualified_room.ok_or(DurabilityError::Unavailable)?;
            let spell=owner.spells.indexed(intent.spell).ok_or(DurabilityError::Unavailable)?;
            let item_fence=CurrentCharacterItemFence{character_id:fence.character_id,game_session_id:session,
                connection_generation:fence.connection_generation,character_lease_generation:fence.character_lease_generation,
                runtime_scope:fence.runtime_scope,scope_ownership_generation:fence.scope_ownership_generation};
            let mut tx=items::begin_spell_owner_transaction(holder,deadline).await?;
            let authority=items::assert_spell_item_authority_in_transaction(&mut tx,permit,owner.root,owner.character,
                owner.holder,&item_fence,command,content.source_digest()).await.map_err(|_|DurabilityError::Unavailable)?;
            // S (§1.1): the guarded span, through the staged installation and the verdict.
            let verdict={
                let (runtime,states,objects)=match guards.as_mut(){
                    Some((runtime,states,objects))=>(&mut **runtime,&mut **states,&**objects),
                    None=>return Err(DurabilityError::Unavailable),
                };
                let state=states.get(runtime,actor,session).ok_or(DurabilityError::Unavailable)?;
                let owned=load_owned_cast_facts_in_transaction(&mut tx,owner.root,owner.character,owner.holder,&item_fence,
                    command,runtime,actor,state,active,*access,owner.owner_now().get()).await.map_err(|_|DurabilityError::Unavailable)?;
                if pending.is_none(){
                    let decision=oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(nonce(
                        b"oteryn:world-item-draw:v1",actor,session,command_id,&[]));
                    let stream=oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(content.source_digest());
                    let mut ordinal=0_u64;let mut invalid_draw=false;
                    let mut draw=|minimum,maximum|{
                        let sample=oteryn_simulation_determinism::deterministic_decision_u64(&stream,decision,
                            "spell.cast.draw",ordinal);
                        let next=ordinal.checked_add(1);
                        match (sample,next){
                            (Ok(sample),Some(next))=>{ordinal=next;crate::spell::uniform_draw(sample,minimum,maximum)},
                            _=>{invalid_draw=true;minimum},
                        }
                    };
                    let prepared=match prepare_item_grant(&mut tx,&authority,runtime,states,room,objects,content,owned.clone(),
                        spell,intent,fence,command,owner.owner_now(),&mut draw).await{
                        Ok(p)=>p,Err(disposition)=>return Ok(NativeCastDispatch::Outcome(SpellCastOutcome{disposition,vitals:None})),
                    };
                    if invalid_draw{return Err(DurabilityError::Unavailable);}
                    *pending=Some(prepared);
                }
                let attempt=pending.as_mut().ok_or(DurabilityError::Unavailable)?;
                let reconnect=crate::durability::admission_journal::spell_reconnect::prove_pending_spell_reconnect(
                    &mut tx,&authority,&attempt.fence,&item_fence).await?;
                if states.get(runtime,actor,session)!=Some(&attempt.before)
                    || runtime.read_actor_position(actor).map_err(|_|DurabilityError::Unavailable)?!=attempt.position
                    || !current_facts_match(attempt,&owned,reconnect.as_ref())
                    || attempt.fence.character_lease_generation!=fence.character_lease_generation {
                    return Err(DurabilityError::Unavailable);
                }
                states.presentations.as_mut().ok_or(DurabilityError::Unavailable)?
                    .hold_prepared_before_sql(runtime,&attempt.presentation,&attempt.batch)
                    .map_err(|_|DurabilityError::Unavailable)?;
                states.presentations.as_ref().ok_or(DurabilityError::Unavailable)?
                    .validate_prepared(runtime,&attempt.presentation,&attempt.batch)
                    .map_err(|_|DurabilityError::Unavailable)?;
                attempt.player.validate_current(runtime,states).map_err(|_|DurabilityError::Unavailable)?;
                runtime.reserve_spell_batch(&mut attempt.physical).map_err(|_|DurabilityError::Unavailable)?;
                owned.check_current_access(spell,owner.owner_now().get())
                    .map_err(|_|items::SpellItemError::Rejected(WORLD_ITEM_NEW_GRANT_REFUSED))
            };
            // Release after S: the COMMIT and the post-commit transaction hold only the lane.
            *guards=None;
            let attempt=pending.as_ref().ok_or(DurabilityError::Unavailable)?;
            let outcome=match items::apply_spell_items_in_transaction_guarded(&mut tx,&authority,&attempt.request,verdict).await{
                Ok(outcome)=>outcome,
                Err(items::SpellItemError::Rejected(WORLD_ITEM_NEW_GRANT_REFUSED))=>{
                    // The writer holds the exact-command lock and found no historical receipt.
                    // Only a successful rollback proves this attempt cannot later COMMIT.
                    tx.rollback().await.map_err(|_|DurabilityError::Unavailable)?;
                    let mut runtime=owner.runtime.lock().await;
                    let mut states=owner.spell_states.lock().await;
                    if attempt.physical.will_apply(){
                        runtime.release_definitely_uncommitted_spell_batch(&attempt.physical)
                            .map_err(|_|DurabilityError::Unavailable)?;
                    }
                    states.presentations.as_mut().ok_or(DurabilityError::Unavailable)?
                        .release_definitely_uncommitted(&attempt.presentation);
                    *pending=None;
                    return Ok(NativeCastDispatch::Outcome(SpellCastOutcome{disposition:SpellCastDisposition::Rejected,
                        vitals:super::observe_vitals(&runtime,&states,actor,session)}));
                }
                Err(_)=>return Err(DurabilityError::Unavailable),
            };
            let historical=matches!(outcome,SpellItemTransactionOutcome::AlreadyCommitted(_));
            let attempt=pending.take().ok_or(DurabilityError::Unavailable)?;
            let mut window=permit.open_commit_window(attempt,UnresolvedSpellCommit::park_world_item);
            // A historical outcome is marked before any fallible follow-up, so each later
            // failure parks the attempt and keeps the lane fenced.
            if historical{window.mark_already_committed();}
            let training=async{
                let attempt=window.attempt();
                let formula=content.training_formula().ok_or(DurabilityError::Unavailable)?;
                Ok::<_,DurabilityError>(match attempt.training.request(){
                    Some(request)=>Some(crate::durability::character_build::prepare_character_build_in_transaction(
                        owner.root,&mut tx,owner.character,owner.holder,attempt.fence,request.clone(),formula)
                        .await.map_err(|_|DurabilityError::Unavailable)?),None=>None,
                })
            }.await;
            let training=match training{
                Ok(training)=>training,
                Err(error)=>{
                    if !historical && let Ok(attempt)=window.reclaim_uncommitted(){*pending=Some(attempt);}
                    return Err(error);
                },
            };
            let committed=match outcome{
                SpellItemTransactionOutcome::Applied(descriptor)=>{
                    match items::stage_spell_owner_commit(&mut tx,&authority,descriptor,None,None,deadline).await{
                        Ok(staged)=>crate::durability::spell_owner_commit::commit_spell_owner_transaction(tx,staged,&mut window).await,
                        Err(_)=>Err(DurabilityError::Unavailable),
                    }
                },
                SpellItemTransactionOutcome::AlreadyCommitted(descriptor)=>{
                    let proof=items::reconcile_spell_owner_commit_in_transaction(&mut tx,&authority,descriptor,None)
                        .await.map_err(|_|DurabilityError::Unavailable);drop(tx);proof
                },
            };
            let proof=match committed{
                Ok(proof)=>proof,
                Err(error)=>{
                    // Proven uncommitted: the attempt goes back to the marker. Otherwise the
                    // window parks it.
                    if let Ok(attempt)=window.reclaim_uncommitted(){*pending=Some(attempt);}
                    return Err(error);
                },
            };
            // From here every early return parks the attempt through the window's Drop.
            let training=training.map(|p|p.after_commit(&proof)).transpose().map_err(|_|DurabilityError::InvalidStoredState)?;
            let receipt=training.as_ref().map(|r|match r{BuildCommitOutcome::Committed(r)|BuildCommitOutcome::AlreadyCommitted(r)=>r});
            let mut runtime=owner.runtime.lock().await;
            let mut states=owner.spell_states.lock().await;
            // Phase 1: every fallible check borrows the attempt; nothing moves.
            let checked={
                let attempt=window.attempt();
                let anchor=attempt.batch.anchor.as_ref().ok_or(DurabilityError::InvalidStoredState)?;
                attempt.training.check_install(&attempt.before,&attempt.paid,anchor,receipt)
                    .map_err(|_|DurabilityError::InvalidStoredState)?;
                let preview=attempt.training.qualified_preview(&attempt.paid).map_err(|_|DurabilityError::InvalidStoredState)?;
                attempt.player.check_rebind_training(&runtime,&states,&preview).map_err(|_|DurabilityError::InvalidStoredState)?;
                let checked=check_owner_batch(&runtime,&states,&attempt.physical,Some(&attempt.player))
                    .map_err(|_|DurabilityError::InvalidStoredState)?;
                if states.presentations.is_none(){return Err(DurabilityError::InvalidStoredState);}
                checked
            };
            // Phase 2: no awaited work, allocation, new random draw or fallible branch.
            let mut attempt=window.install();
            attempt.training.install_into(&mut attempt.paid);
            attempt.player.install_rebind_training(attempt.paid);
            let receipt=install_owner_batch(&mut runtime,&mut states,attempt.physical,Some(attempt.player),checked);
            if receipt.applied && let Some(presentations)=states.presentations.as_mut(){
                presentations.install_preflighted(attempt.presentation,&receipt);
            }
            Ok(NativeCastDispatch::Outcome(SpellCastOutcome{disposition:SpellCastDisposition::Cast,
                vitals:super::observe_vitals(&runtime,&states,actor,session)}))
        })).await;
        let (_, _, permit, guards, leftover) = &mut context;
        *guards = None;
        let parked = permit.has_unresolved();
        let leftover = leftover.take();
        let mut states = self.spell_states.lock().await;
        super::PendingSpellMarker::settle(
            &mut states.pending_world_items,
            actor,
            session,
            command,
            intent,
            leftover,
            parked,
        );
        match result {
            Ok(result) => result,
            Err(_) => NativeCastDispatch::Pending,
        }
    }
}
