use super::*;
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
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
    let book = book();
    let say = |text: &str, micros| {
        conversations.talk(
            Some(&book),
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
        Some(&book()),
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
        Some(&book()),
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
    let book = book();
    conversations.talk(
        Some(&book),
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
        Some(&book),
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
    let book = book();
    let say = |micros| {
        conversations
            .talk(
                Some(&book),
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
    assert_eq!(
        conversations
            .talk(
                None,
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
                Some(&book()),
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
