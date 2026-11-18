//! Source conditions are sampled by the existing live owner at command admission.
//! The sealed sample authorizes only that command, fence, callback and generation;
//! no runtime lock is acquired while the SQL reward transaction holds its locks.
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
use crate::foundation::{ChannelRuntimeV1, CommandRef, ExactActorRef};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_domain::domain::condition::{ConditionType, DotElement, StatusKind};
use oteryn_gameplay_foundation::foundation::condition::ConditionStore;
use oteryn_gameplay_quests::quest::reward_callbacks::ActorConditionRequirement;
use std::collections::BTreeSet;

#[derive(Debug)]
pub struct QuestActorConditionProof {
    fence: CurrentCharacterGameplayFence,
    cause: QuestCause,
    item_disguise: Option<(String, String)>,
    callback: String,
    generation: [u8; 32],
    active: BTreeSet<ActorConditionRequirement>,
    temporary_look: Option<u32>,
}
impl QuestActorConditionProof {
    /// The caller retains runtime -> current player-state locks while sampling.
    #[allow(
        clippy::too_many_arguments,
        reason = "binds independent live condition owners to one fenced source command"
    )]
    pub fn read_current(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        callback: &str,
        native: &NativeGameplayState,
        spell_conditions: &ConditionStore<String>,
        now: u64,
    ) -> Option<Self> {
        let lease = runtime
            .bound_attacker_lease(actor, fence.game_session_id)
            .ok()?;
        if lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || lease.generation() != fence.character_lease_generation
            || command.game_session_id() != fence.game_session_id
            || !spell_conditions.accepts_time(now)
        {
            return None;
        }
        let conditions = runtime
            .actor_conditions_at(actor, Some(fence.game_session_id), now)
            .ok()?;
        let active = conditions
            .visible_statuses(now)
            .chain(spell_conditions.visible_statuses(now))
            .filter_map(|(kind, _)| match kind {
                ConditionType::DamageOverTime(DotElement::Fire) => {
                    Some(ActorConditionRequirement::Fire)
                }
                ConditionType::Status(StatusKind::Drunk) => Some(ActorConditionRequirement::Drunk),
                _ => None,
            })
            .collect();
        Some(Self {
            fence,
            cause: QuestCause::Command(command),
            item_disguise: None,
            callback: callback.into(),
            generation: native.source_digest(),
            active,
            temporary_look: Self::current_temporary_look(
                runtime, actor, fence.game_session_id, native, spell_conditions, now,
            ),
        })
    }
    /// Reuse the displayed outfit qualification of both live condition owners.
    /// If the stores disagree, no source may authorize a temporary-look reward.
    pub fn current_temporary_look(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: crate::foundation::GameSessionId,
        native: &NativeGameplayState,
        spell: &ConditionStore<String>,
        now: u64,
    ) -> Option<u32> {
        use oteryn_gameplay_domain::domain::condition::ConflictKey;
        use oteryn_gameplay_foundation::foundation::condition::TemporaryDisplayedAppearance;

        let carrier = runtime.actor_conditions_at(actor, Some(session), now).ok()?;
        if native.source_digest() != runtime.content_pin().server_artifact_digest()
            || !spell.accepts_time(now)
            || !carrier.accepts_time(now)
        {
            return None;
        }
        let qualified = |appearance, look: Option<u32>| {
            let Some(TemporaryDisplayedAppearance::Outfit(selection)) = appearance else {
                return None;
            };
            let look = look?;
            native.spell_appearances()?.for_selection(look, selection)?;
            Some(look)
        };
        let spell_look = qualified(spell.displayed_temporary_appearance_at(now), spell.outfit_at(now));
        let carrier_look = qualified(carrier.displayed_temporary_appearance_at(now), carrier.outfit_at(now));
        if (spell.active_at(ConflictKey::Outfit, now).is_some() && spell_look.is_none())
            || (carrier.active_at(ConflictKey::Outfit, now).is_some() && carrier_look.is_none())
            || (spell_look.is_some() && carrier_look.is_some() && spell_look != carrier_look)
        {
            return None;
        }
        spell_look.or(carrier_look)
    }

    pub(super) fn permits_temporary_look(
        &self,
        fence: CurrentCharacterGameplayFence,
        request: &QuestTransitionRequest,
        generation: [u8; 32],
        look: u32,
    ) -> bool {
        self.permits(fence, request, generation, &[]) && self.temporary_look == Some(look)
    }

    /// STEP uses the same sealed occurrence proof as command-only condition gates.
    #[allow(
        clippy::too_many_arguments,
        reason = "binds the live condition sample to one fenced native STEP occurrence"
    )]
    pub fn read_world_current(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        cause: QuestCause,
        callback: &str,
        native: &NativeGameplayState,
        spell_conditions: &ConditionStore<String>,
        now: u64,
    ) -> Option<Self> {
        let QuestCause::Step(command) = cause else {
            return None;
        };
        let mut proof = Self::read_current(
            runtime,
            actor,
            fence,
            command,
            callback,
            native,
            spell_conditions,
            now,
        )?;
        proof.cause = cause;
        if let Some(
            oteryn_gameplay_foundation::foundation::condition::TemporaryDisplayedAppearance::Item(
                item,
            ),
        ) = spell_conditions.displayed_temporary_appearance_at(now)
            && item.artifact_digest() == native.source_digest()
            && item.revision_ref() == "definition-r1"
            && native
                .spell_appearances()?
                .for_item(item.definition_key(), item.revision_ref())
                .is_some()
        {
            let condition = spell_conditions
                .get(oteryn_gameplay_domain::domain::condition::ConflictKey::Outfit)?;
            proof.item_disguise = Some((
                item.definition_key().into(),
                condition.definition().key().into(),
            ));
        }
        Some(proof)
    }

    pub(crate) fn permits_world(
        &self,
        fence: CurrentCharacterGameplayFence,
        request: &QuestTransitionRequest,
        generation: [u8; 32],
        required: &oteryn_gameplay_quests::quest::world_callbacks::ItemDisguiseRequirement,
    ) -> bool {
        self.permits(fence, request, generation, &[])
            && self
                .item_disguise
                .as_ref()
                .is_some_and(|(item, condition)| {
                    item == &required.item && condition == &required.condition
                })
    }

    pub(super) fn permits(
        &self,
        fence: CurrentCharacterGameplayFence,
        request: &QuestTransitionRequest,
        generation: [u8; 32],
        required: &[ActorConditionRequirement],
    ) -> bool {
        self.fence.item_fence() == fence.item_fence()
            && request.cause == self.cause
            && request.transition_key == self.callback
            && generation == self.generation
            && required.iter().all(|kind| self.active.contains(kind))
    }
}
