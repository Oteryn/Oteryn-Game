//! Source horse rental stays on command7 and the existing current NPC conversation.
use super::*;
use crate::durability::horse_rental::rules;
use crate::durability::horse_rental::{HorseRentalOutcome, HorseRentalRequest};
use crate::durability::item_fee_burn::{FeeBurnCause, FeeBurnRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::UseCommand;
#[derive(Clone)]
pub(super) struct State {
    context: (u64, u64, String),
    actor: ExactActorRef,
    npc: EntityRef,
    asked: bool,
}
pub(super) struct Step {
    state: State,
    text: String,
}
pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    if !rules::supported_npc(&context.2) {
        return None;
    }
    let text = intent.text.trim().to_ascii_lowercase();
    let prior = admission
        .states
        .npc_conversations
        .sessions
        .lock()
        .ok()?
        .get(&command.game_session_id)?
        .horse_rental
        .clone();
    let state = prior
        .filter(|s| s.context == context && s.actor == actor && s.npc == intent.npc_actor)
        .unwrap_or(State {
            context,
            actor,
            npc: intent.npc_actor,
            asked: false,
        });
    let word = |needle: &str| {
        text.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|part| part == needle)
    };
    if word("transport") {
        return None;
    }
    let action = if word("rent") || word("horses") {
        "rent"
    } else if state.asked && word("yes") {
        "yes"
    } else if state.asked && word("no") {
        "no"
    } else {
        return None;
    };
    let text = action.to_owned();
    Some(Step { state, text })
}
fn reply(intent: &NpcTalkIntent, text: &str, changed: bool) -> NpcTalkOutcome {
    NpcTalkOutcome {
        unknown: false,
        inventory_changed: changed,
        disposition: NpcIntentDisposition::Ok,
        conversation: Some(NpcConversation {
            open: Some((intent.npc_actor, NpcConversationState::Open)),
            lines: vec![text.into()],
        }),
        window: None,
    }
}
fn install(admission: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId, state: State) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(s) = sessions.get_mut(&session)
    {
        s.horse_rental = Some(state)
    }
}
pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let unknown = || NpcTalkOutcome {
        unknown: true,
        ..rejected()
    };
    let mut state = step.state;
    let Some(item) = command.item_fence else {
        return rejected();
    };
    let Some(permit) = admission.spell_lane_permit().await else {
        return unknown();
    };
    let mut slot = admission
        .revision_sequencer
        .acquire(item.character_id)
        .await;
    let Ok(Some(fence)) = admission.current_quest_fence(command.game_session_id).await else {
        return unknown();
    };
    if fence.item_fence() != item {
        return rejected();
    }
    {
        let runtime = admission.runtime.lock().await;
        let Ok(spells) = admission.states.spell_states.try_lock() else {
            return unknown();
        };
        if spells.is_dead(actor)
            || !spells.spell_idle(&runtime, actor, command.game_session_id)
            || !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|p| p.control_loss.is_none())
            || admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            ) != Some(state.context.clone())
        {
            return rejected();
        }
    }
    if step.text == "no" {
        state.asked = false;
        install(admission, command.game_session_id, state);
        return reply(&intent, "Maybe another time.", false);
    }
    if matches!(step.text.as_str(), "rent" | "horses") {
        match admission
            .root
            .horse_station_rent_available(admission.character, admission.world_id)
            .await
        {
            Ok(true) => {
                state.asked = true;
                install(admission, command.game_session_id, state);
                return reply(&intent, rules::ASK, false);
            }
            Ok(false) => return reply(&intent, rules::LOOSE, false),
            Err(_) => return unknown(),
        }
    }
    if !state.asked {
        return rejected();
    }
    let Ok(id) = CommandId::new(command.command_id) else {
        return rejected();
    };
    let derive = |part: u8| {
        super::super::npc_trade_command::derived(
            format!("oteryn:horse-rental:v1:{part}").as_bytes(),
            command.game_session_id,
            command.command_id,
        )
    };
    let backpack = match admission
        .root
        .read_character_backpack(admission.character, fence.character_id)
        .await
    {
        Ok(b) => b,
        Err(_) => return unknown(),
    };
    let coins = {
        let runtime = admission.runtime.lock().await;
        super::super::charm_progression::fee_change(
            admission,
            &runtime,
            backpack.as_ref(),
            [derive(3), derive(4)],
        )
    };
    let Some(coins) = coins else {
        return rejected();
    };
    let Some(native) = admission
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
    else {
        return rejected();
    };
    let request = HorseRentalRequest {
        command: CommandRef::new(command.game_session_id, id),
        npc: state.context.2.clone(),
        fee: FeeBurnRequest {
            cause: FeeBurnCause::HorseRental {
                occurrence: derive(0),
            },
            fee_gold_units: rules::RENTAL_GOLD,
            transaction_id: derive(1),
            event_id: derive(2),
            occurred_at_unix_ms: super::super::npc_trade_command::unix_ms_now(),
            server_build_id: "oteryn-game-server".into(),
            change: coins,
        },
    };
    let result = admission
        .root
        .commit_horse_rental(
            admission.character,
            admission.holder,
            fence,
            request,
            &permit,
            native,
        )
        .await;
    slot.observe_endgame_progression();
    drop(slot);
    let (text, changed) = match result {
        Ok(HorseRentalOutcome::Rented { .. }) => (rules::SUCCESS, true),
        Ok(HorseRentalOutcome::AlreadyOwned) => ("You already have a horse.", false),
        Ok(HorseRentalOutcome::HorsesLoose) => (rules::LOOSE, false),
        Ok(HorseRentalOutcome::InsufficientGold) => {
            ("You do not have enough money to rent a horse!", false)
        }
        Err(_error) => {
            #[cfg(test)]
            eprintln!("SRV7_HORSE_RENTAL_COMMIT_FAILED {_error:?}");
            return unknown();
        }
    };
    state.asked = false;
    install(admission, command.game_session_id, state);
    reply(&intent, text, changed)
}
