//! Explicit candidate parameter caster on the actual Channel/Character owners.
//! The original normalized v2 intent and private result survive unknown COMMIT.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::native_combat_cast::NativeCastDispatch;
use super::{
    ChannelSpellStates, PlayerBatchPreflight, SpellCastOutcome, commit_owner_batch,
    stage_player_batch,
};
use crate::ability::{AbilityOccurrence, RevisionSet};
use crate::content::{QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};
use crate::durability::character_build::{BuildCommitOutcome, BuildOccurrence};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::spell_items_abi::*;
use crate::durability::{DurabilityError, spell_item_transaction as items};
use crate::foundation::{
    ChannelRuntimeV1, CommandRef, ExactActorRef, GameSessionId, StagedSpellBatch,
};
use crate::spell::cast::{PlayerSpellState, prepare_native_callback_owner_cast_with_caster};
use crate::spell::combat_batch::OwnerCombatBatch;
use crate::spell::harmony::HarmonyMultiplier;
use crate::spell::mana_training::PreparedPlayerTraining;
use crate::spell::native_house_movement::{HouseList, HouseMovementPlan};
use crate::spell::owned_cast_facts::OwnedCastFacts;
use crate::spell::{Execution, OperationalCastFacts, SpellDefinition};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::actor_spell_v2::*;
use oteryn_simulation_determinism::SemanticTimeMicros;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};

const MAX_PENDING_PARAMETERS: usize = 256;

pub(super) fn named_player_spell(spell: &SpellDefinition) -> bool {
    matches!(
        spell.name.as_str(),
        "Heal Friend" | "Nature's Embrace" | "Restore Balance"
    ) && spell
        .authored
        .as_ref()
        .is_some_and(|p| p.header.targeting.parameter == "player_name")
        && matches!(
            spell.execution,
            Execution::Effects(_) | Execution::AbilityVariants(_)
        )
        && spell.needs_target
        && !spell.aggressive
}
#[allow(
    clippy::large_enum_variant,
    reason = "transient owner result; boxing would add an allocation to the owner turn"
)]
pub(super) enum NamedPlayerResolution {
    Found(QualifiedNamedPlayer),
    Absent(&'static str),
}

/// Private construction requires the durable name and current source access
/// reads above. The proposal remains bound to that physical SQL transaction.
pub(in crate::gameplay_transport) struct QualifiedNamedPlayer {
    actor: ExactActorRef,
    session: GameSessionId,
    position: crate::foundation::MovementPositionSnapshot,
    transaction: String,
    command: CommandRef,
    content: [u8; 32],
}
impl QualifiedNamedPlayer {
    pub(in crate::gameplay_transport) fn actor(&self) -> ExactActorRef {
        self.actor
    }
    pub(in crate::gameplay_transport) async fn validate_in_transaction(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        runtime: &ChannelRuntimeV1,
        authority: &items::SpellItemAuthority,
    ) -> Result<(), items::SpellItemError> {
        items::check_transaction(tx, authority).await?;
        let xid: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
            .fetch_one(&mut **tx)
            .await?;
        if xid != self.transaction
            || self.command != authority.command()
            || self.content != authority.compatible_content_digest()
            || self.content != runtime.content_pin().server_artifact_digest()
            || runtime
                .positioned_player_for_session(self.session)
                .map_err(|_| items::SpellItemError::Rejected("named actor owner"))?
                != Some((self.actor, self.position))
            || self.position.context() != runtime.pinned_movement_context()
        {
            return Err(items::SpellItemError::Rejected(
                "named actor transaction changed",
            ));
        }
        Ok(())
    }
}

/// A source name is a query only. Current admission, access and physical owners
/// independently qualify the actor inside the same caster transaction.
pub(super) async fn resolve_named_player_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &items::SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    intent: &ParameterSpellCastIntent,
) -> Result<NamedPlayerResolution, SpellCastDisposition> {
    let admissions =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(root.clone());
    let targets = root
        .lookup_spell_player_names_in_transaction(
            tx,
            authority,
            intent.parameter.as_deref().unwrap_or(""),
        )
        .await
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
    if targets.len() > 1 {
        return Ok(NamedPlayerResolution::Absent("name_is_ambiguous"));
    }
    let Some(target) = targets.first() else {
        return Ok(NamedPlayerResolution::Absent(
            "player_with_this_name_is_not_online",
        ));
    };
    let Some(session) = target.session_candidate() else {
        return Ok(NamedPlayerResolution::Absent(
            "player_with_this_name_is_not_online",
        ));
    };
    let current = admissions
        .current_session_in_transaction(tx, session)
        .await
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
    if current.session_state() != crate::foundation::GameSessionState::Active
        || current.current_transport().is_none()
        || current.current_control_loss_epoch().is_some()
        || current.current_character_lease().character_id().as_bytes()
            != target.character_id().as_bytes()
    {
        return Ok(NamedPlayerResolution::Absent(
            "player_with_this_name_is_not_online",
        ));
    }
    if current.current_runtime_scope() != authority.runtime_scope()
        || current.current_scope_generation().get() != authority.scope_generation()
        || authority.runtime_scope()
            != crate::foundation::RuntimeScopeRefV1::channel(
                runtime.binding().world_id(),
                runtime.binding().channel_id(),
            )
        || authority.compatible_content_digest() != runtime.content_pin().server_artifact_digest()
    {
        return Err(SpellCastDisposition::NotAvailable);
    }
    let target_group = crate::durability::spell_familiar_group::read_current_group_in_transaction(
        tx,
        authority,
        &admissions,
        session,
    )
    .await
    .map_err(|_| SpellCastDisposition::NotAvailable)?;
    let caster_group = crate::durability::spell_familiar_group::read_current_group_in_transaction(
        tx,
        authority,
        &admissions,
        authority.game_session_id(),
    )
    .await
    .map_err(|_| SpellCastDisposition::NotAvailable)?;
    if target_group.access() && !caster_group.access() {
        return Ok(NamedPlayerResolution::Absent("source_no_error"));
    }
    let Some((actor, position)) = runtime
        .positioned_player_for_session(session)
        .map_err(|_| SpellCastDisposition::NotAvailable)?
    else {
        return Err(SpellCastDisposition::NotAvailable);
    };
    if position.context() != runtime.pinned_movement_context() {
        return Err(SpellCastDisposition::NotAvailable);
    }
    let state = states
        .get(runtime, actor, session)
        .ok_or(SpellCastDisposition::NotAvailable)?;
    if state.vitals().health == 0 {
        return Ok(NamedPlayerResolution::Absent("source_no_error"));
    }
    let transaction: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut **tx)
        .await
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
    Ok(NamedPlayerResolution::Found(QualifiedNamedPlayer {
        actor,
        session,
        position,
        transaction,
        command: authority.command(),
        content: authority.compatible_content_digest(),
    }))
}

pub(in crate::gameplay_transport) struct ParameterCastDispatch {
    pub(crate) cast: NativeCastDispatch,
    pub(crate) result: Option<ParameterSpellCastResult>,
}
impl ParameterCastDispatch {
    fn bare(cast: NativeCastDispatch) -> Self {
        Self { cast, result: None }
    }
}
#[derive(Debug)]
pub(crate) struct PreparedParameterCast {
    actor: ExactActorRef,
    session: GameSessionId,
    intent: ParameterSpellCastIntent,
    fence: CurrentCharacterGameplayFence,
    before: PlayerSpellState,
    paid: Option<PlayerSpellState>,
    position: crate::foundation::MovementPositionSnapshot,
    owned: OwnedCastFacts,
    batch: OwnerCombatBatch,
    request: Option<SpellItemTransactionRequest>,
    definition: crate::durability::item_mint::TypedDefinitionRef,
    result: ParameterSpellCastResult,
    result_bytes: Vec<u8>,
    training: Option<PreparedPlayerTraining>,
    physical: Option<StagedSpellBatch>,
    player: Option<PlayerBatchPreflight>,
    relocation: Option<crate::foundation::QualifiedSpellRelocation>,
}
fn nonce(
    tag: &[u8],
    actor: ExactActorRef,
    session: GameSessionId,
    command: u64,
    extra: &[u8],
) -> [u8; 16] {
    let h = Sha256::new()
        .chain_update(tag)
        .chain_update(actor.placement_identity())
        .chain_update(session.as_bytes())
        .chain_update(command.to_be_bytes())
        .chain_update(extra)
        .finalize();
    let mut b = [0; 16];
    b.copy_from_slice(&h[..16]);
    b[6] = (b[6] & 15) | 0x70;
    b[8] = (b[8] & 63) | 0x80;
    b
}
fn current_facts_match(
    p: &PreparedParameterCast,
    current: &OwnedCastFacts,
    reconnect: Option<
        &crate::durability::admission_journal::spell_reconnect::SpellReconnectTransition,
    >,
) -> bool {
    let mut binding = p.owned.binding().clone();
    let mut equipment = p.owned.equipment().clone();
    if current.binding().connection_generation != binding.connection_generation {
        let Ok(connection) =
            crate::foundation::ConnectionGeneration::new(current.binding().connection_generation)
        else {
            return false;
        };
        if !reconnect.is_some_and(|proof| {
            proof.matches(
                p.fence.connection_generation,
                connection,
                p.batch.command,
                p.fence.character_id,
                p.fence.character_lease_generation,
                p.fence.runtime_scope,
                p.fence.scope_ownership_generation.get(),
            )
        }) {
            return false;
        }
        binding.connection_generation = current.binding().connection_generation;
    }
    let expected_build = if current.binding().character_revision == binding.character_revision {
        p.owned.durable_build()
    } else if binding.character_revision.checked_add(1)
        == Some(current.binding().character_revision)
    {
        let Some(request) = p.training.as_ref().and_then(|t| t.request()) else {
            return false;
        };
        binding.character_revision = current.binding().character_revision;
        equipment.character_revision = binding.character_revision;
        &request.after
    } else {
        return false;
    };
    current.binding() == &binding
        && current.equipment() == &equipment
        && current.durable_build() == expected_build
}
pub(super) fn source_feedback(
    reason: &str,
    effect: Option<&str>,
) -> Result<Option<PrivateSpellFeedback>, SpellCastDisposition> {
    let text = match reason {
        "source_no_error" => "No error.",
        "player_with_this_name_is_not_online" => "A player with this name is not online.",
        "exiva_protected" => {
            "The character you are trying to find with Exiva is currently protected from your spell."
        }
        "name_is_ambiguous" => "Player name is ambiguous.",
        "not_possible" => "Sorry, not possible.",
        "not_enough_room" => "There is not enough room.",
        _ => return Err(SpellCastDisposition::NotAvailable),
    };
    Ok(Some(PrivateSpellFeedback {
        text: text.into(),
        effect: match effect {
            None => PrivateSpellEffect::None,
            Some("poff") => PrivateSpellEffect::Poff,
            _ => return Err(SpellCastDisposition::NotAvailable),
        },
    }))
}
#[allow(clippy::too_many_arguments)]
async fn prepare_parameter(
    tx: &mut Transaction<'_, Postgres>,
    authority: &items::SpellItemAuthority,
    admissions: &crate::durability::fresh_admission::FreshAdmissionStore,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &crate::world_runtime::LocalObjectRuntime,
    content: &NativeGameplayState,
    owned: OwnedCastFacts,
    spell: &SpellDefinition,
    intent: ParameterSpellCastIntent,
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    now: SemanticTimeMicros,
) -> Result<PreparedParameterCast, SpellCastDisposition> {
    let Execution::NativeProfile(profile) = &spell.execution else {
        return Err(SpellCastDisposition::Rejected);
    };
    if intent.intent.target != SpellTarget::None || intent.intent.aim_at_target || spell.aggressive
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let b = owned.binding();
    let actor = b.actor;
    let session = command.game_session_id();
    if b.session != session
        || b.content_digest != content.source_digest()
        || authority.command() != command
        || authority.character_id_bytes() != b.character
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let before = states
        .get(runtime, actor, session)
        .ok_or(SpellCastDisposition::Rejected)?
        .clone();
    let position = runtime
        .read_actor_position(actor)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let p = position.position();
    let tile = crate::spell::world_execution::qualified_combat_tile(room, runtime, objects, p)
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
        in_protection_zone: tile.protection_zone(),
        target_tile_solid: None,
        target_tile_creature: None,
    };
    let caster = owned
        .caster(&before, spell, content, HarmonyMultiplier::ONE, now.get())
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
    let mut common =
        prepare_native_callback_owner_cast_with_caster(&before, spell, &operational, now, &caster)?;
    let behavior = &profile.spell()["execution"]["native_behavior"];
    let params = &behavior["parameters"];
    let mut relocation_proof = None;
    let mut cooldown_only = false;
    let mut result = ParameterSpellCastResult {
        disposition: SpellCastDisposition::Cast,
        feedback: None,
        editor: None,
    };
    match behavior["key"].as_str() {
        Some("locate_message") if params["source"].as_str() == Some("online_player") => {
            let plan = crate::spell::find_person_execution::plan_find_person_in_transaction(
                tx,
                authority,
                admissions,
                runtime,
                profile,
                actor,
                position,
                intent.parameter.as_deref().unwrap_or(""),
            )
            .await
            .map_err(|_| SpellCastDisposition::NotAvailable)?;
            match plan {
                HouseMovementPlan::Message {
                    text,
                    effect: "magic_blue",
                } => {
                    result.feedback = Some(PrivateSpellFeedback {
                        text,
                        effect: PrivateSpellEffect::MagicBlue,
                    })
                }
                HouseMovementPlan::Refused {
                    reason,
                    effect,
                    cast_succeeds,
                    start_cooldown,
                } => {
                    result.disposition = if cast_succeeds {
                        SpellCastDisposition::Cast
                    } else {
                        SpellCastDisposition::Rejected
                    };
                    result.feedback = source_feedback(reason, effect)?;
                    if start_cooldown && !cast_succeeds {
                        common =
                            crate::spell::cast::prepare_locate_name_failure_owner_cast_with_caster(
                                &before,
                                spell,
                                &operational,
                                now,
                                &caster,
                            )?;
                        cooldown_only = true;
                    }
                }
                _ => return Err(SpellCastDisposition::Rejected),
            }
        }
        Some("house_access") if params["action"].as_str() == Some("edit_list") => {
            if intent.parameter.is_some() {
                return Err(SpellCastDisposition::Rejected);
            }
            match crate::spell::house_execution::open_aleta_editor_in_transaction(
                tx, authority, room, runtime, profile, actor, position,
            )
            .await
            {
                Ok(e) => {
                    result.editor = Some(SpellHouseEditor {
                        editor_id: e.editor_id,
                        house_key: e.house_key,
                        list: match e.list {
                            HouseList::Guest => HouseEditorList::Guest,
                            HouseList::Subowner => HouseEditorList::Subowner,
                            HouseList::Door(n) => HouseEditorList::Door(
                                std::num::NonZeroU32::new(n)
                                    .ok_or(SpellCastDisposition::Rejected)?,
                            ),
                        },
                        ownership_revision: std::num::NonZeroU64::new(e.ownership_revision)
                            .ok_or(SpellCastDisposition::Rejected)?,
                        acl_revision: e.acl_revision,
                        text: e.text,
                    })
                }
                Err(crate::spell::house_execution::Error::NotInHouse) => {
                    result.disposition = SpellCastDisposition::Rejected;
                    if params["no_house_failure_silent"].as_bool() != Some(true) {
                        result.feedback = source_feedback("not_possible", Some("poff"))?;
                    }
                }
                Err(crate::spell::house_execution::Error::Storage(
                    items::SpellItemError::Rejected("House ACL edit denied"),
                )) => {
                    result.disposition = if params["denied_cast_succeeds"].as_bool() == Some(true) {
                        SpellCastDisposition::Cast
                    } else {
                        SpellCastDisposition::Rejected
                    };
                    if result.disposition != SpellCastDisposition::Cast {
                        result.feedback = source_feedback("not_possible", Some("poff"))?;
                    }
                }
                Err(_) => return Err(SpellCastDisposition::NotAvailable),
            }
        }
        Some("vertical_move") => {
            if params["mode"].as_str() == Some("rope_up") && intent.parameter.is_some() {
                return Err(SpellCastDisposition::Rejected);
            }
            let outcome = crate::spell::world_execution::prepare_relocation(
                profile,
                intent.parameter.as_deref().unwrap_or(""),
                runtime,
                room,
                objects,
                actor,
                position,
                tx,
                authority,
            )
            .await
            .map_err(|_| SpellCastDisposition::NotAvailable)?;
            match outcome {
                crate::spell::world_execution::WorldRelocationOutcome::Move(proof) => {
                    relocation_proof = Some(proof);
                    result.feedback = Some(PrivateSpellFeedback {
                        text: String::new(),
                        effect: PrivateSpellEffect::Teleport,
                    });
                }
                crate::spell::world_execution::WorldRelocationOutcome::RopeTeleportDenied => {
                    result.feedback = Some(PrivateSpellFeedback {
                        text: String::new(),
                        effect: PrivateSpellEffect::Poff,
                    });
                }
                crate::spell::world_execution::WorldRelocationOutcome::Refused { reason } => {
                    result.disposition = SpellCastDisposition::Rejected;
                    result.feedback = source_feedback(reason, Some("poff"))?;
                }
                _ => return Err(SpellCastDisposition::Rejected),
            }
        }
        Some("house_access") if params["action"].as_str() == Some("kick") => {
            let outcome = crate::spell::world_execution::prepare_house_kick(
                profile,
                intent.parameter.as_deref().unwrap_or(""),
                runtime,
                room,
                objects,
                admissions,
                actor,
                position,
                tx,
                authority,
            )
            .await
            .map_err(|_| SpellCastDisposition::NotAvailable)?;
            match outcome {
                crate::spell::world_execution::WorldRelocationOutcome::Move(proof) => {
                    relocation_proof = Some(proof)
                }
                crate::spell::world_execution::WorldRelocationOutcome::HouseTeleportDenied => (),
                crate::spell::world_execution::WorldRelocationOutcome::Refused { reason } => {
                    result.disposition = SpellCastDisposition::Rejected;
                    result.feedback = source_feedback(reason, Some("poff"))?;
                }
                _ => return Err(SpellCastDisposition::Rejected),
            }
        }
        Some("house_access") => return Err(SpellCastDisposition::NotAvailable),
        _ => return Err(SpellCastDisposition::Rejected),
    }
    let result_bytes =
        encode_parameter_spell_cast_result(&result).map_err(|_| SpellCastDisposition::Rejected)?;
    let intent_bytes =
        encode_parameter_spell_cast_intent(&intent).map_err(|_| SpellCastDisposition::Rejected)?;
    let identity = &spell
        .authored
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?
        .header
        .identity;
    let definition = crate::durability::item_mint::TypedDefinitionRef {
        family: "Spell".into(),
        production_key: identity.key.clone(),
        revision_ref: identity.revision.clone(),
    };
    let paid_success = result.disposition == SpellCastDisposition::Cast || cooldown_only;
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let revision = RevisionSet::new(
        "ruleset:spell-native-r20",
        &format!("content:{}", hex(&content.source_digest())),
        "world:spell-pve-candidate-r21",
        "formula:spell-p2-r20",
        "simulation:v1",
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    let occurrence = AbilityOccurrence::new(
        &format!(
            "parameter:{}",
            hex(&nonce(
                b"oteryn:parameter-occurrence:v2",
                actor,
                session,
                command.command_id().get(),
                &intent_bytes
            ))
        ),
        revision,
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    let mut batch=OwnerCombatBatch{caster:actor,attacker:crate::foundation::CharacterId::decode(&b.character).map_err(|_|SpellCastDisposition::Rejected)?,current_lease_generation:b.lease_generation,
        command,occurrence:occurrence.into(),anchor:paid_success.then(||common.anchor.clone()),now_ms:now.get()/1000,effects:Vec::new(),deferred:None,
        binding:serde_json::to_vec(&json!({"intent":intent_bytes,"result":result_bytes,"spell":profile.spell(),"before":format!("{before:?}"),"paid":format!("{:?}",common.next),"owned":format!("{owned:?}")})).map_err(|_|SpellCastDisposition::Rejected)?};
    // Consume the transaction-borrowing source proof while Item locks are held.
    // The Foundation seal and real reserved destination survive unknown COMMIT.
    let relocation = relocation_proof
        .map(|proof| crate::foundation::QualifiedSpellRelocation::bind(&mut batch, proof))
        .transpose()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let (request, training, physical, player, paid) = if paid_success {
        let formula = content
            .training_formula()
            .ok_or(SpellCastDisposition::NotAvailable)?;
        let training = before
            .prepare_paid_training(
                &common.next,
                &common.anchor,
                formula,
                BuildOccurrence::from_bytes(nonce(
                    b"oteryn:parameter-training:v2",
                    actor,
                    session,
                    command.command_id().get(),
                    &intent_bytes,
                ))
                .map_err(|_| SpellCastDisposition::Rejected)?,
                now.get(),
            )
            .map_err(|_| SpellCastDisposition::Rejected)?;
        let request = SpellItemTransactionRequest {
            command,
            spell: definition.clone(),
            catalog_digest: content.source_digest(),
            transaction_id: nonce(
                b"oteryn:parameter-cost:v2",
                actor,
                session,
                command.command_id().get(),
                &intent_bytes,
            ),
            event_id: nonce(
                b"oteryn:parameter-event:v2",
                actor,
                session,
                command.command_id().get(),
                &batch.binding,
            ),
            cost: crate::spell::companion_lifecycle::familiar_cost_binding(
                &before,
                &common.next,
                &common.anchor,
            )
            .map_err(|_| SpellCastDisposition::Rejected)?,
            caster_origin: Some(SourceCasterOrigin {
                actor,
                character_lease_generation: b.lease_generation,
            }),
            operations: Vec::new(),
            companion: None,
            direct_companion: None,
        };
        let physical = match &relocation {
            Some(seal) => runtime.stage_spell_batch_with_relocation(&batch, seal),
            None => runtime.stage_spell_batch(&batch),
        }
        .map_err(|_| SpellCastDisposition::Rejected)?;
        let player = stage_player_batch(runtime, states, &batch, Some(common.next.clone()))
            .map_err(|_| SpellCastDisposition::Rejected)?;
        (
            Some(request),
            Some(training),
            Some(physical),
            Some(player),
            Some(common.next),
        )
    } else {
        (None, None, None, None, None)
    };
    Ok(PreparedParameterCast {
        actor,
        session,
        intent,
        fence,
        before,
        paid,
        position,
        owned,
        batch,
        request,
        definition,
        result,
        result_bytes,
        training,
        physical,
        player,
        relocation,
    })
}
impl ChannelSpellStates {
    pub(crate) fn has_pending_parameters(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        self.pending_parameters
            .iter()
            .any(|p| p.actor == actor && p.session == session)
    }
}
impl super::super::ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn reconcile_pending_parameters_for_control_loss<
        A: super::super::spell_access_facts::CurrentSpellAccessOwner + Sync,
    >(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        access: &A,
    ) -> Option<ParameterCastDispatch> {
        let original = {
            let states = self.spell_states.lock().await;
            states
                .pending_parameters
                .iter()
                .find(|p| p.actor == actor && p.session == session)
                .map(|p| (p.batch.command.command_id().get(), p.intent.clone()))
        };
        let (command, intent) = original?;
        let result = self
            .cast_parameters(actor, session, command, &intent, access)
            .await;
        if self
            .spell_states
            .lock()
            .await
            .has_pending_parameters(actor, session)
        {
            Some(ParameterCastDispatch::bare(NativeCastDispatch::Pending))
        } else {
            Some(result)
        }
    }
    /// Called only after the separate candidate capability was actually selected.
    /// Parameter bytes never reinterpret the accepted v1 command.
    pub(in crate::gameplay_transport) async fn cast_parameters<
        A: super::super::spell_access_facts::CurrentSpellAccessOwner + Sync,
    >(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        intent: &ParameterSpellCastIntent,
        access: &A,
    ) -> ParameterCastDispatch {
        use super::super::spell_access_facts::load_owned_cast_facts_in_transaction;
        use crate::durability::fresh_admission::FreshAdmissionStore;
        use crate::foundation::{CommandId, GameSessionState};
        let Some((spell, active_spell)) = self.spells.source_indexed(intent.intent.spell) else {
            return ParameterCastDispatch::bare(NativeCastDispatch::NotApplicable);
        };
        if named_player_spell(spell) || super::native_combat_cast::direct_companion_spell(spell) {
            return self
                .cast_named_player_parameters(actor, session, command_id, intent, access)
                .await;
        }
        let Execution::NativeProfile(profile) = &spell.execution else {
            return ParameterCastDispatch::bare(NativeCastDispatch::NotApplicable);
        };
        let key = profile.spell()["execution"]["native_behavior"]["key"]
            .as_str()
            .unwrap_or("");
        if !matches!(key, "locate_message" | "house_access" | "vertical_move") {
            return ParameterCastDispatch::bare(NativeCastDispatch::NotApplicable);
        }
        let rejected = || {
            ParameterCastDispatch::bare(NativeCastDispatch::Outcome(SpellCastOutcome::rejected()))
        };
        if !active_spell || command_id == 0 {
            return rejected();
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
            return ParameterCastDispatch::bare(NativeCastDispatch::Pending);
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
            return ParameterCastDispatch::bare(NativeCastDispatch::Pending);
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
        let mut runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
        if runtime.player_control_facts(actor, session).is_err()
            || runtime.owner_fence().is_err()
            || runtime.content_pin().server_artifact_digest() != content.source_digest()
        {
            return rejected();
        }
        // Only an actual committed physical receipt permits immediate replay.
        // The exact v2 bytes include the parameter, unlike the old v1 helper.
        let Ok(attacker) = crate::foundation::CharacterId::decode(character.as_bytes()) else {
            return rejected();
        };
        match runtime.retained_spell_batch(
            actor,
            attacker,
            fence.character_lease_generation,
            command,
        ) {
            Ok(Some(batch)) => {
                let replay = (|| {
                    let saved: serde_json::Value = serde_json::from_slice(&batch.binding).ok()?;
                    let bytes = encode_parameter_spell_cast_intent(intent).ok()?;
                    if saved["intent"] != serde_json::json!(bytes) {
                        return None;
                    }
                    let result_bytes: Vec<u8> =
                        serde_json::from_value(saved["result"].clone()).ok()?;
                    let result = decode_parameter_spell_cast_result(&result_bytes).ok()?;
                    let staged = runtime.stage_spell_batch(&batch).ok()?;
                    if staged.will_apply() {
                        return None;
                    }
                    commit_owner_batch(&mut runtime, &mut states, staged, None).ok()?;
                    Some(result)
                })();
                return match replay {
                    Some(result) => ParameterCastDispatch {
                        cast: NativeCastDispatch::Outcome(SpellCastOutcome {
                            disposition: result.disposition,
                            vitals: super::observe_vitals(&runtime, &states, actor, session),
                        }),
                        result: Some(result),
                    },
                    None => rejected(),
                };
            }
            Ok(None) => (),
            Err(_) => return rejected(),
        }
        let retained_index = states
            .pending_parameters
            .iter()
            .position(|p| p.actor == actor && p.session == session);
        if let Some(i) = retained_index {
            if states.pending_parameters[i].intent != *intent
                || states.pending_parameters[i].batch.command != command
            {
                return ParameterCastDispatch::bare(NativeCastDispatch::Pending);
            }
        } else if states.pending_parameters.len() >= MAX_PENDING_PARAMETERS
            || states.pending_parameters.try_reserve(1).is_err()
        {
            return rejected();
        }
        let retained = retained_index.map(|i| states.pending_parameters.remove(i));
        let Ok(pass) = self.root.try_issue_semantic_pass() else {
            if let Some(p) = retained {
                states.pending_parameters.push(p);
            }
            return ParameterCastDispatch::bare(NativeCastDispatch::Pending);
        };
        let intent = intent.clone();
        // The descriptor is outside the bounded callback before any committing
        // await. Deadline cancellation cannot discard the original occurrence.
        let mut context = (self, access, &mut *runtime, &mut *states, retained);
        let result=pass.run_with_context(&mut context,move|holder,deadline,ctx|Box::pin(async move{
            let (owner,access,runtime,states,pending)=ctx;
            let active=owner.active_generation.ok_or(DurabilityError::Unavailable)?;
            let content=active.native_gameplay().ok_or(DurabilityError::Unavailable)?;
            let room=owner.qualified_room.ok_or(DurabilityError::Unavailable)?;
            let spell=owner.spells.indexed(intent.intent.spell).ok_or(DurabilityError::Unavailable)?;
            let item_fence=CurrentCharacterItemFence{character_id:fence.character_id,game_session_id:session,
                connection_generation:fence.connection_generation,character_lease_generation:fence.character_lease_generation,
                runtime_scope:fence.runtime_scope,scope_ownership_generation:fence.scope_ownership_generation};
            let mut tx=items::begin_spell_owner_transaction(holder,deadline).await?;
            let authority=items::assert_spell_item_authority_in_transaction(&mut tx,owner.root,owner.character,
                owner.holder,&item_fence,command,content.source_digest()).await.map_err(|_|DurabilityError::Unavailable)?;
            let state=states.get(runtime,actor,session).ok_or(DurabilityError::Unavailable)?;
            let owned=load_owned_cast_facts_in_transaction(&mut tx,owner.root,owner.character,owner.holder,&item_fence,
                command,runtime,actor,state,active,*access,owner.owner_now().get()).await.map_err(|_|DurabilityError::Unavailable)?;
            if pending.is_none(){
                let identity=&spell.authored.as_ref().ok_or(DurabilityError::Unavailable)?.header.identity;
                let definition=crate::durability::item_mint::TypedDefinitionRef{family:"Spell".into(),production_key:identity.key.clone(),revision_ref:identity.revision.clone()};
                if let Some(history)=crate::durability::spell_parameter_result::read_parameter_result_in_transaction(&mut tx,&authority,&definition,&intent).await.map_err(|_|DurabilityError::Unavailable)?{
                    if !history.historical() || history.cost_transaction().is_some(){return Err(DurabilityError::Unavailable)}
                    let result=history.result().map_err(|_|DurabilityError::InvalidStoredState)?;
                    return Ok(ParameterCastDispatch{cast:NativeCastDispatch::Outcome(SpellCastOutcome{disposition:result.disposition,vitals:None}),result:Some(result)})
                }
                let objects=owner.door.lock().await;
                let prepared=match prepare_parameter(&mut tx,&authority,&FreshAdmissionStore::from_root(owner.root.clone()),runtime,states,room,&objects,content,owned.clone(),
                    spell,intent.clone(),fence,command,owner.owner_now()).await{
                    Ok(p)=>p,Err(disposition)=>return Ok(ParameterCastDispatch{cast:NativeCastDispatch::Outcome(SpellCastOutcome{disposition,vitals:None}),result:None}),
                };
                *pending=Some(prepared);
            }
            let attempt=pending.as_mut().ok_or(DurabilityError::Unavailable)?;
            let reconnect=crate::durability::admission_journal::spell_reconnect::prove_pending_spell_reconnect(&mut tx,&authority,&attempt.fence,&item_fence).await?;
            if states.get(runtime,actor,session)!=Some(&attempt.before)
                || runtime.read_actor_position(actor).map_err(|_|DurabilityError::Unavailable)?!=attempt.position
                || !current_facts_match(attempt,&owned,reconnect.as_ref())
                || attempt.fence.character_lease_generation!=fence.character_lease_generation {
                return Err(DurabilityError::Unavailable);
            }
            if let Some(player)=attempt.player.as_ref(){player.validate_current(runtime,states).map_err(|_|DurabilityError::Unavailable)?;}
            if let Some(physical)=attempt.physical.as_mut(){runtime.reserve_spell_batch(physical).map_err(|_|DurabilityError::Unavailable)?;}
            let private_result=attempt.result.clone();
            let existing=crate::durability::spell_parameter_result::read_parameter_result_in_transaction(&mut tx,&authority,&attempt.definition,&attempt.intent).await.map_err(|_|DurabilityError::Unavailable)?;
            if let Some(record)=&existing
                && record.bytes()!=attempt.result_bytes.as_slice(){return Err(DurabilityError::InvalidStoredState)}
            if existing.is_none()
                && let Some(editor)=&attempt.result.editor{
                    let presence=crate::spell::house_execution::current_house_presence(room,runtime,session,actor,attempt.position).map_err(|_|DurabilityError::Unavailable)?;
                    let list=match editor.list{HouseEditorList::Guest=>HouseList::Guest,HouseEditorList::Subowner=>HouseList::Subowner,HouseEditorList::Door(n)=>HouseList::Door(n.get())};
                    let present:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_editors WHERE editor_id=encode($1,'hex')::uuid)").bind(editor.editor_id.as_slice()).fetch_one(&mut *tx).await?;
                    if !present{
                    let observed=crate::durability::house_spell_acl::restage_editor_in_transaction(&mut tx,&authority,&presence,list,editor.editor_id).await.map_err(|_|DurabilityError::Unavailable)?;
                    if observed.house_key!=editor.house_key || observed.list!=list || observed.ownership_revision!=editor.ownership_revision.get() || observed.acl_revision!=editor.acl_revision || observed.text!=editor.text{return Err(DurabilityError::Unavailable)}
                    }
                }
            let Some(request)=attempt.request.as_ref() else{
                let record=crate::durability::spell_parameter_result::write_parameter_result_in_transaction(&mut tx,&authority,&attempt.definition,&attempt.intent,&attempt.result,None).await.map_err(|_|DurabilityError::Unavailable)?;
                if !record.historical(){
                    let commit=crate::durability::spell_owner_commit::stage_parameter_result_commit(&mut tx,&record,deadline).await?;
                    crate::durability::spell_owner_commit::commit_spell_owner_transaction(tx,commit).await?;
                } else {drop(tx);}
                let result=private_result; *pending=None;
                return Ok(ParameterCastDispatch{cast:NativeCastDispatch::Outcome(SpellCastOutcome{disposition:result.disposition,vitals:None}),result:Some(result)})
            };
            let outcome=items::apply_spell_items_in_transaction_guarded(&mut tx,&authority,request,
                ||owned.check_current_access(spell,owner.owner_now().get())
                    .map_err(|_|items::SpellItemError::Rejected("current new grant access")))
                .await.map_err(|_|DurabilityError::Unavailable)?;
            crate::durability::spell_parameter_result::write_parameter_result_in_transaction(&mut tx,&authority,&attempt.definition,&attempt.intent,&attempt.result,Some(request.transaction_id)).await.map_err(|_|DurabilityError::Unavailable)?;
            let formula=content.training_formula().ok_or(DurabilityError::Unavailable)?;
            let training=match attempt.training.as_ref().ok_or(DurabilityError::InvalidStoredState)?.request(){
                Some(request)=>Some(crate::durability::character_build::prepare_character_build_in_transaction(
                    owner.root,&mut tx,owner.character,owner.holder,CurrentCharacterGameplayFence{connection_generation:fence.connection_generation,..attempt.fence},request.clone(),formula)
                    .await.map_err(|_|DurabilityError::Unavailable)?),None=>None,
            };
            let proof=match outcome{
                SpellItemTransactionOutcome::Applied(descriptor)=>{
                    let pending=items::stage_spell_owner_commit(&mut tx,&authority,descriptor,None,None,deadline)
                        .await.map_err(|_|DurabilityError::Unavailable)?;
                    crate::durability::spell_owner_commit::commit_spell_owner_transaction(tx,pending).await?
                },
                SpellItemTransactionOutcome::AlreadyCommitted(descriptor)=>{
                    let proof=items::reconcile_spell_owner_commit_in_transaction(&mut tx,&authority,descriptor,None)
                        .await.map_err(|_|DurabilityError::Unavailable)?;drop(tx);proof
                },
            };
            let training=training.map(|p|p.after_commit(&proof)).transpose().map_err(|_|DurabilityError::InvalidStoredState)?;
            let receipt=training.as_ref().map(|r|match r{BuildCommitOutcome::Committed(r)|BuildCommitOutcome::AlreadyCommitted(r)=>r});
            // No awaited work, allocation or new random draw after real COMMIT.
            let mut paid=attempt.paid.take().ok_or(DurabilityError::InvalidStoredState)?;
            attempt.training.as_mut().ok_or(DurabilityError::InvalidStoredState)?.prepare_install(&attempt.before,&mut paid,
                attempt.batch.anchor.as_ref().ok_or(DurabilityError::InvalidStoredState)?,receipt)
                .map_err(|_|DurabilityError::InvalidStoredState)?;
            let mut player=attempt.player.take().ok_or(DurabilityError::InvalidStoredState)?;
            player.rebind_training(runtime,states,paid).map_err(|_|DurabilityError::InvalidStoredState)?;
            let physical=attempt.physical.take().ok_or(DurabilityError::InvalidStoredState)?;
            let receipt=commit_owner_batch(runtime,states,physical,Some(player)).map_err(|_|DurabilityError::InvalidStoredState)?;
            let _=receipt;
            *pending=None;
            Ok(ParameterCastDispatch{cast:NativeCastDispatch::Outcome(SpellCastOutcome{disposition:private_result.disposition, vitals:super::observe_vitals(runtime,states,actor,session)}),result:Some(private_result)})
        })).await;
        if let Some(p) = context.4.take() {
            context.3.pending_parameters.push(p);
        }
        match result {
            Ok(result) => result,
            Err(_) => ParameterCastDispatch::bare(NativeCastDispatch::Pending),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn replay_binding_distinguishes_same_spell_different_player_text() {
        let a = decode_parameter_spell_cast_intent(&[
            8, 2, 18, 4, 8, 1, 16, 1, 26, 3, b'B', b'o', b'b',
        ])
        .unwrap();
        let mut b = a.clone();
        b.parameter = Some("Bobby".into());
        assert_ne!(
            encode_parameter_spell_cast_intent(&a).unwrap(),
            encode_parameter_spell_cast_intent(&b).unwrap()
        );
        let (_runtime, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(90);
        assert_ne!(
            nonce(
                b"parameter-test",
                actor,
                session,
                1,
                &encode_parameter_spell_cast_intent(&a).unwrap()
            ),
            nonce(
                b"parameter-test",
                actor,
                session,
                1,
                &encode_parameter_spell_cast_intent(&b).unwrap()
            )
        );
    }
    #[test]
    fn exact_source_cancel_feedback_and_unknown_reason_refusal() {
        let ambiguous = source_feedback("name_is_ambiguous", Some("poff"))
            .unwrap()
            .unwrap();
        assert_eq!(ambiguous.text, "Player name is ambiguous.");
        assert_eq!(ambiguous.effect, PrivateSpellEffect::Poff);
        let protected = source_feedback("exiva_protected", None).unwrap().unwrap();
        assert_eq!(
            protected.text,
            "The character you are trying to find with Exiva is currently protected from your spell."
        );
        assert_eq!(protected.effect, PrivateSpellEffect::None);
        assert!(source_feedback("unqualified_source_message", None).is_err());
    }
}
