use std::sync::Arc;

use super::*;
use crate::content::item_ref::ItemDefinitionIndex;
use crate::content::{ProjectV2DefinitionRef, ProjectV2Family};
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::NpcTalkBook;
use crate::gameplay_transport::actor_spell::tests::runtime_with_capacity;
use crate::gameplay_transport::npc_talk::NpcTalkEntry;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const BASE: u64 = 10_000_000;

fn at(micros: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(micros)
}

/// A player at the entry tile and an NPC `distance` tiles east of it.
fn fixture(distance: u32) -> (ChannelRuntimeV1, ExactActorRef, GameSessionId, EntityRef) {
    let (mut runtime, actor, session) = runtime_with_capacity(0x71, 3);
    runtime
        .initialize_first_entry_position(actor)
        .expect("position");
    let spot = runtime
        .read_actor_position(actor)
        .expect("position")
        .position();
    let table = NpcPlacementTable {
        npcs: vec![NpcPlacements {
            key: "oteryn:npc.alice".to_owned(),
            placements: vec![NpcPlacement {
                floor: -i8::try_from(spot.floor).expect("floor"),
                y: u16::try_from(spot.y).expect("y"),
                x: u16::try_from(spot.x + i32::try_from(distance).expect("distance")).expect("x"),
                direction: MovementFacing::South,
            }],
        }],
    };
    runtime.place_npcs(&table, |_| true, true).expect("placed");
    let npc = runtime.visible_entities().npcs[0];
    let reference = EntityRef {
        identity: npc.actor.placement_identity(),
        generation: npc.generation,
    };
    (runtime, actor, session, reference)
}

fn book() -> NpcTalkBook {
    NpcTalkBook::default().with(
        "oteryn:npc.alice",
        NpcTalkEntry::new("Alice", Some("Alice buys and sells goods.")),
    )
}

/// The chat state of the session: admitted, serving capability 3, with the NPC book installed.
fn chat_of(session: GameSessionId) -> ChatRuntime {
    let chat = ChatRuntime::default();
    chat.admit(session, "Aela", at(BASE));
    chat.set_npc_service(session);
    chat.install_npc_talk(Arc::new(book()));
    chat
}

fn intent(npc_actor: EntityRef, text: &str) -> NpcTalkIntent {
    NpcTalkIntent {
        npc_actor,
        text: text.to_owned(),
    }
}

#[test]
fn a_conversation_opens_answers_and_closes() {
    let (runtime, actor, session, npc) = fixture(2);
    let conversations = NpcConversations::default();
    let chat = chat_of(session);
    let say = |text: &str, micros| {
        conversations.talk(
            &chat,
            &runtime,
            actor,
            session,
            &intent(npc, text),
            at(micros),
        )
    };
    let greeted = say("hello", BASE);
    assert_eq!(greeted.disposition, NpcIntentDisposition::Ok);
    let conversation = greeted.conversation.expect("payload");
    assert_eq!(conversation.open, Some((npc, NpcConversationState::Open)));
    assert_eq!(
        conversation.lines,
        vec!["Greetings, I am Alice.".to_owned()]
    );
    let topic = say("job", BASE + 300_000);
    assert_eq!(
        topic.conversation.expect("payload").lines,
        vec!["Alice buys and sells goods.".to_owned()]
    );
    let farewell = say("bye", BASE + 600_000);
    let closed = farewell.conversation.expect("payload");
    assert_eq!(closed.open, None);
    assert_eq!(closed.lines, vec!["Farewell.".to_owned()]);
}

#[test]
fn a_topic_without_a_greeting_has_no_conversation() {
    let (runtime, actor, session, npc) = fixture(2);
    let outcome = NpcConversations::default().talk(
        &chat_of(session),
        &runtime,
        actor,
        session,
        &intent(npc, "trade"),
        at(BASE),
    );
    assert_eq!(outcome.disposition, NpcIntentDisposition::NoConversation);
    assert!(outcome.conversation.is_none());
}

#[test]
fn an_npc_out_of_range_is_not_in_range() {
    let (runtime, actor, session, npc) = fixture(TALK_RANGE_TILES + 1);
    let outcome = NpcConversations::default().talk(
        &chat_of(session),
        &runtime,
        actor,
        session,
        &intent(npc, "hello"),
        at(BASE),
    );
    assert_eq!(outcome.disposition, NpcIntentDisposition::NotInRange);
}

#[test]
fn a_stale_generation_is_stale_and_ends_the_conversation() {
    let (runtime, actor, session, npc) = fixture(2);
    let conversations = NpcConversations::default();
    let chat = chat_of(session);
    conversations.talk(
        &chat,
        &runtime,
        actor,
        session,
        &intent(npc, "hello"),
        at(BASE),
    );
    let stale = EntityRef {
        generation: npc.generation + 1,
        ..npc
    };
    let outcome = conversations.talk(
        &chat,
        &runtime,
        actor,
        session,
        &intent(stale, "job"),
        at(BASE + 1),
    );
    assert_eq!(outcome.disposition, NpcIntentDisposition::Stale);
    // The conversation was held with that NPC: it ends, with an empty domain 7 payload.
    assert_eq!(outcome.conversation, Some(NpcConversation::default()));
}

#[test]
fn the_fifth_intent_in_a_second_is_rate_limited_and_the_window_slides() {
    let (runtime, actor, session, npc) = fixture(2);
    let conversations = NpcConversations::default();
    let chat = chat_of(session);
    let say = |micros| {
        conversations
            .talk(
                &chat,
                &runtime,
                actor,
                session,
                &intent(npc, "hello"),
                at(micros),
            )
            .disposition
    };
    for index in 0..u64::from(MAX_TALK_INTENTS_PER_SECOND) {
        assert_eq!(say(BASE + index), NpcIntentDisposition::Ok);
    }
    assert_eq!(say(BASE + 10), NpcIntentDisposition::RateLimited);
    assert_eq!(
        say(BASE + RATE_WINDOW_MICROS + 10),
        NpcIntentDisposition::Ok
    );
}

#[test]
fn no_book_or_empty_text_is_rejected() {
    let (runtime, actor, session, npc) = fixture(2);
    let conversations = NpcConversations::default();
    let bare = ChatRuntime::default();
    bare.admit(session, "Aela", at(BASE));
    bare.set_npc_service(session);
    assert_eq!(
        conversations
            .talk(
                &bare,
                &runtime,
                actor,
                session,
                &intent(npc, "hello"),
                at(BASE)
            )
            .disposition,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(
        conversations
            .talk(
                &chat_of(session),
                &runtime,
                actor,
                session,
                &intent(npc, ""),
                at(BASE + RATE_WINDOW_MICROS)
            )
            .disposition,
        NpcIntentDisposition::Rejected
    );
}

#[test]
fn a_chat_greeting_and_a_command_share_one_conversation() {
    let (runtime, actor, session, npc) = fixture(2);
    let chat = chat_of(session);
    let conversations = NpcConversations::default();
    // As if a chat `say` had greeted the NPC.
    let npc_actor = runtime.visible_entities().npcs[0].actor;
    chat.converse(session, |engaged| *engaged = Some(npc_actor))
        .expect("session");
    let topic = conversations.talk(
        &chat,
        &runtime,
        actor,
        session,
        &intent(npc, "job"),
        at(BASE + 300_000),
    );
    assert_eq!(topic.disposition, NpcIntentDisposition::Ok);
    let shown = conversations.follow(&chat, &runtime, actor, session, None);
    let shown = shown.expect("payload").conversation.expect("conversation");
    assert_eq!(shown.open.map(|(n, _)| n), Some(npc));
}

#[test]
fn walking_out_of_range_closes_the_conversation_with_the_farewell() {
    let (runtime, actor, session, npc) = fixture(TALK_RANGE_TILES + 1);
    let chat = chat_of(session);
    let conversations = NpcConversations::default();
    let npc_actor = runtime.visible_entities().npcs[0].actor;
    chat.converse(session, |engaged| *engaged = Some(npc_actor))
        .expect("session");
    let closed = conversations.follow(&chat, &runtime, actor, session, Some(npc));
    let closed = closed.expect("payload").conversation.expect("conversation");
    assert_eq!(closed.open, None);
    assert_eq!(closed.lines, vec!["Farewell.".to_owned()]);
    assert!(chat.engaged(session).is_none());
}

#[test]
fn forgetting_a_session_drops_its_rate_window() {
    let (runtime, actor, session, npc) = fixture(2);
    let chat = chat_of(session);
    let conversations = NpcConversations::default();
    conversations.talk(
        &chat,
        &runtime,
        actor,
        session,
        &intent(npc, "hello"),
        at(BASE),
    );
    conversations.forget(session);
    assert!(conversations.sessions.lock().expect("lock").is_empty());
}

#[test]
fn a_malformed_intent_spends_the_rate_allowance() {
    let (runtime, actor, session, npc) = fixture(2);
    let chat = chat_of(session);
    let conversations = NpcConversations::default();
    let malformed = intent(npc, "");
    for _ in 0..MAX_TALK_INTENTS_PER_SECOND {
        let outcome = conversations.talk(&chat, &runtime, actor, session, &malformed, at(BASE));
        assert_eq!(outcome.disposition, NpcIntentDisposition::Rejected);
    }
    let outcome = conversations.talk(&chat, &runtime, actor, session, &malformed, at(BASE));
    assert_eq!(outcome.disposition, NpcIntentDisposition::RateLimited);
}

#[test]
fn a_placed_npc_without_a_talk_entry_is_silent() {
    let (runtime, actor, session, npc) = fixture(2);
    let chat = ChatRuntime::default();
    chat.admit(session, "Aela", at(BASE));
    chat.set_npc_service(session);
    chat.install_npc_talk(Arc::new(NpcTalkBook::default()));
    let outcome = NpcConversations::default().talk(
        &chat,
        &runtime,
        actor,
        session,
        &intent(npc, "hello"),
        at(BASE),
    );
    assert_eq!(outcome.disposition, NpcIntentDisposition::NoConversation);
    assert!(outcome.conversation.is_none());
}

#[test]
fn a_command_from_out_of_range_closes_with_the_farewell() {
    let (runtime, actor, session, npc) = fixture(TALK_RANGE_TILES + 1);
    let chat = chat_of(session);
    let npc_actor = runtime.visible_entities().npcs[0].actor;
    chat.converse(session, |engaged| *engaged = Some(npc_actor))
        .expect("session");
    let outcome = NpcConversations::default().talk(
        &chat,
        &runtime,
        actor,
        session,
        &intent(npc, "job"),
        at(BASE),
    );
    assert_eq!(outcome.disposition, NpcIntentDisposition::NotInRange);
    let closed = outcome.conversation.expect("payload");
    assert_eq!(closed.open, None);
    assert_eq!(closed.lines, vec!["Farewell.".to_owned()]);
    assert!(chat.engaged(session).is_none());
}

#[test]
fn an_unanswered_command_to_another_npc_keeps_the_current_conversation() {
    let (runtime, actor, session, npc) = fixture(2);
    let chat = chat_of(session);
    // Engaged with some other NPC than the one the command names.
    chat.converse(session, |engaged| *engaged = Some(actor))
        .expect("session");
    let outcome = NpcConversations::default().talk(
        &chat,
        &runtime,
        actor,
        session,
        &intent(npc, "job"),
        at(BASE),
    );
    assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
    assert!(outcome.conversation.is_none());
    assert_eq!(chat.engaged(session), Some(actor));
}

const SWORD: &str = "oteryn:item.tibia.sword";
const ROPE: &str = "oteryn:item.tibia.rope";

fn items() -> ItemDefinitionIndex {
    ItemDefinitionIndex::decode(
        &serde_json::to_vec(&serde_json::json!({
            "schema": crate::content::item_ref::ITEM_KEYS_SCHEMA,
            "records": [[ROPE, "definition-r1"], [SWORD, "definition-r1"]],
        }))
        .expect("json"),
    )
    .expect("index")
}

fn offer(
    key: &str,
    revision: &str,
    direction: ProjectV2ServiceOfferDirection,
    unit_price: u64,
    count: Option<u32>,
) -> ProjectV2ServiceOffer {
    ProjectV2ServiceOffer {
        item: ProjectV2DefinitionRef {
            family: ProjectV2Family::Item,
            key: key.to_owned(),
            revision: revision.to_owned(),
        },
        direction,
        unit_price,
        currency: None,
        count,
        sub_type: None,
        parity_pending: false,
    }
}

fn trader_chat(session: GameSessionId, offers: Vec<ProjectV2ServiceOffer>) -> ChatRuntime {
    let chat = ChatRuntime::default();
    chat.admit(session, "Aela", at(BASE));
    chat.set_npc_service(session);
    chat.install_npc_talk(Arc::new(NpcTalkBook::default().with(
        "oteryn:npc.alice",
        NpcTalkEntry::new("Alice", Some("Alice trades.")).with_offers(offers),
    )));
    chat
}

#[test]
fn offers_project_in_order_with_the_player_side_and_drop_what_does_not_resolve() {
    use ProjectV2ServiceOfferDirection::{BuyFromPlayer, SellToPlayer};
    let projected = project_offers(
        &[
            offer(SWORD, "definition-r1", SellToPlayer, 500, None),
            offer(ROPE, "definition-r1", BuyFromPlayer, 15, Some(2)),
            offer(
                "oteryn:item.tibia.unknown",
                "definition-r1",
                SellToPlayer,
                1,
                None,
            ),
            offer(ROPE, "definition-r9", SellToPlayer, 1, None),
            offer(
                ROPE,
                "definition-r1",
                SellToPlayer,
                u64::from(u32::MAX) + 1,
                None,
            ),
        ],
        &items(),
    );
    assert_eq!(
        projected,
        vec![
            NpcOffer {
                item_definition_ref: 2,
                side: NpcTradeSide::Buy,
                count: 1,
                unit_price: 500
            },
            NpcOffer {
                item_definition_ref: 1,
                side: NpcTradeSide::Sell,
                count: 2,
                unit_price: 15
            },
        ]
    );
}

#[test]
fn trade_opens_the_window_for_the_speaker_and_a_farewell_closes_it() {
    let (runtime, actor, session, npc) = fixture(2);
    let conversations = NpcConversations::default();
    let chat = trader_chat(
        session,
        vec![offer(
            SWORD,
            "definition-r1",
            ProjectV2ServiceOfferDirection::SellToPlayer,
            500,
            None,
        )],
    );
    let index = items();
    let say = |text: &str, micros| {
        conversations.talk_in(
            &chat,
            &runtime,
            actor,
            session,
            &intent(npc, text),
            at(micros),
            Some(&index),
        )
    };
    assert!(say("hello", BASE).window.is_none());
    let opened = say("trade", BASE + 300_000).window.expect("window");
    assert_eq!(
        opened.open,
        Some((npc, chat.npc_talk().expect("book").catalogue_revision()))
    );
    assert_eq!(opened.offers.len(), 1);
    assert!(say("job", BASE + 600_000).window.is_none());
    let closed = say("bye", BASE + 900_000).window.expect("closed");
    assert_eq!(closed, NpcTradeWindow::default());
}

#[test]
fn no_window_without_offers_or_a_pinned_index() {
    let (runtime, actor, session, npc) = fixture(2);
    let conversations = NpcConversations::default();
    let index = items();
    let bare = trader_chat(session, Vec::new());
    for (chat, items, micros) in [(&bare, Some(&index), BASE)] {
        conversations.talk_in(
            chat,
            &runtime,
            actor,
            session,
            &intent(npc, "hello"),
            at(micros),
            items,
        );
        let outcome = conversations.talk_in(
            chat,
            &runtime,
            actor,
            session,
            &intent(npc, "trade"),
            at(micros + 300_000),
            items,
        );
        assert!(outcome.window.is_none());
    }
    let stocked = trader_chat(
        session,
        vec![offer(
            SWORD,
            "definition-r1",
            ProjectV2ServiceOfferDirection::SellToPlayer,
            500,
            None,
        )],
    );
    let other = NpcConversations::default();
    other.talk_in(
        &stocked,
        &runtime,
        actor,
        session,
        &intent(npc, "hello"),
        at(BASE),
        None,
    );
    let outcome = other.talk_in(
        &stocked,
        &runtime,
        actor,
        session,
        &intent(npc, "trade"),
        at(BASE + 300_000),
        None,
    );
    assert!(outcome.window.is_none());
}

#[test]
fn walking_away_closes_the_window_with_the_conversation() {
    let (runtime, actor, session, npc) = fixture(2);
    let conversations = NpcConversations::default();
    let chat = trader_chat(
        session,
        vec![offer(
            SWORD,
            "definition-r1",
            ProjectV2ServiceOfferDirection::SellToPlayer,
            500,
            None,
        )],
    );
    let index = items();
    for (text, micros) in [("hello", BASE), ("trade", BASE + 300_000)] {
        conversations.talk_in(
            &chat,
            &runtime,
            actor,
            session,
            &intent(npc, text),
            at(micros),
            Some(&index),
        );
    }
    // The follow tick sees the conversation intact, so nothing changes.
    assert!(
        conversations
            .follow(&chat, &runtime, actor, session, Some(npc))
            .is_none()
    );
    chat.end_engagement(session, runtime.visible_entities().npcs[0].actor);
    let update = conversations
        .follow(&chat, &runtime, actor, session, Some(npc))
        .expect("update");
    assert_eq!(update.window, Some(NpcTradeWindow::default()));
    assert!(
        conversations
            .follow(&chat, &runtime, actor, session, None)
            .is_none()
    );
}
