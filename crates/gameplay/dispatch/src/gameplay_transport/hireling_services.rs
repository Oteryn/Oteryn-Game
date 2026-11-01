//! Dynamic Hirelings reuse command 7 conversations and command 8 trade windows.
//! Durable admission proves the current House, placement and owner's current entitlements.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::domain::hireling::{
    Skill, catalogue,
    dialogue::{Action, Dialogue, Topic},
};
use crate::durability::house_hireling::{HirelingSnapshot, npc_identity};
use oteryn_protocol_oteryn::npc_service::{MAX_OFFERS, NpcOffer, NpcTradeSide};
use sha2::{Digest, Sha256};

#[path = "hireling_services/purchase.rs"]
mod purchase;
#[path = "hireling_services/trade.rs"]
mod trade;
pub(in crate::gameplay_transport) use trade::{authorize_trade, resolve_trade};

type Context = (u64, u64, String);

pub(super) struct State {
    context: Context,
    pub(super) npc: EntityRef,
    pub(super) viewer: ExactActorRef,
    dialogue: Dialogue,
    snapshot: HirelingSnapshot,
    trade: Option<trade::Window>,
    stash: bool,
}

fn rejected() -> NpcTalkOutcome {
    NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
}
fn unknown() -> NpcTalkOutcome {
    NpcTalkOutcome {
        unknown: true,
        ..rejected()
    }
}

pub(super) fn bank_selected(
    owner: &NpcConversations,
    session: GameSessionId,
    context: &Context,
) -> bool {
    owner
        .sessions
        .lock()
        .ok()
        .and_then(|states| {
            states
                .get(&session)
                .and_then(|state| state.hireling.as_ref())
                .map(|state| {
                    state.context == *context
                        && state.dialogue.topic == Topic::Bank
                        && state.snapshot.has_skill(Skill::Banker as u16)
                })
        })
        .unwrap_or(false)
}

/// Returns `None` only to hand the already admitted Bank conversation to its native owner.
pub(super) async fn talk(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    context: Context,
) -> Option<NpcTalkOutcome> {
    let (Some(fence), Some(id)) = (command.item_fence, npc_identity(&context.2)) else {
        return Some(rejected());
    };
    let snapshot = match admission
        .root
        .load_house_hireling_service(
            admission.character,
            admission.holder,
            fence,
            *admission.channel_id.as_bytes(),
            id,
        )
        .await
    {
        Ok(Some(snapshot)) => snapshot,
        Ok(None) => return Some(rejected()),
        Err(_) => return Some(unknown()),
    };
    let (action, selected_topic) = {
        let runtime = admission.runtime.lock().await;
        if admission.states.npc_conversations.current_npc(
            &admission.states.chat,
            &runtime,
            actor,
            command.game_session_id,
            intent,
        ) != Some(context.clone())
        {
            return Some(rejected());
        }
        let Ok(mut states) = admission.states.npc_conversations.sessions.lock() else {
            return Some(rejected());
        };
        let session = states.entry(command.game_session_id).or_default();
        if session
            .hireling
            .as_ref()
            .is_none_or(|state| state.context != context)
        {
            session.hireling = Some(State {
                context: context.clone(),
                npc: intent.npc_actor,
                viewer: actor,
                dialogue: Dialogue::default(),
                snapshot: snapshot.clone(),
                trade: None,
                stash: false,
            });
        }
        let Some(state) = session.hireling.as_mut() else {
            return Some(rejected());
        };
        state.snapshot = snapshot.clone();
        let bound = if intent.text.trim().eq_ignore_ascii_case("surprise") {
            6
        } else {
            5
        };
        let Some(content) = admission
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return Some(rejected());
        };
        let Some(roll) = purchase::draw(content.source_digest(), command, id, bound) else {
            return Some(rejected());
        };
        let action = state.dialogue.step(
            &intent.text,
            snapshot.skills,
            snapshot.owner == *fence.character_id.as_bytes(),
            roll,
        );
        if action == Action::OpenStash {
            state.stash = true;
        }
        if matches!(action, Action::Close) {
            state.stash = false;
            state.trade = None;
        }
        (action, state.dialogue.topic)
    };
    let reply = |line: String| Some(NpcConversations::bank_reply(intent, line));
    match action {
        Action::Bank => None,
        Action::Reply(line) => {
            if let Topic::PackageConfirmation(index) = selected_topic {
                let package = &catalogue::PACKAGES[index];
                reply(format!("The {} package costs {} gold. Do you want to buy it?", package.name, package.gold))
            } else if selected_topic == Topic::Packages {
                reply(format!("Available imbuement packages: {}.", catalogue::PACKAGES.iter().map(|p| p.name).collect::<Vec<_>>().join(", ")))
            } else { reply(line.into()) }
        }
        Action::Services => reply("I can help you with bank, food, stash, goods, and imbuement packages. My master can also change my outfit or ask me to return to my lamp.".into()),
        Action::OpenStash => reply("Your supply stash is available while you remain here with me.".into()),
        Action::Trade { category, page } => Some(trade::open(admission, actor, command, intent, context, category, page).await),
        Action::Cook(plan) => Some(purchase::cook(admission, actor, command, intent, context, snapshot, plan).await),
        Action::Package(index) => Some(purchase::package(admission, actor, command, intent, context, snapshot, index).await),
        Action::ReturnLamp => match super::super::hireling::return_to_lamp(admission, actor, command, snapshot.id).await {
            Some(oteryn_protocol_oteryn::hireling::management::Reply::Committed)=>{
                let mut outcome=NpcConversations::bank_reply(intent,"As you wish!".into());outcome.inventory_changed=true;Some(outcome)
            }
            Some(_)=>Some(rejected()),None=>Some(unknown()),
        },
        Action::Outfit => reply("Choose one of your unlocked appearances in Hireling management.".into()),
        Action::Close => Some(NpcTalkOutcome { unknown: false, inventory_changed: false,
            disposition: NpcIntentDisposition::Ok,
            conversation: Some(NpcConversation { open: None, lines: vec!["Good bye!".into()] }),
            window: Some(NpcTradeWindow::default()) }),
    }
}

/// A remembered UI choice grants no access by itself: re-read the current durable actor,
/// current Steward skill, current House and exact live conversation before every stash request.
pub(in crate::gameplay_transport) async fn stash_access(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
) -> Option<[u8; 16]> {
    let fence = command.item_fence?;
    let (id, npc, context) = admission
        .states
        .npc_conversations
        .sessions
        .lock()
        .ok()
        .and_then(|states| {
            let state = states.get(&command.game_session_id)?.hireling.as_ref()?;
            state
                .stash
                .then(|| (state.snapshot.id, state.npc, state.context.clone()))
        })?;
    let Ok(Some(snapshot)) = admission
        .root
        .load_house_hireling_service(
            admission.character,
            admission.holder,
            fence,
            *admission.channel_id.as_bytes(),
            id,
        )
        .await
    else {
        return None;
    };
    if !snapshot.has_skill(Skill::Steward as u16) {
        return None;
    }
    let runtime = admission.runtime.lock().await;
    (admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        &runtime,
        actor,
        command.game_session_id,
        &NpcTalkIntent {
            npc_actor: npc,
            text: String::new(),
        },
    ) == Some(context))
    .then_some(id)
}

pub(in crate::gameplay_transport) async fn resume(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    intent: &NpcTalkIntent,
) -> Option<NpcTalkOutcome> {
    purchase::resume(admission, command, intent).await
}
