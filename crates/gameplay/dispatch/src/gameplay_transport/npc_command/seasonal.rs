//! Christmas present requests use the permanent source flag and normal MINT owner.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::ItemStackClass;
use crate::durability::reward_claim_mint::{
    RewardClaimMintContent, RewardClaimMintError, RewardClaimMintOutcome, RewardClaimMintRequest,
    RewardClaimRefusal, SANTA_PRESENT_CLAIM, SantaPresentRequest, santa_failed_script_claim,
    santa_present_items,
};
use crate::foundation::{CommandId, CommandRef};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub(super) struct Pending {
    character: crate::domain::CharacterId,
    request: SantaPresentRequest,
    intent: NpcTalkIntent,
    context: (u64, u64, String),
}
fn clear(admission: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(state) = sessions.get_mut(&session)
    {
        state.seasonal = None;
    }
}
async fn committed_reply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    intent: &NpcTalkIntent,
    context: &(u64, u64, String),
    bag_only: bool,
) -> NpcTalkOutcome {
    if bag_only {
        clear(admission, session);
        let mut outcome = NpcTalkOutcome::only(NpcIntentDisposition::Ok);
        outcome.inventory_changed = true;
        return outcome;
    }
    let runtime = admission.runtime.lock().await;
    let current = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        &runtime,
        actor,
        session,
        intent,
    ) == Some(context.clone());
    let mut outcome = NpcConversations::bank_reply(intent, "Merry Christmas!".into());
    outcome.inventory_changed = true;
    if !current && let Some(conversation) = outcome.conversation.as_mut() {
        conversation.open = None;
    }
    clear(admission, session);
    outcome
}

// Crystal data/npclib/npc.lua:13-19 checks a left word boundary only;
// "presents" and "present?" qualify, but any alphanumeric prefix blocks it.
fn present_topic(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("present")
        && !lower
            .match_indices("present")
            .any(|(index, _)| index != 0 && lower.as_bytes()[index - 1].is_ascii_alphanumeric())
}

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<(u64, u64, String)> {
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    // Crystal data-global/npc/santa_claus.lua:103-105 accepts the present topic.
    (context.2 == "oteryn:npc.santa_claus" && present_topic(&intent.text)).then_some(context)
}

fn draw(
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    key: &str,
    upper: u64,
) -> Option<u8> {
    for index in 0..1024 {
        let value = deterministic_decision_u64(root, occurrence, key, index).ok()?;
        if value < u64::MAX - u64::MAX % upper {
            return Some((value % upper + 1) as u8);
        }
    }
    None
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    context: (u64, u64, String),
) -> NpcTalkOutcome {
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let unknown = || NpcTalkOutcome {
        unknown: true,
        ..rejected()
    };
    let (Some(fence), Ok(id), Some(native)) = (
        command.item_fence,
        CommandId::new(command.command_id),
        admission
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay),
    ) else {
        return rejected();
    };
    let Some(_permit) = admission.spell_lane_permit().await else {
        return rejected();
    };
    if !admission
        .states
        .player_trades
        .physical_service(command.game_session_id)
    {
        return rejected();
    }
    let _revision = admission
        .revision_sequencer
        .acquire(fence.character_id)
        .await;
    let Ok(Some(current)) = admission.current_quest_fence(command.game_session_id).await else {
        return rejected();
    };
    if current.item_fence() != fence {
        return rejected();
    }
    let (chance, group, definitions) = {
        let runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.lock().await;
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            ) != Some(context.clone())
            || !runtime
                .bound_attacker_lease(actor, command.game_session_id)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
        {
            return rejected();
        }
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let mut digest = Sha256::new();
        digest.update(b"oteryn:santa-present:v1");
        digest.update(command.game_session_id.as_bytes());
        digest.update(command.command_id.to_be_bytes());
        digest.update(fence.character_id.as_bytes());
        let digest = digest.finalize();
        let mut occurrence_bytes = [0_u8; 16];
        occurrence_bytes.copy_from_slice(&digest[..16]);
        let occurrence = DecisionOccurrenceId::from_bytes(occurrence_bytes);
        let (Some(chance), Some(group)) = (
            draw(&root, occurrence, "santa.present.chance", 100),
            draw(&root, occurrence, "santa.present.group", 10),
        ) else {
            return rejected();
        };
        let Some(index) =
            super::super::item_ref_admission::pinned_index(&runtime, admission.active_generation)
        else {
            return rejected();
        };
        let mut definitions = Vec::new();
        // The nil branch still executes addItem6510 before the source error.
        // This bag-only plan does not synthesize an empty reward group.
        let ids = santa_present_items(chance, group).unwrap_or(&[]);
        {
            for id in std::iter::once(&6510).chain(ids) {
                let key = format!("oteryn:item.tibia.i{id}");
                let Some(revision) = index.revision(&key) else {
                    return rejected();
                };
                definitions.push(TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key,
                    revision_ref: revision.into(),
                });
            }
        }
        (chance, group, definitions)
    };
    let reference = CommandRef::new(command.game_session_id, id);
    let gift = if definitions.is_empty() {
        None
    } else {
        let Ok(Some(current)) = admission
            .root
            .read_character_backpack(admission.character, fence.character_id)
            .await
        else {
            return rejected();
        };
        let facts = |definition: &TypedDefinitionRef| {
            super::super::item_ref_admission::generation_item_facts(native, definition)
        };
        let (Ok(item), Ok(backpack)) =
            (facts(&definitions[0]), facts(&current.backpack.definition))
        else {
            return rejected();
        };
        let mut contents = Vec::new();
        let mut count = 1;
        for definition in &definitions[1..] {
            let Ok(item) = facts(definition) else {
                return rejected();
            };
            if matches!(
                definition.production_key.as_str(),
                "oteryn:item.tibia.i2992" | "oteryn:item.tibia.i3599"
            ) {
                count = 10;
            }
            let quantity = if matches!(item.stack, ItemStackClass::Stackable { .. }) {
                count
            } else {
                1
            };
            contents.push(RewardClaimMintContent {
                item,
                fluid_subtype: None,
                charges_remaining: None,
                quantity,
                key_number: None,
                text: None,
            });
        }
        let revision = format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        Some(RewardClaimMintRequest {
            command: reference,
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: if chance <= 90 {
                    SANTA_PRESENT_CLAIM.into()
                } else {
                    santa_failed_script_claim(reference)
                },
                revision_ref: "oteryn:santa-present.v1".into(),
            },
            source_placement: "oteryn:npc.santa_claus".into(),
            item,
            quantity: 1,
            contents,
            root_text: None,
            root_charges_remaining: None,
            root_fluid_subtype: None,
            backpack,
            backpack_outputs: false,
            content_revision: revision,
            ruleset_revision: "oteryn:santa-present.v1".into(),
            sim_revision: "oteryn:santa-present.v1".into(),
            achievement: None,
            quest_transition: None,
            key_number: None,
        })
    };
    {
        let runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.lock().await;
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            ) != Some(context.clone())
            || !admission
                .states
                .player_trades
                .physical_service(command.game_session_id)
        {
            return rejected();
        }
    }
    let weights = super::super::item_ref_admission::NativeItemWeights(native)
        .with_wheel(admission.premium_coordinator);
    let request = SantaPresentRequest {
        command: reference,
        reset_epoch: admission.reset_epoch,
        chance,
        group,
        gift,
    };
    {
        let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
            return rejected();
        };
        let state = sessions.entry(command.game_session_id).or_default();
        if state.seasonal.is_some() || state.cake.is_some() {
            return unknown();
        }
        state.seasonal = Some(Pending {
            character: fence.character_id,
            request: request.clone(),
            intent: intent.clone(),
            context: context.clone(),
        });
    }
    match admission
        .root
        .claim_santa_present(
            admission.character,
            admission.holder,
            fence,
            request,
            &weights,
        )
        .await
    {
        Ok(RewardClaimMintOutcome::Committed(_) | RewardClaimMintOutcome::AlreadyCommitted(_)) => {
            committed_reply(
                admission,
                actor,
                command.game_session_id,
                &intent,
                &context,
                chance > 90,
            )
            .await
        }
        Err(RewardClaimMintError::Unavailable(_)) => unknown(),
        Err(error) => {
            clear(admission, command.game_session_id);
            if matches!(
                error,
                RewardClaimMintError::Refused(RewardClaimRefusal::AlreadyClaimed)
            ) {
                NpcConversations::bank_reply(&intent, "You can't get other present.".into())
            } else {
                rejected()
            }
        }
    }
}

pub(in crate::gameplay_transport) async fn resume(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
) -> Option<NpcTalkOutcome> {
    let pending = admission
        .states
        .npc_conversations
        .sessions
        .lock()
        .ok()?
        .get(&command.game_session_id)?
        .seasonal
        .clone()?;
    if pending.request.command.command_id().get() != command.command_id {
        return None;
    }
    let unknown = || NpcTalkOutcome {
        unknown: true,
        ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
    };
    let Ok(Some(current)) = admission.current_quest_fence(command.game_session_id).await else {
        return Some(unknown());
    };
    if current.character_id != pending.character {
        return Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected));
    }
    match admission
        .root
        .reconcile_santa_present(
            admission.character,
            admission.holder,
            current.item_fence(),
            pending.request.clone(),
        )
        .await
    {
        Ok(Some(_)) => Some(
            committed_reply(
                admission,
                actor,
                command.game_session_id,
                &pending.intent,
                &pending.context,
                pending.request.chance > 90,
            )
            .await,
        ),
        Ok(None) => {
            clear(admission, command.game_session_id);
            Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected))
        }
        Err(_) => Some(unknown()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn w05d_all_normal_santa_items_have_materializable_native_facts() -> Result<(), String> {
        let world = crate::foundation::WorldId::decode(&[
            1, 155, 140, 0, 0, 42, 112, 0, 128, 0, 0, 0, 0, 0, 0, 42,
        ])
        .map_err(|error| format!("{error:?}"))?;
        let (controller, _) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let generation = controller.active().ok_or("normal active generation")?;
        let native = generation
            .native_gameplay()
            .ok_or("normal native provider")?;
        let mut items = std::collections::BTreeSet::from([6510]);
        for group in 1..=10 {
            items.extend(
                santa_present_items(1, group)
                    .ok_or("source normal group")?
                    .iter()
                    .copied(),
            );
        }
        assert_eq!(items.len(), 16);
        for item in items {
            let definition = TypedDefinitionRef {
                family: "Item".into(),
                production_key: format!("oteryn:item.tibia.i{item}"),
                revision_ref: "definition-r1".into(),
            };
            let facts =
                super::super::super::item_ref_admission::generation_item_facts(native, &definition)
                    .map_err(|error| format!("Santa item{item}: {error:?}"))?;
            assert!(facts.weight.is_some(), "Santa item{item} weight");
            assert!(
                !matches!(facts.stack, ItemStackClass::Unknown),
                "Santa item{item} stack class"
            );
            if item == 6510 {
                assert_eq!(facts.container_capacity, Some(5));
            }
        }
        Ok(())
    }

    #[test]
    fn santa_topic_preserves_donor_left_boundary_and_plural_acceptance() {
        for accepted in ["present", "PRESENT", "a present?", "presents", "my_present"] {
            assert!(present_topic(accepted), "{accepted}");
        }
        for refused in ["represent", "present represent", "unrelated", "1present"] {
            assert!(!present_topic(refused), "{refused}");
        }
    }
}
