//! DAILY-1: real admitted commands and shrine USE. The spell lane holds position
//! stable through a claim; the native Root independently fences every DB pass.
use super::ComposedFreshAdmission;
use super::capabilities::OfferedCapability;
use super::connection::{FreshAdmissionAuthority, UseCommand, UseOutcome};
use crate::content::daily_reward::DailyRewardContent;
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::character_inbox::RewardClaimInboxMintOutput;
use crate::durability::daily_reward_claim::{
    DailyRewardClaimError, DailyRewardClaimOutcome, DailyRewardClaimRequest,
};
use crate::durability::daily_reward_delivery::DailyRewardDeliveryPlan;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass};
use crate::foundation::{CommandId, CommandRef, ExactActorRef, GameSessionId};
use oteryn_protocol_oteryn::daily_reward as wire;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct DailyRewardSessions(
    std::sync::Mutex<BTreeMap<GameSessionId, (ExactActorRef, u64)>>,
);

/// No-capability clients retain the existing empty rejection payload.
pub(super) async fn dispatch(
    authority: &impl FreshAdmissionAuthority,
    actor: ExactActorRef,
    command: UseCommand,
    command_type: u32,
    payload: &[u8],
) -> Option<Vec<u8>> {
    let claim = if command_type == wire::COMMAND_TYPE_DAILY_REWARD_CLAIM {
        match wire::DailyRewardClaim::decode(payload) {
            Ok(claim) => Some(claim),
            Err(_) => return refusal(3),
        }
    } else {
        if wire::decode_daily_reward_query(payload).is_err() {
            return None;
        }
        None
    };
    authority.daily_reward(actor, command, claim).await
}
fn refusal(disposition: u8) -> Option<Vec<u8>> {
    wire::DailyRewardResult {
        disposition,
        transaction_id: None,
    }
    .encode()
    .ok()
}

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    let mut result = base.to_vec();
    if available {
        result.push(OfferedCapability {
            id: wire::CAPABILITY_DAILY_REWARD_V1,
            requires: &[],
        });
    }
    result
}

pub(super) fn result(
    command_type: u32,
    payload: Option<Vec<u8>>,
) -> (crate::foundation::CommandStatus, Vec<u8>) {
    use crate::foundation::CommandStatus;
    match payload {
        Some(bytes) => (
            if command_type == wire::COMMAND_TYPE_DAILY_REWARD_CLAIM
                && !matches!(bytes.get(1), Some(1 | 2))
            {
                CommandStatus::Rejected
            } else {
                CommandStatus::Accepted
            },
            bytes,
        ),
        None => (CommandStatus::Rejected, Vec::new()),
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn daily_content(&self) -> Option<(&NativeGameplayState, DailyRewardContent)> {
        let native = self.active_generation?.native_gameplay()?;
        Some((native, DailyRewardContent::load(native)?))
    }

    // At most nine tiles and the map's own per-tile entry bound. A shrine is
    // recognized by the admitted production definition, never by client bytes.
    async fn daily_in_reach(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        content: &DailyRewardContent,
    ) -> bool {
        use crate::map::view::MapFacts;
        let mut runtime = self.runtime.lock().await;
        if runtime.player_control_facts(actor, session).is_err()
            || self.states.spell_states.lock().await.is_dead(actor)
            || super::item_ref_admission::pinned_index(&runtime, self.active_generation).is_none()
        {
            return false;
        }
        if let Some(at) = super::bundle_map_position::bundle_map_position(
            &mut runtime,
            self.movement_cells,
            actor,
        ) {
            let Some(index) =
                super::item_ref_admission::pinned_index(&runtime, self.active_generation)
            else {
                return false;
            };
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (Some(x), Some(y)) = (
                        at.at.x.checked_add(dx).and_then(|x| u16::try_from(x).ok()),
                        at.at.y.checked_add(dy).and_then(|y| u16::try_from(y).ok()),
                    ) else {
                        continue;
                    };
                    let Ok(floor) = i8::try_from(at.at.floor) else {
                        return false;
                    };
                    let pos = crate::map::overlay::TilePos { x, y, floor };
                    if let Some(tile) = at.map.overlay.base().tile(x, y, floor) {
                        for (ordinal, (id, _)) in tile
                            .ids()
                            .iter()
                            .zip(tile.depths())
                            .filter(|(_, depth)| **depth == 0)
                            .enumerate()
                        {
                            let Ok(ordinal) = u8::try_from(ordinal) else {
                                break;
                            };
                            if let Some(facts) = at.map.facts.base_entry(pos, ordinal, *id)
                                && let oteryn_protocol_oteryn::world_map::MapDefinition::Item(
                                    reference,
                                ) = facts.definition
                                && let Some(definition) =
                                    super::item_look::typed_definition(index, reference)
                                && content.shrine_source_ids.iter().any(|id| {
                                    definition.production_key == format!("oteryn:item.tibia.i{id}")
                                })
                            {
                                return true;
                            }
                        }
                    }
                }
            }
            return false;
        }
        if self.movement_cells.scope().coordinate_frame.as_str()
            != content.entry_shrine.coordinate_frame
        {
            return false;
        }
        let shrine_cell = crate::content::LogicalCell {
            x: content.entry_shrine.x,
            y: content.entry_shrine.y,
            z: i32::from(content.entry_shrine.floor),
        };
        if self
            .movement_cells
            .index()
            .lookup(self.movement_cells.scope(), shrine_cell)
            != Ok(crate::content::CollisionClass::Walkable)
        {
            return false;
        }
        let Ok(position) = runtime.borrow_movement_position().read(actor) else {
            return false;
        };
        let p = position.position();
        p.floor == content.entry_shrine.floor
            && p.x.abs_diff(content.entry_shrine.x) <= 1
            && p.y.abs_diff(content.entry_shrine.y) <= 1
    }

    pub(super) async fn open_daily_shrine(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        let (_, content) = self.daily_content()?;
        if target.placement != content.entry_shrine.placement.as_bytes() {
            return None;
        }
        let fence = command.item_fence?;
        let reachable = target.expected_revision == 0
            && self
                .daily_in_reach(actor, command.game_session_id, &content)
                .await;
        if reachable {
            let runtime = self.runtime.lock().await;
            let mut sessions = self.states.daily_rewards.0.lock().ok()?;
            sessions.retain(|session, (actor, _)| {
                runtime.player_control_facts(*actor, *session).is_ok()
            });
            sessions.insert(
                command.game_session_id,
                (actor, fence.connection_generation.get()),
            );
        }
        Some(UseOutcome {
            disposition: if reachable {
                UseDisposition::Committed
            } else {
                UseDisposition::TooFar
            },
            ..UseOutcome::rejected()
        })
    }

    pub(super) async fn daily_command(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        claim: Option<wire::DailyRewardClaim>,
    ) -> Option<Vec<u8>> {
        let (native, content) = self.daily_content()?;
        let fence = command.item_fence?;
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).ok()?,
        );
        let _permit = self.spell_lane_permit().await?;
        let reachable = self
            .daily_in_reach(actor, command.game_session_id, &content)
            .await;
        let opened = self
            .states
            .daily_rewards
            .0
            .lock()
            .ok()?
            .get(&command.game_session_id)
            .map(|(_, generation)| *generation)
            == Some(fence.connection_generation.get());
        let view = match self
            .root
            .read_daily_reward_view(self.character, self.holder, fence, command_ref)
            .await
        {
            Ok(view) => view,
            Err(_) => return refusal(5),
        };
        let Some(claim) = claim else {
            return wire::DailyRewardWall {
                lane_day: view.lane_day,
                streak: view.streak,
                jokers: view.jokers,
                claimed: view.receipt.is_some(),
                claim_enabled: opened && reachable && view.receipt.is_none(),
            }
            .encode()
            .ok();
        };
        // The canonical pick binding survives reconnect and is independent of
        // random output identities. The native mint reservation binds those too.
        let mut digest = Sha256::new()
            .chain_update(b"oteryn:daily-reward-picks:v1\0")
            .chain_update(content.revision.as_bytes());
        for pick in &claim.picks {
            digest.update((pick.definition.len() as u32).to_be_bytes());
            digest.update(pick.definition.as_bytes());
            digest.update(pick.count.to_be_bytes());
        }
        let binding = digest.finalize().into();
        if let Some(receipt) = view.settled.or(view.receipt) {
            return refusal(if receipt.request_binding == binding {
                2
            } else {
                6
            });
        }
        if !opened || !reachable {
            return refusal(4);
        }
        let vocation = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.has_pending_spell_commit(actor, command.game_session_id) {
                return refusal(7);
            }
            states
                .get(&runtime, actor, command.game_session_id)?
                .character_facts()
                .vocation
        };
        let Some(mut plan) = plan(native, &content, view.lane_day, &claim, vocation) else {
            return refusal(3);
        };
        plan.content_revision = native
            .source_digest()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let transaction_id = crate::operator_files::uuid_v7().ok()?;
        for _ in &plan.outputs {
            plan.item_instance_ids
                .push(crate::operator_files::uuid_v7().ok()?);
        }
        let request = DailyRewardClaimRequest {
            command: command_ref,
            reset_epoch: view.epoch,
            transaction_id,
            request_binding: binding,
        };
        match self
            .root
            .commit_daily_reward_claim(self.character, self.holder, fence, request, plan)
            .await
        {
            Ok(DailyRewardClaimOutcome::Committed(receipt)) => wire::DailyRewardResult {
                disposition: 1,
                transaction_id: Some(receipt.transaction_id),
            }
            .encode()
            .ok(),
            Ok(DailyRewardClaimOutcome::AlreadyClaimed(_)) => refusal(2),
            Err(DailyRewardClaimError::BindingMismatch) => refusal(6),
            Err(DailyRewardClaimError::InvalidRequest) => refusal(3),
            Err(DailyRewardClaimError::AuthorityRejected) => refusal(7),
            Err(_) => refusal(5),
        }
    }
}

fn plan(
    native: &NativeGameplayState,
    content: &DailyRewardContent,
    day: u8,
    claim: &wire::DailyRewardClaim,
    vocation: crate::spell::Vocation,
) -> Option<DailyRewardDeliveryPlan> {
    let mut plan = DailyRewardDeliveryPlan {
        lane_day: day,
        content_revision: String::new(),
        definition_revision: content.revision.clone(),
        outputs: Vec::new(),
        charges: Vec::new(),
        item_instance_ids: Vec::new(),
    };
    let total: u32 = claim.picks.iter().map(|p| p.count).sum();
    let expected = match day {
        1 | 2 => 5,
        4 => 10,
        6 => 1,
        3 | 5 | 7 => 0,
        _ => return None,
    };
    if total != expected {
        return None;
    }
    for pick in &claim.picks {
        let (facts, charges) = if matches!(day, 1 | 2 | 4) {
            let record = content
                .potion_source_ids
                .iter()
                .filter_map(|id| native.item_policy_for_source_id(*id))
                .map(|p| p.record())
                .find(|r| r.production_definition.production_key == pick.definition)?;
            // The unpromoted vocation has no mana lane; no level filter is applied.
            if vocation == crate::spell::Vocation::None
                && record
                    .production_definition
                    .production_key
                    .ends_with(".i268")
            {
                return None;
            }
            let definition = TypedDefinitionRef {
                family: "Item".into(),
                production_key: pick.definition.clone(),
                revision_ref: record.production_definition.revision_ref.clone(),
            };
            (
                super::item_ref_admission::generation_item_facts(native, &definition).ok()?,
                None,
            )
        } else {
            let item = content
                .items
                .iter()
                .find(|i| i.kind == "training" && i.key == pick.definition)?;
            (
                original_item(&item.key, &content.revision),
                Some(item.charges),
            )
        };
        plan.outputs.push(RewardClaimInboxMintOutput {
            source_line: plan.outputs.len() as u32,
            item: facts,
            quantity: pick.count,
            key_number: None,
        });
        plan.charges.push(charges);
    }
    if day == 5 {
        let item = content.items.iter().find(|i| i.kind == "converter")?;
        plan.outputs.push(RewardClaimInboxMintOutput {
            source_line: 0,
            item: original_item(&item.key, &content.revision),
            quantity: 1,
            key_number: None,
        });
        plan.charges.push(Some(item.charges));
    }
    Some(plan)
}
fn original_item(key: &str, revision: &str) -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        definition: TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: revision.into(),
        },
        weight: Some(0),
        stack: ItemStackClass::NonStackable,
        container_capacity: None,
        container_slot_equip_pattern: false,
    }
}

macro_rules! authority_methods {
    () => {
        fn daily_reward_available(&self) -> bool {
            false
        }
        fn daily_reward(
            &self,
            _actor: crate::foundation::ExactActorRef,
            _command: $crate::gameplay_transport::connection::UseCommand,
            _claim: Option<oteryn_protocol_oteryn::daily_reward::DailyRewardClaim>,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn daily_reward_available(&self) -> bool {
            self.daily_content().is_some()
        }
        async fn daily_reward(
            &self,
            actor: ExactActorRef,
            command: connection::UseCommand,
            claim: Option<oteryn_protocol_oteryn::daily_reward::DailyRewardClaim>,
        ) -> Option<Vec<u8>> {
            self.daily_command(actor, command, claim).await
        }
    };
}
pub(super) use composed_methods;
